//! 「分身の術」構想の仕上げ: `open-easy-web`でドメインを登録する際、
//! すでに稼働中の共有バックエンド(`open-web-server`または
//! `poem-cosmo-tauri`)の管理APIを実際に呼び出し、そのドメインを動的に
//! 登録する。これにより、ドメインを追加するたびに新しいバックエンド
//! プロセスを個別インストール・起動する必要が無くなる
//! (2026-07-16、ユーザー指示)。
//!
//! `open-web-server`側は`POST /admin/tenants`(HTTPルーティング)+
//! `POST /admin/tenants/:host/tls`(TLS証明書、任意)、
//! `poem-cosmo-tauri`側は`POST /admin/appserver-tenants`
//! (アプリケーションサーバー層のルーティング)。どちらも「共有インスタンス
//! への動的テナント登録」という同じ目的だが、リポジトリごとにAPI形状が
//! 異なるため、ここで吸収する。

use serde::{Deserialize, Serialize};

/// `SiteProfile.app_server`(WASM側、`profiles.rs`)と対応する、
/// どちらの共有バックエンドへ登録するかの選択。
///
/// `AruaruLlm`(2026-07-18追加): `aruaru-llm`(契約不要の独自AI
/// チャットコマース応答サービス、`open-cuda`とSET構成)も同じ
/// 「分身の術」(共有インスタンスへの動的テナント登録、ドメインごとの
/// 個別インストール不要)パターンを採用しているため、既存の
/// open-runo/poem-cosmo-tauriと同じ管理APIから登録できるようにする。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppServerKind {
    OpenRuno,
    PoemCosmoTauri,
    AruaruLlm,
}

/// `POST /api/sites/:name/register-appserver`のリクエストボディ。
#[derive(Debug, Deserialize)]
pub struct RegisterAppserverRequest {
    /// 共有バックエンド管理APIのベースURL(例:
    /// "http://127.0.0.1:8080")。この共有インスタンス自体は
    /// 既に稼働中である前提——本関数はそこへ動的登録するだけで、
    /// 新規プロセスは一切起動しない。
    pub shared_endpoint: String,
    pub kind: AppServerKind,
    /// このサイトのバックエンド実処理先(例: "127.0.0.1:9001")。
    pub backend_addr: String,
    /// 共有バックエンド側の管理API認証(open-web-serverは
    /// `x-admin-token`ヘッダ、poem-cosmo-tauriは`x-api-key`ヘッダ
    /// ——両方ともこの1つの値をそのまま使う。未設定の共有インスタンス
    /// 側では無視される)。
    #[serde(default)]
    pub admin_key: Option<String>,
    /// `open-web-server`向けのみ必須(`TenantConfig.db_uri`)。
    /// poem-cosmo-tauri向け登録では無視される。
    #[serde(default)]
    pub db_uri: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    #[error("open-web-server registration requires db_uri")]
    MissingDbUri,
    #[error("request to {endpoint} failed: {source}")]
    Http { endpoint: String, #[source] source: reqwest::Error },
    #[error("{endpoint} responded with {status}: {body}")]
    UnexpectedStatus { endpoint: String, status: reqwest::StatusCode, body: String },
}

/// `host`を、`req.kind`が指す共有バックエンドへ実際に動的登録する。
/// 成功すれば、そのバックエンドは次のリクエストから即座に`host`向けの
/// トラフィックを処理できる(バックエンドプロセスの再起動は不要——
/// `TenantRegistry`/`SharedDispatcher`はどちらも`RwLock`で実行時更新
/// できる設計のため)。
pub async fn register(client: &reqwest::Client, host: &str, req: &RegisterAppserverRequest) -> Result<(), RegisterError> {
    match req.kind {
        AppServerKind::OpenRuno => register_open_web_server(client, host, req).await,
        AppServerKind::PoemCosmoTauri => register_poem_cosmo_tauri(client, host, req).await,
        AppServerKind::AruaruLlm => register_aruaru_llm(client, host, req).await,
    }
}

async fn register_open_web_server(client: &reqwest::Client, host: &str, req: &RegisterAppserverRequest) -> Result<(), RegisterError> {
    let Some(db_uri) = &req.db_uri else {
        return Err(RegisterError::MissingDbUri);
    };

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    enum Backend {
        OpenRuno,
    }
    #[derive(Serialize)]
    struct TenantConfig<'a> {
        host: &'a str,
        backend: Backend,
        backend_addr: &'a str,
        db_uri: &'a str,
    }

