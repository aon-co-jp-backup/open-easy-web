//! Connect プロトコルの「単純呼び出し(unary)」で受ける管理API(2026-11-04新設、ユーザー指示「POST + RS-JSON にして」)。
//!
//! 形式は `POST /open_easy_web.admin.v1.<Service>/<Method>`、本文は JSON(`content-type: application/json`)。
//! RPoem の管理API(`open_runo.admin.v1.Admin`、gRPC・gRPC-web・Connect)と呼び方をそろえつつ、`curl` から
//! そのまま呼べる。**JSON の読み込みは RS-JSON の `from_str_secure`(サイズ・深さ・重複キーの上限つき)、書き出しは
//! `rust_json::fast`**。エラーは Connect の形式 `{"code": "...", "message": "..."}`(HTTP ステータスは Connect 仕様の対応表)。
//!
//! 認証は既存の管理APIと同じ `x-admin-token`。**REST は全廃する方針**(2026-11-04)で、お知らせ・ストレージは REST の経路を持たず
//! Connect のみ。他の既存の `/admin/...`(REST 風)は、触るときに順にここへ移す。スキーマは `proto/open_easy_web/admin/v1/admin.proto` に書いてある
//! (今は説明用で、コード生成には使っていない)。
//!
//! メソッド: `Notify/{Get, SetRecipients, Send, SendTest}`、
//! `Storage/{GetMaintenance, SetSettings, RunHealth, RunTidy, RunOptimize}`。

use crate::{update_notify, storage_maintenance, AppState, BoxBody};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const PREFIX: &str = "/open_easy_web.admin.v1.";
const MAX_BODY: usize = 16 * 1024;

/// Connect のエラーコードと、対応する HTTP ステータス。
fn status_of(code: &str) -> StatusCode {
    match code {
        "invalid_argument" | "failed_precondition" | "out_of_range" => StatusCode::BAD_REQUEST,
        "unauthenticated" => StatusCode::UNAUTHORIZED,
        "permission_denied" => StatusCode::FORBIDDEN,
        "not_found" => StatusCode::NOT_FOUND,
        "unimplemented" => StatusCode::NOT_IMPLEMENTED,
        "unavailable" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn reply(status: StatusCode, bytes: Vec<u8>) -> Response<BoxBody> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(bytes)))
        .expect("static response headers are always valid")
}

fn fail(code: &str, message: impl Into<String>) -> Response<BoxBody> {
    #[derive(Serialize)]
    struct E {
        code: String,
        message: String,
    }
    let body = rust_json::fast::to_vec(&E { code: code.to_string(), message: message.into() }).unwrap_or_else(|_| b"{}".to_vec());
    reply(status_of(code), body)
}

fn ok(value: &impl Serialize) -> Response<BoxBody> {
    match rust_json::fast::to_vec(value) {
        Ok(b) => reply(StatusCode::OK, b),
        Err(_) => fail("internal", "serialize failed"),
    }
}

/// 一定時間で比べる(トークンの長さ以外の情報を、応答時間から漏らさない)。
fn token_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn authorize(req: &Request<Incoming>) -> Result<(), Response<BoxBody>> {
    let Ok(expected) = std::env::var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN") else {
        return Err(fail("unavailable", "admin API is disabled (OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN not set)"));
    };
    match req.headers().get("x-admin-token").and_then(|v| v.to_str().ok()) {
        Some(p) if token_eq(p, &expected) => Ok(()),
        _ => Err(fail("unauthenticated", "invalid or missing x-admin-token")),
    }
}

fn is_json(req: &Request<Incoming>) -> bool {
    req.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or("").trim().eq_ignore_ascii_case("application/json"))
        .unwrap_or(false)
}

/// 本文が空、または `{}` のようなオブジェクトであること(引数なしのメソッド用)。
fn expect_empty(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        return Ok(());
    }
    let text = std::str::from_utf8(bytes).map_err(|e| format!("本文が UTF-8 ではありません: {e}"))?;
    let limits = rust_json::Limits { max_bytes: MAX_BODY, max_depth: 16, reject_duplicate_keys: true };
    match rust_json::parse_secure(text, &limits) {
        Ok(serde_json::Value::Object(_)) => Ok(()),
        Ok(_) => Err("本文は JSON オブジェクトです".to_string()),
        Err(e) => Err(format!("invalid JSON: {e}")),
    }
}

#[derive(Deserialize)]
struct RecipientsReq {
    emails: Vec<String>,
}

#[derive(Deserialize, Default)]
struct ApplyReq {
    #[serde(default)]
    apply: bool,
}

