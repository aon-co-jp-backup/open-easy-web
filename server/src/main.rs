//! open-easy-web-server: フォルダー作成・アップロード・AI自動PHP判定・
//! nginx+PHP-FPM自動構成を担うバックエンド。
//!
//! open-easy-web本体(WASM UI)はこれまでサーバー実体を持たなかった
//! (`scripts/serve.sh`は静的ファイル配信のみ)。このバイナリは、UIから
//! フォルダー作成・アップロード・AI判定・自動構成を呼べるようにするための
//! 最小限のREST APIを、tokio/hyperを直接使って(重量級フレームワーク非
//! 依存)提供する。

mod appserver_registration;
mod auth;
mod mail;
mod php_detector;
mod sms;
mod tls;
mod totp;
mod upload;
mod users;
mod vhost;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

type BoxBody = Full<Bytes>;

struct AppState {
    /// サイトのwebroot群を格納する親ディレクトリ(例: `/var/www`)。
    sites_root: PathBuf,
    /// AI判定の重み永続化ファイル。
    ai_state_path: PathBuf,
    /// PHP-FPM用nginx vhostテンプレート(HTTPS、証明書取得後に使う)。
    nginx_template: PathBuf,
    /// 証明書取得前に使う、HTTPのみのnginx vhostテンプレート。
    nginx_http_only_template: PathBuf,
    /// vhostの書き込み先(`/etc/nginx/conf.d`)。
    nginx_conf_d: PathBuf,
    /// Let's Encryptアカウント登録に使うメールアドレス。
    acme_email: String,
    bind_ip: String,
    php_fpm_upstream: String,
    /// WASMバンドルの配信元ディレクトリ。
    static_dir: PathBuf,
    weights: Mutex<php_detector::PhpSignalWeights>,
    auth: auth::AuthStore,
    users: users::UserStore,
    smtp: Option<mail::SmtpConfig>,
    sms: Option<sms::SmsConfig>,
}

/// 唯一ログインを許可するアカウント(ユーザー指示、2026-07-15、
/// セキュリティ上の理由で公開登録を廃止し固定アカウントのみに限定)。
/// 起動のたびに`seed_fixed_account`で冪等に保証される——`/api/auth/register`
/// は無効化されているため、これ以外のアカウントは今後一切作成できない。
/// 実際の個人情報をソースに残さないため、値は環境変数から読む
/// (`OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL`は必須、backup_email/phoneは任意)。
fn fixed_account_email() -> String {
    std::env::var("OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL")
        .expect("OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL must be set — no default account is hardcoded")
}

impl AppState {
    fn from_env() -> Self {
        let sites_root = env_path("OPEN_EASYWEB_SITES_ROOT", "/var/www");
        let ai_state_path = env_path(
            "OPEN_EASYWEB_AI_STATE",
            "/var/www/.open-easy-web-ai-state.json",
        );
        let weights = php_detector::load_weights(&ai_state_path);
        let users = users::UserStore::load(env_path(
            "OPEN_EASYWEB_USERS_STATE",
            "/var/www/.open-easy-web-users.json",
        ));
        let fixed_account_email = fixed_account_email();
        let fixed_account_phone = std::env::var("OPEN_EASYWEB_FIXED_ACCOUNT_PHONE").ok();
        let fixed_account_backup_email =
            std::env::var("OPEN_EASYWEB_FIXED_ACCOUNT_BACKUP_EMAIL").ok();
        users.seed_fixed_account(
            &fixed_account_email,
            fixed_account_phone.as_deref(),
            fixed_account_backup_email.as_deref(),
        );
        Self {
            sites_root,
            ai_state_path,
            nginx_template: env_path(
                "OPEN_EASYWEB_PHP_TEMPLATE",
                "deploy/nginx/vhost-php.conf.template",
            ),
            nginx_http_only_template: env_path(
                "OPEN_EASYWEB_PHP_HTTP_ONLY_TEMPLATE",
                "deploy/nginx/vhost-php-http-only.conf.template",
            ),
            nginx_conf_d: env_path("OPEN_EASYWEB_NGINX_CONF_D", "/etc/nginx/conf.d"),
            acme_email: std::env::var("OPEN_EASYWEB_ACME_EMAIL")
                .unwrap_or_else(|_| fixed_account_email.clone()),
            bind_ip: std::env::var("OPEN_EASYWEB_SITE_BIND_IP").unwrap_or_else(|_| "0.0.0.0".into()),
            php_fpm_upstream: std::env::var("OPEN_EASYWEB_PHP_FPM_UPSTREAM")
                .unwrap_or_else(|_| "unix:/run/php-fpm/www.sock".into()),
            static_dir: env_path("OPEN_EASYWEB_STATIC_DIR", "."),
            weights: Mutex::new(weights),
            auth: auth::AuthStore::default(),
            users,
            smtp: mail::SmtpConfig::from_env(),
            sms: sms::SmsConfig::from_env(),
        }
    }

    /// ログインID(主メール・セカンドメール・電話番号のいずれか)から、
    /// アカウントの主メールアドレスを解決する。
    fn resolve_account_email(&self, contact: &str) -> Option<String> {
        if self.users.exists(contact) {
            return Some(contact.to_string());
        }
        if let Some(email) = self.users.find_email_by_backup_email(contact) {
            return Some(email);
        }
        self.users.find_email_by_phone(contact)
    }

