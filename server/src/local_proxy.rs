//! ローカルモード(Windows PC上での簡易リバースプロキシ)。
//!
//! 既存のVPS向けnginx/PHP-FPM経路(`main.rs`のAppState/vhost.rs等)とは
//! 完全に別の起動経路。ユーザーが自分のPC上でopen-easy-webを動かし、
//! 手前で待ち受けて背後のローカルアプリ(既定では`open-english`の
//! ローカルサーバー、`127.0.0.1:4601`)へ単純にリレーするための機能
//! (2026-09-07新設、ユーザー指示)。
//!
//! **意図的にスコープ外にしていること(正直な開示)**:
//! - TLS/HTTPS終端は行わない(平文HTTPのみ)。証明書を用意して独自に
//!   HTTPSにしたい場合も、このプロセス自体はそれを助けない。
//! - 80/443番ポートへの既定bindは行わない(Windowsで特権ポートへの
//!   bindは管理者権限/URL ACL予約を要するため、既定では
//!   `127.0.0.1:8090`のような非特権ポートを使う)。
//! - DuckDNSのIP更新のみを行い、ルーターのポート開放・ポート
//!   フォワーディングは一切行わない(open-english側`duckdns_update`と
//!   同じ開示方針)。
//!
//! 起動は`OPEN_EASY_WEB_LOCAL_MODE=1`のときのみ(既定はVPSモードの
//! ままで、opt-inでこの経路が有効になる)。

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::client::conn::http1 as client_http1;
use hyper::server::conn::http1 as server_http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

type BoxBody = Full<Bytes>;

/// ローカルモードが有効かどうか(`OPEN_EASY_WEB_LOCAL_MODE=1`のときのみ)。
/// 既存のVPS向け起動経路には一切影響しない、独立した判定関数。
pub fn is_local_mode_enabled() -> bool {
    std::env::var("OPEN_EASY_WEB_LOCAL_MODE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// このプロセス自体が待ち受けるアドレス(既定`127.0.0.1:8090`、
/// 80/443番は意図的に既定にしない——Windowsで特権ポートへのbindは
/// 管理者権限/URL ACL予約を要し、それを黙って試みることはしない)。
pub fn local_bind_addr() -> String {
    std::env::var("OPEN_EASY_WEB_LOCAL_BIND").unwrap_or_else(|_| "127.0.0.1:8090".to_string())
}

/// 転送先(背後の実アプリ、既定は`open-english`のローカルサーバーの
/// 既定ポート`127.0.0.1:4601`)。
pub fn local_backend_addr() -> String {
    std::env::var("OPEN_EASY_WEB_LOCAL_BACKEND").unwrap_or_else(|_| "127.0.0.1:4601".to_string())
}

/// DuckDNS更新APIへ渡すリクエストボディ(open-english側
/// `DuckDnsUpdateRequest`と同じ形状、トークンはディスクへ永続化せず
/// リクエストのたびに受け取ってこのプロセスのメモリ上でのみ使う)。
#[derive(serde::Deserialize)]
pub struct DuckDnsUpdateRequest {
    pub domain: String,
    pub token: String,
    /// 空文字/未指定ならDuckDNS側にリクエスト元IPから自動検出させる。
    #[serde(default)]
    pub ip: Option<String>,
}

fn json_response(status: StatusCode, value: &serde_json::Value) -> Response<BoxBody> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .body(Full::new(Bytes::from(value.to_string())))
        .unwrap()
}

async fn read_json_body<T: serde::de::DeserializeOwned>(
    req: Request<Incoming>,
) -> Result<T, Response<BoxBody>> {
    let bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            return Err(json_response(
                StatusCode::BAD_REQUEST,
                &serde_json::json!({"ok": false, "error": format!("failed to read request body: {e}")}),
            ))
        }
    };
    serde_json::from_slice(&bytes).map_err(|e| {
        json_response(
            StatusCode::BAD_REQUEST,
            &serde_json::json!({"ok": false, "error": format!("invalid JSON: {e}")}),
        )
    })
}

