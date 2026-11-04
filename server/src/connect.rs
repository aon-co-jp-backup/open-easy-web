//! Connect プロトコルの「単純呼び出し(unary)」で受ける API の全体(2026-11-04新設・同日、既存の REST 風 API をすべて移行)。
//!
//! 形式は `POST /open_easy_web.admin.v1.<Service>/<Method>`、本文は JSON(`content-type: application/json`)。
//! RPoem の管理API(`open_runo.admin.v1.Admin`)と呼び方をそろえつつ、`curl` からそのまま呼べる。
//! **JSON の読み込みは RS-JSON の `parse_secure` / `from_str_secure`(サイズ・深さ・重複キーの上限つき)、書き出しは `rust_json::fast`**。
//! エラーは Connect の形式 `{"code": "...", "message": "..."}`(HTTP ステータスは Connect 仕様の対応表)。
//!
//! 認証: 管理系(`Notify`・`Storage`・`DistSync`・`System`・`Settings`・`LolipopCron`・`AutoUpdate`)は `x-admin-token`。
//! サイト操作(`Sites`)・TOTP 設定などは `Authorization: Bearer <セッション>`(各ハンドラの中で検査する)。
//! ログイン前の `Auth/*`・`Account/*` は認証なし(レート制御や OTP の検証はハンドラ側)。
//!
//! **REST は全廃する方針**(2026-11-04)。REST のまま残すのは、外部の規格や配布の都合が形を決めるものだけ:
//! `/healthz`、`/ime/...`(aruaru-IME の中継と配布)、静的ファイル、および **移行期限つきの互換**として
//! `POST /api/auth/request-otp`・`/api/auth/verify-otp`(`open-upstream-watch` が使っている。Connect 版を配備したら削除する)。
//! メールの確認リンクは API ではなく**画面のURL**(`#/confirm-email-change?token=…`)で、GET では何も確定せず、確認ボタンを押したときだけ
//! `Account/ConfirmEmailChange`(POST、RS-JSON)で確定する。ファイル送信も `Sites/Upload`(base64 の JSON)。
//! スキーマは `proto/open_easy_web/admin/v1/admin.proto`(説明用で、コード生成には使っていない)。

use crate::{storage_maintenance, update_notify, AppState, BoxBody, Req};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const PREFIX: &str = "/open_easy_web.admin.v1.";
/// ファイル送信(`Sites/Upload`)の本文の上限(JSON + base64 なので、中身はおよそ 96 MiB まで)。ほかは 64 KiB。
pub const UPLOAD_MAX: usize = 128 * 1024 * 1024;
/// 本文の上限は `dispatch` の入口で掛けている(Connect は 64 KiB まで)。
const ADMIN_SERVICES: [&str; 7] = ["Notify", "Storage", "DistSync", "System", "Settings", "LolipopCron", "AutoUpdate"];

/// Connect のエラーコードと、対応する HTTP ステータス。
fn status_of(code: &str) -> StatusCode {
    match code {
        "invalid_argument" | "failed_precondition" | "out_of_range" => StatusCode::BAD_REQUEST,
        "unauthenticated" => StatusCode::UNAUTHORIZED,
        "permission_denied" => StatusCode::FORBIDDEN,
        "not_found" => StatusCode::NOT_FOUND,
        "aborted" | "already_exists" => StatusCode::CONFLICT,
        "resource_exhausted" => StatusCode::TOO_MANY_REQUESTS,
        "unimplemented" => StatusCode::NOT_IMPLEMENTED,
        "unavailable" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// 旧形式のエラー(HTTP ステータス)を Connect のコードへ。
fn code_of(status: StatusCode) -> &'static str {
    match status.as_u16() {
        400 => "invalid_argument",
        401 => "unauthenticated",
        403 => "permission_denied",
        404 => "not_found",
        405 | 501 => "unimplemented",
        409 => "aborted",
        412 => "failed_precondition",
        413 | 429 => "resource_exhausted",
        502 | 503 | 504 => "unavailable",
        s if s >= 500 => "internal",
        _ => "unknown",
    }
}

fn reply(status: StatusCode, bytes: Vec<u8>) -> Response<BoxBody> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("cache-control", "no-store")
        .body(Full::new(Bytes::from(bytes)))
        .expect("static response headers are always valid")
}

