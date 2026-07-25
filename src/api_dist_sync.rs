//! `/admin/dist-sync/*`(このサーバー自身の分散同期+ディザスタリカバリ
//! 管理API、`server/src/dist_sync.rs`参照)への薄い`fetch()`ラッパー。
//!
//! `api_free_domain.rs`と異なり、呼び出し先は**この`open-easy-web-server`
//! 自身**(同一オリジン)である想定だが、開発時にWASM UIを別ポート
//! (`python -m http.server`等)で配信するケースもあるため、同じく
//! `RequestMode::Cors`を使う(同一オリジンの場合は実質無害)。

use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

async fn call<T: Serialize>(
    base_url: &str,
    path: &str,
    method: &str,
    admin_token: &str,
    body: Option<&T>,
) -> Result<Value, String> {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);

    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::Cors);

    let headers = Headers::new().map_err(|e| format!("headers init failed: {e:?}"))?;
    headers.set("x-admin-token", admin_token).ok();

    if let Some(b) = body {
        let body_str = serde_json::to_string(b).map_err(|e| format!("request encode failed: {e}"))?;
        opts.set_body(&JsValue::from_str(&body_str));
        headers.set("Content-Type", "application/json").ok();
    }
    opts.set_headers(&headers);

    let request =
        Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("request build failed: {e:?}"))?;

    let resp_value = JsFuture::from(crate::dom::window().fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch failed: {e:?}"))?;
    let response: Response = resp_value.dyn_into().map_err(|_| "not a Response".to_string())?;
    let status = response.status();

    let json_value = JsFuture::from(response.json().map_err(|e| format!("json() failed: {e:?}"))?)
        .await
        .map_err(|e| format!("body read failed: {e:?}"))?;
    let parsed: Value = js_sys::JSON::stringify(&json_value)
        .ok()
        .and_then(|s| s.as_string())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);

    if (200..300).contains(&status) {
        Ok(parsed)
    } else {
        Err(format!("HTTP {status}: {parsed}"))
    }
}

#[derive(Serialize)]
struct RegisterTargetBody<'a> {
    host: &'a str,
    port: u16,
    username: &'a str,
    password_env: &'a str,
    remote_backup_dir: &'a str,
    label: Option<&'a str>,
}

/// `POST /admin/dist-sync/targets` — 他VPSへの分散同期クローンDB先を1件登録。
#[allow(clippy::too_many_arguments)]
pub async fn register_target(
    base_url: &str,
    admin_token: &str,
    host: &str,
    port: u16,
    username: &str,
    password_env: &str,
    remote_backup_dir: &str,
    label: Option<&str>,
) -> Result<Value, String> {
    call(
        base_url,
        "/admin/dist-sync/targets",
        "POST",
        admin_token,
        Some(&RegisterTargetBody { host, port, username, password_env, remote_backup_dir, label }),
    )
    .await
}

/// `GET /admin/dist-sync/targets` — 登録済みの分散同期先一覧を取得。
pub async fn list_targets(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, "/admin/dist-sync/targets", "GET", admin_token, None).await
}

/// `DELETE /admin/dist-sync/targets/:id` — 分散同期先を1件削除。
pub async fn delete_target(base_url: &str, admin_token: &str, id: &str) -> Result<Value, String> {
    call::<()>(base_url, &format!("/admin/dist-sync/targets/{id}"), "DELETE", admin_token, None).await
}

#[derive(Serialize, Default)]
struct DisasterFallbackBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    google_drive: Option<Value>,
}

/// `POST /admin/dist-sync/disaster-fallback` — メール宛先を設定
/// (「スキップ」の場合はこの関数自体を呼ばない、非ブロッキング設計)。
pub async fn set_disaster_fallback_email(
    base_url: &str,
    admin_token: &str,
    smtp_host: &str,
    smtp_port: u16,
    smtp_username: &str,
    smtp_password_env: &str,
    from_address: &str,
    to_address: &str,
) -> Result<Value, String> {
    let email = serde_json::json!({
        "smtp_host": smtp_host,
        "smtp_port": smtp_port,
        "smtp_username": smtp_username,
        "smtp_password_env": smtp_password_env,
        "from_address": from_address,
        "to_address": to_address,
        "allow_plaintext_for_testing": false,
    });
    call(
        base_url,
        "/admin/dist-sync/disaster-fallback",
        "POST",
        admin_token,
        Some(&DisasterFallbackBody { email: Some(email), google_drive: None }),
    )
    .await
}

/// `POST /admin/dist-sync/disaster-fallback` — Googleドライブ宛先を設定。
/// **正直な開示**: サーバー側APIは実装済みだが、ウィザードUI
/// (`setup_wizard_ui.rs`)側はこのパスでは時間の制約によりメール退避先の
/// フォームのみ配線した(次回課題)。この関数自体はAPIクライアントとして
/// 完成しており、UI配線を追加すればすぐ使える。
#[allow(dead_code)]
pub async fn set_disaster_fallback_google_drive(
    base_url: &str,
    admin_token: &str,
    backup_folder_name: &str,
    client_id_env: &str,
    client_secret_env: &str,
    refresh_token_env: &str,
) -> Result<Value, String> {
    let google_drive = serde_json::json!({
        "backup_folder_name": backup_folder_name,
        "client_id_env": client_id_env,
        "client_secret_env": client_secret_env,
        "refresh_token_env": refresh_token_env,
    });
    call(
        base_url,
        "/admin/dist-sync/disaster-fallback",
        "POST",
        admin_token,
        Some(&DisasterFallbackBody { email: None, google_drive: Some(google_drive) }),
    )
    .await
}

/// `POST /admin/dist-sync/first-time-setup` — 登録済みの全同期先/退避先へ
/// 疎通確認(`ensure_ready`)を試み、結果レポートを返す。
pub async fn run_first_time_setup(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, "/admin/dist-sync/first-time-setup", "POST", admin_token, None).await
}