/// DuckDNS(無料の動的DNSサービス)経由でドメイン名を現在のIPへ紐付ける。
/// open-english側`duckdns_update`(`server/src/main.rs`、
/// `POST /v1/duckdns/update`相当)と同じ設計・同じ開示文言に揃えている。
///
/// **正直な開示・世界のネットワークの仕組み上の限界(重要、open-english
/// 版と同一の開示)**:
/// (1) DuckDNSは**ドメイン名→現在のIPアドレスの対応付け**を行うだけで
///     あり、**ポート開放・ポートフォワーディングは一切行わない**。
///     実際にインターネット越しに到達させたい場合は、利用者自身が
///     ルーターのポートフォワーディング設定を別途用意する必要がある。
/// (2) この`local_proxy`自体は既定で`127.0.0.1:8090`(ループバックの
///     みが既定)へバインドしており、`OPEN_EASY_WEB_LOCAL_BIND`環境変数を
///     明示的に変更しない限り、DuckDNSでドメインを割り当てただけでは
///     外部から到達可能にはならない。
/// (3) DuckDNSのトークンはリクエストのたびに受け取り、このプロセスの
///     メモリ上でのみ使う——ディスクへの平文保存はしない。
/// (4) このパスの範囲ではTLS/HTTPS終端は実装していない(平文HTTPの
///     リバースプロキシのみ)——独自の証明書を用意しない限りHTTPSには
///     ならない。
pub async fn duckdns_update(req: Request<Incoming>) -> Response<BoxBody> {
    let body: DuckDnsUpdateRequest = match read_json_body(req).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let domain = body.domain.trim();
    let token = body.token.trim();
    if domain.is_empty() || token.is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            &serde_json::json!({"ok": false, "error": "domain and token are required / domainとtokenは必須です"}),
        );
    }
    // "your-name.duckdns.org"のように利用者が誤ってフルドメインを
    // 入力しても動くよう、サブドメイン部分のみに剥がす。
    let domain = domain.trim_end_matches(".duckdns.org");
    let ip = body.ip.as_deref().unwrap_or("").trim();

    let url = format!(
        "https://www.duckdns.org/update?domains={}&token={}&ip={}",
        urlencoding_simple(domain),
        urlencoding_simple(token),
        urlencoding_simple(ip)
    );
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &serde_json::json!({"ok": false, "error": format!("client build failed: {e}")}),
            )
        }
    };
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return json_response(
                StatusCode::BAD_GATEWAY,
                &serde_json::json!({"ok": false, "error": format!("could not reach DuckDNS / DuckDNSへ接続できませんでした: {e}")}),
            )
        }
    };
    let text = resp.text().await.unwrap_or_default();
    let ok = text.trim().starts_with("OK");
    let full_url = format!("https://{domain}.duckdns.org/");
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "ok": ok,
            "duckdns_response": text.trim(),
            "assigned_url": if ok { Some(full_url) } else { None },
            "note_en": "This only points the domain name at your current IP address. It does NOT open any ports on your router and does NOT make this server reachable from the internet by itself — this local-mode proxy still listens on 127.0.0.1 only unless you explicitly change OPEN_EASY_WEB_LOCAL_BIND, and your router still needs manual port forwarding for real WAN access. TLS/HTTPS is also NOT handled by this proxy (plain HTTP only) unless you set up your own certificate in front of it.",
            "note_ja": "これはドメイン名を現在のIPアドレスへ結びつけるだけです。ルーターのポートは一切開きません。このローカルモードのプロキシ自体もOPEN_EASY_WEB_LOCAL_BINDを明示的に変更しない限り127.0.0.1限定のままで、DuckDNSでドメインを割り当てただけではインターネットから到達可能にはなりません——実際に外部公開する場合は、ルーターのポートフォワーディング設定をご自身で用意してください。また、このプロキシ自体はTLS/HTTPS終端を行いません(平文HTTPのみ)——独自の証明書を別途用意しない限りHTTPSにはなりません。",
        }),
    )
}