    let endpoint = format!("{}/admin/tenants", req.shared_endpoint.trim_end_matches('/'));
    let mut builder = client.post(&endpoint).json(&TenantConfig {
        host,
        backend: Backend::OpenRuno,
        backend_addr: &req.backend_addr,
        db_uri,
    });
    if let Some(key) = &req.admin_key {
        builder = builder.header("x-admin-token", key);
    }

    send_and_check(builder, &endpoint).await
}

async fn register_poem_cosmo_tauri(client: &reqwest::Client, host: &str, req: &RegisterAppserverRequest) -> Result<(), RegisterError> {
    #[derive(Serialize)]
    struct AppserverTenant<'a> {
        host: &'a str,
        backend_addr: &'a str,
    }

    let endpoint = format!("{}/admin/appserver-tenants", req.shared_endpoint.trim_end_matches('/'));
    let mut builder = client.post(&endpoint).json(&AppserverTenant { host, backend_addr: &req.backend_addr });
    if let Some(key) = &req.admin_key {
        builder = builder.header("x-api-key", key);
    }

    send_and_check(builder, &endpoint).await
}

/// `aruaru-llm`の`POST /admin/tenants`(`src/tenants.rs::TenantRegistry`)
/// へ動的登録する。`backend_addr`/`db_uri`は使わない(`aruaru-llm`は
/// バックエンドproxy先を持たず、単に「どのドメインが利用中か」を
/// 記録するだけのため)。
async fn register_aruaru_llm(client: &reqwest::Client, host: &str, req: &RegisterAppserverRequest) -> Result<(), RegisterError> {
    #[derive(Serialize)]
    struct TenantInfo<'a> {
        host: &'a str,
        label: Option<&'a str>,
    }

    let endpoint = format!("{}/admin/tenants", req.shared_endpoint.trim_end_matches('/'));
    let mut builder = client.post(&endpoint).json(&TenantInfo { host, label: None });
    if let Some(key) = &req.admin_key {
        builder = builder.header("x-admin-token", key);
    }

    send_and_check(builder, &endpoint).await
}