    fn site_dir(&self, name: &str) -> Option<PathBuf> {
        upload::safe_relative_path(name).map(|rel| self.sites_root.join(rel))
    }
}

fn env_path(key: &str, default: &str) -> PathBuf {
    std::env::var(key).map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(default))
}

fn json_response(status: StatusCode, value: &impl serde::Serialize) -> Response<BoxBody> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .expect("static response headers are always valid")
}

fn error_response(status: StatusCode, message: impl Into<String>) -> Response<BoxBody> {
    json_response(status, &serde_json::json!({ "error": message.into() }))
}

/// `/api/sites/<name>/<rest...>` の `<name>` と `<rest>` を取り出す。
fn parse_site_path(path: &str) -> Option<(String, String)> {
    let rest = path.strip_prefix("/api/sites/")?;
    let (name, rest) = rest.split_once('/')?;
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), format!("/{rest}")))
}

/// `Authorization: Bearer <token>` を検証する。open-easy-web全体で唯一の
/// 認証方式(固定パスワードを廃したメールOTPログイン)に統一されている
/// ため、サイト操作系エンドポイントは全てこれを通す。
fn require_session(state: &AppState, req: &Request<Incoming>) -> Result<String, Response<BoxBody>> {
    let token = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let Some(token) = token else {
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            "missing Authorization: Bearer <token> header / Authorizationヘッダがありません",
        ));
    };
    state.auth.session_email(token).ok_or_else(|| {
        error_response(
            StatusCode::UNAUTHORIZED,
            "session expired or invalid, please log in again / セッションが無効です。再度ログインしてください",
        )
    })
}

async fn dispatch(state: Arc<AppState>, req: Request<Incoming>) -> Response<BoxBody> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // `DELETE /api/sites/<name>` (アクション部分なし) はドメイン削除。
    if method == Method::DELETE {
        if let Some(site) = path.strip_prefix("/api/sites/").filter(|s| !s.is_empty() && !s.contains('/')) {
            if let Err(unauthorized) = require_session(&state, &req) {
                return unauthorized;
            }
            return delete_site(&state, site).await;
        }
    }

    if let Some((site, action)) = parse_site_path(&path) {
        if let Err(unauthorized) = require_session(&state, &req) {
            return unauthorized;
        }
        return match (&method, action.as_str()) {
            (&Method::POST, "/folder") => create_folder(&state, &site).await,
            (&Method::POST, "/upload") => upload_files(&state, &site, req).await,
            (&Method::POST, "/detect-and-configure") => detect_and_configure(&state, &site).await,
            (&Method::POST, "/correct") => correct_detection(&state, &site, req).await,
            (&Method::POST, "/register-appserver") => register_appserver(&site, req).await,
            _ => error_response(StatusCode::NOT_FOUND, "unknown site action"),
        };
    }

    match (&method, path.as_str()) {
        (&Method::GET, "/healthz") => json_response(StatusCode::OK, &serde_json::json!({"status":"ok"})),
        // 公開の新規登録は無効化済み(ユーザー指示、2026-07-15、セキュリティ
        // 上の理由)。ログイン可能なのは起動時にシードされる固定アカウント
        // (FIXED_ACCOUNT_EMAIL)のみ——`/api/auth/register`自体を存在しない
        // パスにすることで、登録経路があること自体を外部に見せない。
        (&Method::POST, "/api/auth/request-otp") => request_otp(&state, req).await,
        (&Method::POST, "/api/auth/verify-otp") => verify_otp(&state, req).await,
        (&Method::POST, "/api/auth/totp-login") => totp_login(&state, req).await,
        (&Method::POST, "/api/auth/logout") => logout(&state, req).await,
        (&Method::POST, "/api/auth/totp/setup") => totp_setup(&state, &req).await,
        (&Method::POST, "/api/auth/totp/enable") => totp_enable(&state, req).await,
        (&Method::POST, "/api/auth/totp/disable") => totp_disable(&state, &req).await,
        (&Method::POST, "/api/auth/request-email-change") => request_email_change(&state, req).await,
        (&Method::GET, p) if p.starts_with("/api/auth/confirm-email-change") => {
            confirm_email_change(&state, &req).await
        }
        (&Method::GET, "/") => serve_static(&state, "index.html", "text/html; charset=utf-8").await,
        (&Method::GET, p) if p.starts_with("/pkg/") => {
            let rel = p.trim_start_matches('/');
            let content_type = if rel.ends_with(".wasm") {
                "application/wasm"
            } else {
                "application/javascript"
            };
            serve_static(&state, rel, content_type).await
        }
        _ => error_response(StatusCode::NOT_FOUND, "not found"),
    }
}

async fn serve_static(state: &AppState, rel: &str, content_type: &'static str) -> Response<BoxBody> {
    let path = state.static_dir.join(rel);
    match tokio::fs::read(&path).await {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", content_type)
            .body(Full::new(Bytes::from(bytes)))
            .expect("static response headers are always valid"),
        Err(_) => error_response(StatusCode::NOT_FOUND, "not found"),
    }
}

