//! DATABASE暗号化(2026-07-31新設、ユーザー指示「open-easy-webで管理する
//! 機能でDATABASEを暗号化するON と OFFをGUIでデフォルトはONにして選択可能に」
//! +「コマンドベースでもセッティングしているとAIが判断したらDATABASEは
//! 暗号化しますかYes OR Noなどで英語と日本語で質問して対応する機能を付けて
//! GUIならON OR OFFで」)。
//!
//! このバイナリが管理する「DATABASE」に相当するのは、`users.rs`の
//! `UserStore`が永続化するJSONファイル(アカウントのメールアドレス・
//! セカンドメール・電話番号・TOTPシークレットを含む、最も機微な
//! ローカルデータストア)。このファイルをディスク上でAES-256-GCM
//! (ランダムnonceを毎回生成する高速なAEAD暗号、ハードウェア
//! アクセラレーション〈AES-NI〉が利用可能な環境ではネイティブ命令で
//! 高速に処理される)で暗号化する。
//!
//! ## 設定方法(3通り、`auto_update.rs`と同じ優先順位パターン)
//!
//! 1. **既定値**: 有効(ON)。
//! 2. **環境変数** `OPEN_EASYWEB_DB_ENCRYPTION=false`(初回起動時のみの
//!    初期値として使われる)。
//! 3. **GUI**(`POST /admin/db-encryption`、`x-admin-token`認証)または
//!    **対話式コマンドライン**(初回起動かつ端末が対話的〈TTY〉な場合、
//!    英語・日本語併記で"Encrypt the DATABASE? (DATABASEを暗号化
//!    しますか?) [Y/n]"と尋ねる)——いずれも一度設定されると
//!    `.open-easy-web-db-encryption.json`へ永続化され、以後は環境変数
//!    より優先される(再起動しても保持)。
//!
//! ## ワイヤーフォーマット(ON/OFF切り替えを安全にするための設計)
//!
//! 永続化ファイルの先頭1バイトを「このファイル自体が暗号化されているか」
//! のマーカーとして使う(`0x01`=暗号化済み、`0x00`=平文)——**現在の
//! 設定値ではなく、実際にそのファイルを書いた時点の状態で復号方法を
//! 判定する**。これにより、暗号化ONの状態で書かれたファイルを後から
//! OFFにしても正しく読める(そのファイル自体は暗号化されたまま残るが、
//! 次に書き込まれた時点で新しい設定に従って書き直される)、逆にOFFの
//! 状態で書かれた平文ファイルをONにした後でも正しく読める、という
//! 安全な移行を保証する。
//!
//! ## 鍵管理(正直な開示)
//!
//! 鍵(32バイト、`OsRng`でCSPRNG生成)はキーファイル
//! (既定`.open-easy-web-db-encryption.key`)に平文で保存する——
//! これは`KeyGuardian`(open-web-server側)がAPIキーをハッシュのみ
//! 保存するのとは異なり、対称暗号の性質上、復号のたびに鍵原文が
//! 必要なため不可避のトレードオフである。**最終的な防衛線はホストOS
//! 自体のファイルシステム権限(誰がこのファイルを読めるか)**であり、
//! このキーファイルへのアクセス制御自体を強化する仕組み(HSM・
//! KMS等)は今回のスコープ外として正直に明記する。

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, AeadCore, Key, Nonce};
use serde::{Deserialize, Serialize};

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const MARKER_ENCRYPTED: u8 = 0x01;
const MARKER_PLAIN: u8 = 0x00;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedSettings {
    enabled: bool,
}

pub struct DbEncryptionState {
    settings_path: PathBuf,
    enabled: RwLock<bool>,
    key: [u8; KEY_LEN],
}

impl DbEncryptionState {
    /// `settings_path`に既存の永続化ファイルがあればそれを使い、無ければ
    /// `OPEN_EASYWEB_DB_ENCRYPTION`環境変数(既定`true`=ON)を初期値と
    /// する。鍵は`key_path`から読み込み、無ければ新規生成して永続化する。
    pub fn load(settings_path: PathBuf, key_path: &Path) -> Self {
        let enabled = match std::fs::read(&settings_path) {
            Ok(bytes) => serde_json::from_slice::<PersistedSettings>(&bytes).map(|s| s.enabled).unwrap_or(true),
            Err(_) => std::env::var("OPEN_EASYWEB_DB_ENCRYPTION").map(|v| v != "false" && v != "0").unwrap_or(true),
        };
        let key = load_or_generate_key(key_path);
        Self { settings_path, enabled: RwLock::new(enabled), key }
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read().unwrap()
    }