async fn send_and_check(builder: reqwest::RequestBuilder, endpoint: &str) -> Result<(), RegisterError> {
    let resp = builder.send().await.map_err(|source| RegisterError::Http { endpoint: endpoint.to_string(), source })?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().await.unwrap_or_default();
    Err(RegisterError::UnexpectedStatus { endpoint: endpoint.to_string(), status, body })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use std::sync::{Arc, Mutex};

    use http_body_util::{BodyExt, Full};
    use hyper::body::{Bytes, Incoming};
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    /// 実TCPループバック上に、リクエストのメソッド/パス/ヘッダ/ボディを
    /// そのまま記録する超薄いモックサーバーを立てる(共有バックエンドの
    /// 実物を用意せずに、"正しいAPIを正しい形で叩いたか"を実HTTPで検証
    /// するため——このリポジトリの既存統合テストと同じ「モックで済ませず
    /// 実TCP経由で検証する」方針を踏襲)。
    #[derive(Default)]
    struct RecordedRequest {
        method: String,
        path: String,
        header: Option<String>,
        body: serde_json::Value,
    }

    async fn start_recording_server(header_name: &'static str) -> (std::net::SocketAddr, Arc<Mutex<Option<RecordedRequest>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let recorded: Arc<Mutex<Option<RecordedRequest>>> = Arc::new(Mutex::new(None));
        let recorded_clone = Arc::clone(&recorded);

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let io = TokioIo::new(stream);
            let recorded = Arc::clone(&recorded_clone);
            let service = service_fn(move |req: Request<Incoming>| {
                let recorded = Arc::clone(&recorded);
                async move {
                    let method = req.method().to_string();
                    let path = req.uri().path().to_string();
                    let header = req.headers().get(header_name).and_then(|v| v.to_str().ok()).map(str::to_string);
                    let bytes = req.into_body().collect().await.unwrap().to_bytes();
                    let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
                    *recorded.lock().unwrap() = Some(RecordedRequest { method, path, header, body });
                    Ok::<_, Infallible>(Response::new(Full::new(Bytes::from_static(b"{}"))))
                }
            });
            let _ = http1::Builder::new().serve_connection(io, service).await;
        });

        (addr, recorded)
    }

    #[tokio::test]
    async fn registers_open_web_server_tenant_with_expected_shape() {
        let (addr, recorded) = start_recording_server("x-admin-token").await;
        let req = RegisterAppserverRequest {
            shared_endpoint: format!("http://{addr}"),
            kind: AppServerKind::OpenRuno,
            backend_addr: "127.0.0.1:9001".to_string(),
            admin_key: Some("secret-token".to_string()),
            db_uri: Some("postgres://localhost/shop".to_string()),
        };

        register(&reqwest::Client::new(), "shop.example.jp", &req).await.expect("registration should succeed");

        let r = recorded.lock().unwrap().take().expect("server should have recorded a request");
        assert_eq!(r.method, "POST");
        assert_eq!(r.path, "/admin/tenants");
        assert_eq!(r.header.as_deref(), Some("secret-token"));
        assert_eq!(r.body["host"], "shop.example.jp");
        assert_eq!(r.body["backend"], "open_runo");
        assert_eq!(r.body["backend_addr"], "127.0.0.1:9001");
        assert_eq!(r.body["db_uri"], "postgres://localhost/shop");
    }

    #[tokio::test]
    async fn registers_poem_cosmo_tauri_tenant_with_expected_shape() {
        let (addr, recorded) = start_recording_server("x-api-key").await;
        let req = RegisterAppserverRequest {
            shared_endpoint: format!("http://{addr}"),
            kind: AppServerKind::PoemCosmoTauri,
            backend_addr: "127.0.0.1:9100".to_string(),
            admin_key: Some("test-key".to_string()),
            db_uri: None,
        };

        register(&reqwest::Client::new(), "app.example.jp", &req).await.expect("registration should succeed");

        let r = recorded.lock().unwrap().take().expect("server should have recorded a request");
        assert_eq!(r.method, "POST");
        assert_eq!(r.path, "/admin/appserver-tenants");
        assert_eq!(r.header.as_deref(), Some("test-key"));
        assert_eq!(r.body["host"], "app.example.jp");
        assert_eq!(r.body["backend_addr"], "127.0.0.1:9100");
    }

    #[tokio::test]
    async fn registers_aruaru_llm_tenant_with_expected_shape() {
        let (addr, recorded) = start_recording_server("x-admin-token").await;
        let req = RegisterAppserverRequest {
            shared_endpoint: format!("http://{addr}"),
            kind: AppServerKind::AruaruLlm,
            backend_addr: "unused".to_string(),
            admin_key: Some("llm-admin-token".to_string()),
            db_uri: None,
        };

        register(&reqwest::Client::new(), "e-gov.info", &req).await.expect("registration should succeed");

        let r = recorded.lock().unwrap().take().expect("server should have recorded a request");
        assert_eq!(r.method, "POST");
        assert_eq!(r.path, "/admin/tenants");
        assert_eq!(r.header.as_deref(), Some("llm-admin-token"));
        assert_eq!(r.body["host"], "e-gov.info");
    }

    #[tokio::test]
    async fn open_web_server_registration_without_db_uri_is_rejected_before_any_http_call() {
        let req = RegisterAppserverRequest {
            shared_endpoint: "http://127.0.0.1:1".to_string(), // unreachable on purpose
            kind: AppServerKind::OpenRuno,
            backend_addr: "127.0.0.1:9001".to_string(),
            admin_key: None,
            db_uri: None,
        };

        let err = register(&reqwest::Client::new(), "shop.example.jp", &req).await.unwrap_err();
        assert!(matches!(err, RegisterError::MissingDbUri));
    }
}