fn error_body(code: &str, message: &str) -> Vec<u8> {
    #[derive(Serialize)]
    struct E<'a> {
        code: &'a str,
        message: &'a str,
    }
    rust_json::fast::to_vec(&E { code, message }).unwrap_or_else(|_| b"{}".to_vec())
}

fn fail(code: &str, message: impl AsRef<str>) -> Response<BoxBody> {
    reply(status_of(code), error_body(code, message.as_ref()))
}

fn ok(value: &impl Serialize) -> Response<BoxBody> {
    match rust_json::fast::to_vec(value) {
        Ok(b) => reply(StatusCode::OK, b),
        Err(_) => fail("internal", "serialize failed"),
    }
}

/// 既存ハンドラの応答を Connect の形へそろえる。成功(2xx)は 200 にして本文はそのまま。
/// 失敗は `{"error": "..."}` を `{"code","message"}` に直す(HTTP ステータスは変えない)。
async fn finish(resp: Response<BoxBody>) -> Response<BoxBody> {
    let (mut parts, body) = resp.into_parts();
    let status = parts.status;
    if status.is_success() {
        parts.status = StatusCode::OK;
        parts.headers.insert("cache-control", hyper::header::HeaderValue::from_static("no-store"));
        return Response::from_parts(parts, body);
    }
    let bytes = body.collect().await.map(|c| c.to_bytes()).unwrap_or_default();
    if let Ok(v) = crate::json_in::parse_with_limit::<serde_json::Value>(&bytes, 1024 * 1024) {
        if v.get("code").is_some() {
            return reply(status, bytes.to_vec());
        }
        if let Some(msg) = v.get("error").and_then(|m| m.as_str()) {
            return reply(status, error_body(code_of(status), msg));
        }
    }
    reply(status, error_body(code_of(status), &String::from_utf8_lossy(&bytes)))
}

/// 本文が上限を超えたときの応答(`dispatch` の入口の読み込みで使う)。Connect の経路では Connect の形式で返す。
pub fn payload_too_large() -> Response<BoxBody> {
    reply(StatusCode::PAYLOAD_TOO_LARGE, error_body("resource_exhausted", "request body too large / 本文が大きすぎます"))
}

/// 一定時間で比べる(トークンの長さ以外の情報を、応答時間から漏らさない)。
fn token_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn authorize(req: &Req) -> Result<(), Response<BoxBody>> {
    let Ok(expected) = std::env::var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN") else {
        return Err(fail("unavailable", "admin API is disabled (OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN not set)"));
    };
    match req.headers().get("x-admin-token").and_then(|v| v.to_str().ok()) {
        Some(p) if token_eq(p.trim(), &expected) => Ok(()),
        _ => Err(fail("unauthenticated", "invalid or missing x-admin-token")),
    }
}

fn is_json(req: &Req) -> bool {
    req.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or("").trim().eq_ignore_ascii_case("application/json"))
        .unwrap_or(false)
}

/// 本文が空、または JSON オブジェクトであること(RS-JSON の `parse_secure`: サイズ・深さ・重複キーの上限つき)。
fn validate_object(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        return Ok(());
    }
    let text = std::str::from_utf8(bytes).map_err(|e| format!("本文が UTF-8 ではありません: {e}"))?;
    let limits = rust_json::Limits { max_bytes: 64 * 1024, max_depth: 16, reject_duplicate_keys: true };
    match rust_json::parse_secure(text, &limits) {
        Ok(serde_json::Value::Object(_)) => Ok(()),
        Ok(_) => Err("本文は JSON オブジェクトです".to_string()),
        Err(e) => Err(format!("invalid JSON: {e}")),
    }
}