/// `POST /api/sites/:name/folder` — サイト用webrootディレクトリを作成する。
async fn create_folder(state: &AppState, site: &str) -> Response<BoxBody> {
    let Some(dir) = state.site_dir(site) else {
        return error_response(StatusCode::BAD_REQUEST, "invalid site name");
    };
    match tokio::fs::create_dir_all(&dir).await {
        Ok(()) => json_response(
            StatusCode::OK,
            &serde_json::json!({
                "message_ja": "フォルダーを作成しました。",
                "message_en": "Folder created.",
                "path": dir.to_string_lossy(),
            }),
        ),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("mkdir failed: {e}")),
    }
}

/// `POST /api/sites/:name/upload` — `multipart/form-data`でファイル群を
/// webrootへ書き込む。各パートの`filename`をサイトディレクトリからの
/// 相対パスとして扱う(ブラウザの`webkitdirectory`が相対パス込みの
/// ファイル名を送ってくることを利用)。
async fn upload_files(state: &AppState, site: &str, req: Request<Incoming>) -> Response<BoxBody> {
    let Some(dir) = state.site_dir(site) else {
        return error_response(StatusCode::BAD_REQUEST, "invalid site name");
    };

    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !content_type.starts_with("multipart/form-data") {
        return error_response(StatusCode::BAD_REQUEST, "expected multipart/form-data");
    }
    let Some(boundary) = upload::multipart_boundary(&content_type) else {
        return error_response(StatusCode::BAD_REQUEST, "missing multipart boundary");
    };

    let bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };

    let fields = match upload::parse(&bytes, &boundary) {
        Ok(fields) => fields,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, e),
    };

    if tokio::fs::create_dir_all(&dir).await.is_err() {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "failed to prepare site directory");
    }

    let mut written = Vec::new();
    for field in fields {
        let Some(filename) = field.filename else { continue };
        let Some(rel) = upload::safe_relative_path(&filename) else {
            return error_response(
                StatusCode::BAD_REQUEST,
                format!("unsafe file path rejected: {filename}"),
            );
        };
        let dest = dir.join(&rel);
        if let Some(parent) = dest.parent() {
            if tokio::fs::create_dir_all(parent).await.is_err() {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, "failed to create subdirectory");
            }
        }
        if tokio::fs::write(&dest, &field.data).await.is_err() {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("failed to write {filename}"));
        }
        written.push(rel.to_string_lossy().to_string());
    }

    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "message_ja": format!("{}件のファイルをアップロードしました。", written.len()),
            "message_en": format!("Uploaded {} file(s).", written.len()),
            "files": written,
        }),
    )
}

/// `POST /api/sites/:name/detect-and-configure` — AI判定を実行し、PHPと
/// 判定されればnginx+PHP-FPM vhostを自動生成・配置・reloadする。
async fn detect_and_configure(state: &AppState, site: &str) -> Response<BoxBody> {
    let Some(dir) = state.site_dir(site) else {
        return error_response(StatusCode::BAD_REQUEST, "invalid site name");
    };
    let weights = state.weights.lock().unwrap().clone();
    let detection = match php_detector::detect(&dir, &weights) {
        Ok(d) => d,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("detection failed: {e}")),
    };

    if !detection.is_php {
        return json_response(
            StatusCode::OK,
            &serde_json::json!({
                "is_php": false,
                "confidence": detection.confidence,
                "message_ja": "PHPサイトとは判定されませんでした。自動構成は行いません。",
                "message_en": "Not detected as a PHP site; skipping automatic configuration.",
            }),
        );
    }

    let req = vhost::AutoTlsRequest {
        domain: site,
        bind_ip: &state.bind_ip,
        php_fpm_upstream: &state.php_fpm_upstream,
        webroot: &dir,
        http_only_template_path: &state.nginx_http_only_template,
        https_template_path: &state.nginx_template,
        nginx_conf_d: &state.nginx_conf_d,
        acme_email: &state.acme_email,
    };
    match vhost::apply_with_auto_tls(&req) {
        Ok(outcome) => json_response(
            StatusCode::OK,
            &serde_json::json!({
                "is_php": true,
                "confidence": detection.confidence,
                "vhost_path": outcome.vhost_path.to_string_lossy(),
                "https_enabled": outcome.https_enabled,
                "message_ja": if outcome.https_enabled {
                    "🤖 PHPサイトと判定し、nginx+PHP-FPM+HTTPSを自動構成しました。".to_string()
                } else {
                    format!(
                        "🤖 PHPサイトと判定し、nginx+PHP-FPMを自動構成しました(HTTPは有効)。{}",
                        outcome.tls_note.clone().unwrap_or_default()
                    )
                },
                "message_en": if outcome.https_enabled {
                    "🤖 Detected a PHP site and auto-configured nginx+PHP-FPM+HTTPS.".to_string()
                } else {
                    format!(
                        "🤖 Detected a PHP site and auto-configured nginx+PHP-FPM (HTTP only). {}",
                        outcome.tls_note.clone().unwrap_or_default()
                    )
                },
            }),
        ),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// `DELETE /api/sites/:name` — ドメイン登録を取り消す(nginx vhostのみ削除、
/// アップロード済みファイル・取得済み証明書は保持する)。
async fn delete_site(state: &AppState, site: &str) -> Response<BoxBody> {
    if state.site_dir(site).is_none() {
        return error_response(StatusCode::BAD_REQUEST, "invalid site name");
    }
    match vhost::remove(site, &state.nginx_conf_d) {
        Ok(()) => json_response(
            StatusCode::OK,
            &serde_json::json!({
                "message_ja": "ドメインの登録を削除しました(アップロード済みファイル・証明書は保持されています)。",
                "message_en": "Domain registration removed (uploaded files and certificates are preserved).",
            }),
        ),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[derive(serde::Deserialize)]
struct CorrectRequest {
    is_php: bool,
}

/// `POST /api/sites/:name/correct` — 判定結果への手動訂正を受け、
/// AIの重みをEWMA式で補正・永続化する(自己学習)。
async fn correct_detection(state: &AppState, site: &str, req: Request<Incoming>) -> Response<BoxBody> {
    let Some(dir) = state.site_dir(site) else {
        return error_response(StatusCode::BAD_REQUEST, "invalid site name");
    };
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: CorrectRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };

    let mut weights = state.weights.lock().unwrap();
    if let Err(e) = php_detector::correct(&dir, &mut weights, payload.is_php) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("correction failed: {e}"));
    }
    if let Err(e) = php_detector::save_weights(&state.ai_state_path, &weights) {
        tracing::warn!(error = %e, "failed to persist AI weights (correction still applied in-memory)");
    }

    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "message_ja": "訂正を記録し、AIの判定基準を更新しました。",
            "message_en": "Correction recorded; AI detection weights updated.",
            "weights": &*weights,
        }),
    )
}

