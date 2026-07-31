//! メールアドレスをIDとするユーザー登録台帳。
//!
//! 固定パスワードは持たない(認証は`auth.rs`のOTP)。バックアップ連絡先を
//! 必須とする——電話番号(SMS OTP用)を登録できるならそれを、
//! 電話番号が無い場合は代わりにセカンドメールアドレスを必須で登録する
//! (「最初のメールが使えなくなった場合に詰む」状態を避けるため)。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::db_encryption::DbEncryptionState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub email: String,
    /// SMS OTPのバックアップ連絡先。「なし」を選んだ場合は`None`——
    /// その場合は`backup_email`が必須になる。
    pub phone: Option<String>,
    /// `phone`が`None`の場合に必須のセカンドメール(OTP受信用のもう一つの経路)。
    pub backup_email: Option<String>,
    /// 認証アプリ(TOTP)2FAが有効な場合の共有シークレット(base32)。
    /// `pending_totp_secret`経由でセットアップ確認が完了した後にのみ
    /// ここへ移される——確認前の値をここに置くと、確認を経ずに
    /// 2FAが有効化されたことになってしまうため。
    #[serde(default)]
    pub totp_secret: Option<String>,
    /// セットアップ中(確認コード入力待ち)のシークレット。確認成功で
    /// `totp_secret`へ昇格し、ここはクリアされる。
    #[serde(default)]
    pub pending_totp_secret: Option<String>,
}

/// 主メール以外の変更可能な連絡先の種別(主メール自体は`rename_email`が
/// 別途担当——アカウントの識別子そのものであり、扱いが異なるため)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactField {
    Phone,
    BackupEmail,
}

impl ContactField {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "phone" => Some(Self::Phone),
            "backup_email" => Some(Self::BackupEmail),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum RegisterError {
    MissingBackupContact,
    AlreadyRegistered,
    InvalidEmail,
}

impl RegisterError {
    #[allow(dead_code)] // 公開登録エンドポイント廃止に伴い現在は未呼び出し(将来の管理UI向けに保持)。
    pub fn message_ja(&self) -> &'static str {
        match self {
            RegisterError::MissingBackupContact => {
                "電話番号を登録しない場合、セカンドメールアドレスの登録が必須です。"
            }
            RegisterError::AlreadyRegistered => "このメールアドレスは既に登録されています。",
            RegisterError::InvalidEmail => "メールアドレスの形式が不正です。",
        }
    }
    #[allow(dead_code)] // 公開登録エンドポイント廃止に伴い現在は未呼び出し(将来の管理UI向けに保持)。
    pub fn message_en(&self) -> &'static str {
        match self {
            RegisterError::MissingBackupContact => {
                "A second (backup) email address is required when no phone number is registered."
            }
            RegisterError::AlreadyRegistered => "This email address is already registered.",
            RegisterError::InvalidEmail => "The email address format is invalid.",
        }
    }
}

pub struct UserStore {
    state_path: PathBuf,
    users: Mutex<HashMap<String, UserRecord>>,
    /// ディスク永続化時のDATABASE暗号化(`db_encryption.rs`参照、
    /// 既定ON)。`enroll`/`persist`双方でこれを経由することで、
    /// このファイルが実際に管理する最も機微なデータ(メール・電話番号・
    /// TOTPシークレット)が平文でディスクに残らないようにする。
    encryption: Arc<DbEncryptionState>,
}

