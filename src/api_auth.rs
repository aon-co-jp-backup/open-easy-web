//! `open-easy-web-server`のOTP認証APIへの薄い`fetch()`ラッパー。
//!
//! 固定パスワードは扱わない——メール1・メール2・電話番号のいずれかを
//! 「連絡先」として入力し、そこへ送られるOTPで認証する。セッション
//! トークンは`localStorage`に保持し、サイト操作系APIの呼び出し時に
//! `Authorization: Bearer`ヘッダとして付与する。

use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

const TOKEN_KEY: &str = "openeasyweb_session_token_v1";
const ACCOUNT_EMAIL_KEY: &str = "openeasyweb_account_email_v1";

pub fn saved_token() -> Option<String> {
    crate::dom::window().local_storage().ok().flatten()?.get_item(TOKEN_KEY).ok().flatten()
}

pub fn saved_account_email() -> Option<String> {
    crate::dom::window()
        .local_storage()
        .ok()
        .flatten()?
        .get_item(ACCOUNT_EMAIL_KEY)
        .ok()
        .flatten()
}

fn save_session(token: &str, email: &str) {
    if let Some(storage) = crate::dom::window().local_storage().ok().flatten() {
        let _ = storage.set_item(TOKEN_KEY, token);
        let _ = storage.set_item(ACCOUNT_EMAIL_KEY, email);
    }
}

pub fn clear_session() {
    if let Some(storage) = crate::dom::window().local_storage().ok().flatten() {
        let _ = storage.remove_item(TOKEN_KEY);
        let _ = storage.remove_item(ACCOUNT_EMAIL_KEY);
    }
}