/// `POST /api/sites/:name/register-appserver` — 「分身の術」構想の仕上げ:
/// このサイトのドメイン(`site`)を、既に稼働中の共有バックエンド
/// (`open-web-server`または`poem-cosmo-tauri`)へ動的登録する
/// (`appserver_registration`モジュール参照)。WASM側の「🔗 共有バックエンド
/// へ登録」ボタンはこのエンドポイントを呼ぶ想定で先に実装されていたが、
/// サーバー側のルート配線自体が漏れていた——2026-07-17に発見・追加。
async fn register_appserver(site: &str, req: Request<Incoming>) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: appserver_registration::RegisterAppserverRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };

    let client = reqwest::Client::new();
    match appserver_registration::register(&client, site, &payload).await {
        Ok(()) => json_response(
            StatusCode::OK,
            &serde_json::json!({
                "message_ja": "共有バックエンドへ登録しました。",
                "message_en": "Registered with the shared backend.",
            }),
        ),
        Err(e) => error_response(StatusCode::BAD_GATEWAY, e.to_string()),
    }
}

#[derive(serde::Deserialize)]
struct ContactRequest {
    /// 主メール・セカンドメール・電話番号のいずれか(ログインIDとして
    /// 入力された値をそのまま渡す)。
    contact: String,
}

/// `POST /api/auth/request-otp` — 登録済みの連絡先(メール1・メール2・
/// 電話番号のいずれか)をIDとしてOTPを発行し、同じ経路(メール宛なら
/// SMTP、電話番号宛ならSMS)で送信する。固定パスワードは一切扱わない。
async fn request_otp(state: &AppState, req: Request<Incoming>) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: ContactRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };

    if state.resolve_account_email(&payload.contact).is_none() {
        return error_response(
            StatusCode::NOT_FOUND,
            "this contact is not registered / この連絡先は登録されていません",
        );
    }

    let auth::RequestOtpOutcome::Issued(code) = state.auth.request_otp(&payload.contact);
    let is_email = payload.contact.contains('@');

    if is_email {
        let Some(smtp) = state.smtp.clone() else {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "SMTP is not configured on this server (OPEN_EASYWEB_SMTP_* env vars missing)",
            );
        };
        match mail::send_otp(smtp, payload.contact.clone(), code).await {
            Ok(()) => json_response(
                StatusCode::OK,
                &serde_json::json!({
                    "message_ja": "ワンタイムパスワードをメールで送信しました。",
                    "message_en": "A one-time password has been sent to your email.",
                }),
            ),
            Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    } else {
        let Some(sms_config) = state.sms.clone() else {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "SMS is not configured on this server (OPEN_EASYWEB_SMS_* env vars missing); use email instead / SMS未設定のためメールをご利用ください",
            );
        };
        match sms::send_otp(sms_config, payload.contact.clone(), code).await {
            Ok(()) => json_response(
                StatusCode::OK,
                &serde_json::json!({
                    "message_ja": "ワンタイムパスワードをSMSで送信しました。",
                    "message_en": "A one-time password has been sent via SMS.",
                }),
            ),
            Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }
}

#[derive(serde::Deserialize)]
struct TotpLoginRequest {
    account_email: String,
    totp_code: String,
}

