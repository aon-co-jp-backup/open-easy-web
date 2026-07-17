//! メールアドレス(ID)・セカンドメール・電話番号を経路とした、
//! ワンタイムパスワード(OTP)認証。
//!
//! 固定パスワードは一切保存しない——ログインのたびにランダムな6桁の
//! コードを生成し、指定の連絡先(主メール・セカンドメール・電話番号の
//! いずれか、`users::UserStore`で登録範囲内のものだけが有効)へ送信する。
//! OTPは平文では保存せず、SHA-256ハッシュのみをメモリ上に保持する
//! (有効期限5分・最大試行回数5回)。OTP自体は「どの連絡先宛か」しか
//! 知らない——検証成功後、呼び出し側(`main.rs`)が
//! `users::UserStore`でその連絡先の持ち主(主メールアドレス)を解決し、
//! `create_session`でその主メールに対するセッションを発行する。

use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const OTP_TTL: Duration = Duration::from_secs(5 * 60);
const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);
const EMAIL_CHANGE_TTL: Duration = Duration::from_secs(30 * 60);
const MAX_ATTEMPTS: u32 = 5;

fn hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn generate_otp() -> String {
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1_000_000u32))
}

fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 24] = rng.gen();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

struct PendingOtp {
    code_hash: String,
    expires_at: Instant,
    attempts: u32,
}

struct Session {
    email: String,
    expires_at: Instant,
}

/// 連絡先(主メール・セカンドメール・電話番号のいずれか)の変更保留情報。
/// `field`は`"email"`(主メール、`rename_email`で処理)・`"phone"`・
/// `"backup_email"`のいずれか。
struct PendingContactChange {
    account_email: String,
    field: String,
    new_value: String,
    expires_at: Instant,
}

#[derive(Default)]
pub struct AuthStore {
    /// キーは連絡先(主メール・セカンドメール・電話番号のいずれか)そのもの。
    pending: Mutex<HashMap<String, PendingOtp>>,
    sessions: Mutex<HashMap<String, Session>>,
    contact_changes: Mutex<HashMap<String, PendingContactChange>>,
}

pub enum RequestOtpOutcome {
    Issued(String),
}

impl AuthStore {
    /// `contact`(主メール・セカンドメール・電話番号のいずれか)宛の
    /// OTPを発行し、送信すべきコードを返す(ハッシュのみ保存)。
    pub fn request_otp(&self, contact: &str) -> RequestOtpOutcome {
        let code = generate_otp();
        let mut pending = self.pending.lock().unwrap();
        pending.insert(
            contact.to_string(),
            PendingOtp {
                code_hash: hash(&code),
                expires_at: Instant::now() + OTP_TTL,
                attempts: 0,
            },
        );
        RequestOtpOutcome::Issued(code)
    }

    /// `contact`宛に発行済みのOTPを検証する。連絡先の持ち主(主メール)
    /// を解決するのは呼び出し側の責務——このメソッドはOTPの正しさだけを見る。
    pub fn consume_otp(&self, contact: &str, code: &str) -> Result<(), VerifyError> {
        let mut pending = self.pending.lock().unwrap();
        let Some(entry) = pending.get_mut(contact) else {
            return Err(VerifyError::NotRequested);
        };
        if Instant::now() > entry.expires_at {
            pending.remove(contact);
            return Err(VerifyError::Expired);
        }
        if entry.attempts >= MAX_ATTEMPTS {
            pending.remove(contact);
            return Err(VerifyError::TooManyAttempts);
        }
        entry.attempts += 1;
        if entry.code_hash != hash(code) {
            return Err(VerifyError::Mismatch);
        }
        pending.remove(contact);
        Ok(())
    }

    /// `account_email`(主メールアドレス)に対するセッションを発行する。
    pub fn create_session(&self, account_email: &str) -> String {
        let token = generate_token();
        self.sessions.lock().unwrap().insert(
            token.clone(),
            Session {
                email: account_email.to_string(),
                expires_at: Instant::now() + SESSION_TTL,
            },
        );
        token
    }

    /// トークンから有効なセッションのメールアドレスを取得する。
    pub fn session_email(&self, token: &str) -> Option<String> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get(token)?;
        if Instant::now() > session.expires_at {
            sessions.remove(token);
            return None;
        }
        Some(session.email.clone())
    }

    pub fn logout(&self, token: &str) {
        self.sessions.lock().unwrap().remove(token);
    }

    /// 連絡先変更リクエストを発行する(`field`は`"email"`/`"phone"`/
    /// `"backup_email"`)。確認リンクは(`new_value`ではなく)**必ず
    /// 現在の主メール(`account_email`)へ**送る——アカウント乗っ取り
    /// 防止のため、変更は現在のメールアドレスの持ち主にしか実行できない。
    pub fn request_contact_change(&self, account_email: &str, field: &str, new_value: &str) -> String {
        let token = generate_token();
        self.contact_changes.lock().unwrap().insert(
            token.clone(),
            PendingContactChange {
                account_email: account_email.to_string(),
                field: field.to_string(),
                new_value: new_value.to_string(),
                expires_at: Instant::now() + EMAIL_CHANGE_TTL,
            },
        );
        token
    }

    /// 連絡先変更の確認リンクを踏んだ際に呼ぶ。成功すれば
    /// `(account_email, field, new_value)`を返す(実際の適用は
    /// `UserStore`側で行う)。
    pub fn confirm_contact_change(&self, token: &str) -> Option<(String, String, String)> {
        let mut changes = self.contact_changes.lock().unwrap();
        let entry = changes.remove(token)?;
        if Instant::now() > entry.expires_at {
            return None;
        }
        Some((entry.account_email, entry.field, entry.new_value))
    }
}