fn normalize(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

/// 電話番号の表記ゆれ(ハイフン・スペイン等)を吸収する。先頭の`+`は
/// 保持し、それ以外の非数字は取り除く(`090-1234-5678`と`09075555011`を
/// 同一視するため)。
pub fn normalize_phone(phone: &str) -> String {
    let phone = phone.trim();
    let mut out = String::new();
    for (i, c) in phone.chars().enumerate() {
        if c == '+' && i == 0 {
            out.push(c);
        } else if c.is_ascii_digit() {
            out.push(c);
        }
    }
    out
}

impl UserStore {
    /// **透過的な暗号化**: 呼び出し元(管理者・他モジュール)はこの後の
    /// `register`/`find_by_email`等を従来通り平文の`UserRecord`として
    /// 扱えばよく、ディスク上で暗号化されているかどうかを一切意識する
    /// 必要が無い(ユーザー指示、2026-07-31)。暗号化/復号は`load`/
    /// `persist`のこの境界だけで完結する——盗まれるリスクがあるのは
    /// 「ディスク上のファイル」であり、それだけを自動的に保護する設計。
    pub fn load(state_path: PathBuf, encryption: Arc<DbEncryptionState>) -> Self {
        let users = std::fs::read(&state_path)
            .ok()
            .and_then(|bytes| {
                match encryption.decrypt(&bytes) {
                    Ok(plaintext) => Some(plaintext),
                    Err(e) => {
                        // 本暗号化機能の導入前(マーカーバイトを一切
                        // 付けずに素のJSONを書いていた旧フォーマット)の
                        // ファイルを読み込んだ場合、`decrypt`は先頭バイトを
                        // 未知のマーカーとしてエラーを返す。既存の運用
                        // データを失わないよう、その場合はファイル全体を
                        // そのまま平文JSONとして再解釈するフォールバックを
                        // 用意する(次回`persist`で新フォーマットへ
                        // 自動的に書き直される、一度限りの移行パス)。
                        if serde_json::from_slice::<HashMap<String, UserRecord>>(&bytes).is_ok() {
                            tracing::info!("persisted user registry was in the pre-encryption plain format; migrating to encrypted format on next write");
                            Some(bytes)
                        } else {
                            tracing::warn!(error = %e, "failed to decrypt persisted user registry, starting empty");
                            None
                        }
                    }
                }
            })
            .and_then(|plaintext| serde_json::from_slice::<HashMap<String, UserRecord>>(&plaintext).ok())
            .unwrap_or_default();
        Self { state_path, users: Mutex::new(users), encryption }
    }

    fn persist(&self, users: &HashMap<String, UserRecord>) {
        if let Ok(json) = serde_json::to_vec_pretty(users) {
            let stored = self.encryption.encrypt(&json);
            if let Err(e) = std::fs::write(&self.state_path, stored) {
                tracing::warn!(error = %e, "failed to persist user registry");
            }
        }
    }

    pub fn register(
        &self,
        email: &str,
        phone: Option<String>,
        backup_email: Option<String>,
    ) -> Result<UserRecord, RegisterError> {
        if !email.contains('@') {
            return Err(RegisterError::InvalidEmail);
        }
        let phone = phone.filter(|p| !p.trim().is_empty()).map(|p| normalize_phone(&p));
        let backup_email = backup_email.filter(|e| !e.trim().is_empty());
        if phone.is_none() && backup_email.is_none() {
            return Err(RegisterError::MissingBackupContact);
        }

        let key = normalize(email);
        let mut users = self.users.lock().unwrap();
        if users.contains_key(&key) {
            return Err(RegisterError::AlreadyRegistered);
        }
        let record = UserRecord {
            email: email.to_string(),
            phone,
            backup_email,
            totp_secret: None,
            pending_totp_secret: None,
        };
        users.insert(key, record.clone());
        self.persist(&users);
        Ok(record)
    }

    pub fn exists(&self, email: &str) -> bool {
        self.users.lock().unwrap().contains_key(&normalize(email))
    }

    #[allow(dead_code)] // 将来のアカウント情報表示エンドポイント向けに公開している未使用API。
    pub fn find_by_email(&self, email: &str) -> Option<UserRecord> {
        self.users.lock().unwrap().get(&normalize(email)).cloned()
    }

    /// 登録済みの電話番号から、その持ち主のアカウントメールアドレスを引く
    /// (ハイフン等の表記ゆれは正規化して比較する)。
    pub fn find_email_by_phone(&self, phone: &str) -> Option<String> {
        let target = normalize_phone(phone);
        self.users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.phone.as_deref().map(normalize_phone) == Some(target.clone()))
            .map(|u| u.email.clone())
    }

    /// 登録済みのセカンドメールから、その持ち主のアカウントメールアドレスを引く。
    pub fn find_email_by_backup_email(&self, backup_email: &str) -> Option<String> {
        let target = normalize(backup_email);
        self.users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.backup_email.as_ref().map(|e| normalize(e)) == Some(target.clone()))
            .map(|u| u.email.clone())
    }

    /// アカウントの主メールアドレスを変更する(呼び出し元が
    /// `old_email`の所有者であることを別途確認済みである前提)。
    pub fn rename_email(&self, old_email: &str, new_email: &str) -> bool {
        let old_key = normalize(old_email);
        let new_key = normalize(new_email);
        let mut users = self.users.lock().unwrap();
        if users.contains_key(&new_key) {
            return false;
        }
        let Some(mut record) = users.remove(&old_key) else {
            return false;
        };
        record.email = new_email.to_string();
        users.insert(new_key, record);
        self.persist(&users);
        true
    }

    /// 電話番号・セカンドメールを変更する(呼び出し元が`account_email`の
    /// 所有者であることを別途確認済みである前提——主メール変更は
    /// `rename_email`が別途担当する)。`field`は`"phone"`または
    /// `"backup_email"`。
    pub fn update_contact(&self, account_email: &str, field: ContactField, new_value: &str) -> bool {
        let key = normalize(account_email);
        let mut users = self.users.lock().unwrap();
        let Some(record) = users.get_mut(&key) else {
            return false;
        };
        match field {
            ContactField::Phone => record.phone = Some(normalize_phone(new_value)),
            ContactField::BackupEmail => record.backup_email = Some(new_value.to_string()),
        }
        self.persist(&users);
        true
    }

    /// TOTP2FAが有効かどうか(セットアップ完了済みの`totp_secret`があるか)。
    pub fn totp_enabled(&self, account_email: &str) -> bool {
        self.users
            .lock()
            .unwrap()
            .get(&normalize(account_email))
            .is_some_and(|r| r.totp_secret.is_some())
    }

    /// 有効化済みのTOTPシークレットを取得する(コード検証用)。
    pub fn totp_secret(&self, account_email: &str) -> Option<String> {
        self.users
            .lock()
            .unwrap()
            .get(&normalize(account_email))
            .and_then(|r| r.totp_secret.clone())
    }

    /// TOTPセットアップを開始する。確認前の`pending_totp_secret`として保存し、
    /// 呼び出し元(APIハンドラ)がQRコード用のprovisioning URIを組み立てる。
    pub fn begin_totp_setup(&self, account_email: &str, secret_b32: &str) -> bool {
        let key = normalize(account_email);
        let mut users = self.users.lock().unwrap();
        let Some(record) = users.get_mut(&key) else { return false };
        record.pending_totp_secret = Some(secret_b32.to_string());
        self.persist(&users);
        true
    }

    /// セットアップ中のシークレットを取得する(確認コード検証用)。
    pub fn pending_totp_secret(&self, account_email: &str) -> Option<String> {
        self.users
            .lock()
            .unwrap()
            .get(&normalize(account_email))
            .and_then(|r| r.pending_totp_secret.clone())
    }

    /// 確認コードが正しかった場合に呼ぶ——pendingを本採用へ昇格させる。
    pub fn confirm_totp_setup(&self, account_email: &str) -> bool {
        let key = normalize(account_email);
        let mut users = self.users.lock().unwrap();
        let Some(record) = users.get_mut(&key) else { return false };
        let Some(pending) = record.pending_totp_secret.take() else { return false };
        record.totp_secret = Some(pending);
        self.persist(&users);
        true
    }

    /// TOTP2FAを無効化する(本採用・セットアップ中の両方をクリア)。
    pub fn disable_totp(&self, account_email: &str) -> bool {
        let key = normalize(account_email);
        let mut users = self.users.lock().unwrap();
        let Some(record) = users.get_mut(&key) else { return false };
        record.totp_secret = None;
        record.pending_totp_secret = None;
        self.persist(&users);
        true
    }

    /// 起動時に固定アカウントを保証する(冪等)。既に`email`が登録済みなら
    /// 何もしない——毎回上書きすると、UI経由の連絡先変更(`update_contact`/
    /// `rename_email`)が次回起動でリセットされてしまうため。
    /// 「このメールアドレス以外ではログインできないようにする」ための
    /// 唯一の登録経路として使う(公開の`register`エンドポイントは別途
    /// 無効化すること)。
    pub fn seed_fixed_account(&self, email: &str, phone: Option<&str>, backup_email: Option<&str>) {
        if self.exists(email) {
            return;
        }
        match self.register(email, phone.map(str::to_string), backup_email.map(str::to_string)) {
            Ok(_) => tracing::info!(email, "seeded fixed account"),
            Err(e) => tracing::error!(email, ?e, "failed to seed fixed account"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_encryption() -> Arc<DbEncryptionState> {
        let key_path = std::env::temp_dir().join(format!("owsrv-userstore-enc-{}.key", uuid::Uuid::new_v4()));
        Arc::new(DbEncryptionState::load(&key_path))
    }

    fn store() -> UserStore {
        let dir = tempfile::tempdir().unwrap();
        UserStore::load(dir.path().join("users.json"), test_encryption())
    }

    #[test]
    fn registering_without_phone_or_backup_email_fails() {
        let store = store();
        let result = store.register("user@example.com", None, None);
        assert!(matches!(result, Err(RegisterError::MissingBackupContact)));
    }

    #[test]
    fn registering_with_only_backup_email_succeeds() {
        let store = store();
        let result = store.register(
            "user@example.com",
            None,
            Some("backup@example.com".into()),
        );
        assert!(result.is_ok());
        assert!(store.exists("user@example.com"));
    }

    #[test]
    fn registering_with_phone_none_selected_but_no_backup_email_fails() {
        let store = store();
        // 「なし」選択 = 空文字列/Noneとして届く想定。
        let result = store.register("user@example.com", Some("".into()), None);
        assert!(matches!(result, Err(RegisterError::MissingBackupContact)));
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let store = store();
        store.register("user@example.com", Some("+819000000000".into()), None).unwrap();
        let result = store.register("user@example.com", Some("+819011111111".into()), None);
        assert!(matches!(result, Err(RegisterError::AlreadyRegistered)));
    }

    #[test]
    fn find_by_phone_and_backup_email_resolve_to_primary_account() {
        let store = store();
        store
            .register("user@example.com", Some("+819000000000".into()), Some("backup@example.com".into()))
            .unwrap();
        assert_eq!(store.find_email_by_phone("+819000000000").as_deref(), Some("user@example.com"));
        assert_eq!(
            store.find_email_by_backup_email("backup@example.com").as_deref(),
            Some("user@example.com")
        );
    }

    #[test]
    fn rename_email_moves_the_record_to_the_new_key() {
        let store = store();
        store.register("old@example.com", Some("+819000000000".into()), None).unwrap();
        assert!(store.rename_email("old@example.com", "new@example.com"));
        assert!(!store.exists("old@example.com"));
        assert!(store.exists("new@example.com"));
    }

    #[test]
    fn rename_email_fails_if_target_already_taken() {
        let store = store();
        store.register("old@example.com", Some("+819000000000".into()), None).unwrap();
        store.register("new@example.com", Some("+819011111111".into()), None).unwrap();
        assert!(!store.rename_email("old@example.com", "new@example.com"));
    }

    #[test]
    fn normalize_phone_ignores_hyphens_and_keeps_leading_plus() {
        assert_eq!(normalize_phone("090-1234-5678"), "09012345678");
        assert_eq!(normalize_phone("+81 90 1234 5678"), "+819012345678");
    }

    #[test]
    fn find_by_phone_matches_regardless_of_hyphenation() {
        let store = store();
        store.register("user@example.com", Some("090-1234-5678".into()), None).unwrap();
        assert_eq!(store.find_email_by_phone("09012345678").as_deref(), Some("user@example.com"));
        assert_eq!(store.find_email_by_phone("090-1234-5678").as_deref(), Some("user@example.com"));
    }

    #[test]
    fn seed_fixed_account_is_idempotent_and_does_not_clobber_later_edits() {
        let store = store();
        store.seed_fixed_account("owner@example.com", Some("090-1111-2222"), Some("owner2@example.com"));
        assert!(store.exists("owner@example.com"));
        // ユーザーが後で電話番号を変更したとする。
        store.update_contact("owner@example.com", ContactField::Phone, "090-9999-8888");
        // 再度シードしても、既存アカウントは上書きされない。
        store.seed_fixed_account("owner@example.com", Some("090-1111-2222"), Some("owner2@example.com"));
        let record = store.find_by_email("owner@example.com").unwrap();
        assert_eq!(record.phone.as_deref(), Some("09099998888"));
    }

    #[test]
    fn persists_and_reloads_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path: PathBuf = dir.path().join("users.json");
        let encryption = test_encryption();
        {
            let store = UserStore::load(path.clone(), Arc::clone(&encryption));
            store.register("user@example.com", Some("+819000000000".into()), None).unwrap();
        }
        // 実際にディスク上のファイルが平文JSONとして読めない(=本当に
        // 暗号化されている)ことを直接確認する——マーカーバイト・
        // ランダムnonce・暗号文のみで、"user@example.com"という文字列が
        // そのままバイト列に含まれていないことの実証。
        let raw = std::fs::read(&path).unwrap();
        assert!(!raw.windows(b"user@example.com".len()).any(|w| w == b"user@example.com"), "plaintext email must not appear in the encrypted file on disk");

        let reloaded = UserStore::load(path, encryption);
        assert!(reloaded.exists("user@example.com"));
    }

    /// 本暗号化機能の導入前(2026-07-31以前)に書かれた、マーカーバイトの
    /// 無い素のJSONファイルを読み込んでも、既存データを失わずに正しく
    /// 移行できることを確認する(本番の`/var/www/.open-easy-web-users.json`
    /// が実際にこの旧フォーマットだったため、デプロイ前に発見・対応した)。
    #[test]
    fn migrates_legacy_unencrypted_file_without_losing_data() {
        let dir = tempfile::tempdir().unwrap();
        let path: PathBuf = dir.path().join("users.json");
        // マーカーバイトを一切付けない、旧`serde_json::to_string_pretty`
        // 相当の素のJSONをそのまま書き込む(旧`persist`の再現)。
        let mut legacy = HashMap::new();
        legacy.insert(
            "legacy@example.com".to_string(),
            UserRecord { email: "legacy@example.com".to_string(), phone: None, backup_email: Some("legacy2@example.com".to_string()), totp_secret: None, pending_totp_secret: None },
        );
        std::fs::write(&path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

        let store = UserStore::load(path.clone(), test_encryption());
        assert!(store.exists("legacy@example.com"), "legacy plaintext data must be readable after upgrading to the encrypting UserStore");

        // 次回の書き込みで、実際に暗号化フォーマットへ移行することを確認する。
        store.update_contact("legacy@example.com", ContactField::Phone, "090-0000-0000");
        let raw = std::fs::read(&path).unwrap();
        assert_eq!(raw[0], 0x01, "file must be rewritten in the encrypted marker format after the next write");
    }
}