    /// GUI/HTTP API/対話式コマンドライン経由での設定変更。ディスクへ
    /// 永続化するため、プロセス再起動後も設定が保持される。
    pub fn set_enabled(&self, enabled: bool) -> std::io::Result<()> {
        *self.enabled.write().unwrap() = enabled;
        if let Some(parent) = self.settings_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let bytes = serde_json::to_vec_pretty(&PersistedSettings { enabled })
            .expect("PersistedSettings serialization is infallible");
        std::fs::write(&self.settings_path, bytes)
    }

    /// 現在の設定に従い、`plaintext`を暗号化(有効時)または素通し(無効時)
    /// して返す。先頭1バイトに実際に施した処理を示すマーカーを付ける
    /// (`decrypt`はこのマーカーを見て復号方法を判定する——現在の設定
    /// フラグではなくファイル自体の記録に従うことで、ON/OFF切り替え
    /// 後も過去に書かれたファイルを安全に読める)。
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        if !self.is_enabled() {
            let mut out = Vec::with_capacity(plaintext.len() + 1);
            out.push(MARKER_PLAIN);
            out.extend_from_slice(plaintext);
            return out;
        }
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext).expect("AES-256-GCM encryption cannot fail for in-memory buffers");
        let mut out = Vec::with_capacity(1 + NONCE_LEN + ciphertext.len());
        out.push(MARKER_ENCRYPTED);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        out
    }

    /// `encrypt`が付けたマーカーを読み、元の平文を復元する。
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let Some((&marker, rest)) = data.split_first() else {
            return Err("persisted database file is empty".to_string());
        };
        match marker {
            MARKER_PLAIN => Ok(rest.to_vec()),
            MARKER_ENCRYPTED => {
                if rest.len() < NONCE_LEN {
                    return Err("persisted database file is truncated (missing nonce)".to_string());
                }
                let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);
                let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
                cipher.decrypt(Nonce::from_slice(nonce_bytes), ciphertext).map_err(|e| format!("failed to decrypt persisted database: {e}"))
            }
            other => Err(format!("unknown database encryption marker byte: {other:#04x}")),
        }
    }
}

fn load_or_generate_key(key_path: &Path) -> [u8; KEY_LEN] {
    if let Ok(bytes) = std::fs::read(key_path) {
        if bytes.len() == KEY_LEN {
            let mut key = [0u8; KEY_LEN];
            key.copy_from_slice(&bytes);
            return key;
        }
        tracing::warn!(path = %key_path.display(), "db_encryption: existing key file has unexpected length, regenerating");
    }
    let key: [u8; KEY_LEN] = Aes256Gcm::generate_key(&mut OsRng).into();
    if let Some(parent) = key_path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    if let Err(e) = std::fs::write(key_path, key) {
        tracing::warn!(error = %e, path = %key_path.display(), "db_encryption: failed to persist encryption key, using an in-memory-only key for this process lifetime");
    }
    key
}