/// `POST /api/auth/totp-login` — メールOTPを一切経由せず、認証アプリの
/// TOTPコードだけでログインする(ユーザー指示、2026-07-17: 「メールOTP
/// または2FA、どちらか一方だけでログイン可能」)。既存の`verify-otp`は
/// これまで通りメールOTPが必須(2FA有効時はさらにTOTPも必須)のままで、
/// 変更していない——このエンドポイントは「TOTPだけで完結する」もう
/// 一方の代替経路として追加した。TOTPが有効化されていないアカウントでは
/// 使用できない(そのアカウントにとっての2つ目の要素が存在しないため)。
async fn totp_login(state: &AppState, req: Request<Incoming>) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: TotpLoginRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };

    if !state.users.totp_enabled(&payload.account_email) {
        return error_response(
            StatusCode::FORBIDDEN,
            "TOTP is not enabled for this account / このアカウントは認証アプリの2段階認証が有効になっていません",
        );
    }

    let secret = state.users.totp_secret(&payload.account_email).unwrap_or_default();
    let Some(secret_bytes) = totp::base32_to_secret(&secret) else {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "invalid stored TOTP secret");
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if !totp::verify_code(&secret_bytes, &payload.totp_code, now) {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "invalid authenticator code / 認証アプリのコードが正しくありません",
        );
    }

    let token = state.auth.create_session(&payload.account_email);
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "token": token,
            "email": payload.account_email,
            "message_ja": "認証アプリのコードでログインしました。",
            "message_en": "Logged in with your authenticator app code.",
        }),
    )
}

#[derive(serde::Deserialize)]
struct VerifyOtpRequest {
    contact: String,
    code: String,
    /// アカウントでTOTP2FAが有効な場合に必須の、認証アプリの6桁コード。
    #[serde(default)]
    totp_code: Option<String>,
}

/// `POST /api/auth/verify-otp` — OTPを検証し、成功すればセッション
/// トークンを返す。`contact`にどの登録済み連絡先を入力しても、同じ
/// アカウント(主メール)に対するセッションが発行される。
async fn verify_otp(state: &AppState, req: Request<Incoming>) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: VerifyOtpRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };

    let Some(account_email) = state.resolve_account_email(&payload.contact) else {
        return error_response(StatusCode::NOT_FOUND, "unknown contact");
    };

    match state.auth.consume_otp(&payload.contact, &payload.code) {
        Ok(()) => {
            if state.users.totp_enabled(&account_email) {
                let Some(totp_code) = payload.totp_code.as_deref() else {
                    return json_response(
                        StatusCode::OK,
                        &serde_json::json!({
                            "totp_required": true,
                            "message_ja": "このアカウントは認証アプリの2段階認証が有効です。6桁のコードを入力してください。",
                            "message_en": "This account requires an authenticator app code. Please enter the 6-digit code.",
                        }),
                    );
                };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let secret = state.users.totp_secret(&account_email).unwrap_or_default();
                let Some(secret_bytes) = totp::base32_to_secret(&secret) else {
                    return error_response(StatusCode::INTERNAL_SERVER_ERROR, "invalid stored TOTP secret");
                };
                if !totp::verify_code(&secret_bytes, totp_code, now) {
                    return error_response(
                        StatusCode::UNAUTHORIZED,
                        "invalid authenticator code / 認証アプリのコードが正しくありません",
                    );
                }
            }

            let token = state.auth.create_session(&account_email);
            json_response(
                StatusCode::OK,
                &serde_json::json!({
                    "token": token,
                    "email": account_email,
                    "message_ja": "ログインしました。",
                    "message_en": "Logged in.",
                }),
            )
        }
        Err(e) => error_response(
            StatusCode::UNAUTHORIZED,
            format!("{} / {}", e.message_ja(), e.message_en()),
        ),
    }
}

/// `POST /api/auth/totp/setup` — 認証アプリ2FAのセットアップを開始する
/// (セッション必須)。新しいシークレットを生成しpending状態で保存、
/// QRコード表示用のprovisioning URIを返す。まだ有効化はされない
/// (`/enable`で確認コードを送るまでは既存のログインフローに影響しない)。
async fn totp_setup(state: &AppState, req: &Request<Incoming>) -> Response<BoxBody> {
    let account_email = match require_session(state, req) {
        Ok(email) => email,
        Err(resp) => return resp,
    };
    let secret = totp::generate_secret();
    let secret_b32 = totp::secret_to_base32(&secret);
    if !state.users.begin_totp_setup(&account_email, &secret_b32) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "failed to begin TOTP setup");
    }
    let uri = totp::provisioning_uri(&account_email, "open-easy-web", &secret_b32);
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "secret": secret_b32,
            "provisioning_uri": uri,
            "message_ja": "認証アプリでこのQRコード(またはシークレット)を登録し、表示された6桁コードで有効化してください。",
            "message_en": "Add this QR code (or secret) to your authenticator app, then confirm with the displayed 6-digit code.",
        }),
    )
}

#[derive(serde::Deserialize)]
struct TotpCodeRequest {
    code: String,
}

/// `POST /api/auth/totp/enable` — セットアップ中のシークレットを、認証
/// アプリで実際に生成された確認コードで検証し、有効化する。
async fn totp_enable(state: &AppState, req: Request<Incoming>) -> Response<BoxBody> {
    let account_email = match require_session(state, &req) {
        Ok(email) => email,
        Err(resp) => return resp,
    };
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: TotpCodeRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };
    let Some(pending) = state.users.pending_totp_secret(&account_email) else {
        return error_response(StatusCode::BAD_REQUEST, "no TOTP setup in progress / セットアップが開始されていません");
    };
    let Some(secret_bytes) = totp::base32_to_secret(&pending) else {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "invalid pending TOTP secret");
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if !totp::verify_code(&secret_bytes, &payload.code, now) {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "invalid authenticator code / 認証アプリのコードが正しくありません",
        );
    }
    state.users.confirm_totp_setup(&account_email);
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "message_ja": "認証アプリの2段階認証を有効化しました。次回ログインから必要になります。",
            "message_en": "Authenticator app 2FA is now enabled. It will be required starting with your next login.",
        }),
    )
}

