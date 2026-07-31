//! `/admin/db-encryption`(DATABASE暗号化ON/OFF設定API、
//! `server/src/db_encryption.rs`参照)への薄い`fetch()`ラッパー。
//! `api_auto_update.rs`と同じパターン。

use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

async fn call<T: Serialize>(base_url: &str, path: &str, method: &str, admin_token: &str, body: Option<&T>) -> Result<Value, String> {
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

    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("request build failed: {e:?}"))?;

    let resp_value = JsFuture::from(crate::dom::window().fetch_with_request(&request)).await.map_err(|e| format!("fetch failed: {e:?}"))?;
    let response: Response = resp_value.dyn_into().map_err(|_| "not a Response".to_string())?;
    let status = response.status();

    let json_value = JsFuture::from(response.json().map_err(|e| format!("json() failed: {e:?}"))?).await.map_err(|e| format!("body read failed: {e:?}"))?;
    let parsed: Value =
        js_sys::JSON::stringify(&json_value).ok().and_then(|s| s.as_string()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);

    if (200..300).contains(&status) {
        Ok(parsed)
    } else {
        Err(format!("HTTP {status}: {parsed}"))
    }
}

/// `GET /admin/db-encryption` — 現在のDATABASE暗号化設定(既定ON)を取得。
pub async fn get_status(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, "/admin/db-encryption", "GET", admin_token, None).await
}

#[derive(Serialize)]
struct SetEnabledBody {
    enabled: bool,
}

/// `POST /admin/db-encryption` — DATABASE暗号化のON/OFFを切り替える
/// (GUIのトグルスイッチから呼ぶ想定)。
pub async fn set_enabled(base_url: &str, admin_token: &str, enabled: bool) -> Result<Value, String> {
    call(base_url, "/admin/db-encryption", "POST", admin_token, Some(&SetEnabledBody { enabled })).await
}
