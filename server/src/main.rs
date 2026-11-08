//! open-easy-web-server: フォルダー作成・アップロード・AI自動PHP判定・
//! nginx+PHP-FPM自動構成を担うバックエンド。
//!
//! open-easy-web本体(WASM UI)はこれまでサーバー実体を持たなかった
//! (`scripts/serve.sh`は静的ファイル配信のみ)。このバイナリは、UIから
//! フォルダー作成・アップロード・AI判定・自動構成を呼べるようにするための
//! 最小限のREST APIを、tokio/hyperを直接使って(重量級フレームワーク非
//! 依存)提供する。

mod appserver_registration;
mod aruaru_llm_client;
mod auth;
mod auto_update;
mod db_encryption;
mod disk_breakdown;
mod dist_sync;
mod local_proxy;
mod lolipop_cron;
mod connect;
mod json_in;
mod storage_maintenance;
mod update_notify;
mod memory_breakdown;
mod memory_governor;
mod mail;
mod php_detector;
mod power_profile;
mod sms;
mod system_disk;
mod system_memory;
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
/// 本文を読み込み済みのリクエスト(`dispatch` の入口で、上限つきで読んでから各ハンドラへ渡す)。
type Req = Request<Full<Bytes>>;

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
    /// 分散同期先(VPS群)+ディザスタ用オフサイト退避先(Email/Googleドライブ)
    /// のレジストリ(`dist_sync.rs`参照、open_raid_z_coreの再利用)。
    dist_sync: Arc<dist_sync::DistSyncRegistry>,
    /// 深夜バックグラウンド自動アップデートの有効/無効設定
    /// (`auto_update.rs`参照、環境変数またはGUI/API経由で変更可能)。
    auto_update: Arc<auto_update::AutoUpdateState>,
    /// ロリポップ!向け時間指定オートクロール(`lolipop_cron.rs`参照)。
    lolipop_cron: Arc<lolipop_cron::LolipopCron>,
    /// ストレージ(HDD/SSD)のメンテナンス: 健康チェック・無駄なデータの整理・TRIM/最適化(`storage_maintenance.rs`参照)。
    storage_maintenance: Arc<storage_maintenance::StorageMaintenance>,
    /// 更新のお知らせメール: 宛先の登録・送信・履歴(`update_notify.rs`参照)。
    update_notify: Arc<update_notify::UpdateNotify>,
    /// 自分自身のリスンアドレス(`auto_update`のヘルスチェック・
    /// 「今すぐ確認」機能が新プロセスへ問い合わせる先として使う)。
    server_bind_addr: std::net::SocketAddr,
    /// `accept_loop`へ「新規接続の受付を止めて」と伝えるフラグ
    /// (`auto_update`のゼロダウンタイム切り替えで使う)。
    stop_accepting: Arc<std::sync::atomic::AtomicBool>,
    /// 電源プロファイル(省メモリ/省電力/常時電源接続、組み合わせ選択可能、
    /// `open-web-server`側`power_profile.rs`と同じ設計を移植、
    /// 2026-07-31追加)。管理API経由で再起動不要に切り替えられる。
    power_profile: Arc<power_profile::PowerProfileRegistry>,
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
        // DATABASE暗号化(`db_encryption.rs`)は常に自動で有効——管理者が
        // 意識する設定項目・トグル・質問は無い(ユーザー指示、2026-07-31)。
        let db_encryption = Arc::new(db_encryption::DbEncryptionState::load(&env_path(
            "OPEN_EASYWEB_DB_ENCRYPTION_KEY_FILE",
            "/var/www/.open-easy-web-db-encryption.key",
        )));
        let users = users::UserStore::load(env_path("OPEN_EASYWEB_USERS_STATE", "/var/www/.open-easy-web-users.json"), db_encryption);
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
            dist_sync: Arc::new(dist_sync::DistSyncRegistry::new(env_path(
                "OPEN_EASYWEB_DIST_SYNC_JOURNAL_DIR",
                "/var/www/.open-easy-web-dist-sync-journal",
            ))),
            auto_update: Arc::new(auto_update::AutoUpdateState::load(env_path(
                "OPEN_EASYWEB_AUTO_UPDATE_SETTINGS_FILE",
                "/var/www/.open-easy-web-auto-update.json",
            ))),
            lolipop_cron: Arc::new(lolipop_cron::LolipopCron::load(env_path(
                "OPEN_EASYWEB_LOLIPOP_CRON_FILE",
                "/var/www/.open-easy-web-lolipop-cron.json",
            ))),
            storage_maintenance: Arc::new(storage_maintenance::StorageMaintenance::load(
                env_path("OPEN_EASYWEB_STORAGE_MAINTENANCE_FILE", "/var/www/.open-easy-web-storage-maintenance.json"),
                std::env::var("OPEN_EASYWEB_STORAGE_TIDY_CONFIG").map(PathBuf::from).unwrap_or_else(|_| vps_tidy::config::default_config_path()),
            )),
            update_notify: Arc::new(update_notify::UpdateNotify::load(env_path("OPEN_EASYWEB_UPDATE_NOTIFY_FILE", "/var/www/.open-easy-web-update-notify.json"))),
            server_bind_addr: std::env::var("OPEN_EASYWEB_SERVER_BIND")
                .unwrap_or_else(|_| "0.0.0.0:8090".into())
                .parse()
                .unwrap_or_else(|_| ([0, 0, 0, 0], 8090).into()),
            stop_accepting: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            power_profile: Arc::new(power_profile::PowerProfileRegistry::new()),
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