/// `POST /api/auth/totp/disable` — 認証アプリ2FAを無効化する(セッション必須)。
async fn totp_disable(state: &AppState, req: &Request<Incoming>) -> Response<BoxBody> {
    let account_email = match require_session(state, req) {
        Ok(email) => email,
        Err(resp) => return resp,
    };
    state.users.disable_totp(&account_email);
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "message_ja": "認証アプリの2段階認証を無効化しました。",
            "message_en": "Authenticator app 2FA has been disabled.",
        }),
    )
}

#[derive(serde::Deserialize)]
struct RequestEmailChangeRequest {
    /// `"email"`(主メール)・`"phone"`・`"backup_email"`のいずれか。
    /// 省略時は`"email"`(既存クライアント互換)。
    #[serde(default = "default_change_field")]
    field: String,
    new_value: String,
}

fn default_change_field() -> String {
    "email".to_string()
}

/// `POST /api/auth/request-email-change` — ログイン中(セッション必須)の
/// アカウントに対し、主メール・セカンドメール・電話番号いずれかの変更を
/// リクエストする。確認リンクは**新しい連絡先ではなく、現在登録済みの
/// 主メールアドレスへ**送信する——アカウント乗っ取り防止のため、変更は
/// 現在のメールアドレスの持ち主にしか完了できない。
async fn request_email_change(state: &AppState, req: Request<Incoming>) -> Response<BoxBody> {
    let current_email = match require_session(state, &req) {
        Ok(email) => email,
        Err(response) => return response,
    };
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: RequestEmailChangeRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };
    if payload.field == "email" && !payload.new_value.contains('@') {
        return error_response(StatusCode::BAD_REQUEST, "invalid new email address");
    }
    if !["email", "phone", "backup_email"].contains(&payload.field.as_str()) {
        return error_response(StatusCode::BAD_REQUEST, "invalid field (must be email/phone/backup_email)");
    }

    let token = state.auth.request_contact_change(&current_email, &payload.field, &payload.new_value);

    let Some(smtp) = state.smtp.clone() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "SMTP is not configured on this server",
        );
    };
    match mail::send_contact_change_confirmation(smtp, current_email, payload.field, payload.new_value, token)
        .await
    {
        Ok(()) => json_response(
            StatusCode::OK,
            &serde_json::json!({
                "message_ja": "確認リンクを現在のメールアドレス宛に送信しました。",
                "message_en": "A confirmation link has been sent to your current email address.",
            }),
        ),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// `GET /api/auth/confirm-email-change?token=...` — 現在のメール宛に
/// 送られた確認リンクを踏んだ際のエンドポイント。`field`に応じて
/// 主メール改名(`rename_email`)・電話/セカンドメール更新
/// (`update_contact`)のどちらかを行う。
async fn confirm_email_change(state: &AppState, req: &Request<Incoming>) -> Response<BoxBody> {
    let query = req.uri().query().unwrap_or("");
    let token = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("token="))
        .unwrap_or("");
    if token.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "missing token");
    }

    let Some((account_email, field, new_value)) = state.auth.confirm_contact_change(token) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid or expired confirmation link / 無効または期限切れのリンクです",
        );
    };

    let applied = if field == "email" {
        state.users.rename_email(&account_email, &new_value)
    } else if let Some(contact_field) = users::ContactField::parse(&field) {
        state.users.update_contact(&account_email, contact_field, &new_value)
    } else {
        false
    };

    if applied {
        json_response(
            StatusCode::OK,
            &serde_json::json!({
                "message_ja": format!("{field} を {new_value} に変更しました。"),
                "message_en": format!("{field} changed to {new_value}."),
            }),
        )
    } else {
        error_response(
            StatusCode::CONFLICT,
            "the new value could not be applied (e.g. email already registered, or unknown account) / \
             変更を適用できませんでした(メールアドレス重複、またはアカウント不明)",
        )
    }
}

#[derive(serde::Deserialize)]
struct TokenRequest {
    token: String,
}

/// `POST /api/auth/logout` — セッショントークンを失効させる。
async fn logout(state: &AppState, req: Request<Incoming>) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: TokenRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };
    state.auth.logout(&payload.token);
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "message_ja": "ログアウトしました。",
            "message_en": "Logged out.",
        }),
    )
}

async fn route(
    state: Arc<AppState>,
    req: Request<Incoming>,
) -> Result<Response<BoxBody>, std::convert::Infallible> {
    Ok(dispatch(state, req).await)
}

