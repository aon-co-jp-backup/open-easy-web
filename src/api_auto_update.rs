//! `/admin/auto-update`(このサーバー自身の深夜バックグラウンド自動
//! アップデート設定API、`server/src/auto_update.rs`参照)への薄い
//! `fetch()`ラッパー。`api_dist_sync.rs`と同じパターン。

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

/// `GET /admin/auto-update` — 現在の有効/無効設定と実行中バージョンを取得。
pub async fn get_status(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, "/admin/auto-update", "GET", admin_token, None).await
}

#[derive(Serialize)]
struct SetEnabledBody {
    enabled: bool,
}

/// `POST /admin/auto-update` — 深夜バックグラウンド自動アップデートの
/// 有効/無効を切り替える(GUIのトグルスイッチから呼ぶ想定)。
pub async fn set_enabled(base_url: &str, admin_token: &str, enabled: bool) -> Result<Value, String> {
    call(base_url, "/admin/auto-update", "POST", admin_token, Some(&SetEnabledBody { enabled })).await
}

/// `GET /admin/system/memory` — 現在のシステムメモリ使用状況
/// (総メモリ・使用中・空き容量・使用率)を取得する(2026-07-31追加)。
pub async fn get_memory_snapshot(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, "/admin/system/memory", "GET", admin_token, None).await
}

#[derive(Serialize)]
struct SetPowerProfileBody {
    profiles: Vec<String>,
}

/// `POST /admin/easyweb-power-profile` — 電源プロファイルを設定する
/// (2026-07-31追加、メモリ使用状況セクションの「省メモリ版に変更」
/// ボタンから呼ぶ想定)。**正直な開示**: パス名に`easyweb-`を含むのは、
/// `open-web-server`(このアプリのリバースプロキシ)自身が同名の
/// `/admin/power-profile`管理APIを既に持っており、素の`/admin/
/// power-profile`だとそちらに横取りされ、このアプリのバックエンドまで
/// 到達しない実バグが本番環境で発覚したため(詳細は`server/src/
/// main.rs`のコメント参照)。
pub async fn set_power_profile(base_url: &str, admin_token: &str, profiles: &[&str]) -> Result<Value, String> {
    let body = SetPowerProfileBody { profiles: profiles.iter().map(|s| s.to_string()).collect() };
    call(base_url, "/admin/easyweb-power-profile", "POST", admin_token, Some(&body)).await
}