#[derive(Debug)]
pub enum VerifyError {
    NotRequested,
    Expired,
    TooManyAttempts,
    Mismatch,
}

impl VerifyError {
    pub fn message_ja(&self) -> &'static str {
        match self {
            VerifyError::NotRequested => "この連絡先宛にOTPは発行されていません。",
            VerifyError::Expired => "OTPの有効期限が切れました。再度リクエストしてください。",
            VerifyError::TooManyAttempts => "試行回数の上限を超えました。再度リクエストしてください。",
            VerifyError::Mismatch => "コードが正しくありません。",
        }
    }

    pub fn message_en(&self) -> &'static str {
        match self {
            VerifyError::NotRequested => "No OTP was requested for this contact.",
            VerifyError::Expired => "The OTP has expired. Please request a new one.",
            VerifyError::TooManyAttempts => "Too many attempts. Please request a new OTP.",
            VerifyError::Mismatch => "The code is incorrect.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otp_roundtrip_succeeds_with_correct_code() {
        let store = AuthStore::default();
        let RequestOtpOutcome::Issued(code) = store.request_otp("user@example.com");
        store.consume_otp("user@example.com", &code).unwrap();
        let token = store.create_session("user@example.com");
        assert_eq!(store.session_email(&token).as_deref(), Some("user@example.com"));
    }

    #[test]
    fn wrong_code_is_rejected_and_does_not_consume_the_otp() {
        let store = AuthStore::default();
        let RequestOtpOutcome::Issued(code) = store.request_otp("user@example.com");
        assert!(matches!(
            store.consume_otp("user@example.com", "000000"),
            Err(VerifyError::Mismatch)
        ));
        assert!(store.consume_otp("user@example.com", &code).is_ok());
    }

    #[test]
    fn exceeding_max_attempts_invalidates_the_otp() {
        let store = AuthStore::default();
        let RequestOtpOutcome::Issued(_code) = store.request_otp("user@example.com");
        for _ in 0..MAX_ATTEMPTS {
            let _ = store.consume_otp("user@example.com", "000000");
        }
        assert!(matches!(
            store.consume_otp("user@example.com", "000000"),
            Err(VerifyError::TooManyAttempts) | Err(VerifyError::NotRequested)
        ));
    }

    #[test]
    fn verifying_without_a_prior_request_fails() {
        let store = AuthStore::default();
        assert!(matches!(
            store.consume_otp("nobody@example.com", "123456"),
            Err(VerifyError::NotRequested)
        ));
    }

    #[test]
    fn logout_invalidates_the_session_token() {
        let store = AuthStore::default();
        let RequestOtpOutcome::Issued(code) = store.request_otp("user@example.com");
        store.consume_otp("user@example.com", &code).unwrap();
        let token = store.create_session("user@example.com");
        store.logout(&token);
        assert_eq!(store.session_email(&token), None);
    }

    #[test]
    fn otp_via_phone_contact_still_resolves_to_the_account_email() {
        let store = AuthStore::default();
        // request-otp-sms 相当: キーは電話番号だが、セッションは
        // 呼び出し側が解決した主メールに対して発行される。
        let RequestOtpOutcome::Issued(code) = store.request_otp("+819000000000");
        store.consume_otp("+819000000000", &code).unwrap();
        let token = store.create_session("owner@example.com");
        assert_eq!(store.session_email(&token).as_deref(), Some("owner@example.com"));
    }

    #[test]
    fn contact_change_confirmation_round_trips_account_field_and_new_value() {
        let store = AuthStore::default();
        let token = store.request_contact_change("owner@example.com", "email", "new@example.com");
        let (account, field, new_value) = store.confirm_contact_change(&token).unwrap();
        assert_eq!(account, "owner@example.com");
        assert_eq!(field, "email");
        assert_eq!(new_value, "new@example.com");
        // トークンは一度使うと消費される。
        assert!(store.confirm_contact_change(&token).is_none());
    }

    #[test]
    fn contact_change_supports_phone_and_backup_email_fields() {
        let store = AuthStore::default();
        let token = store.request_contact_change("owner@example.com", "phone", "+819000000000");
        let (account, field, new_value) = store.confirm_contact_change(&token).unwrap();
        assert_eq!(account, "owner@example.com");
        assert_eq!(field, "phone");
        assert_eq!(new_value, "+819000000000");
    }

    #[test]
    fn unknown_contact_change_token_is_rejected() {
        let store = AuthStore::default();
        assert!(store.confirm_contact_change("does-not-exist").is_none());
    }
}