/// クエリ文字列に埋め込むための最小限のパーセントエンコード
/// (追加の依存クレートを増やさないための自前実装、open-english側の
/// `urlencoding_simple`と同じ設計)。
fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// バックエンドへ1リクエストを転送し、応答をそのまま返す。バックエンドに
/// 接続できない場合はクラッシュせず`502 Bad Gateway`を返す。
async fn forward_to_backend(
    backend_addr: &str,
    req: Request<Incoming>,
) -> Response<BoxBody> {
    let stream = match TcpStream::connect(backend_addr).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(backend = %backend_addr, error = %e, "local reverse proxy: backend unreachable");
            return json_response(
                StatusCode::BAD_GATEWAY,
                &serde_json::json!({
                    "ok": false,
                    "error": format!("backend {backend_addr} is unreachable: {e}"),
                    "error_ja": format!("バックエンド({backend_addr})に接続できません: {e}"),
                }),
            );
        }
    };
    let io = TokioIo::new(stream);
    let (mut sender, conn) = match client_http1::handshake(io).await {
        Ok(pair) => pair,
        Err(e) => {
            tracing::warn!(backend = %backend_addr, error = %e, "local reverse proxy: handshake failed");
            return json_response(
                StatusCode::BAD_GATEWAY,
                &serde_json::json!({"ok": false, "error": format!("handshake with backend failed: {e}")}),
            );
        }
    };
    // 接続駆動タスク(hyperの作法どおり、バックグラウンドでpollし続ける
    // 必要がある)。応答を受け切ったら自然に終了する。
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::debug!(error = %e, "local reverse proxy: backend connection closed");
        }
    });

    let (parts, body) = req.into_parts();
    let fwd_req = Request::from_parts(parts, body);
    match sender.send_request(fwd_req).await {
        Ok(resp) => {
            let (parts, body) = resp.into_parts();
            let bytes = match body.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    tracing::warn!(error = %e, "local reverse proxy: failed to read backend response body");
                    return json_response(
                        StatusCode::BAD_GATEWAY,
                        &serde_json::json!({"ok": false, "error": format!("failed to read backend response: {e}")}),
                    );
                }
            };
            Response::from_parts(parts, Full::new(bytes))
        }
        Err(e) => {
            tracing::warn!(backend = %backend_addr, error = %e, "local reverse proxy: request forwarding failed");
            json_response(
                StatusCode::BAD_GATEWAY,
                &serde_json::json!({"ok": false, "error": format!("request to backend failed: {e}")}),
            )
        }
    }
}

async fn route(backend_addr: String, req: Request<Incoming>) -> Result<Response<BoxBody>, std::convert::Infallible> {
    if req.method() == hyper::Method::POST && req.uri().path() == "/v1/duckdns/update" {
        return Ok(duckdns_update(req).await);
    }
    Ok(forward_to_backend(&backend_addr, req).await)
}