/// 旧形式のメール確認リンクに出す案内ページ(副作用なし)。新しい形式のリンクは、画面のURL(`/#/confirm-email-change?token=…`)で、
/// 確認ボタンを押したときだけ Connect の POST で確定する。
fn old_confirm_link_page() -> Response<BoxBody> {
    let html = "<!DOCTYPE html><html lang=\"ja\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"robots\" content=\"noindex\"><title>確認リンクの形式が変わりました</title></head><body style=\"font:18px system-ui,sans-serif;max-width:640px;margin:24px auto;padding:0 16px\"><h2>確認リンクの形式が変わりました / The confirmation link format has changed</h2><p>このリンクは古い形式のため、使えなくなりました。お手数ですが、<a href=\"/\">ログイン画面</a>からもう一度、連絡先の変更を申請し直してください(新しい形式の確認メールが届きます)。</p><p>This link uses an old format and no longer works. Please <a href=\"/\">log in</a> and request the contact change again; you will receive a new confirmation email.</p></body></html>";
    Response::builder()
        .status(StatusCode::GONE)
        .header("content-type", "text/html; charset=utf-8")
        .header("cache-control", "no-store")
        .body(Full::new(Bytes::from(html)))
        .expect("static response headers are always valid")
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

/// `Authorization: Bearer <token>` を検証する。open-easy-web全体で唯一の
/// 認証方式(固定パスワードを廃したメールOTPログイン)に統一されている
/// ため、サイト操作系エンドポイントは全てこれを通す。
fn require_session(state: &AppState, req: &Req) -> Result<String, Response<BoxBody>> {
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

/// 配布中の版(VPS の `/var/lib/aruaru-ime/downloads/VERSION`。無ければ 0.2.1)。
fn ime_version() -> String {
    std::fs::read_to_string("/var/lib/aruaru-ime/downloads/VERSION").map(|v| v.trim().to_string()).ok().filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.')).unwrap_or_else(|| "0.2.1".to_string())
}

/// OS ごとのインストーラーへのリンク(Windows はこのサーバーから、ほかは GitHub Releases から)。
fn ime_installers_html() -> String {
    let v = ime_version();
    let gh = format!("https://github.com/aon-co-jp/aruaru-IME/releases/download/v{v}");
    format!(
        "<ul class=\"ime-dl\">\
<li data-os=\"windows\"><a href=\"/ime/download/aruaru-ime-windows-x86_64-setup.exe\">Windows (64bit) インストーラー / installer</a></li>\
<li data-os=\"mac\"><a href=\"{gh}/aruaru-ime-macos-arm64.tar.gz\">macOS (Apple Silicon)</a> ・ <a href=\"{gh}/aruaru-ime-macos-x86_64.tar.gz\">macOS (Intel)</a></li>\
<li data-os=\"linux\"><a href=\"{gh}/aruaru-ime-linux-x86_64.tar.gz\">Linux (x86_64)</a></li>\
<li data-os=\"android\"><a href=\"{gh}/aruaru-ime-android.apk\">Android (APK)</a> ・ <a href=\"/ime\">スマホ・タブレットはオンライン画面でも使えます / or use the online screen</a></li>\
</ul><p><small>v{v} ・ <a href=\"https://github.com/aon-co-jp/aruaru-IME/releases\">すべての版 / all releases</a> ・ 署名されていないため、警告が出ることがあります / unsigned: your OS may warn.</small></p>\
<script>(function(){{var u=navigator.userAgent,o=/Android/.test(u)?'android':/Mac/.test(u)?'mac':/Linux/.test(u)?'linux':/Win/.test(u)?'windows':'';var l=document.querySelector('.ime-dl [data-os=\"'+o+'\"]');if(l){{l.style.fontWeight='bold';l.insertAdjacentText('afterbegin','▶ ');}}}})();</script>"
    )
}

/// トップページ先頭の aruaru-IME の紹介。
fn ime_banner_html() -> String {
    format!(
        "<section id=\"aruaru-ime-intro\" style=\"margin:12px auto;max-width:960px;padding:12px 16px;border:1px solid #888;border-radius:8px;font-family:system-ui,sans-serif\">\
<h2 style=\"margin:0 0 6px\">aruaru-IME</h2>\
<p style=\"margin:0 0 8px\">漢字に変換することを前提に、打つそばから変換結果を出す AI の日本語入力(IME)。日本語⇔英語の相互翻訳、中国語・世界の言語、用語辞書、季節用語などに対応。Windows / macOS / Linux / Android。<br>\
An AI IME that shows the converted (kanji) result as you type, with Japanese⇔English translation, Chinese, many languages and term dictionaries.</p>\
<p style=\"margin:0 0 6px\"><a href=\"/ime\"><b>ブラウザで試す(オンライン画面) / Try it in the browser</b></a></p>\
<div><b>ダウンロード / Download</b>{}</div></section>",
        ime_installers_html()
    )
}

/// トップページの open-mirror の紹介(ページ本体は /open-mirror、デモは /open-mirror/demo)。
fn mirror_banner_html() -> String {
    "<section id=\"open-mirror-intro\" style=\"margin:12px auto;max-width:960px;padding:12px 16px;border:1px solid #888;border-radius:8px;font-family:system-ui,sans-serif\"><h2 style=\"margin:0 0 6px\">open-mirror</h2><p style=\"margin:0 0 8px\">必要なものだけを選んで、バックアップ・ミラーするツール。いまは GitHub の公開リポジトリを毎日 GitLab へ一方向でミラーしています。PC・スマホ・タブレット(ネットに繋がったら自動で開始)、VPS、レンタルサーバー、ブログなどを、厳選リストで選んで保存できるようにする予定です(予定の部分はまだ動きません)。<br>A tool that backs up and mirrors only what you choose. Today it mirrors the public GitHub repositories to GitLab every day; selecting PCs, phones, tablets (auto-start when online), VPS, rental servers and blogs from a curation list is planned and does not run yet.</p><p style=\"margin:0\"><a href=\"/open-mirror\"><b>紹介 / About</b></a> ・ <a href=\"/open-mirror/demo\"><b>デモを試す / Try the demo</b></a> ・ <a href=\"https://github.com/aon-co-jp/open-mirror\">GitHub</a></p></section>".to_string()
}

/// トップページの open-media の紹介(ダウンロード・紹介は /open-media、デモは /open-media/demo/)。
fn media_banner_html() -> String {
    "<section id=\"open-media-intro\" style=\"margin:12px auto;max-width:960px;padding:12px 16px;border:1px solid #888;border-radius:8px;font-family:system-ui,sans-serif\"><h2 style=\"margin:0 0 6px\">open-media</h2><p style=\"margin:0 0 8px\">CD・DVD・Blu-ray の書き込み・変換と、高音質プレーヤーを1つにしたアプリ。音楽CDを排他モード+高品質アップサンプル(例: 44.1kHz→352.8kHz、32bit)で再生し、DVD・MP4・MKV はFull HD / 4Kへ引き上げながら再生します。Windows / macOS / Linux / Android に対応。<br>One app that combines disc burning/conversion with a hi-fi player: it plays audio CDs in exclusive mode with high-quality upsampling (e.g. 44.1 to 352.8 kHz, 32-bit) and upconverts DVD / MP4 / MKV to Full HD / 4K while playing. For Windows, macOS, Linux and Android.</p><p style=\"margin:0\"><a href=\"/open-media\"><b>ダウンロード・紹介 / Download &amp; About</b></a> ・ <a href=\"/open-media/demo/\"><b>デモを試す / Try the demo</b></a> ・ <a href=\"https://github.com/aon-co-jp/open-media/releases/latest\">最新版 / Latest release</a> ・ <a href=\"https://github.com/aon-co-jp/open-media\">GitHub</a></p></section>".to_string()
}

/// aruaru-IME のオンライン画面(/ime)の変換 API。VPS 上のサーバー運転の aruaru-IME エンジン(127.0.0.1:4660)へ中継する。
/// 通すのは /health(例外)と Connect の Key・Session だけ、本文は4KBまで。**入力内容はこのサーバーを通る**(エンジン側は学習・保存をしない)。
async fn ime_api(method: &Method, path: &str, req: Request<Incoming>) -> Response<BoxBody> {
    let target = match (method, path) {
        (&Method::GET, "/ime/api/health") => "/health",
        (&Method::POST, "/ime/api/aruaru_ime.v1.Ime/Key") => "/aruaru_ime.v1.Ime/Key",
        (&Method::POST, "/ime/api/aruaru_ime.v1.Ime/Session") => "/aruaru_ime.v1.Ime/Session",
        // 期限つき互換(2026-12-06 まで): 配布済みの Android アプリ・古いページ向け。上流は Connect へ写す。期限後に削除する。
        (&Method::POST, "/ime/api/key") => "/aruaru_ime.v1.Ime/Key",
        (&Method::POST, "/ime/api/session") => "/aruaru_ime.v1.Ime/Session",
        _ => return error_response(StatusCode::NOT_FOUND, "not found"),
    };
    // 期限つき互換(旧 /ime/api/key|session)の利用を数える。2026-12-06 の削除判断の材料(入力内容は記録しない)。
    if path == "/ime/api/key" || path == "/ime/api/session" {
        eprintln!("ime-legacy-route-hit path={path}");
    }
    // 回数制限: 1つの接続元あたり1秒に40回まで、全体で1秒に400回まで
    {
        static HITS: std::sync::OnceLock<Mutex<(std::time::Instant, u32, std::collections::HashMap<String, u32>)>> = std::sync::OnceLock::new();
        let ip = req.headers().get("x-forwarded-for").and_then(|v| v.to_str().ok()).and_then(|v| v.split(',').next()).unwrap_or("unknown").trim().to_string();
        let m = HITS.get_or_init(|| Mutex::new((std::time::Instant::now(), 0, Default::default())));
        let mut g = m.lock().unwrap_or_else(|e| e.into_inner());
        if g.0.elapsed() >= std::time::Duration::from_secs(1) {
            *g = (std::time::Instant::now(), 0, Default::default());
        }
        g.1 += 1;
        let n = { let e = g.2.entry(ip).or_insert(0); *e += 1; *e };
        if n > 40 || g.1 > 400 {
            return error_response(StatusCode::TOO_MANY_REQUESTS, "too many requests / リクエストが多すぎます");
        }
    }
    let body = match http_body_util::Limited::new(req.into_body(), 4096).collect().await {
        Ok(b) => b.to_bytes(),
        Err(_) => return error_response(StatusCode::PAYLOAD_TOO_LARGE, "too large"),
    };
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    let client = CLIENT.get_or_init(|| reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().expect("client"));
    let url = format!("http://127.0.0.1:4660{target}");
    let r = if method == Method::GET { client.get(&url).send().await } else { client.post(&url).body(body.to_vec()).send().await };
    match r {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let ct = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("application/json").to_string();
            let bytes = resp.bytes().await.unwrap_or_default();
            Response::builder().status(status).header("content-type", ct).header("cache-control", "no-store").body(Full::new(bytes)).expect("static headers")
        }
        Err(_) => error_response(StatusCode::BAD_GATEWAY, "ime engine unavailable"),
    }
}

async fn dispatch(state: Arc<AppState>, req: Request<Incoming>) -> Response<BoxBody> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    if path.starts_with("/ime/api/") {
        return ime_api(&method, &path, req).await;
    }
    // ここから先は、本文を**上限つきで先に読み込んで**から各ハンドラへ渡す(以前はハンドラごとに無制限に読んでいた)。
    // 通常の本文は 64 KiB まで。ファイル送信(`Sites/Upload`)だけ 128 MiB まで。
    let req = {
        let limit = if path.starts_with(connect::PREFIX) && path.ends_with(".Sites/Upload") { connect::UPLOAD_MAX } else { json_in::DEFAULT_MAX };
        let (parts, body) = req.into_parts();
        match http_body_util::Limited::new(body, limit).collect().await {
            Ok(c) => Request::from_parts(parts, Full::new(c.to_bytes())),
            Err(_) => {
                return if path.starts_with(connect::PREFIX) {
                    connect::payload_too_large()
                } else {
                    error_response(StatusCode::PAYLOAD_TOO_LARGE, "request body too large / 本文が大きすぎます")
                };
            }
        }
    };
    // インストーラーを選ぶページ(Windows / macOS / Linux / Android のスマホ・タブレット)
    if method == Method::GET && (path == "/ime/download" || path == "/ime/download/") {
        let html = format!(
            "<!DOCTYPE html><html lang=\"ja\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>aruaru-IME ダウンロード</title><style>:root{{--bg:#111;--fg:#fff;--acc:#4a9eff}}@media (prefers-color-scheme:light){{:root{{--bg:#fff;--fg:#111}}}}body{{margin:0;background:var(--bg);color:var(--fg);font:18px system-ui,sans-serif;padding:16px;max-width:760px}}a{{color:var(--acc)}}li{{margin:10px 0}}</style></head><body><h2>aruaru-IME ダウンロード / Download</h2><p>お使いの OS を選んでください。 / Choose your OS.</p>{}<p><b>Android のスマホ・タブレット</b>: APK を入れたら、アプリを開き、案内に従って「aruaru-IME」キーボードを有効にしてください。 / On Android phones and tablets: install the APK, open the app and enable the aruaru-IME keyboard.</p><p><b>macOS / Linux</b>: ダウンロードしたファイルを展開して <code>sh install.sh</code> を実行します(管理者権限は不要)。 / Extract and run <code>sh install.sh</code> (no admin needed).</p><p><a href=\"/ime\">← オンライン画面 / Online screen</a></p></body></html>",
            ime_installers_html()
        );
        return Response::builder().status(StatusCode::OK).header("content-type", "text/html; charset=utf-8").header("cache-control", "no-store").body(Full::new(Bytes::from(html))).expect("static headers");
    }
    // aruaru-IME のインストーラーの配布(GitHub を経由せず、このサーバーから直接)。決まった4ファイルだけ。
    if let (&Method::GET, Some(name)) = (&method, path.strip_prefix("/ime/download/")) {
        const FILES: [&str; 5] = [
            "dict.bin",
            "aruaru-ime-windows-x86_64-setup.exe",
            "aruaru-ime-windows-x86_64-setup.exe.sha256",
            "aruaru-ime-windows-x86_64.zip",
            "aruaru-ime-windows-x86_64.zip.sha256",
        ];
        if !FILES.contains(&name) {
            return error_response(StatusCode::NOT_FOUND, "not found");
        }
        return match std::fs::read(format!("/var/lib/aruaru-ime/downloads/{name}")) {
            Ok(b) => {
                let ct = if name.ends_with(".exe") { "application/vnd.microsoft.portable-executable" } else if name.ends_with(".zip") { "application/zip" } else { "text/plain; charset=utf-8" };
                Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", ct)
                    .header("content-disposition", format!("attachment; filename=\"{name}\""))
                    .header("cache-control", "no-cache")
                    .body(Full::new(Bytes::from(b)))
                    .expect("static headers")
            }
            Err(_) => error_response(StatusCode::NOT_FOUND, "not found"),
        };
    }

    // `/open_easy_web.admin.v1.<Service>/<Method>` — Connect(POST、RS-JSON)(POST、RS-JSON、`connect.rs`参照)。
    // 新しい管理APIは Connect のみ(REST は作らない。2026-11-04 方針: REST 全廃)。既存の REST 風 `/admin/...` は、触るときに順に移す。
    if let Some(rest) = path.strip_prefix(connect::PREFIX) {
        return connect::dispatch(&state, req, rest).await;
    }

    // DATABASE暗号化(`db_encryption.rs`)は常時自動で有効であり、ON/OFF
    // 切り替えの管理API・GUI・対話式質問は意図的に持たない(ユーザー指示、
    // 2026-07-31「管理者が意識しないで済む用に裏で処理しましょう」)。
    // `users.rs`の`load`/`persist`境界だけで透過的に暗号化・復号される。

    match (&method, path.as_str()) {
        (&Method::GET, "/healthz") => json_response(StatusCode::OK, &serde_json::json!({"status":"ok"})),
        // aruaru-IME のオンライン画面。入力は利用者自身のPCで動く aruaru-IME のエンジン(127.0.0.1:4660)へ直接つなぐ
        // (入力内容はこのサーバーを通らない)。エンジン側は https://easy-web.tokyo からの接続だけを許可する。
        (&Method::GET, "/ime") => {
            let mut app = open_runo_crossui::ime_app("/ime/api", "0.2.0");
            app.root = open_runo_crossui::ime_root_with(&app.bridge, "/ime/download", "http://127.0.0.1:4660/ui");
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .header("cache-control", "no-store")
                .body(Full::new(Bytes::from(open_runo_crossui::page_html(&app, "/ime/api").replace("</main>", &("<div id=\"notice\"><small><b>ご利用上の注意 / Notes</b><ul><li>この画面は、入力した内容をサーバー(easy-web.tokyo)で変換します。入力はサーバーに送られますが、保存・学習はしません。<br>This screen converts your input on the server (easy-web.tokyo). Your input is sent to the server but is not stored or learned.</li><li>自分のPCだけで変換したいときは、従来どおりインストールして「このPCの画面で開く」(http://127.0.0.1:4660/ui)を使ってください。<br>To convert only on your own PC, install the app as usual and use \"Open on this PC\" (http://127.0.0.1:4660/ui).</li><li>個人の学習や登録語は、オンライン画面には反映されません。<br>Your personal learning and registered words are not applied on this online screen.</li><li>中国語・季節用語・優先順位などの辞書は、サーバー側でも同じものが使われます。<br>The same dictionaries (Chinese, seasonal terms, candidate priorities, etc.) are used on the server.</li><li>短時間に大量の操作をすると、一時的に制限されることがあります。<br>Very rapid bursts of requests may be temporarily limited.</li></ul></small></div>".to_string() + "<div id=\"installers\"><b>インストーラー(OSを選ぶ) / Installers</b>" + &ime_installers_html() + "</div></main>")))))
                .expect("static response headers are always valid")
        }
        // ログイン関連の API は Connect(`Auth/*`・`Account/*`、`connect.rs`)へ移した。旧 REST の `request-otp`・`verify-otp` は削除した(2026-11-04)。
        // 旧形式のメール確認リンク(`GET /api/auth/confirm-email-change?token=…`)。**GET では何も確定しない**
        // (メールの先読みでトークンが消費されないように)。新しい形式の案内ページだけを返す。
        (&Method::GET, p) if p.starts_with("/api/auth/confirm-email-change") => old_confirm_link_page(),
        (&Method::GET, "/") => {
            // トップページの先頭に aruaru-IME の紹介とダウンロードを差し込む(SPA シェルは変更しない)
            let mut r = serve_static(&state, "index.html", "text/html; charset=utf-8").await;
            if r.status() == StatusCode::OK {
                let (parts, body) = r.into_parts();
                let bytes = body.collect().await.map(|c| c.to_bytes()).unwrap_or_default();
                let html = String::from_utf8_lossy(&bytes).into_owned();
                let html = html.replacen("<body>", &format!("<body>{}{}{}", ime_banner_html(), mirror_banner_html(), media_banner_html()), 1);
                r = Response::from_parts(parts, Full::new(Bytes::from(html)));
                r.headers_mut().remove("content-length");
            }
            r
        }
        // QR確認ログインページ(2026-08-28新設)。スマホ/タブレット/WEBカメラ
        // 搭載端末で開き、開いた時点で自動的に確認が完了する単体完結ページ。
        (&Method::GET, "/qr-confirm.html") => serve_static(&state, "qr-confirm.html", "text/html; charset=utf-8").await,
        // 2026-09-07変更(ユーザー指示): 従来は`/rsync`にRSync使い方ガイド
        // (SPAシェル、`index.html`の`#rsync-guide-section`)を配信していたが、
        // 名前が似ている別プロジェクト`rs-sync`(GitHub複数アカウント・
        // 複数プロバイダのリポジトリ同期ツール、`easy-web.tokyo/rs-sync/`)と
        // 混同されやすいと判明したため、`/rsync`・`/rsync/`は実プロジェクト
        // `rs-sync`へ302リダイレクトするよう変更した。旧ガイド本文
        // (`#rsync-guide-section`、`index.html`内)自体は削除していない
        // (直接`index.html`を開けば引き続き閲覧可能)——このルートだけを
        // リダイレクトに切り替えた。
        (&Method::GET, "/rsync") | (&Method::GET, "/rsync/") => Response::builder()
            .status(StatusCode::FOUND)
            .header("Location", "https://easy-web.tokyo/rs-sync/")
            .body(Full::new(Bytes::new()))
            .expect("static response headers are always valid"),
        // エコシステム関連プロジェクト紹介ページ(2026-08-24新設)。
        // `/rsync`と同じくSPAシェルを返し、表示するセクションはWASM側が
        // `location.pathname`から判定する(新しい仕組みは増やしていない)。
        (&Method::GET, "/projects") | (&Method::GET, "/projects/") => {
            serve_static(&state, "index.html", "text/html; charset=utf-8").await
        }
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

/// アップロードする1ファイル(`Sites/Upload` の本文)。`path` はサイトディレクトリからの相対パス
/// (ブラウザの `webkitdirectory` が相対パス込みのファイル名を送ってくることを利用)、`data_base64` は中身。
#[derive(serde::Deserialize)]
pub(crate) struct UploadFile {
    pub(crate) path: String,
    pub(crate) data_base64: String,
}

/// `Sites/Upload`(Connect、POST、RS-JSON)— ファイル群を webroot へ書き込む。
pub(crate) async fn upload_files(state: &AppState, site: &str, files: Vec<UploadFile>) -> Response<BoxBody> {
    use base64::Engine as _;
    let Some(dir) = state.site_dir(site) else {
        return error_response(StatusCode::BAD_REQUEST, "invalid site name");
    };

    if tokio::fs::create_dir_all(&dir).await.is_err() {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, "failed to prepare site directory");
    }

    let mut written = Vec::new();
    for file in files {
        let filename = file.path;
        let data = match base64::engine::general_purpose::STANDARD.decode(file.data_base64.as_bytes()) {
            Ok(d) => d,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid base64 for {filename}: {e}")),
        };
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
        // 実書き込みは`DisasterRecoveryManager::protect_write`(切断耐性
        // ジャーナル)経由で保護する(`dist_sync.rs`の`protect_site_write`
        // 参照)。ジャーナルのfsync+実ファイル書き込みはいずれもブロッキング
        // I/Oのため`spawn_blocking`へ退避する。本体への書き込みが失敗しても
        // ジャーナルは既に永続化済みなので、再接続後のリプレイで復旧できる。
        let dataset = format!("{site}/{}", rel.to_string_lossy());
        let registry = Arc::clone(&state.dist_sync);
        let data_for_write = data.clone();
        let dest_for_write = dest.clone();
        let write_result = tokio::task::spawn_blocking(move || {
            dist_sync::protect_site_write(&registry, &dataset, data_for_write, dest_for_write)
        })
        .await;
        match write_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("failed to write {filename}: {e}"));
            }
            Err(e) => {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("write task panicked for {filename}: {e}"));
            }
        }
        // 実際にサイトファイルが書き込まれた直後、分散同期先(VPS群)が
        // 1件でも登録されていれば、その書き込みをSFTP経由で複製する
        // (`dist_sync.rs`参照)。`spawn_replication`は同期先が0件の場合は
        // 何もせず即座に戻る(既存動作を変えない)。同期先がある場合も
        // `tokio::spawn`でデタッチされるため、このアップロードリクエストの
        // レスポンスは複製の完了を一切待たない(遅い/到達不能なVPSが
        // あってもアップロードは遅延しない)。
        dist_sync::spawn_replication(&state.dist_sync, format!("{site}/{}", rel.to_string_lossy()), Bytes::from(data));
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
async fn correct_detection(state: &AppState, site: &str, req: Req) -> Response<BoxBody> {
    let Some(dir) = state.site_dir(site) else {
        return error_response(StatusCode::BAD_REQUEST, "invalid site name");
    };
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: CorrectRequest = match json_in::parse(&bytes) {
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
async fn register_appserver(site: &str, req: Req) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: appserver_registration::RegisterAppserverRequest = match json_in::parse(&bytes) {
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
async fn request_otp(state: &AppState, req: Req) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: ContactRequest = match json_in::parse(&bytes) {
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
async fn totp_login(state: &AppState, req: Req) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: TotpLoginRequest = match json_in::parse(&bytes) {
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

fn qr_confirm_url(req_host: &str, scheme: &str, id: &str) -> String {
    format!("{scheme}://{req_host}/qr-confirm.html?id={id}")
}

fn request_host_scheme(req: &Req) -> (String, &'static str) {
    let host = req.headers().get(hyper::header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("localhost").to_string();
    let scheme = if req.headers().get("x-forwarded-proto").and_then(|v| v.to_str().ok()) == Some("https") {
        "https"
    } else {
        "http"
    };
    (host, scheme)
}

fn qr_login_start_response(host: &str, scheme: &str, id: String) -> Response<BoxBody> {
    let confirm_url = qr_confirm_url(host, scheme, &id);
    match totp::qr_svg(&confirm_url) {
        Ok(qr_svg) => json_response(
            StatusCode::OK,
            &serde_json::json!({
                "ok": true,
                "qr_login_id": id,
                "qr_svg": qr_svg,
                "confirm_url": confirm_url,
            }),
        ),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("failed to generate QR code: {e}")),
    }
}

#[derive(serde::Deserialize)]
struct QrLoginStartRequest {
    account_email: String,
}

/// `POST /api/auth/qr-login/start` — `qr`モード(QR撮影のみ、事前のOTP
/// 検証なし)専用の開始エンドポイント。TOTPが有効化されているアカウント
/// のみ対象(そのアカウントに対して既に「認証を要求してよい持ち主」で
/// あることが確認済みという前提を流用するため——TOTP登録済み=一度は
/// セッション認証済みでセットアップを完了している、という既存の要件と
/// 整合させた設計)。
async fn qr_login_start(state: &AppState, req: Req) -> Response<BoxBody> {
    let (host, scheme) = request_host_scheme(&req);
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: QrLoginStartRequest = match json_in::parse(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };
    if !state.users.totp_enabled(&payload.account_email) {
        return error_response(
            StatusCode::FORBIDDEN,
            "TOTP is not enabled for this account, so QR-only login is not available / このアカウントは認証アプリの2段階認証が有効になっていないため、QR撮影のみでのログインは利用できません",
        );
    }
    let id = state.auth.start_qr_login(&payload.account_email);
    qr_login_start_response(&host, scheme, id)
}

#[derive(serde::Deserialize)]
struct QrLoginIdRequest {
    id: String,
}

/// `POST /api/auth/qr-login/confirm` — QR確認ページ(`qr-confirm.html`、
/// スマホ/タブレット/WEBカメラ搭載端末で開く)がページ読み込み時に
/// 自動的に呼ぶ(ユーザー指示「撮影すると自動受信・自動承認」——ボタン
/// 操作は不要)。
async fn qr_login_confirm(state: &AppState, req: Req) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: QrLoginIdRequest = match json_in::parse(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };
    match state.auth.confirm_qr_login(&payload.id) {
        Ok(()) => json_response(StatusCode::OK, &serde_json::json!({"ok": true})),
        Err(e) => error_response(StatusCode::BAD_REQUEST, format!("{} / {}", e.message_ja(), e.message_en())),
    }
}

/// `GET /api/auth/qr-login/status?id=` — プライマリ端末が数秒おきに
/// ポーリングし、別端末での確認が済んだかを見る。
async fn qr_login_status(state: &AppState, req: &Req) -> Response<BoxBody> {
    let query = req.uri().query().unwrap_or("");
    let id = query.split('&').find_map(|kv| kv.strip_prefix("id=")).unwrap_or("");
    if id.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "missing id");
    }
    match state.auth.qr_login_status(id) {
        Some(confirmed) => json_response(StatusCode::OK, &serde_json::json!({"confirmed": confirmed})),
        None => error_response(StatusCode::NOT_FOUND, "this QR login link is invalid or has expired"),
    }
}

/// `POST /api/auth/qr-login/finish` — 別端末での確認が済んだことを
/// ポーリングで確認したプライマリ端末が呼ぶ。ここで初めて実際の
/// セッショントークンを発行する。
async fn qr_login_finish(state: &AppState, req: Req) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: QrLoginIdRequest = match json_in::parse(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };
    match state.auth.finish_qr_login(&payload.id) {
        Ok(token) => json_response(StatusCode::OK, &serde_json::json!({"ok": true, "token": token})),
        Err(e) => error_response(StatusCode::BAD_REQUEST, format!("{} / {}", e.message_ja(), e.message_en())),
    }
}

/// `GET /api/auth/qr-login/whoami?id=` — QR確認ページが「どのアカウント
/// のログインを確認しようとしているか」を、マスク済みメールアドレスで
/// 表示するために使う。
async fn qr_login_whoami(state: &AppState, req: &Req) -> Response<BoxBody> {
    let query = req.uri().query().unwrap_or("");
    let id = query.split('&').find_map(|kv| kv.strip_prefix("id=")).unwrap_or("");
    match state.auth.qr_login_masked_email(id) {
        Some(masked) => json_response(StatusCode::OK, &serde_json::json!({"ok": true, "masked_email": masked})),
        None => error_response(StatusCode::NOT_FOUND, "this QR login link is invalid or has expired"),
    }
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
async fn verify_otp(state: &AppState, req: Req) -> Response<BoxBody> {
    let (host, scheme) = request_host_scheme(&req);
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: VerifyOtpRequest = match json_in::parse(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };

    let Some(account_email) = state.resolve_account_email(&payload.contact) else {
        return error_response(StatusCode::NOT_FOUND, "unknown contact");
    };

    match state.auth.consume_otp(&payload.contact, &payload.code) {
        Ok(()) => {
            // `otp_qr`モード(2026-08-28新設)かつTOTP登録済みなら、
            // TOTPコード入力の代わりにQR確認(第二要素)を要求する。
            if state.auth.login_mode() == auth::LOGIN_MODE_OTP_QR && state.users.totp_enabled(&account_email) {
                let id = state.auth.start_qr_login(&account_email);
                return qr_login_start_response(&host, scheme, id);
            }
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
async fn totp_setup(state: &AppState, req: &Req) -> Response<BoxBody> {
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
    // 2026-08-27追加: 従来はprovisioning URI(テキスト)のみで、認証アプリへの
    // 登録には手入力が必要だった。QRコード(SVG)を追加し、open-english/
    // rs-syncと同じ「QR併記」方式へ統一する(ユーザー指示)。生成失敗時も
    // セットアップ自体は続行できるよう、qr_svgはnull・error付きで返す
    // (テキストのシークレットは引き続き使えるため、動くふりをせず正直に
    // 開示しつつ全体を失敗させない)。
    let (qr_svg, qr_error) = match totp::qr_svg(&uri) {
        Ok(svg) => (Some(svg), None),
        Err(e) => (None, Some(e)),
    };
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "secret": secret_b32,
            "provisioning_uri": uri,
            "qr_svg": qr_svg,
            "qr_error": qr_error,
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
async fn totp_enable(state: &AppState, req: Req) -> Response<BoxBody> {
    let account_email = match require_session(state, &req) {
        Ok(email) => email,
        Err(resp) => return resp,
    };
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: TotpCodeRequest = match json_in::parse(&bytes) {
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
async fn totp_disable(state: &AppState, req: &Req) -> Response<BoxBody> {
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
async fn request_email_change(state: &AppState, req: Req) -> Response<BoxBody> {
    let current_email = match require_session(state, &req) {
        Ok(email) => email,
        Err(response) => return response,
    };
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: RequestEmailChangeRequest = match json_in::parse(&bytes) {
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

/// `Account/ConfirmEmailChange`(Connect、POST、RS-JSON、本文 `{"token":"…"}`)— 現在のメール宛に送られた確認リンク
/// (画面のURL `…/#/confirm-email-change?token=…`)の確認ボタンを押したときに呼ばれる。`field`に応じて
/// 主メール改名(`rename_email`)・電話/セカンドメール更新(`update_contact`)のどちらかを行う。
/// **GET では何も起こさない**(メールの先読み・プレビューでトークンが消費されないよう、確定は POST だけ)。
async fn confirm_email_change(state: &AppState, token: &str) -> Response<BoxBody> {
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
async fn logout(state: &AppState, req: Req) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: TokenRequest = match json_in::parse(&bytes) {
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

/// `stop`が`true`になったら新規接続の受付を止めて戻る(既存の接続処理は
/// `tokio::spawn`済みのタスクとして独立に完走する、途中で打ち切らない)。
/// 深夜自動アップデートのゼロダウンタイム切り替え(`auto_update.rs`参照)
/// で、新バイナリが健全になったのを確認してから旧プロセスがこの受付を
/// 停止するために使う。`stop`を一切操作しない既存の呼び出し方
/// (`Arc::new(AtomicBool::new(false))`を渡すだけ)であれば、無限ループで
/// 受付し続ける従来の挙動と完全に同じ。
async fn accept_loop(listener: TcpListener, state: Arc<AppState>, stop: Arc<std::sync::atomic::AtomicBool>) -> anyhow::Result<()> {
    loop {
        if stop.load(std::sync::atomic::Ordering::SeqCst) {
            tracing::info!("accept_loop: stopping (auto-update handoff to new process)");
            return Ok(());
        }
        let (stream, _peer) = tokio::select! {
            res = listener.accept() => match res {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!(error = %e, "failed to accept connection");
                    continue;
                }
            },
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => continue,
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

/// `--version`引数を処理する。深夜自動アップデート
/// (`auto_update::verify_binary_runs`)が、ダウンロードした新バイナリが
/// 実際に起動できることを確認するために使う——tracing初期化・ポート
/// bind・その他起動処理を一切行わず、即座にバージョン文字列を出力して
/// 終了する(壊れたバイナリでも高速かつ副作用無く判定できるようにする
/// ため、`main()`の一番最初でチェックする)。
fn handle_version_flag_and_exit_if_present() {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("open-easy-web-server {}", auto_update::current_version_display());
        std::process::exit(0);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    handle_version_flag_and_exit_if_present();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // ローカルモード(Windows PC上での簡易リバースプロキシ+DuckDNS、
    // `OPEN_EASY_WEB_LOCAL_MODE=1`のときのみ有効、2026-09-07新設):
    // 既存のVPS向けnginx/PHP-FPM経路とは完全に独立した早期分岐。
    // ここで分岐した場合、以降のVPS向け起動処理(AppState::from_env等)
    // には一切到達しない。
    if local_proxy::is_local_mode_enabled() {
        return local_proxy::run_local_mode().await;
    }

    let state = Arc::new(AppState::from_env());

    // 起動時に、前回終了時(クラッシュ・電源断・ディスク切断)にジャーナルへ
    // 記録されたが本体へ未反映だった書き込みを復元する(`dist_sync.rs`の
    // `replay_pending_writes`参照)。ブロッキングI/Oのため`spawn_blocking`
    // へ退避。失敗してもサーバー起動自体は妨げない(ログのみ、既存の
    // 「補助機能の失敗は権威パスをブロックしない」方針を踏襲)。
    {
        let dist_sync = Arc::clone(&state.dist_sync);
        let sites_root = state.sites_root.clone();
        match tokio::task::spawn_blocking(move || dist_sync::replay_pending_writes(&dist_sync, &sites_root)).await {
            Ok(Ok(count)) => {
                if count > 0 {
                    tracing::info!(count, "起動時ジャーナルリプレイで未反映の書き込みを復元しました");
                }
            }
            Ok(Err(e)) => tracing::warn!(error = %e, "起動時ジャーナルリプレイに失敗しました(サーバー起動は継続します)"),
            Err(e) => tracing::warn!(error = %e, "起動時ジャーナルリプレイタスクがパニックしました(サーバー起動は継続します)"),
        }
    }

    let bind_addr = state.server_bind_addr;

    // 深夜自動アップデートが有効な場合のみ`SO_REUSEPORT`(Unix限定)で
    // bindする——新旧プロセスが同じポートへ同時にbindでき、ハンドオフ
    // 通信無しでほぼゼロダウンタイムの切り替えができるようにするため
    // (`auto_update.rs`モジュールdoc参照)。無効時(既定)は従来通りの
    // 通常bindのままで、挙動は一切変わらない。
    let listener = bind_listener(bind_addr, state.auto_update.is_enabled()).await?;
    tracing::info!(%bind_addr, auto_update_enabled = state.auto_update.is_enabled(), "open-easy-web-server listening");

    tokio::spawn(lolipop_cron::run_scheduler(Arc::clone(&state.lolipop_cron)));
    tokio::spawn(storage_maintenance::run_scheduler(Arc::clone(&state.storage_maintenance)));

    {
        let auto_update_state = Arc::clone(&state.auto_update);
        let stop_accepting = Arc::clone(&state.stop_accepting);
        tokio::spawn(async move {
            auto_update::run_daily_check_loop(auto_update_state, bind_addr, stop_accepting).await;
        });
    }

    let stop_accepting = Arc::clone(&state.stop_accepting);
    accept_loop(listener, state, stop_accepting).await
}

/// `reuse_port`が`true`(深夜自動アップデート有効時のみ)ならUnix系OSで
/// `SO_REUSEPORT`を設定してからbindする(`socket2`経由、Windowsでは
/// `SO_REUSEPORT`相当が無いため通常のbindにフォールバックする)。
/// `reuse_port`が`false`(既定)の場合は、従来通り`TcpListener::bind`を
/// そのまま呼ぶだけで、依存も増やさない・挙動も一切変えない。
async fn bind_listener(addr: std::net::SocketAddr, reuse_port: bool) -> anyhow::Result<TcpListener> {
    if !reuse_port {
        return Ok(TcpListener::bind(addr).await?);
    }
    #[cfg(unix)]
    {
        use socket2::{Domain, Protocol, Socket, Type};
        let domain = if addr.is_ipv6() { Domain::IPV6 } else { Domain::IPV4 };
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
        socket.set_reuse_address(true)?;
        socket.set_reuse_port(true)?;
        socket.set_nonblocking(true)?;
        socket.bind(&addr.into())?;
        socket.listen(1024)?;
        Ok(TcpListener::from_std(socket.into())?)
    }
    #[cfg(not(unix))]
    {
        tracing::warn!("OPEN_EASYWEB_AUTO_UPDATE is enabled but SO_REUSEPORT is not available on this OS; falling back to a normal bind (handoff will not be zero-downtime here)");
        Ok(TcpListener::bind(addr).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN`を読み書きするテストが複数に
    /// 増えた(2026-08-28、`otp_qr`ログインモードの管理APIテスト追加)ため、
    /// 直列化する(Rustの既定テストランナーは並列実行するため、環境変数を
    /// 複数テストが同時に書き換えるとフレーキーになる——open-english側
    /// `local_agent.rs`/`vps_agent.rs`で実際に踏んだのと同じ既知の罠)。
    static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
            users: users::UserStore::load(dir.path().join("users.json"), Arc::new(db_encryption::DbEncryptionState::load(&dir.path().join("db-encryption.key")))),
            smtp: None,
            sms: None,
            dist_sync: Arc::new(dist_sync::DistSyncRegistry::new(dir.path().join("dist-sync-journal"))),
            auto_update: Arc::new(auto_update::AutoUpdateState::load(dir.path().join("auto-update.json"))),
            lolipop_cron: Arc::new(lolipop_cron::LolipopCron::load(dir.path().join("lolipop-cron.json"))),
            storage_maintenance: Arc::new(storage_maintenance::StorageMaintenance::load(dir.path().join("storage-maintenance.json"), dir.path().join("storage-tidy.toml"))),
            update_notify: Arc::new(update_notify::UpdateNotify::load(dir.path().join("update-notify.json"))),
            server_bind_addr: ([127, 0, 0, 1], 0).into(),
            stop_accepting: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            power_profile: Arc::new(power_profile::PowerProfileRegistry::new()),
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_state = state.clone();
        tokio::spawn(async move {
            let _ = accept_loop(listener, server_state, Arc::new(std::sync::atomic::AtomicBool::new(false))).await;
        });
        (addr, state)
    }



    /// `Sites/Upload`(Connect、POST、RS-JSON、ファイルは base64): セッション必須、パストラバーサル拒否、実際にディスクへ書かれる。
    #[tokio::test]
    async fn upload_over_connect_requires_session_rejects_traversal_and_writes_files() {
        let (addr, state) = spawn_test_server().await;
        let client = reqwest::Client::new();
        let url = format!("http://{addr}/open_easy_web.admin.v1.Sites/Upload");
        let body = serde_json::json!({
            "site": "up.example.tokyo",
            "files": [
                {"path": "index.php", "data_base64": "PD9waHAgZWNobyAxOyA/Pg=="},
                {"path": "sub/dir/style.css", "data_base64": "Ym9keXt9"},
            ],
        });

        // 認証なし → 401(Connect の形式)。
        let res = client.post(&url).json(&body).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);

        state.users.register("up@example.com", None, Some("up2@example.com".into())).unwrap();
        let token = state.auth.create_session("up@example.com");

        let res = client.post(&url).bearer_auth(&token).json(&body).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let v: serde_json::Value = res.json().await.unwrap();
        assert_eq!(v["files"].as_array().unwrap().len(), 2);
        let root = state.site_dir("up.example.tokyo").unwrap();
        assert_eq!(std::fs::read(root.join("index.php")).unwrap(), b"<?php echo 1; ?>");
        assert_eq!(std::fs::read(root.join("sub/dir/style.css")).unwrap(), b"body{}");

        // 64 KiB を超える本文(通常の上限より大きいが、アップロードでは許可される)も通る。
        {
            use base64::Engine as _;
            let big = base64::engine::general_purpose::STANDARD.encode(vec![b'A'; 300_000]);
            let res = client
                .post(&url)
                .bearer_auth(&token)
                .json(&serde_json::json!({"site": "up.example.tokyo", "files": [{"path": "big.bin", "data_base64": big}]}))
                .send()
                .await
                .unwrap();
            assert_eq!(res.status(), reqwest::StatusCode::OK);
            assert_eq!(std::fs::metadata(root.join("big.bin")).unwrap().len(), 300_000);
            // 一方、Connect の通常のメソッドは 64 KiB を超えると 413。
            let res = client
                .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/RequestOtp"))
                .json(&serde_json::json!({"contact": "x".repeat(70_000)}))
                .send()
                .await
                .unwrap();
            assert_eq!(res.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
        }

        // パストラバーサル・絶対パスは 400(何も書かない)。
        for bad in ["../evil.php", "/etc/passwd", "a/../../b"] {
            let res = client
                .post(&url)
                .bearer_auth(&token)
                .json(&serde_json::json!({"site": "up.example.tokyo", "files": [{"path": bad, "data_base64": "AA=="}]}))
                .send()
                .await
                .unwrap();
            assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST, "{bad}");
        }
        // 不正な base64 は 400。
        let res = client
            .post(&url)
            .bearer_auth(&token)
            .json(&serde_json::json!({"site": "up.example.tokyo", "files": [{"path": "x.txt", "data_base64": "!!!"}]}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
        // キー重複の本文は 400(RS-JSON)。
        let res = client
            .post(&url)
            .bearer_auth(&token)
            .header("content-type", "application/json")
            .body(r#"{"site":"a.tokyo","site":"b.tokyo","files":[]}"#)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
    }
    /// メールの確認リンク: GET では何も確定せず(古い形式の案内ページ=410)、Connect の `Account/ConfirmEmailChange`
    /// (POST、RS-JSON)でだけ確定する。トークンは1回限り。
    #[tokio::test]
    async fn contact_change_is_confirmed_only_by_connect_post_not_by_get() {
        let (addr, state) = spawn_test_server().await;
        let client = reqwest::Client::new();
        state.users.register("change@example.com", None, Some("change2@example.com".into())).unwrap();
        let token = state.auth.request_contact_change("change@example.com", "email", "renamed@example.com");

        // 旧形式のリンク(GET)は、何も確定せず、案内ページ(410)だけを返す。
        let res = client.get(format!("http://{addr}/api/auth/confirm-email-change?token={token}")).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::GONE);
        assert!(res.text().await.unwrap().contains("確認リンクの形式が変わりました"));
        assert!(state.users.exists("change@example.com"), "GET must not apply the change");

        // 空のトークン・不正なトークンは 400。
        let url = format!("http://{addr}/open_easy_web.admin.v1.Account/ConfirmEmailChange");
        let res = client.post(&url).json(&serde_json::json!({"token": ""})).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
        let res = client.post(&url).json(&serde_json::json!({"token": "nope"})).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);

        // POST で確定する。
        let res = client.post(&url).json(&serde_json::json!({"token": token})).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        assert!(state.users.exists("renamed@example.com"));
        // 同じトークンは、もう使えない。
        let res = client.post(&url).json(&serde_json::json!({"token": token})).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn site_actions_require_a_valid_session_over_real_http() {
        let (addr, _state) = spawn_test_server().await;
        let client = reqwest::Client::new();

        // 認証ヘッダ無し → 401。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Sites/CreateFolder")).json(&serde_json::json!({"site": "example.tokyo"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);

        // 不正なトークン → 401。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Sites/CreateFolder")).json(&serde_json::json!({"site": "example.tokyo"}))
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
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/RequestOtp"))
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
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/RequestOtp"))
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
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/VerifyOtp"))
            .json(&serde_json::json!({"contact": "user2@example.com", "code": code}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["email"].as_str(), Some("user@example.com"));
        let token = body["token"].as_str().unwrap().to_string();

        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Sites/CreateFolder")).json(&serde_json::json!({"site": "example.tokyo"}))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);

        // DELETE /api/sites/:name — 未登録ドメイン(vhostファイルが無い)への
        // 削除リクエストは冪等にOKを返す。認証無しでは401。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Sites/Delete")).json(&serde_json::json!({"site": "example.tokyo"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);

        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Sites/Delete")).json(&serde_json::json!({"site": "example.tokyo"}))
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
            .post(format!("http://{addr}/open_easy_web.admin.v1.Sites/RegisterAppserver"))
            .json(&serde_json::json!({"site": "example.tokyo",
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
            .post(format!("http://{addr}/open_easy_web.admin.v1.Sites/RegisterAppserver"))
            .bearer_auth(&token)
            .json(&serde_json::json!({"site": "example.tokyo",
                "shared_endpoint": "http://127.0.0.1:1",
                "kind": "poem_cosmo_tauri",
                "backend_addr": "127.0.0.1:9100",
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_GATEWAY);
    }

    /// `/admin/dist-sync/*`一連の実HTTPフロー(2026-07-25新設): 未設定
    /// (無効化)→トークン設定→登録→一覧→削除→一覧、を実際にHTTP経由で検証する。
    #[tokio::test]
    async fn dist_sync_admin_api_register_list_and_delete_over_real_http() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        let (addr, _state) = spawn_test_server().await;
        let client = reqwest::Client::new();

        // 環境変数未設定の間はこの管理API自体が無効(503)であることを確認
        // (誤って無認証公開しない安全側デフォルト)。
        std::env::remove_var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN");
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.DistSync/ListTargets")).header("content-type", "application/json").body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);

        // 環境変数を設定してから、間違ったトークンでは401。
        // NOTE: `cargo test`はデフォルトでテストを並列実行するため、環境変数
        // をテスト間で共有するとフレーキーになりうる。このプロセス内で
        // 環境変数を読むテストは本テストのみ(grepで確認済み)なので許容する。
        std::env::set_var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN", "test-dist-sync-token");
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.DistSync/ListTargets")).header("content-type", "application/json").body("{}")
            .header("x-admin-token", "wrong-token")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);

        // 正しいトークンで登録。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.DistSync/RegisterTarget"))
            .header("x-admin-token", "test-dist-sync-token")
            .json(&serde_json::json!({
                "host": "vps1.example.tokyo",
                "port": 22,
                "username": "sync-user",
                "password_env": "EASYWEB_VPS1_SFTP_PASSWORD",
                "remote_backup_dir": "/home/sync-user/easyweb-sync",
                "label": "VPS1",
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        let target_id = body["target"]["id"].as_str().unwrap().to_string();
        assert_eq!(body["target"]["host"], "vps1.example.tokyo");

        // 一覧に反映されている。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.DistSync/ListTargets")).header("content-type", "application/json").body("{}")
            .header("x-admin-token", "test-dist-sync-token")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["targets"].as_array().unwrap().len(), 1);

        // ディザスタ用退避先(Email)の設定も通ること(実SMTP接続はしない、
        // 設定を受け付けるだけの経路)。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.DistSync/SetDisasterFallback"))
            .header("x-admin-token", "test-dist-sync-token")
            .json(&serde_json::json!({
                "email": {
                    "smtp_host": "smtp.example.invalid",
                    "smtp_port": 587,
                    "smtp_username": "backup@example.invalid",
                    "smtp_password_env": "EASYWEB_BACKUP_SMTP_PASSWORD",
                    "from_address": "backup@example.invalid",
                    "to_address": "admin@example.invalid",
                }
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);

        // 削除。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.DistSync/DeleteTarget")).json(&serde_json::json!({"id": target_id}))
            .header("x-admin-token", "test-dist-sync-token")
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);

        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.DistSync/ListTargets")).header("content-type", "application/json").body("{}")
            .header("x-admin-token", "test-dist-sync-token")
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["targets"].as_array().unwrap().len(), 0);

        std::env::remove_var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN");
    }

    #[tokio::test]
    async fn totp_setup_enable_then_requires_code_on_next_login() {
        let (addr, state) = spawn_test_server().await;
        let client = reqwest::Client::new();
        state.users.register("totp-user@example.com", None, Some("totp-user2@example.com".into())).unwrap();

        // ログインしてセッションを得る(TOTP未設定の1回目はコード不要)。
        let auth::RequestOtpOutcome::Issued(code) = state.auth.request_otp("totp-user@example.com");
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/VerifyOtp"))
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
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/TotpSetup")).header("content-type", "application/json").body("{}")
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        let secret_b32 = body["secret"].as_str().unwrap().to_string();
        assert!(body["provisioning_uri"].as_str().unwrap().starts_with("otpauth://totp/"));
        // 2026-08-27追加: QRコード(SVG)が実際に生成されることを確認
        // (open-english/rs-syncと同じ「QR併記」方式への統一)。
        let qr_svg = body["qr_svg"].as_str().expect("qr_svg should be present and non-null on success");
        assert!(qr_svg.contains("<svg"), "qr_svg should contain an actual <svg> element, got: {qr_svg}");
        assert!(qr_svg.len() > 100, "qr_svg should contain real QR module data, not a stub");
        assert!(body["qr_error"].is_null(), "qr_error should be null on success");

        // 間違ったコードでは有効化できない。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/TotpEnable")).header("content-type", "application/json").body("{}")
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
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/TotpEnable")).header("content-type", "application/json").body("{}")
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
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/VerifyOtp"))
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
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/TotpLogin")).header("content-type", "application/json").body("{}")
            .json(&serde_json::json!({"account_email": "totp-user@example.com", "totp_code": code_for_login}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["email"].as_str(), Some("totp-user@example.com"));
        assert!(body["token"].as_str().is_some());
    }

    /// TOTP登録済みのアカウントを1件作り、`account_email`と有効化済み
    /// シークレットを返すテストヘルパー(2026-08-28新設、QR系テストの
    /// 前提を共通化)。
    async fn register_and_enable_totp(addr: std::net::SocketAddr, state: &Arc<AppState>, email: &str) -> Vec<u8> {
        let client = reqwest::Client::new();
        state.users.register(email, None, Some(format!("backup-{email}"))).unwrap();
        let auth::RequestOtpOutcome::Issued(code) = state.auth.request_otp(email);
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/VerifyOtp"))
            .json(&serde_json::json!({"contact": email, "code": code}))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = res.json().await.unwrap();
        let token = body["token"].as_str().unwrap().to_string();

        let res = client.post(format!("http://{addr}/open_easy_web.admin.v1.Auth/TotpSetup")).header("content-type", "application/json").body("{}").bearer_auth(&token).send().await.unwrap();
        let body: serde_json::Value = res.json().await.unwrap();
        let secret_b32 = body["secret"].as_str().unwrap().to_string();
        let secret_bytes = totp::base32_to_secret(&secret_b32).unwrap();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let correct_code = totp::code_at(&secret_bytes, now);
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/TotpEnable")).header("content-type", "application/json").body("{}")
            .bearer_auth(&token)
            .json(&serde_json::json!({"code": correct_code}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        secret_bytes
    }

    /// `qr`モード(QR撮影のみ、事前のOTP検証なし)の一連の流れを実HTTP
    /// 経由で検証する(2026-08-28新設)。
    #[tokio::test]
    async fn qr_only_login_full_flow_over_real_http() {
        let (addr, state) = spawn_test_server().await;
        register_and_enable_totp(addr, &state, "qr-user@example.com").await;
        let client = reqwest::Client::new();

        // start: TOTP登録済みアカウントならQRセッションを開始できる。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginStart")).header("content-type", "application/json").body("{}")
            .json(&serde_json::json!({"account_email": "qr-user@example.com"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        let id = body["qr_login_id"].as_str().unwrap().to_string();
        assert!(body["qr_svg"].as_str().unwrap().contains("<svg"));
        assert!(body["confirm_url"].as_str().unwrap().contains(&id));

        // whoami: マスク済みメールアドレスが取得できる(実メールアドレス
        // そのものは返さない)。
        let res = client.post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginWhoami")).json(&serde_json::json!({"id": id})).send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        let masked = body["masked_email"].as_str().unwrap();
        assert!(masked.contains("***@example.com"));
        assert!(!masked.contains("qr-user@example.com"));

        // status: 確認前はfalse。
        let res = client.post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginStatus")).json(&serde_json::json!({"id": id})).send().await.unwrap();
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["confirmed"].as_bool(), Some(false));

        // confirm: 確認端末(qr-confirm.html)側のリクエストを模す。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginConfirm")).header("content-type", "application/json").body("{}")
            .json(&serde_json::json!({"id": id}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);

        // status: 確認後はtrue。
        let res = client.post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginStatus")).json(&serde_json::json!({"id": id})).send().await.unwrap();
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["confirmed"].as_bool(), Some(true));

        // finish: プライマリ端末がセッショントークンを受け取る。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginFinish")).header("content-type", "application/json").body("{}")
            .json(&serde_json::json!({"id": id}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        let token = body["token"].as_str().unwrap().to_string();
        assert_eq!(state.auth.session_email(&token).as_deref(), Some("qr-user@example.com"));

        // 使い捨て: 同じidで再度finishを呼ぶと失敗する。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginFinish")).header("content-type", "application/json").body("{}")
            .json(&serde_json::json!({"id": id}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
    }

    /// TOTP未登録アカウントは`qr`モードで拒否される(実HTTP経由)。
    #[tokio::test]
    async fn qr_only_login_rejects_accounts_without_totp() {
        let (addr, state) = spawn_test_server().await;
        state.users.register("no-totp-qr@example.com", None, Some("backup@example.com".into())).unwrap();
        let client = reqwest::Client::new();
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginStart")).header("content-type", "application/json").body("{}")
            .json(&serde_json::json!({"account_email": "no-totp-qr@example.com"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::FORBIDDEN);
    }

    /// `otp_qr`モード: メールOTP検証成功後、TOTPコード入力の代わりに
    /// QR確認セッションが返され、確認後にログインが完了することを実HTTP
    /// 経由で検証する(2026-08-28新設、真の2FA)。
    #[tokio::test]
    async fn otp_qr_mode_uses_qr_confirmation_as_the_second_factor_over_real_http() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN", "test-dist-sync-token");
        let (addr, state) = spawn_test_server().await;
        register_and_enable_totp(addr, &state, "otpqr-user@example.com").await;
        let client = reqwest::Client::new();

        // 管理APIでotp_qrモードへ切り替える。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Settings/SetLoginMode"))
            .header("x-admin-token", "test-dist-sync-token")
            .json(&serde_json::json!({"login_mode": "otp_qr"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["login_mode"].as_str(), Some("otp_qr"));

        // GETでも反映が確認できる。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Settings/GetLoginMode")).header("content-type", "application/json").body("{}")
            .header("x-admin-token", "test-dist-sync-token")
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = res.json().await.unwrap();
        assert_eq!(body["login_mode"].as_str(), Some("otp_qr"));

        // メールOTPを検証すると、TOTPコードではなくQRセッションが返る。
        let auth::RequestOtpOutcome::Issued(code) = state.auth.request_otp("otpqr-user@example.com");
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/VerifyOtp"))
            .json(&serde_json::json!({"contact": "otpqr-user@example.com", "code": code}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        assert!(body["token"].is_null(), "no session token should be issued before QR confirmation");
        let id = body["qr_login_id"].as_str().expect("qr_login_id should be present in otp_qr mode").to_string();

        // QR確認→finishでセッション発行。
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginConfirm")).header("content-type", "application/json").body("{}")
            .json(&serde_json::json!({"id": id}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/QrLoginFinish")).header("content-type", "application/json").body("{}")
            .json(&serde_json::json!({"id": id}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = res.json().await.unwrap();
        let token = body["token"].as_str().unwrap().to_string();
        assert_eq!(state.auth.session_email(&token).as_deref(), Some("otpqr-user@example.com"));

        // 後始末: 他テストへ影響しないよう既定のotpへ戻す。
        let _ = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Settings/SetLoginMode"))
            .header("x-admin-token", "test-dist-sync-token")
            .json(&serde_json::json!({"login_mode": "otp"}))
            .send()
            .await;
        std::env::remove_var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN");
    }

    /// 不正な`login_mode`値は拒否される。
    #[tokio::test]
    async fn login_mode_admin_api_rejects_unknown_value() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN", "test-dist-sync-token");
        let (addr, _state) = spawn_test_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Settings/SetLoginMode"))
            .header("x-admin-token", "test-dist-sync-token")
            .json(&serde_json::json!({"login_mode": "bogus"}))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
        std::env::remove_var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN");
    }

    #[tokio::test]
    async fn totp_login_rejects_accounts_without_totp_enabled() {
        let (addr, state) = spawn_test_server().await;
        state.users.register("no-totp@example.com", None, Some("no-totp2@example.com".into())).unwrap();
        let client = reqwest::Client::new();
        let res = client
            .post(format!("http://{addr}/open_easy_web.admin.v1.Auth/TotpLogin")).header("content-type", "application/json").body("{}")
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
            users: users::UserStore::load(dir.path().join("users.json"), Arc::new(db_encryption::DbEncryptionState::load(&dir.path().join("db-encryption.key")))),
            sms: None,
            smtp: None,
            dist_sync: Arc::new(dist_sync::DistSyncRegistry::new(dir.path().join("dist-sync-journal"))),
            auto_update: Arc::new(auto_update::AutoUpdateState::load(dir.path().join("auto-update.json"))),
            lolipop_cron: Arc::new(lolipop_cron::LolipopCron::load(dir.path().join("lolipop-cron.json"))),
            storage_maintenance: Arc::new(storage_maintenance::StorageMaintenance::load(dir.path().join("storage-maintenance.json"), dir.path().join("storage-tidy.toml"))),
            update_notify: Arc::new(update_notify::UpdateNotify::load(dir.path().join("update-notify.json"))),
            server_bind_addr: ([127, 0, 0, 1], 0).into(),
            stop_accepting: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            power_profile: Arc::new(power_profile::PowerProfileRegistry::new()),
        };
        assert!(state.site_dir("../etc").is_none());
        assert_eq!(state.site_dir("audiocafe.tokyo"), Some(PathBuf::from("/var/www/audiocafe.tokyo")));
    }
}
