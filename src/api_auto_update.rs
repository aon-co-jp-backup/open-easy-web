//! 管理API(自動アップデート・システム状況・電源プロファイルなど)への薄い`fetch()`ラッパー。方式は Connect の単純呼び出し
//! (`POST /open_easy_web.admin.v1.<Service>/<Method>`、JSON)で、REST ではない。共通の`call`/`rpc`は、ほかの`api_*.rs`も再利用する。

use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

pub(crate) async fn call<T: Serialize>(base_url: &str, path: &str, method: &str, admin_token: &str, body: Option<&T>) -> Result<Value, String> {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);

    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::Cors);

    let headers = Headers::new().map_err(|e| format!("headers init failed: {e:?}"))?;
    // 実バグ修正(2026-09-07): 管理トークン入力欄からコピペした値に前後の
    // 空白・改行が混入していても常にサーバー側の期待値と一致するよう、
    // ここで一律trimしてから送信する(以前は各呼び出し元がtrimしたり
    // しなかったりで不統一だった)。
    headers.set("x-admin-token", admin_token.trim()).ok();

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
        // Connect のエラー形式 `{"code","message"}` の message を優先して見せる。
        let msg = parsed.get("message").and_then(|m| m.as_str()).map(|m| m.to_string()).unwrap_or_else(|| parsed.to_string());
        Err(format!("HTTP {status}: {msg}"))
    }
}

/// Connect の単純呼び出し: `POST /open_easy_web.admin.v1.<Service>/<Method>`、本文は JSON(引数なしなら `{}`)。
/// REST は使わない(2026-11-04 方針: REST 全廃、`CLAUDE.md`「API 方針」参照)。
const NS: &str = "/open_easy_web.admin.v1.";

pub(crate) async fn rpc<T: Serialize>(base_url: &str, service_method: &str, admin_token: &str, body: &T) -> Result<Value, String> {
    call(base_url, &format!("{NS}{service_method}"), "POST", admin_token, Some(body)).await
}

pub(crate) async fn rpc_empty(base_url: &str, service_method: &str, admin_token: &str) -> Result<Value, String> {
    rpc(base_url, service_method, admin_token, &serde_json::json!({})).await
}

/// `AutoUpdate/Get` — 現在の有効/無効設定と実行中バージョンを取得。
pub async fn get_status(base_url: &str, admin_token: &str) -> Result<Value, String> {
    rpc_empty(base_url, "AutoUpdate/Get", admin_token).await
}

#[derive(Serialize)]
struct SetEnabledBody {
    enabled: bool,
}

/// `AutoUpdate/Set` — 深夜バックグラウンド自動アップデートの有効/無効を切り替える(GUIのトグルスイッチから呼ぶ想定)。
pub async fn set_enabled(base_url: &str, admin_token: &str, enabled: bool) -> Result<Value, String> {
    rpc(base_url, "AutoUpdate/Set", admin_token, &SetEnabledBody { enabled }).await
}

/// `System/GetMemory` — 現在のシステムメモリ使用状況(総メモリ・使用中・空き容量・使用率)を取得する。
pub async fn get_memory_snapshot(base_url: &str, admin_token: &str) -> Result<Value, String> {
    rpc_empty(base_url, "System/GetMemory", admin_token).await
}

/// `System/GetDisk` — 現在の実ディスク(HDD/SSD)使用状況(総容量・使用中・使用率、個別ディスクの内訳)を取得する。
pub async fn get_disk_snapshot(base_url: &str, admin_token: &str) -> Result<Value, String> {
    rpc_empty(base_url, "System/GetDisk", admin_token).await
}

/// `System/GetMemoryBreakdown` — メモリ使用状況をアプリ(サービス)別+その他(aruaru-llmのAIでカテゴリ分類)の2枚の円グラフ用データへ分解して取得する。
pub async fn get_memory_breakdown(base_url: &str, admin_token: &str) -> Result<Value, String> {
    rpc_empty(base_url, "System/GetMemoryBreakdown", admin_token).await
}

/// `System/GetDiskBreakdown` — ディスク使用状況をアプリ別+拡張子別+カテゴリ別(aruaru-llmのAI分類)の3枚の円グラフ用データへ分解して取得する。
pub async fn get_disk_breakdown(base_url: &str, admin_token: &str) -> Result<Value, String> {
    rpc_empty(base_url, "System/GetDiskBreakdown", admin_token).await
}

#[derive(Serialize)]
struct SetPowerProfileBody {
    profiles: Vec<String>,
}

/// `Settings/SetPowerProfile` — 電源プロファイルを設定する(メモリ使用状況セクションの「省メモリ版に変更」ボタンから呼ぶ想定)。
/// 以前の REST では、`open-web-server`(リバースプロキシ)自身の同名の `/admin/power-profile` に横取りされる実バグがあり
/// `easyweb-` 接頭辞を付けていた。Connect の経路 `/open_easy_web.admin.v1.…` はその衝突の心配がない。
pub async fn set_power_profile(base_url: &str, admin_token: &str, profiles: &[&str]) -> Result<Value, String> {
    let body = SetPowerProfileBody { profiles: profiles.iter().map(|s| s.to_string()).collect() };
    rpc(base_url, "Settings/SetPowerProfile", admin_token, &body).await
}

/// `Settings/GetPowerProfile` — 現在有効な電源プロファイルの組み合わせ(`profiles: ["power_save", ...]`)を取得する。
pub async fn get_power_profile(base_url: &str, admin_token: &str) -> Result<Value, String> {
    rpc_empty(base_url, "Settings/GetPowerProfile", admin_token).await
}
