//! `open-easy-web-server`のサイト操作API(フォルダー作成・AI判定・訂正は Connect の `Sites/*`、
//! アップロードも Connect の `Sites/Upload`)への薄い`fetch()`ラッパー。全て`Authorization: Bearer`
//! セッショントークンが必要(`api_auth::saved_token()`)。
//! 方式は Connect(POST、RS-JSON)(`POST /open_easy_web.admin.v1.<Service>/<Method>`、POST、RS-JSON)で、REST は使わない
//! (2026-11-04 方針)。ファイルは base64 で JSON に載せ、合計が大きいときは何回かに分けて送る。

use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{FileList, Headers, Request, RequestInit, RequestMode, Response};

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

/// 1回のリクエストに載せる中身の合計(バイト)。base64 で約 4/3 倍になり、サーバーの本文の上限は 128 MiB。
const BATCH_RAW_MAX: f64 = 48.0 * 1024.0 * 1024.0;
/// 1ファイルの上限(バイト)。これを超えるファイルは送れない(サーバーの本文の上限による)。
const FILE_RAW_MAX: f64 = 90.0 * 1024.0 * 1024.0;

#[derive(Serialize)]
struct UploadItem {
    path: String,
    data_base64: String,
}

#[derive(Serialize)]
struct UploadBody<'a> {
    site: &'a str,
    files: Vec<UploadItem>,
}

/// `File` の中身を base64 文字列にする(`FileReader.readAsDataURL` の `data:…;base64,` より後ろを使う)。
async fn file_to_base64(file: &web_sys::File) -> Result<String, String> {
    let reader = web_sys::FileReader::new().map_err(|e| format!("FileReader init failed: {e:?}"))?;
    let r2 = reader.clone();
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let rd = r2.clone();
        let onload = Closure::once(move |_e: web_sys::Event| {
            let _ = resolve.call1(&JsValue::NULL, &rd.result().unwrap_or(JsValue::NULL));
        });
        let onerror = Closure::once(move |_e: web_sys::Event| {
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("file read error"));
        });
        r2.set_onload(Some(onload.as_ref().unchecked_ref()));
        r2.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onload.forget();
        onerror.forget();
    });
    reader.read_as_data_url(file).map_err(|e| format!("readAsDataURL failed: {e:?}"))?;
    let v = JsFuture::from(promise).await.map_err(|e| format!("file read failed: {e:?}"))?;
    let s = v.as_string().ok_or("unexpected FileReader result")?;
    s.split_once(',').map(|(_, b)| b.to_string()).ok_or_else(|| "unexpected data URL".to_string())
}

/// `<input type="file" multiple>` で選択されたファイル群をアップロードする(`Sites/Upload`、Connect、POST、RS-JSON)。
/// 合計が大きいときは、何回かに分けて送る。
pub async fn upload_files(site: &str, files: &FileList) -> Result<Value, String> {
    let mut batch: Vec<UploadItem> = Vec::new();
    let mut batch_raw = 0.0_f64;
    let mut written: Vec<Value> = Vec::new();
    for i in 0..files.length() {
        let Some(file) = files.get(i) else { continue };
        let size = file.size();
        let name = file.name();
        if size > FILE_RAW_MAX {
            return Err(format!("ファイルが大きすぎます(最大 {} MiB): {name} / File too large (max {} MiB)", (FILE_RAW_MAX / 1048576.0) as u64, (FILE_RAW_MAX / 1048576.0) as u64));
        }
        if !batch.is_empty() && batch_raw + size > BATCH_RAW_MAX {
            written.extend(send_batch(site, std::mem::take(&mut batch)).await?);
            batch_raw = 0.0;
        }
        batch.push(UploadItem { path: name, data_base64: file_to_base64(&file).await? });
        batch_raw += size;
    }
    if !batch.is_empty() {
        written.extend(send_batch(site, batch).await?);
    }
    Ok(serde_json::json!({
        "message_ja": format!("{}件のファイルをアップロードしました。", written.len()),
        "message_en": format!("Uploaded {} file(s).", written.len()),
        "files": written,
    }))
}

async fn send_batch(site: &str, files: Vec<UploadItem>) -> Result<Vec<Value>, String> {
    let v = authed_post_json(&format!("{NS}Sites/Upload"), &UploadBody { site, files }).await?;
    Ok(v.get("files").and_then(|f| f.as_array()).cloned().unwrap_or_default())
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