/// `rest` は `Service/Method`(`PREFIX` の後ろ)。
pub async fn dispatch(state: &Arc<AppState>, req: Request<Incoming>, rest: &str) -> Response<BoxBody> {
    if req.method() != Method::POST {
        return reply(StatusCode::METHOD_NOT_ALLOWED, rust_json::fast::to_vec(&serde_json::json!({"code": "unimplemented", "message": "use POST"})).unwrap_or_default());
    }
    if !is_json(&req) {
        let mut r = fail("invalid_argument", "content-type must be application/json");
        *r.status_mut() = StatusCode::UNSUPPORTED_MEDIA_TYPE;
        return r;
    }
    if let Err(r) = authorize(&req) {
        return r;
    }
    let Some((service, method)) = rest.trim_matches('/').split_once('/') else {
        return fail("not_found", "expected Service/Method");
    };
    let (service, method) = (service.to_string(), method.to_string());
    let bytes = match http_body_util::Limited::new(req.into_body(), MAX_BODY).collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return fail("invalid_argument", format!("failed to read body: {e}")),
    };
    match (service.as_str(), method.as_str()) {
        ("Notify", "Get") => match expect_empty(&bytes) {
            Ok(()) => ok(&state.update_notify.view(state.smtp.is_some())),
            Err(e) => fail("invalid_argument", e),
        },
        ("Notify", "SetRecipients") => match update_notify::parse_body::<RecipientsReq>(&bytes) {
            Ok(b) => match state.update_notify.set_recipients(b.emails) {
                Ok(list) => ok(&serde_json::json!({ "recipients": list })),
                Err(e) => fail("invalid_argument", e),
            },
            Err(e) => fail("invalid_argument", e),
        },
        ("Notify", "Send") => match update_notify::parse_body::<update_notify::SendRequest>(&bytes) {
            Ok(b) => match state.update_notify.send(state.smtp.clone(), b, false).await {
                Ok(v) => ok(&v),
                Err(e) => fail("failed_precondition", e),
            },
            Err(e) => fail("invalid_argument", e),
        },
        ("Notify", "SendTest") => match expect_empty(&bytes) {
            Ok(()) => {
                let b = update_notify::SendRequest {
                    kind: "info".to_string(),
                    subject: "テスト送信 / Test notice".to_string(),
                    body: "これは更新のお知らせ機能のテストです。 / This is a test of the update-notice feature.".to_string(),
                };
                match state.update_notify.send(state.smtp.clone(), b, true).await {
                    Ok(v) => ok(&v),
                    Err(e) => fail("failed_precondition", e),
                }
            }
            Err(e) => fail("invalid_argument", e),
        },
        ("Storage", "GetMaintenance") => match expect_empty(&bytes) {
            Ok(()) => ok(&state.storage_maintenance.view()),
            Err(e) => fail("invalid_argument", e),
        },
        ("Storage", "SetSettings") => match storage_maintenance::parse_body::<storage_maintenance::Settings>(&bytes) {
            Ok(s) => match state.storage_maintenance.set_settings(s) {
                Ok(s) => ok(&s),
                Err(e) => fail("invalid_argument", e),
            },
            Err(e) => fail("invalid_argument", e),
        },
        ("Storage", "RunHealth") => match expect_empty(&bytes) {
            Ok(()) => {
                let sm = Arc::clone(&state.storage_maintenance);
                match tokio::task::spawn_blocking(move || sm.run_health()).await {
                    Ok(v) => ok(&v),
                    Err(_) => fail("internal", "task failed"),
                }
            }
            Err(e) => fail("invalid_argument", e),
        },
        ("Storage", "RunTidy") | ("Storage", "RunOptimize") => {
            // `apply` を明示した(true)ときだけ、実際に実行する。省くと、試し実行・計画だけ。
            let apply = if bytes.is_empty() {
                false
            } else {
                match storage_maintenance::parse_body::<ApplyReq>(&bytes) {
                    Ok(b) => b.apply,
                    Err(e) => return fail("invalid_argument", e),
                }
            };
            let sm = Arc::clone(&state.storage_maintenance);
            let tidy = method == "RunTidy";
            match tokio::task::spawn_blocking(move || if tidy { sm.run_tidy(apply) } else { sm.run_optimize(apply) }).await {
                Ok(v) => ok(&v),
                Err(_) => fail("internal", "task failed"),
            }
        }
        _ => fail("not_found", format!("unknown method: {service}/{method}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_map_to_connect_http_statuses() {
        assert_eq!(status_of("invalid_argument"), StatusCode::BAD_REQUEST);
        assert_eq!(status_of("unauthenticated"), StatusCode::UNAUTHORIZED);
        assert_eq!(status_of("not_found"), StatusCode::NOT_FOUND);
        assert_eq!(status_of("unavailable"), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(status_of("whatever"), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn token_comparison_is_exact() {
        assert!(token_eq("abc", "abc"));
        assert!(!token_eq("abc", "abd"));
        assert!(!token_eq("abc", "abcd"));
        assert!(!token_eq("", "a"));
    }

    #[test]
    fn empty_body_rules() {
        assert!(expect_empty(b"").is_ok());
        assert!(expect_empty(b"{}").is_ok());
        assert!(expect_empty(b"[]").is_err());
        assert!(expect_empty(b"{\"a\":1,\"a\":2}").is_err());
    }
}