async fn accept_loop(listener: TcpListener, state: Arc<AppState>) -> anyhow::Result<()> {
    loop {
        let (stream, _peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                tracing::warn!(error = %e, "failed to accept connection");
                continue;
            }
        };
        let io = TokioIo::new(stream);
        let state = state.clone();
        tokio::spawn(async move {
            let service = service_fn(move |req| route(state.clone(), req));
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                tracing::warn!(error = %err, "connection error");
            }
        });
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let state = Arc::new(AppState::from_env());
    let bind_addr: std::net::SocketAddr = std::env::var("OPEN_EASYWEB_SERVER_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8090".into())
        .parse()?;

    tracing::info!(%bind_addr, "open-easy-web-server listening");
    let listener = TcpListener::bind(bind_addr).await?;
    accept_loop(listener, state).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_site_path_splits_name_and_action() {
        let (name, action) = parse_site_path("/api/sites/audiocafe.tokyo/folder").unwrap();
        assert_eq!(name, "audiocafe.tokyo");
        assert_eq!(action, "/folder");
    }

    #[test]
    fn parse_site_path_rejects_missing_action() {
        assert!(parse_site_path("/api/sites/").is_none());
    }

    async fn spawn_test_server() -> (std::net::SocketAddr, Arc<AppState>) {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState {
            sites_root: dir.path().to_path_buf(),
            ai_state_path: dir.path().join("ai-state.json"),
            nginx_template: PathBuf::from("template"),
            nginx_http_only_template: PathBuf::from("template-http-only"),
            nginx_conf_d: dir.path().to_path_buf(),
            acme_email: "test@example.invalid".into(),
            bind_ip: "0.0.0.0".into(),
            php_fpm_upstream: "unix:/run/php-fpm/www.sock".into(),
            static_dir: dir.path().to_path_buf(),
            weights: Mutex::new(php_detector::PhpSignalWeights::default()),
            auth: auth::AuthStore::default(),
            users: users::UserStore::load(dir.path().join("users.json")),
            smtp: None,
            sms: None,
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_state = state.clone();
        tokio::spawn(async move {
            let _ = accept_loop(listener, server_state).await;
        });
        (addr, state)
    }

    #[tokio::test]
    async fn site_actions_require_a_valid_session_over_real_http() {
        let (addr, _state) = spawn_test_server().await;
        let client = reqwest::Client::new();

        // 認証ヘッダ無し → 401。
        let res = client
            .post(format!("http://{addr}/api/sites/example.tokyo/folder"))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);

        // 不正なトークン → 401。
        let res = client
            .post(format!("http://{addr}/api/sites/example.tokyo/folder"))
            .bearer_auth("not-a-real-token")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn otp_login_over_real_http_grants_access_to_site_actions() {
        let (addr, state) = spawn_test_server().await;
        let client = reqwest::Client::new();

        // 未登録の連絡先でOTPをリクエストすると404。
        let res = client
            .post(format!("http://{addr}/api/auth/request-otp"))
            .json(&serde_json::json!({"contact": "user@example.com"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::NOT_FOUND);

        // 公開の /api/auth/register は廃止済みのため、テスト用アカウントは
        // UserStoreへ直接登録する(本番では起動時のseed_fixed_accountのみが
        // アカウントを作る唯一の経路)。
        state.users.register("user@example.com", None, Some("user2@example.com".into())).unwrap();

        // 登録済みでもSMTP未設定なので503(実メール送信はできないが、
        // OTP自体は内部で発行・保存されている)。
        let res = client
            .post(format!("http://{addr}/api/auth/request-otp"))
            .json(&serde_json::json!({"contact": "user@example.com"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);

        // このテストではSMTPを介さず、サーバー内部のAuthStoreから直接
        // OTPを払い出して実HTTP経由のverify-otpを検証する。セカンドメール
        // (user2@example.com)経由でログインしても、同じアカウント
        // (user@example.com)のセッションが発行されることを確認する。
        let auth::RequestOtpOutcome::Issued(code) = state.auth.request_otp("user2@example.com");
        let res = client
            .post(format!("http://{addr}/api/auth/verify-otp"))
            .json(&serde_json::json!({"contact": "user2@example.com", "code": code}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["email"].as_str(), Some("user@example.com"));
        let token = body["token"].as_str().unwrap().to_string();

        let res = client
            .post(format!("http://{addr}/api/sites/example.tokyo/folder"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);

        // DELETE /api/sites/:name — 未登録ドメイン(vhostファイルが無い)への
        // 削除リクエストは冪等にOKを返す。認証無しでは401。
        let res = client
            .delete(format!("http://{addr}/api/sites/example.tokyo"))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);

        let res = client
            .delete(format!("http://{addr}/api/sites/example.tokyo"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn register_appserver_route_requires_session_and_reaches_registration_logic() {
        // このルート自体が長らく配線されておらず(appserver_registration
        // モジュールの関数群がdead codeだった)、2026-07-17に追加した。
        let (addr, state) = spawn_test_server().await;
        let client = reqwest::Client::new();

        // 認証無し → 401(他の/api/sites/*アクションと同じ扱い)。
        let res = client
            .post(format!("http://{addr}/api/sites/example.tokyo/register-appserver"))
            .json(&serde_json::json!({
                "shared_endpoint": "http://127.0.0.1:1",
                "kind": "poem_cosmo_tauri",
                "backend_addr": "127.0.0.1:9100",
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);

        state.users.register("appserver-user@example.com", None, Some("appserver-user2@example.com".into())).unwrap();
        let token = state.auth.create_session("appserver-user@example.com");

        // 認証あり、かつ`shared_endpoint`が到達不能 → appserver_registration
        // 側のHTTPエラーがそのままBAD_GATEWAYとして返る(ルーティング・
        // 認証・ボディのデシリアライズ・呼び出しがすべて実際に動くことの確認)。
        let res = client
            .post(format!("http://{addr}/api/sites/example.tokyo/register-appserver"))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "shared_endpoint": "http://127.0.0.1:1",
                "kind": "poem_cosmo_tauri",
                "backend_addr": "127.0.0.1:9100",
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn totp_setup_enable_then_requires_code_on_next_login() {
        let (addr, state) = spawn_test_server().await;
        let client = reqwest::Client::new();
        state.users.register("totp-user@example.com", None, Some("totp-user2@example.com".into())).unwrap();

        // ログインしてセッションを得る(TOTP未設定の1回目はコード不要)。
        let auth::RequestOtpOutcome::Issued(code) = state.auth.request_otp("totp-user@example.com");
        let res = client
            .post(format!("http://{addr}/api/auth/verify-otp"))
            .json(&serde_json::json!({"contact": "totp-user@example.com", "code": code}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        assert!(body["totp_required"].is_null());
        let token = body["token"].as_str().unwrap().to_string();

        // セットアップ開始(セッション必須)。
        let res = client
            .post(format!("http://{addr}/api/auth/totp/setup"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        let secret_b32 = body["secret"].as_str().unwrap().to_string();
        assert!(body["provisioning_uri"].as_str().unwrap().starts_with("otpauth://totp/"));

        // 間違ったコードでは有効化できない。
        let res = client
            .post(format!("http://{addr}/api/auth/totp/enable"))
            .bearer_auth(&token)
            .json(&serde_json::json!({"code": "000000"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);

        // 実際に生成した正しいコードで有効化。
        // **正直な開示・実バグの経緯(2026-07-23)**: 以前はここで
        // `verify_code`に通る値を0〜100万まで総当たりして探していたが、
        // debugビルドではこの総当たり自体が(運悪く正解が999999に近い
        // 場合)数秒かかることがあり、その間にTOTPの時間窓(30秒×
        // スキュー許容±1ステップ=最大60秒強)を超えてしまい、サーバー側の
        // `verify_code`が`401`を返す——というflaky failureの実際の原因
        // だった(`cargo test`を複数回実行して実際に再現・特定した)。
        // `totp::code_at`(テスト用に`pub`化)で正しいコードを直接計算する
        // ことで、この総当たりの遅延自体を無くして解消する。
        let secret_bytes = totp::base32_to_secret(&secret_b32).unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let correct_code = totp::code_at(&secret_bytes, now);
        let res = client
            .post(format!("http://{addr}/api/auth/totp/enable"))
            .bearer_auth(&token)
            .json(&serde_json::json!({"code": correct_code}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);

        // 次回ログインはOTPだけでは不完全(totp_required: trueが返り、
        // トークンは発行されない)。
        let auth::RequestOtpOutcome::Issued(code2) = state.auth.request_otp("totp-user@example.com");
        let res = client
            .post(format!("http://{addr}/api/auth/verify-otp"))
            .json(&serde_json::json!({"contact": "totp-user@example.com", "code": code2}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["totp_required"].as_bool(), Some(true));
        assert!(body["token"].is_null());

        // `/api/auth/totp-login`: メールOTPを一切経由せず、TOTPコード
        // だけでログインできる(2026-07-17、ユーザー指示の「どちらか一方
        // だけでログイン可能」の代替経路)。
        let now2 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let code_for_login = totp::code_at(&secret_bytes, now2);
        let res = client
            .post(format!("http://{addr}/api/auth/totp-login"))
            .json(&serde_json::json!({"account_email": "totp-user@example.com", "totp_code": code_for_login}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["email"].as_str(), Some("totp-user@example.com"));
        assert!(body["token"].as_str().is_some());
    }

    #[tokio::test]
    async fn totp_login_rejects_accounts_without_totp_enabled() {
        let (addr, state) = spawn_test_server().await;
        state.users.register("no-totp@example.com", None, Some("no-totp2@example.com".into())).unwrap();
        let client = reqwest::Client::new();
        let res = client
            .post(format!("http://{addr}/api/auth/totp-login"))
            .json(&serde_json::json!({"account_email": "no-totp@example.com", "totp_code": "000000"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::FORBIDDEN);
    }

    #[test]
    fn site_dir_rejects_traversal_in_name() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState {
            sites_root: PathBuf::from("/var/www"),
            ai_state_path: PathBuf::from("/tmp/state.json"),
            nginx_template: PathBuf::from("template"),
            nginx_http_only_template: PathBuf::from("template-http-only"),
            nginx_conf_d: PathBuf::from("/etc/nginx/conf.d"),
            acme_email: "test@example.invalid".into(),
            bind_ip: "0.0.0.0".into(),
            php_fpm_upstream: "unix:/run/php-fpm/www.sock".into(),
            static_dir: PathBuf::from("."),
            weights: Mutex::new(php_detector::PhpSignalWeights::default()),
            auth: auth::AuthStore::default(),
            users: users::UserStore::load(dir.path().join("users.json")),
            sms: None,
            smtp: None,
        };
        assert!(state.site_dir("../etc").is_none());
        assert_eq!(state.site_dir("audiocafe.tokyo"), Some(PathBuf::from("/var/www/audiocafe.tokyo")));
    }
}