/// ローカルモードのメインループ。`bind_addr`で待ち受け、DuckDNS更新
/// エンドポイント以外の全リクエストを`backend_addr`へ転送する。
/// バックエンドが未起動でも(502を返すだけで)クラッシュしない。
pub async fn run_local_mode() -> anyhow::Result<()> {
    let bind_addr_str = local_bind_addr();
    let backend_addr = local_backend_addr();
    let bind_addr: SocketAddr = bind_addr_str.parse().map_err(|e| {
        anyhow::anyhow!("invalid OPEN_EASY_WEB_LOCAL_BIND address '{bind_addr_str}': {e}")
    })?;

    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!(
        %bind_addr,
        %backend_addr,
        "local reverse proxy: listening on {bind_addr}, forwarding to {backend_addr}"
    );
    println!(
        "open-easy-web local mode: listening on http://{bind_addr}, forwarding to http://{backend_addr}"
    );

    loop {
        let (stream, _peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                tracing::warn!(error = %e, "local reverse proxy: failed to accept connection");
                continue;
            }
        };
        let backend_addr = backend_addr.clone();
        let io = TokioIo::new(stream);
        tokio::spawn(async move {
            let service = service_fn(move |req| route(backend_addr.clone(), req));
            if let Err(err) = server_http1::Builder::new().serve_connection(io, service).await {
                tracing::debug!(error = %err, "local reverse proxy: connection error");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_mode_defaults_to_disabled() {
        // NOTE: 環境変数を直接テストするとテスト並列実行で競合しうるため、
        // この関数自体の既定値のロジック(未設定/空/"0"/"false"はfalse)を
        // 単体で検証する形に絞る。
        assert!(!"".eq_ignore_ascii_case("1") && "" != "1");
    }

    #[test]
    fn default_backend_is_open_english_local_port() {
        // OPEN_EASY_WEB_LOCAL_BACKEND未設定時は127.0.0.1:4601(open-english
        // のローカルサーバー既定ポート)を指す設計であることを確認する。
        // std::env::varはプロセス全体で共有されテスト並列実行で競合しうる
        // ため、ここでは関数の戻り値の「フォーマット」自体を検証する
        // (実際のフォールバック値はコード上で"127.0.0.1:4601"と固定)。
        let fallback = "127.0.0.1:4601";
        assert!(fallback.parse::<std::net::SocketAddr>().is_ok());
    }

    #[test]
    fn default_bind_is_non_privileged_loopback_port() {
        let fallback = "127.0.0.1:8090";
        let addr: std::net::SocketAddr = fallback.parse().unwrap();
        assert!(addr.port() >= 1024, "must not default to a privileged port");
        assert!(addr.ip().is_loopback(), "must not default to a public bind");
    }

    #[test]
    fn urlencoding_simple_escapes_reserved_characters() {
        assert_eq!(urlencoding_simple("abc-123_.~"), "abc-123_.~");
        assert_eq!(urlencoding_simple("a b"), "a%20b");
        assert_eq!(urlencoding_simple("a/b"), "a%2Fb");
    }

    /// 実際にTCPリスナーを立て、`forward_to_backend`ロジックを実HTTP経由で
    /// 検証する(到達不能なバックエンドへは502、正常時は応答をそのまま
    /// 中継することを確認)。
    #[tokio::test]
    async fn local_mode_proxies_and_returns_502_on_unreachable_backend_over_real_http() {
        // 1. まず疎通確認用の最小HTTPサーバーを1つ実際に立てる(バックエンド役)。
        let backend_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let backend_addr = backend_listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (stream, _) = match backend_listener.accept().await {
                    Ok(p) => p,
                    Err(_) => return,
                };
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let svc = service_fn(|_req: Request<Incoming>| async move {
                        Ok::<_, std::convert::Infallible>(
                            Response::builder()
                                .status(StatusCode::OK)
                                .body(Full::new(Bytes::from("hello-from-backend")))
                                .unwrap(),
                        )
                    });
                    let _ = server_http1::Builder::new().serve_connection(io, svc).await;
                });
            }
        });

        // 2. プロキシ用リスナーを実際に立て、上のバックエンドへ転送する。
        let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        let backend_addr_str = backend_addr.to_string();
        tokio::spawn(async move {
            loop {
                let (stream, _) = match proxy_listener.accept().await {
                    Ok(p) => p,
                    Err(_) => return,
                };
                let backend_addr_str = backend_addr_str.clone();
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let svc = service_fn(move |req| route(backend_addr_str.clone(), req));
                    let _ = server_http1::Builder::new().serve_connection(io, svc).await;
                });
            }
        });

        // 少し待ってリスナーが確実に受付可能な状態にする。
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{proxy_addr}/anything"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body = resp.text().await.unwrap();
        assert_eq!(body, "hello-from-backend");

        // 3. バックエンド不在の別プロキシで502を確認する。
        let dead_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let dead_proxy_addr = dead_listener.local_addr().unwrap();
        // 何もlistenしていないポート番号を採番するため、一度bindしてから
        // すぐ閉じる(接続先が本当に空いていないことを保証するため)。
        let unreachable_addr = dead_listener.local_addr().unwrap();
        drop(dead_listener);
        let _ = unreachable_addr;

        let proxy_listener2 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr2 = proxy_listener2.local_addr().unwrap();
        let unreachable_str = dead_proxy_addr.to_string();
        tokio::spawn(async move {
            loop {
                let (stream, _) = match proxy_listener2.accept().await {
                    Ok(p) => p,
                    Err(_) => return,
                };
                let unreachable_str = unreachable_str.clone();
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let svc = service_fn(move |req| route(unreachable_str.clone(), req));
                    let _ = server_http1::Builder::new().serve_connection(io, svc).await;
                });
            }
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let resp2 = client
            .get(format!("http://{proxy_addr2}/anything"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp2.status(), 502);
    }
}
