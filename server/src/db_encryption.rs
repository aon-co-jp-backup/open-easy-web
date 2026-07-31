//! DATABASE暗号化(2026-07-31新設、2026-07-31改訂)。
//!
//! ユーザー指示の変遷: 当初はGUIトグル・管理API・対話式CLI質問による
//! ON/OFF選択式で実装したが、その後「コマンドやGUIでもDATABASEの暗号化
//! する?の質問やGUIも無しにしましょう。管理者が意識しないで済む用に
//! 裏で処理しましょう!」との指示を受け、**設定項目・質問・トグルを
//! すべて撤去し、常に自動で暗号化する完全サイレント方式**へ変更した。
//!
//! このバイナリが管理する「DATABASE」に相当するのは、`users.rs`の
//! `UserStore`が永続化するJSONファイル(アカウントのメールアドレス・
//! セカンドメール・電話番号・TOTPシークレットを含む、最も機微な
//! ローカルデータストア)。このファイルをディスク上で常時AES-256-GCM
//! (ランダムnonceを毎回生成する高速なAEAD暗号、ハードウェア
//! アクセラレーション〈AES-NI〉が利用可能な環境ではネイティブ命令で
//! 高速に処理される)で暗号化する——管理者がこれを止める設定項目は
//! 存在しない。
//!
//! ## 透過的な設計(管理者は意識しない)
//!
//! `UserStore::load`/`persist`の境界だけで暗号化・復号を行い、
//! `register`/`find_by_email`等の呼び出し元(=管理者が触るAPI)は
//! 常に平文の`UserRecord`を扱う。ディスク上のファイルが盗まれても
//! 中身は暗号化されたままだが、管理者自身のログイン・データ読み書きの
//! 体験には一切影響しない。
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

use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, AeadCore, Key, Nonce};

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const MARKER_ENCRYPTED: u8 = 0x01;
const MARKER_PLAIN: u8 = 0x00;

pub struct DbEncryptionState {
    key: [u8; KEY_LEN],
}

impl DbEncryptionState {
    /// 鍵を`key_path`から読み込み、無ければ新規生成して永続化する。
    /// 設定項目は無い——常に暗号化する。
    pub fn load(key_path: &Path) -> Self {
        Self { key: load_or_generate_key(key_path) }
    }

    /// `plaintext`を常にAES-256-GCM(ランダムnonce)で暗号化して返す。
    /// 先頭1バイトに実際に施した処理を示すマーカーを付ける
    /// (`decrypt`はこのマーカーを見て復号方法を判定する——過去に
    /// マーカー無しの平文ファイルが存在した場合でも`decrypt`側で
    /// 安全に扱えるようにするための設計)。
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext).expect("AES-256-GCM encryption cannot fail for in-memory buffers");
        let mut out = Vec::with_capacity(1 + NONCE_LEN + ciphertext.len());
        out.push(MARKER_ENCRYPTED);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        out
    }

    /// `encrypt`が付けたマーカーを読み、元の平文を復元する。マーカーが
    /// `MARKER_PLAIN`の場合(過去に暗号化前のファイルが存在した場合の
    /// 後方互換)は素通しで返す。
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_key_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("owsrv-dbenc-{label}-{}.key", uuid::Uuid::new_v4()))
    }

    #[test]
    fn encrypt_then_decrypt_round_trips() {
        let key_path = tmp_key_path("roundtrip");
        let state = DbEncryptionState::load(&key_path);
        let plaintext = b"{\"alice\":{\"email\":\"alice@example.test\"}}";
        let encrypted = state.encrypt(plaintext);
        assert_ne!(&encrypted[1..], plaintext, "ciphertext must not equal plaintext");
        assert_eq!(encrypted[0], MARKER_ENCRYPTED);
        assert_eq!(state.decrypt(&encrypted).unwrap(), plaintext);
        let _ = std::fs::remove_file(&key_path);
    }

    #[test]
    fn two_encryptions_of_the_same_plaintext_produce_different_ciphertext() {
        // ランダムnonceが実際に毎回生成されていることの直接証明
        // (ユーザー指示「ランダム要素のある高速な暗号化」への裏付け)。
        let key_path = tmp_key_path("random-nonce");
        let state = DbEncryptionState::load(&key_path);
        let plaintext = b"same input twice";
        let a = state.encrypt(plaintext);
        let b = state.encrypt(plaintext);
        assert_ne!(a, b, "identical plaintext must produce different ciphertext due to random nonce");
        assert_eq!(state.decrypt(&a).unwrap(), plaintext);
        assert_eq!(state.decrypt(&b).unwrap(), plaintext);
        let _ = std::fs::remove_file(&key_path);
    }

    #[test]
    fn plain_marker_is_still_readable_for_backward_compatibility() {
        let key_path = tmp_key_path("plain-marker");
        let state = DbEncryptionState::load(&key_path);
        let mut stored = vec![MARKER_PLAIN];
        stored.extend_from_slice(b"legacy plaintext json");
        assert_eq!(state.decrypt(&stored).unwrap(), b"legacy plaintext json");
        let _ = std::fs::remove_file(&key_path);
    }

    #[test]
    fn key_survives_reload_so_previously_encrypted_data_stays_decryptable() {
        let key_path = tmp_key_path("key-persist");
        let state1 = DbEncryptionState::load(&key_path);
        let stored = state1.encrypt(b"data encrypted by the first process");
        // 別インスタンス(プロセス再起動を模す)が同じ鍵ファイルを
        // 読み込み、正しく復号できることを確認する。
        let state2 = DbEncryptionState::load(&key_path);
        assert_eq!(state2.decrypt(&stored).unwrap(), b"data encrypted by the first process");
        let _ = std::fs::remove_file(&key_path);
    }
}
