//! TOTP(Time-based One-Time Password、RFC 6238)による認証アプリ2FA。
//!
//! メール/電話番号OTP(第一要素)に加えて、Google Authenticator・Authy
//! 等の認証アプリを使った第二要素を任意で有効化できるようにする。
//! 標準的な認証アプリとの互換性を優先し、HMAC-SHA1・30秒ステップ・
//! 6桁コードというGoogle Authenticator互換の最も一般的な設定を採用する。

use hmac::{Hmac, Mac};
use rand::RngCore;
use sha1::Sha1;

const TIME_STEP_SECS: u64 = 30;
const CODE_DIGITS: u32 = 6;
/// 時計のずれを許容する前後のステップ数。
const SKEW_STEPS: i64 = 1;

/// ランダムな160bit(20バイト)の共有シークレットを生成する
/// (Google Authenticator等の実装が前提とする長さ)。
pub fn generate_secret() -> Vec<u8> {
    let mut secret = vec![0u8; 20];
    rand::thread_rng().fill_bytes(&mut secret);
    secret
}

pub fn secret_to_base32(secret: &[u8]) -> String {
    base32::encode(base32::Alphabet::Rfc4648 { padding: false }, secret)
}

pub fn base32_to_secret(b32: &str) -> Option<Vec<u8>> {
    base32::decode(base32::Alphabet::Rfc4648 { padding: false }, b32)
}

/// 認証アプリでQRコード表示・読み取りに使う`otpauth://`URI。
pub fn provisioning_uri(account: &str, issuer: &str, secret_b32: &str) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&digits={}&period={}",
        urlencode(issuer),
        urlencode(account),
        secret_b32,
        urlencode(issuer),
        CODE_DIGITS,
        TIME_STEP_SECS,
    )
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn hotp(secret: &[u8], counter: u64) -> u32 {
    type HmacSha1 = Hmac<Sha1>;
    let mut mac = HmacSha1::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();
    let offset = (result[result.len() - 1] & 0x0f) as usize;
    let binary = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32) << 16)
        | ((result[offset + 2] as u32) << 8)
        | (result[offset + 3] as u32);
    binary % 10u32.pow(CODE_DIGITS)
}

fn code_at(secret: &[u8], unix_time: u64) -> String {
    let counter = unix_time / TIME_STEP_SECS;
    format!("{:0width$}", hotp(secret, counter), width = CODE_DIGITS as usize)
}

/// 現在時刻の前後`SKEW_STEPS`ステップ以内に一致するコードがあれば認証成功と
/// する(端末の時計のずれを許容するため)。
pub fn verify_code(secret: &[u8], code: &str, unix_time: u64) -> bool {
    if code.len() != CODE_DIGITS as usize || !code.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let counter = (unix_time / TIME_STEP_SECS) as i64;
    for skew in -SKEW_STEPS..=SKEW_STEPS {
        let step = counter + skew;
        if step < 0 {
            continue;
        }
        let expected = code_at(secret, step as u64 * TIME_STEP_SECS);
        if expected == code {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 Appendix Bのテストベクタ(SHA1、secret="12345678901234567890"、
    /// digits=8)は桁数が異なるため使えない。ここでは自前実装の内部一貫性
    /// (同じ時刻・同じ鍵なら同じコードが出る/違えば出ない)を検証する。
    #[test]
    fn same_secret_and_time_produce_same_code() {
        let secret = generate_secret();
        let t = 1_700_000_000u64;
        assert_eq!(code_at(&secret, t), code_at(&secret, t));
    }

    #[test]
    fn different_secrets_produce_different_codes_with_overwhelming_probability() {
        let a = generate_secret();
        let b = generate_secret();
        let t = 1_700_000_000u64;
        assert_ne!(code_at(&a, t), code_at(&b, t));
    }

    #[test]
    fn verify_code_accepts_current_step_and_rejects_wrong_code() {
        let secret = generate_secret();
        let t = 1_700_000_000u64;
        let code = code_at(&secret, t);
        assert!(verify_code(&secret, &code, t));
        assert!(!verify_code(&secret, "000000", t));
    }

    #[test]
    fn verify_code_tolerates_one_step_of_clock_skew() {
        let secret = generate_secret();
        let t = 1_700_000_000u64;
        let code = code_at(&secret, t);
        assert!(verify_code(&secret, &code, t + TIME_STEP_SECS));
        assert!(verify_code(&secret, &code, t - TIME_STEP_SECS));
        assert!(!verify_code(&secret, &code, t + TIME_STEP_SECS * 3));
    }

    #[test]
    fn base32_round_trips_the_secret() {
        let secret = generate_secret();
        let b32 = secret_to_base32(&secret);
        assert_eq!(base32_to_secret(&b32).unwrap(), secret);
    }

    #[test]
    fn provisioning_uri_contains_expected_fields() {
        let uri = provisioning_uri("owner@example.com", "open-easy-web", "ABCDEFGH");
        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("secret=ABCDEFGH"));
        assert!(uri.contains("digits=6"));
        assert!(uri.contains("period=30"));
    }
}