/// 型つきで読む(RS-JSON の `from_str_secure`)。
fn typed<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Response<BoxBody>> {
    update_notify::parse_body(bytes).map_err(|e| fail("invalid_argument", e))
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

#[derive(Deserialize)]
struct IdReq {
    id: String,
}

#[derive(Deserialize)]
struct SiteReq {
    site: String,
}

#[derive(Deserialize)]
struct TokenReq {
    token: String,
}

#[derive(Deserialize)]
struct UploadReq {
    site: String,
    files: Vec<crate::UploadFile>,
}

#[derive(Deserialize)]
struct EnabledReq {
    enabled: bool,
}

#[derive(Deserialize)]
struct LoginModeReq {
    login_mode: String,
}

#[derive(Deserialize)]
struct ProfilesReq {
    profiles: Vec<String>,
}

#[derive(Deserialize)]
struct JobIdEnabledReq {
    id: String,
    enabled: bool,
}

/// `&mut Req` から要求そのものを取り出す(中身は空の要求に置き換える。本文・ヘッダは取り出した側が持つ)。
fn take_req(req: &mut Req) -> Req {
    std::mem::replace(req, hyper::Request::new(Full::new(Bytes::new())))
}

/// `id` に使える文字(QR ログインの識別子など)。クエリ文字列へ組み立てるので、記号は通さない。
fn safe_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `rest` は `Service/Method`(`PREFIX` の後ろ)。
pub async fn dispatch(state: &Arc<AppState>, mut req: Req, rest: &str) -> Response<BoxBody> {
    if req.method() != Method::POST {
        return reply(StatusCode::METHOD_NOT_ALLOWED, error_body("unimplemented", "use POST"));
    }
    if !is_json(&req) {
        let mut r = fail("invalid_argument", "content-type must be application/json");
        *r.status_mut() = StatusCode::UNSUPPORTED_MEDIA_TYPE;
        return r;
    }
    let Some((service, method)) = rest.trim_matches('/').split_once('/') else {
        return fail("not_found", "expected Service/Method");
    };
    let (service, method) = (service.to_string(), method.to_string());
    if ADMIN_SERVICES.contains(&service.as_str()) {
        if let Err(r) = authorize(&req) {
            return r;
        }
    }
    let bytes = req.body().clone().collect().await.map(|c| c.to_bytes()).unwrap_or_default();
    // `Sites/Upload` は本文が大きいので、ここでは検証せず、読み込み(RS-JSON、上限 UPLOAD_MAX)の中で検証する。
    if !(service == "Sites" && method == "Upload") {
        if let Err(e) = validate_object(&bytes) {
            return fail("invalid_argument", e);
        }
    }
    let resp = route(state, &mut req, &service, &method, &bytes).await;
    finish(resp).await
}

async fn route(state: &Arc<AppState>, req: &mut Req, service: &str, method: &str, bytes: &[u8]) -> Response<BoxBody> {
    // 引数なしのメソッドは、空または `{}` を受け付ける(`validate_object` 済み)。
    let state_ref: &AppState = state.as_ref();
    match (service, method) {
        // ---- 更新のお知らせメール ----
        ("Notify", "Get") => ok(&state.update_notify.view(state.smtp.is_some())),
        ("Notify", "SetRecipients") => match typed::<RecipientsReq>(bytes) {
            Ok(b) => match state.update_notify.set_recipients(b.emails) {
                Ok(list) => ok(&serde_json::json!({ "recipients": list })),
                Err(e) => fail("invalid_argument", e),
            },
            Err(r) => r,
        },
        ("Notify", "Send") => match typed::<update_notify::SendRequest>(bytes) {
            Ok(b) => match state.update_notify.send(state.smtp.clone(), b, false).await {
                Ok(v) => ok(&v),
                Err(e) => fail("failed_precondition", e),
            },
            Err(r) => r,
        },
        ("Notify", "SendTest") => {
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

        // ---- ストレージ(HDD/SSD)のメンテナンス ----
        ("Storage", "GetMaintenance") => ok(&state.storage_maintenance.view()),
        ("Storage", "SetSettings") => match storage_maintenance::parse_body::<storage_maintenance::Settings>(bytes) {
            Ok(s) => match state.storage_maintenance.set_settings(s) {
                Ok(s) => ok(&s),
                Err(e) => fail("invalid_argument", e),
            },
            Err(e) => fail("invalid_argument", e),
        },
        ("Storage", "RunHealth") => {
            let sm = Arc::clone(&state.storage_maintenance);
            match tokio::task::spawn_blocking(move || sm.run_health()).await {
                Ok(v) => ok(&v),
                Err(_) => fail("internal", "task failed"),
            }
        }
        ("Storage", "RunTidy") | ("Storage", "RunOptimize") => {
            // `apply` を明示した(true)ときだけ、実際に実行する。省くと、試し実行・計画だけ。
            let apply = if bytes.is_empty() {
                false
            } else {
                match typed::<ApplyReq>(bytes) {
                    Ok(b) => b.apply,
                    Err(r) => return r,
                }
            };
            let sm = Arc::clone(&state.storage_maintenance);
            let tidy = method == "RunTidy";
            match tokio::task::spawn_blocking(move || if tidy { sm.run_tidy(apply) } else { sm.run_optimize(apply) }).await {
                Ok(v) => ok(&v),
                Err(_) => fail("internal", "task failed"),
            }
        }

        // ---- 分散同期先・ディザスタ用退避先 ----
        ("DistSync", "ListTargets") => crate::dist_sync::list_targets(&state.dist_sync).await,
        ("DistSync", "RegisterTarget") => crate::dist_sync::register_target(&state.dist_sync, take_req(req)).await,
        ("DistSync", "DeleteTarget") => match typed::<IdReq>(bytes) {
            Ok(b) => crate::dist_sync::delete_target(&state.dist_sync, &b.id).await,
            Err(r) => r,
        },
        ("DistSync", "SetDisasterFallback") => crate::dist_sync::set_disaster_fallback(&state.dist_sync, take_req(req)).await,
        ("DistSync", "FirstTimeSetup") => crate::dist_sync::run_first_time_setup(&state.dist_sync).await,

        // ---- システムの状況 ----
        ("System", "GetMemory") => ok(&crate::system_memory::snapshot()),
        ("System", "GetDisk") => ok(&crate::system_disk::snapshot()),
        ("System", "GetMemoryBreakdown") => memory_breakdown().await,
        ("System", "GetDiskBreakdown") => disk_breakdown().await,

        // ---- 設定: ログイン方式・電源プロファイル ----
        ("Settings", "GetLoginMode") => ok(&serde_json::json!({"login_mode": state.auth.login_mode()})),
        ("Settings", "SetLoginMode") => match typed::<LoginModeReq>(bytes) {
            Ok(p) => {
                if !crate::auth::is_valid_login_mode(&p.login_mode) {
                    return fail("invalid_argument", "login_mode must be one of: otp, qr, otp_qr");
                }
                state.auth.set_login_mode(&p.login_mode);
                ok(&serde_json::json!({"ok": true, "login_mode": p.login_mode}))
            }
            Err(r) => r,
        },
        ("Settings", "GetPowerProfile") => {
            let flags = state.power_profile.get();
            ok(&serde_json::json!({ "profiles": flags.active_pref_values(), "labels": flags.active_labels() }))
        }
        ("Settings", "SetPowerProfile") => match typed::<ProfilesReq>(bytes) {
            Ok(p) => match crate::power_profile::PowerProfileFlags::from_pref_values(&p.profiles) {
                Ok(flags) => {
                    state.power_profile.set(flags);
                    // memory_saver の実効果: このVPS上の各サービス(systemdユニット)へ cgroup 経由の MemoryHigh を適用する。
                    // ブロッキングI/O(systemctl起動)なので spawn_blocking へ退避する。
                    let governor_results = tokio::task::spawn_blocking(move || crate::memory_governor::apply(flags.memory_saver)).await.unwrap_or_default();
                    ok(&serde_json::json!({
                        "profiles": flags.active_pref_values(),
                        "labels": flags.active_labels(),
                        "ai_memory_governor": governor_results,
                    }))
                }
                Err(bad) => fail("invalid_argument", format!("unknown power profile value: {bad}")),
            },
            Err(r) => r,
        },

        // ---- ロリポップ!向け時間指定オートクロール ----
        ("LolipopCron", "ListJobs") => {
            let cron = &state.lolipop_cron;
            ok(&serde_json::json!({
                "tz_offset_hours": cron.tz_offset_hours(),
                "jobs": cron.list().iter().map(crate::lolipop_cron::job_view).collect::<Vec<_>>(),
            }))
        }
        ("LolipopCron", "CreateJob") => match typed::<crate::lolipop_cron::NewJob>(bytes) {
            Ok(new) => match state.lolipop_cron.add(new) {
                Ok(job) => ok(&crate::lolipop_cron::job_view(&job)),
                Err(e) => fail("invalid_argument", e.to_string()),
            },
            Err(r) => r,
        },
        ("LolipopCron", "DeleteJob") => match typed::<IdReq>(bytes) {
            Ok(b) => match state.lolipop_cron.delete(&b.id) {
                Ok(true) => ok(&serde_json::json!({"deleted": b.id})),
                Ok(false) => fail("not_found", "job not found"),
                Err(e) => fail("internal", e.to_string()),
            },
            Err(r) => r,
        },
        ("LolipopCron", "RunJob") => match typed::<IdReq>(bytes) {
            Ok(b) => match state.lolipop_cron.get(&b.id) {
                Some(job) => {
                    let result = state.lolipop_cron.run_job(&job).await;
                    ok(&serde_json::json!({"result": result}))
                }
                None => fail("not_found", "job not found"),
            },
            Err(r) => r,
        },
        ("LolipopCron", "SetJobEnabled") => match typed::<JobIdEnabledReq>(bytes) {
            Ok(b) => match state.lolipop_cron.set_enabled(&b.id, b.enabled) {
                Ok(true) => ok(&serde_json::json!({"enabled": b.enabled})),
                Ok(false) => fail("not_found", "job not found"),
                Err(e) => fail("internal", e.to_string()),
            },
            Err(r) => r,
        },

        // ---- 深夜の自動アップデート ----
        ("AutoUpdate", "Get") => {
            // `last_cycle_started_at` が25時間以上更新されていなければ、深夜バックグラウンドループ自体が停止している可能性がある。
            let seconds_since = state.auto_update.seconds_since_last_cycle();
            ok(&serde_json::json!({
                "enabled": state.auto_update.is_enabled(),
                "current_version": crate::auto_update::current_version_display(),
                "scheduler_healthy": seconds_since < 25 * 3600,
                "scheduler_seconds_since_cycle": seconds_since,
            }))
        }
        ("AutoUpdate", "Set") => match typed::<EnabledReq>(bytes) {
            Ok(p) => match state.auto_update.set_enabled(p.enabled) {
                Ok(()) => ok(&serde_json::json!({
                    "enabled": p.enabled,
                    "message_ja": if p.enabled {
                        "深夜バックグラウンド自動アップデートを有効にしました(次の深夜0時から確認を開始します)。"
                    } else {
                        "深夜バックグラウンド自動アップデートを無効にしました。"
                    },
                })),
                Err(e) => fail("internal", format!("failed to persist setting: {e}")),
            },
            Err(r) => r,
        },
        ("AutoUpdate", "CheckNow") => {
            // 長くかかりうるので、バックグラウンドで実行して、確認の開始だけを即座に返す。
            let bind_addr = state.server_bind_addr;
            let stop_accepting = Arc::clone(&state.stop_accepting);
            tokio::spawn(async move {
                if let Err(e) = crate::auto_update::check_and_apply_update(bind_addr, &stop_accepting).await {
                    tracing::warn!(error = %e, "auto_update: manual check-now failed");
                }
            });
            ok(&serde_json::json!({
                "message_ja": "アップデート確認を開始しました。新しいバージョンがあれば自動的に切り替わります。",
                "message_en": "Update check started. If a newer version is available, it will be applied automatically.",
            }))
        }

        // ---- ログイン(ログイン前から呼ばれる。検証はハンドラ側) ----
        ("Auth", "RequestOtp") => crate::request_otp(state_ref, take_req(req)).await,
        ("Auth", "VerifyOtp") => crate::verify_otp(state_ref, take_req(req)).await,
        ("Auth", "TotpLogin") => crate::totp_login(state_ref, take_req(req)).await,
        ("Auth", "QrLoginStart") => crate::qr_login_start(state_ref, take_req(req)).await,
        ("Auth", "QrLoginConfirm") => crate::qr_login_confirm(state_ref, take_req(req)).await,
        ("Auth", "QrLoginFinish") => crate::qr_login_finish(state_ref, take_req(req)).await,
        ("Auth", "QrLoginStatus") | ("Auth", "QrLoginWhoami") => match typed::<IdReq>(bytes) {
            Ok(b) if safe_id(&b.id) => {
                if let Ok(uri) = format!("/?id={}", b.id).parse() {
                    *req.uri_mut() = uri;
                }
                if method == "QrLoginStatus" {
                    crate::qr_login_status(state_ref, req).await
                } else {
                    crate::qr_login_whoami(state_ref, req).await
                }
            }
            Ok(_) => fail("invalid_argument", "invalid id"),
            Err(r) => r,
        },
        ("Auth", "Logout") => crate::logout(state_ref, take_req(req)).await,
        ("Auth", "TotpSetup") => crate::totp_setup(state_ref, req).await,
        ("Auth", "TotpEnable") => crate::totp_enable(state_ref, take_req(req)).await,
        ("Auth", "TotpDisable") => crate::totp_disable(state_ref, req).await,
        ("Auth", "RequestEmailChange") => crate::request_email_change(state_ref, take_req(req)).await,

        // ---- アカウント(ログイン前から。トークン自体が資格情報) ----
        // メールの確認リンクは画面のURL。確認ボタンを押したときだけ、ここで確定する(GET では何も起きない)。
        ("Account", "ConfirmEmailChange") => match typed::<TokenReq>(bytes) {
            Ok(b) => crate::confirm_email_change(state_ref, &b.token).await,
            Err(r) => r,
        },

        // ---- サイト操作(セッション必須) ----
        ("Sites", m) => {
            if let Err(unauthorized) = crate::require_session(state_ref, req) {
                return unauthorized;
            }
            if m == "Upload" {
                // ファイル送信: { "site": "…", "files": [ { "path": "相対パス", "data_base64": "…" } ] }
                return match crate::json_in::parse_with_limit::<UploadReq>(bytes, UPLOAD_MAX) {
                    Ok(b) => crate::upload_files(state_ref, &b.site, b.files).await,
                    Err(e) => fail("invalid_argument", e),
                };
            }
            let site = match typed::<SiteReq>(bytes) {
                Ok(b) => b.site,
                Err(r) => return r,
            };
            match m {
                "Delete" => crate::delete_site(state_ref, &site).await,
                "CreateFolder" => crate::create_folder(state_ref, &site).await,
                "DetectAndConfigure" => crate::detect_and_configure(state_ref, &site).await,
                "Correct" => crate::correct_detection(state_ref, &site, take_req(req)).await,
                "RegisterAppserver" => crate::register_appserver(&site, take_req(req)).await,
                _ => fail("not_found", format!("unknown method: Sites/{m}")),
            }
        }

        _ => fail("not_found", format!("unknown method: {service}/{method}")),
    }
}

async fn memory_breakdown() -> Response<BoxBody> {
    use crate::{aruaru_llm_client, memory_breakdown as mb, system_memory};
    let mem_snapshot = system_memory::snapshot();
    let candidates = tokio::task::spawn_blocking(mb::other_process_candidates).await.unwrap_or_default();
    let items: Vec<String> = candidates.iter().map(|(comm, _)| comm.clone()).collect();
    let categories = mb::fixed_categories();
    let classified = match aruaru_llm_client::classify_many(&items, &categories).await {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("aruaru-llm classify failed, marking as unclassified: {err}");
            Vec::new()
        }
    };
    let category_of: std::collections::HashMap<&str, &str> = classified.iter().map(|(item, cat)| (item.as_str(), cat.as_str())).collect();
    let other_process_categories: Vec<(String, String, u64)> = candidates
        .into_iter()
        .map(|(comm, bytes)| {
            let category = category_of.get(comm.as_str()).map(|c| c.to_string()).unwrap_or_else(mb::unclassified_label);
            (comm, category, bytes)
        })
        .collect();
    let used_bytes = mem_snapshot.used_bytes;
    let breakdown = tokio::task::spawn_blocking(move || mb::build(used_bytes, other_process_categories))
        .await
        .unwrap_or_else(|_| mb::build(used_bytes, Vec::new()));
    ok(&breakdown)
}

async fn disk_breakdown() -> Response<BoxBody> {
    use crate::{aruaru_llm_client, disk_breakdown as db};
    let (apps, extensions) = tokio::task::spawn_blocking(|| (db::app_disk_usage(), db::other_extension_breakdown()))
        .await
        .unwrap_or((Vec::new(), Vec::new()));
    let items: Vec<String> = extensions.iter().map(|e| e.extension.clone()).collect();
    let categories: Vec<String> = db::DISK_CATEGORIES.iter().map(|s| s.to_string()).collect();
    let classified = match aruaru_llm_client::classify_many(&items, &categories).await {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("aruaru-llm classify failed, marking as unclassified: {err}");
            Vec::new()
        }
    };
    let categories_breakdown = db::build_category_breakdown(&extensions, &classified);
    ok(&serde_json::json!({ "apps": apps, "extensions": extensions, "categories": categories_breakdown }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_map_to_connect_http_statuses() {
        assert_eq!(status_of("invalid_argument"), StatusCode::BAD_REQUEST);
        assert_eq!(status_of("unauthenticated"), StatusCode::UNAUTHORIZED);
        assert_eq!(status_of("not_found"), StatusCode::NOT_FOUND);
        assert_eq!(status_of("aborted"), StatusCode::CONFLICT);
        assert_eq!(status_of("unavailable"), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(status_of("whatever"), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn legacy_statuses_map_to_connect_codes() {
        assert_eq!(code_of(StatusCode::BAD_REQUEST), "invalid_argument");
        assert_eq!(code_of(StatusCode::UNAUTHORIZED), "unauthenticated");
        assert_eq!(code_of(StatusCode::NOT_FOUND), "not_found");
        assert_eq!(code_of(StatusCode::CONFLICT), "aborted");
        assert_eq!(code_of(StatusCode::SERVICE_UNAVAILABLE), "unavailable");
        assert_eq!(code_of(StatusCode::INTERNAL_SERVER_ERROR), "internal");
    }

    #[test]
    fn token_comparison_is_exact() {
        assert!(token_eq("abc", "abc"));
        assert!(!token_eq("abc", "abd"));
        assert!(!token_eq("abc", "abcd"));
        assert!(!token_eq("", "a"));
    }

    #[test]
    fn body_must_be_an_object_without_duplicate_keys() {
        assert!(validate_object(b"").is_ok());
        assert!(validate_object(b"{}").is_ok());
        assert!(validate_object(b"[]").is_err());
        assert!(validate_object(b"{\"a\":1,\"a\":2}").is_err());
        assert!(validate_object(b"not json").is_err());
    }

    #[test]
    fn ids_cannot_inject_query_syntax() {
        assert!(safe_id("abc-DEF_123"));
        assert!(!safe_id(""));
        assert!(!safe_id("a&b=c"));
        assert!(!safe_id("a b"));
        assert!(!safe_id("../x"));
    }

    #[tokio::test]
    async fn finish_converts_legacy_errors_and_normalizes_success() {
        let legacy = Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Full::new(Bytes::from_static(br#"{"error":"nope"}"#)))
            .unwrap();
        let r = finish(legacy).await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        let body = r.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["code"], "unauthenticated");
        assert_eq!(v["message"], "nope");

        let created = Response::builder().status(StatusCode::CREATED).body(Full::new(Bytes::from_static(b"{}"))).unwrap();
        assert_eq!(finish(created).await.status(), StatusCode::OK);
    }
}