/// 初回起動(=永続化された設定ファイルがまだ無い)かつ標準入力が対話的な
/// 端末(TTY)である場合のみ、英語・日本語併記でYes/No質問を行う
/// (`std::io::IsTerminal`、外部crate〈`atty`等〉非依存)。非対話的な
/// 環境(systemdサービス・CI・パイプ経由の起動等)では何も尋ねず、
/// 既定値(環境変数またはON)をそのまま使う——サービス起動をブロック
/// しないための安全側の判断。
///
/// 戻り値: 実際にユーザーへ質問し回答を得て設定を永続化した場合は
/// `true`(呼び出し側はログ等で通知してよい)、質問しなかった場合は
/// `false`。
pub fn maybe_prompt_interactive_setup(settings_path: &Path, state: &DbEncryptionState) -> bool {
    if settings_path.exists() {
        return false;
    }
    if !std::io::stdin().is_terminal() {
        return false;
    }
    println!("Encrypt the DATABASE at rest? (DATABASEを暗号化しますか?) [Y/n]: ");
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    let answer = input.trim().to_ascii_lowercase();
    // 空入力(単にEnter)はデフォルトのYesとして扱う(既定ONの方針と一致)。
    let enabled = !(answer == "n" || answer == "no" || answer == "いいえ");
    match state.set_enabled(enabled) {
        Ok(()) => {
            println!(
                "{}",
                if enabled {
                    "✅ DATABASE encryption enabled. (DATABASE暗号化を有効にしました。)"
                } else {
                    "⚠️ DATABASE encryption disabled. (DATABASE暗号化を無効にしました。)"
                }
            );
        }
        Err(e) => {
            eprintln!("⚠️ failed to persist database encryption setting: {e} (設定の保存に失敗しました)");
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_paths(label: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("owsrv-dbenc-{label}-{}", uuid::Uuid::new_v4()));
        (dir.join("settings.json"), dir.join("key.bin"))
    }

    #[test]
    fn defaults_to_enabled_when_no_settings_file_and_no_env_var() {
        let (settings, key) = tmp_paths("default-on");
        // このテストと並行実行される他テストが同名の環境変数を触らない
        // よう、明示的にunsetした状態で確認する。
        unsafe {
            std::env::remove_var("OPEN_EASYWEB_DB_ENCRYPTION");
        }
        let state = DbEncryptionState::load(settings, &key);
        assert!(state.is_enabled(), "database encryption must default to ON");
        let _ = std::fs::remove_file(&key);
    }

    #[test]
    fn encrypt_then_decrypt_round_trips_when_enabled() {
        let (settings, key) = tmp_paths("roundtrip-on");
        let state = DbEncryptionState::load(settings, &key);
        state.set_enabled(true).unwrap();
        let plaintext = b"{\"alice\":{\"email\":\"alice@example.test\"}}";
        let encrypted = state.encrypt(plaintext);
        assert_ne!(&encrypted[1..], plaintext, "ciphertext must not equal plaintext");
        assert_eq!(encrypted[0], MARKER_ENCRYPTED);
        let decrypted = state.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
        let _ = std::fs::remove_file(&key);
    }

    #[test]
    fn two_encryptions_of_the_same_plaintext_produce_different_ciphertext() {
        // ランダムnonceが実際に毎回生成されていることの直接証明
        // (ユーザー指示「ランダム要素のある高速な暗号化」への裏付け)。
        let (settings, key) = tmp_paths("random-nonce");
        let state = DbEncryptionState::load(settings, &key);
        state.set_enabled(true).unwrap();
        let plaintext = b"same input twice";
        let a = state.encrypt(plaintext);
        let b = state.encrypt(plaintext);
        assert_ne!(a, b, "identical plaintext must produce different ciphertext due to random nonce");
        assert_eq!(state.decrypt(&a).unwrap(), plaintext);
        assert_eq!(state.decrypt(&b).unwrap(), plaintext);
        let _ = std::fs::remove_file(&key);
    }

    #[test]
    fn disabled_state_stores_plaintext_marker_and_round_trips() {
        let (settings, key) = tmp_paths("disabled");
        let state = DbEncryptionState::load(settings, &key);
        state.set_enabled(false).unwrap();
        let plaintext = b"plain json";
        let stored = state.encrypt(plaintext);
        assert_eq!(stored[0], MARKER_PLAIN);
        assert_eq!(&stored[1..], plaintext);
        assert_eq!(state.decrypt(&stored).unwrap(), plaintext);
        let _ = std::fs::remove_file(&key);
    }

    #[test]
    fn toggling_off_after_writing_encrypted_data_can_still_read_it_back() {
        // ON→OFFへ切り替えても、以前ONの時に書いたファイルは読める
        // (マーカーバイトが「現在の設定」ではなく「書いた時点の状態」を
        // 記録しているため)。
        let (settings, key) = tmp_paths("toggle-off-after-write");
        let state = DbEncryptionState::load(settings, &key);
        state.set_enabled(true).unwrap();
        let stored = state.encrypt(b"secret data written while ON");
        state.set_enabled(false).unwrap();
        assert_eq!(state.decrypt(&stored).unwrap(), b"secret data written while ON");
        let _ = std::fs::remove_file(&key);
    }

    #[test]
    fn setting_survives_reload_into_a_fresh_instance() {
        let (settings, key) = tmp_paths("persist-reload");
        let state1 = DbEncryptionState::load(settings.clone(), &key);
        state1.set_enabled(false).unwrap();
        let state2 = DbEncryptionState::load(settings, &key);
        assert!(!state2.is_enabled(), "disabled setting must survive reload from disk");
        let _ = std::fs::remove_file(&key);
    }

    #[test]
    fn key_survives_reload_so_previously_encrypted_data_stays_decryptable() {
        let (settings, key) = tmp_paths("key-persist");
        let state1 = DbEncryptionState::load(settings.clone(), &key);
        state1.set_enabled(true).unwrap();
        let stored = state1.encrypt(b"data encrypted by the first process");
        // 別インスタンス(プロセス再起動を模す)が同じ鍵ファイルを
        // 読み込み、正しく復号できることを確認する。
        let state2 = DbEncryptionState::load(settings, &key);
        assert_eq!(state2.decrypt(&stored).unwrap(), b"data encrypted by the first process");
        let _ = std::fs::remove_file(&key);
    }
}
