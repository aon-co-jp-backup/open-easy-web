//! `open-web-server-gateway`側の無料DDNS(DuckDNS)管理APIへの薄い
//! `fetch()`ラッパー。
//!
//! `open-easy-web-server`自身のAPI(`api_auth.rs`/`api_upload.rs`)とは異なり、
//! 呼び出し先は**別オリジンの`open-web-server`インスタンス**(ユーザーが
//! ベースURLを入力する)であるため`RequestMode::Cors`を使う。
//! `open-web-server`側でCORSヘッダが未設定の場合はブラウザ側でブロック
//! されうる——本モジュールはその制約を回避しない(正直な開示: 同一
//! オリジン配信や、reverse proxy越しの利用を推奨)。

use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

/// `open-web-server`管理APIへ、任意メソッド+JSONボディでリクエストする。
async fn call_admin_api<T: Serialize>(
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
        .map_err(|e| format!("fetch failed (CORS/ネットワーク到達性を確認してください): {e:?}"))?;
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
struct SetupFreeDomainBody {
    domain: String,
    token: String,
}

/// `POST /admin/ddns/setup-free-domain` を呼び、即時疎通確認結果を返す。
pub async fn setup_free_domain(
    base_url: &str,
    admin_token: &str,
    duckdns_domain: &str,
    duckdns_token: &str,
) -> Result<Value, String> {
    call_admin_api(
        base_url,
        "/admin/ddns/setup-free-domain",
        "POST",
        admin_token,
        Some(&SetupFreeDomainBody { domain: duckdns_domain.to_string(), token: duckdns_token.to_string() }),
    )
    .await
}

/// `GET /admin/sftp/connection-info` を呼び、コピペで使える接続コマンドを取得する。
pub async fn sftp_connection_info(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call_admin_api::<()>(base_url, "/admin/sftp/connection-info", "GET", admin_token, None).await
}

/// `GET /admin/ddns/domains` を呼び、登録済みドメイン一覧+残り枠を取得する
/// (最大20件、`open-web-server`側`free_domain::MAX_DUCKDNS_DOMAINS`)。
pub async fn list_domains(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call_admin_api::<()>(base_url, "/admin/ddns/domains", "GET", admin_token, None).await
}

/// `DELETE /admin/ddns/domains/:domain` を呼び、登録を解除する。
pub async fn remove_domain(base_url: &str, admin_token: &str, domain: &str) -> Result<Value, String> {
    call_admin_api::<()>(
        base_url,
        &format!("/admin/ddns/domains/{}", js_sys::encode_uri_component(domain)),
        "DELETE",
        admin_token,
        None,
    )
    .await
}
