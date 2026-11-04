//! `open-easy-web-server`のサイト操作API(フォルダー作成・AI判定・訂正は Connect の `Sites/*`、
//! アップロードだけ `multipart/form-data`)への薄い`fetch()`ラッパー。全て`Authorization: Bearer`
//! セッショントークンが必要(`api_auth::saved_token()`)。
//!
//! 方式は Connect の単純呼び出し(`POST /open_easy_web.admin.v1.<Service>/<Method>`、JSON)で、REST は使わない
//! (2026-11-04 方針)。**例外**: ファイル送信の `upload` だけは、バイナリを JSON に載せると大きくなるため
//! `multipart/form-data` の REST のまま(`CLAUDE.md`「API 方針」参照)。

use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FileList, FormData, Headers, Request, RequestInit, RequestMode, Response};

const NS: &str = "/open_easy_web.admin.v1.";

#[derive(Serialize)]
struct SiteBody<'a> {
    site: &'a str,
}

fn json_to_value(js: &JsValue) -> Value {
    js_sys::JSON::stringify(js)
        .ok()
        .and_then(|s| s.as_string())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

async fn read_response(resp_value: JsValue) -> Result<Value, String> {
    let response: Response = resp_value.dyn_into().map_err(|_| "not a Response".to_string())?;
    let status = response.status();
    let json_value = JsFuture::from(response.json().map_err(|e| format!("json() failed: {e:?}"))?)
        .await
        .map_err(|e| format!("body read failed: {e:?}"))?;
    let parsed = json_to_value(&json_value);
    if (200..300).contains(&status) {
        Ok(parsed)
    } else {
        // Connect のエラー形式 `{"code","message"}` の message を優先し、旧形式の `error` も読む。
        let msg = parsed.get("message").or_else(|| parsed.get("error")).and_then(|v| v.as_str()).unwrap_or("unknown error").to_string();
        Err(format!("HTTP {status}: {msg}"))
    }
}

async fn authed_post_json(path: &str, body: &impl Serialize) -> Result<Value, String> {
    let token = crate::api_auth::saved_token().ok_or("not logged in / ログインしてください")?;
    let body_str = serde_json::to_string(body).map_err(|e| format!("request encode failed: {e}"))?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::SameOrigin);
    opts.set_body(&JsValue::from_str(&body_str));

    let headers = Headers::new().map_err(|e| format!("headers init failed: {e:?}"))?;
    headers.set("Content-Type", "application/json").ok();
    headers.set("Authorization", &format!("Bearer {token}")).ok();
    opts.set_headers(&headers);

    let request =
        Request::new_with_str_and_init(path, &opts).map_err(|e| format!("request build failed: {e:?}"))?;
    let resp_value = JsFuture::from(crate::dom::window().fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch failed: {e:?}"))?;
    read_response(resp_value).await
}

/// `Sites/Delete` — ドメイン登録を取り消す(nginx vhostのみ削除、
/// アップロード済みファイル・証明書は保持される)。
pub async fn delete_domain(site: &str) -> Result<Value, String> {
    authed_post_json(&format!("{NS}Sites/Delete"), &SiteBody { site }).await
}

pub async fn create_folder(site: &str) -> Result<Value, String> {
    authed_post_json(&format!("{NS}Sites/CreateFolder"), &SiteBody { site }).await
}

/// `<input type="file" multiple>` で選択されたファイル群をアップロードする。
pub async fn upload_files(site: &str, files: &FileList) -> Result<Value, String> {
    let token = crate::api_auth::saved_token().ok_or("not logged in / ログインしてください")?;

    let form = FormData::new().map_err(|e| format!("FormData init failed: {e:?}"))?;
    for i in 0..files.length() {
        if let Some(file) = files.get(i) {
            let name = file.name();
            form.append_with_blob_and_filename("files", &file, &name)
                .map_err(|e| format!("append failed: {e:?}"))?;
        }
    }

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::SameOrigin);
    opts.set_body(&form);

    let headers = Headers::new().map_err(|e| format!("headers init failed: {e:?}"))?;
    headers.set("Authorization", &format!("Bearer {token}")).ok();
    opts.set_headers(&headers);

    let path = format!("/api/sites/{site}/upload");
    let request =
        Request::new_with_str_and_init(&path, &opts).map_err(|e| format!("request build failed: {e:?}"))?;
    let resp_value = JsFuture::from(crate::dom::window().fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch failed: {e:?}"))?;
    read_response(resp_value).await
}

pub async fn detect_and_configure(site: &str) -> Result<Value, String> {
    authed_post_json(&format!("{NS}Sites/DetectAndConfigure"), &SiteBody { site }).await
}

#[derive(Serialize)]
struct CorrectBody<'a> {
    site: &'a str,
    is_php: bool,
}

pub async fn correct_detection(site: &str, is_php: bool) -> Result<Value, String> {
    authed_post_json(&format!("{NS}Sites/Correct"), &CorrectBody { site, is_php }).await
}