/// `POST <path>` へJSONボディを送り、レスポンスのJSONを`Value`として返す。
/// `token`が`Some`なら`Authorization: Bearer`ヘッダを付与する。
/// エラー時は`(HTTPステータス, レスポンスJSON or 文字列)`風のメッセージ文字列を返す。
async fn post_json(path: &str, token: Option<&str>, body: &impl Serialize) -> Result<Value, String> {
    let body_str = serde_json::to_string(body).map_err(|e| format!("request encode failed: {e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::SameOrigin);
    opts.set_body(&JsValue::from_str(&body_str));

    let headers = Headers::new().map_err(|e| format!("headers init failed: {e:?}"))?;
    headers.set("Content-Type", "application/json").ok();
    if let Some(t) = token {
        headers.set("Authorization", &format!("Bearer {t}")).ok();
    }
    opts.set_headers(&headers);

    let request =
        Request::new_with_str_and_init(path, &opts).map_err(|e| format!("request build failed: {e:?}"))?;

    let resp_value = JsFuture::from(crate::dom::window().fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch failed: {e:?}"))?;
    let response: Response = resp_value.dyn_into().map_err(|_| "not a Response".to_string())?;
    let status = response.status();

    let json_value = JsFuture::from(response.json().map_err(|e| format!("json() failed: {e:?}"))?)
        .await
        .map_err(|e| format!("body read failed: {e:?}"))?;
    let parsed: Value = serde_wasm_bindgen_lite(&json_value);

    if (200..300).contains(&status) {
        Ok(parsed)
    } else {
        let msg = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
            .to_string();
        Err(format!("HTTP {status}: {msg}"))
    }
}

/// `js_sys`/`serde_json`橋渡し。`serde-wasm-bindgen`crateを新規追加せず、
/// `JSON.stringify`→`serde_json::from_str`で素朴に変換する
/// (このリポジトリの「薄い依存のみ」方針に合わせた)。
fn serde_wasm_bindgen_lite(value: &JsValue) -> Value {
    js_sys::JSON::stringify(value)
        .ok()
        .and_then(|s| s.as_string())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

#[derive(Serialize)]
struct ContactBody {
    contact: String,
}

pub async fn request_otp(contact: &str) -> Result<Value, String> {
    post_json("/api/auth/request-otp", None, &ContactBody { contact: contact.to_string() }).await
}

#[derive(Serialize)]
struct VerifyOtpBody {
    contact: String,
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    totp_code: Option<String>,
}

/// OTP検証の結果。TOTP2FAが有効なアカウントでは、正しいOTPでも
/// `TotpRequired`が返り、`totp_code`を添えて再度呼ぶまでセッションは
/// 発行されない。
pub enum VerifyOtpOutcome {
    LoggedIn(String),
    TotpRequired,
}

pub async fn verify_otp(contact: &str, code: &str, totp_code: Option<&str>) -> Result<VerifyOtpOutcome, String> {
    let value = post_json(
        "/api/auth/verify-otp",
        None,
        &VerifyOtpBody {
            contact: contact.to_string(),
            code: code.to_string(),
            totp_code: totp_code.map(str::to_string),
        },
    )
    .await?;
    if value.get("totp_required").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Ok(VerifyOtpOutcome::TotpRequired);
    }
    let token = value.get("token").and_then(|v| v.as_str()).ok_or("missing token in response")?;
    let email = value.get("email").and_then(|v| v.as_str()).unwrap_or(contact);
    save_session(token, email);
    Ok(VerifyOtpOutcome::LoggedIn(token.to_string()))
}

#[derive(Serialize)]
struct TotpLoginBody {
    account_email: String,
    totp_code: String,
}

/// `POST /api/auth/totp-login` — メールOTPを一切経由せず、認証アプリの
/// コードだけでログインする(サーバー側`server/src/main.rs`の
/// `totp_login`ハンドラに対応、2026-07-17)。
pub async fn totp_login(account_email: &str, totp_code: &str) -> Result<String, String> {
    let value = post_json(
        "/api/auth/totp-login",
        None,
        &TotpLoginBody { account_email: account_email.to_string(), totp_code: totp_code.to_string() },
    )
    .await?;
    let token = value.get("token").and_then(|v| v.as_str()).ok_or("missing token in response")?;
    let email = value.get("email").and_then(|v| v.as_str()).unwrap_or(account_email);
    save_session(token, email);
    Ok(token.to_string())
}

#[derive(Serialize)]
struct TokenBody {
    token: String,
}

pub async fn logout() -> Result<(), String> {
    if let Some(token) = saved_token() {
        let _ = post_json("/api/auth/logout", None, &TokenBody { token }).await;
    }
    clear_session();
    Ok(())
}

#[derive(Serialize)]
struct RequestEmailChangeBody {
    field: String,
    new_value: String,
}

/// `field`は`"email"`(主メール)・`"backup_email"`・`"phone"`のいずれか。
pub async fn request_contact_change(field: &str, new_value: &str) -> Result<Value, String> {
    let token = saved_token().ok_or("not logged in")?;
    post_json(
        "/api/auth/request-email-change",
        Some(&token),
        &RequestEmailChangeBody { field: field.to_string(), new_value: new_value.to_string() },
    )
    .await
}

/// 認証アプリ(TOTP)2FAのセットアップを開始する。QRコード表示用の
/// provisioning URIとシークレットを返す(まだ有効化はされない)。
pub async fn totp_setup() -> Result<Value, String> {
    let token = saved_token().ok_or("not logged in")?;
    post_json("/api/auth/totp/setup", Some(&token), &serde_json::json!({})).await
}

#[derive(Serialize)]
struct TotpCodeBody {
    code: String,
}

/// セットアップ中のTOTPを、認証アプリの確認コードで有効化する。
pub async fn totp_enable(code: &str) -> Result<Value, String> {
    let token = saved_token().ok_or("not logged in")?;
    post_json("/api/auth/totp/enable", Some(&token), &TotpCodeBody { code: code.to_string() }).await
}

/// TOTP2FAを無効化する。
pub async fn totp_disable() -> Result<Value, String> {
    let token = saved_token().ok_or("not logged in")?;
    post_json("/api/auth/totp/disable", Some(&token), &serde_json::json!({})).await
}
