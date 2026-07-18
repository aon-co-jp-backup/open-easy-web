//! 接続プロファイル(サイト)管理。
//!
//! KUSANAGI の「サイト追加/一覧」ダッシュボードのように、open-easy-web自身の
//! 管理画面や、他のプロジェクト(WordPress/Laravel/FastAPIなど任意の
//! バックエンドスタック)のデプロイ先(IPアドレス/ドメイン/サブドメイン/
//! ポート)を複数登録し、ブラウザの `localStorage` に保存してGUIから
//! 切り替え・疎通確認できるようにする。
//!
//! 実際のドメイン取得・DNS登録(レジストラ操作)はここでは行わない
//! (`deploy/` 以下の vhost テンプレート・`scripts/gen-vhost.sh` を参照)。
//! ここで管理するのはあくまで「登録済みサイトの一覧と、どれを選択中か」
//! という設定であり、DB(aruaru-db等)への接続機能は持たない。

use crate::dom::{by_id, document, esc, try_by_id};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};

const STORAGE_KEY: &str = "openeasyweb_site_profiles_v1";
const ACTIVE_KEY: &str = "openeasyweb_active_site_id_v1";

#[derive(Serialize, Deserialize, Clone)]
pub struct SiteProfile {
    pub id: String,
    pub name: String,
    /// "self"(open-easy-web自身の管理画面) | "other"(それ以外の任意のサイト)
    pub purpose: String,
    /// "http" | "https"
    pub protocol: String,
    /// IPアドレス、ドメイン、またはサブドメイン。
    pub host: String,
    pub port: u16,
    pub path: String,
    /// 自由記述のバックエンドスタック(例: "WordPress", "PHP + Laravel",
    /// "Python + FastAPI", "open-web-server (Rust)")。
    /// `scripts/gen-vhost.sh --stack` の選択の目安。open-web-server は
    /// フロントのエンジンではなく `--stack=proxy` のUPSTREAMとして指定する
    /// バックエンドなので、ここに自由記述で残すのみで `webserver_engine`
    /// の選択肢には含めない。
    pub backend_stack: String,
    /// vhostを配信するフロントエンジン。"nginx" | "apache" | "both"。
    /// `scripts/gen-vhost.sh --engine` / `scripts/switch-engine.sh` の
    /// 選択の目安(旧データとの互換のためデフォルトは"both")。
    #[serde(default = "default_engine")]
    pub webserver_engine: String,
    /// このドメインの動的処理を担うアプリケーションサーバー層。
    /// "none"(未割り当て) | "open-runo" | "poem-cosmo-tauri" |
    /// "aruaru-llm"(契約不要の独自AIチャットコマース応答サービス、
    /// `open-cuda`とSET構成——バックエンド接続先ではなくテナント登録
    /// のみを行う点が他の2つと異なる)。Apache+Tomcatの関係と同様、
    /// Webサーバー(nginx/apache/open-web-server)だけでも単体動作
    /// するため、"none"が既定値。
    /// `scripts/gen-vhost.sh --stack=proxy` のUPSTREAM、または
    /// `open-web-server-gateway` の `OPEN_WEB_SERVER_APP_UPSTREAM` に
    /// 対応する接続先は `app_server_upstream` に持つ。
    #[serde(default = "default_app_server")]
    pub app_server: String,
    /// アプリケーションサーバーの接続先(例: "127.0.0.1:8080")。
    /// `app_server` が "none" の場合は無視される。
    #[serde(default)]
    pub app_server_upstream: String,
    /// 「分身の術」構想(2026-07-16): 既に稼働中の共有バックエンド
    /// (`open-web-server`/`poem-cosmo-tauri`)の管理APIベースURL
    /// (例: "http://127.0.0.1:8080")。空なら「共有バックエンドへ登録」
    /// ボタンは無効(=個別インストール運用のまま、後方互換)。
    #[serde(default)]
    pub shared_appserver_endpoint: String,
    /// 共有バックエンド側の管理API認証(`x-admin-token`/`x-api-key`
    /// どちらのヘッダ名を使うかは`app_server`の値から自動選択する)。
    #[serde(default)]
    pub shared_appserver_admin_key: String,
    /// `app_server == "open-runo"`(open-web-server側)のみ必須。
    #[serde(default)]
    pub shared_appserver_db_uri: String,
    /// `open-easy-web-server`自身のセッショントークン(`Authorization:
    /// Bearer`)。**平文でlocalStorageに保存される点に注意**——このアプリ
    /// の他のフィールド(admin_key含む)と同じ保存方式に揃えた最小限の
    /// 運用ツールとしての妥協であり、本番の秘密情報管理には使わないこと。
    #[serde(default)]
    pub shared_appserver_session_token: String,
}

fn default_engine() -> String {
    "both".to_string()
}

fn default_app_server() -> String {
    "none".to_string()
}

impl SiteProfile {
    pub fn url(&self) -> String {
        format!(
            "{}://{}:{}{}",
            self.protocol, self.host, self.port, self.path
        )
    }
}

fn local_storage() -> Option<web_sys::Storage> {
    crate::dom::window().local_storage().ok().flatten()
}

fn default_profiles() -> Vec<SiteProfile> {
    vec![
        SiteProfile {
            id: "seed-self".to_string(),
            name: "open-easy-web(このサイト)".to_string(),
            purpose: "self".to_string(),
            protocol: "https".to_string(),
            host: "easy-web.tokyo".to_string(),
            port: 443,
            path: "/".to_string(),
            backend_stack: "Rust + WebAssembly".to_string(),
            webserver_engine: "nginx".to_string(),
            app_server: "none".to_string(),
            app_server_upstream: String::new(),
            shared_appserver_endpoint: String::new(),
            shared_appserver_admin_key: String::new(),
            shared_appserver_db_uri: String::new(),
            shared_appserver_session_token: String::new(),
        },
        SiteProfile {
            id: "seed-other-example".to_string(),
            name: "WordPressサイト(例)".to_string(),
            purpose: "other".to_string(),
            protocol: "https".to_string(),
            host: "example.com".to_string(),
            port: 443,
            path: "/".to_string(),
            backend_stack: "WordPress (PHP-FPM)".to_string(),
            webserver_engine: "apache".to_string(),
            app_server: "none".to_string(),
            app_server_upstream: String::new(),
            shared_appserver_endpoint: String::new(),
            shared_appserver_admin_key: String::new(),
            shared_appserver_db_uri: String::new(),
            shared_appserver_session_token: String::new(),
        },
    ]
}

/// `seed-self`のドメインが過去の値(`localhost:8080`→`easy-web.tokyo`)の
/// ままになっている既存ユーザーのlocalStorageを、現在の正式ドメイン
/// `easy-web.tokyo`へ一度だけ補正する(2026-07-16、easy-web.tokyoへ
/// 一本化。easy-web.tokyoはopen-easy-webを表示しない方針になったため)。
fn migrate_stale_self_seed(profiles: &mut Vec<SiteProfile>) -> bool {
    let mut changed = false;
    for p in profiles.iter_mut() {
        let is_stale = p.id == "seed-self"
            && ((p.host == "localhost" && p.port == 8080) || p.host == "easy-web.tokyo");
        if is_stale {
            p.protocol = "https".to_string();
            p.host = "easy-web.tokyo".to_string();
            p.port = 443;
            changed = true;
        }
    }
    changed
}

pub fn load_profiles() -> Vec<SiteProfile> {
    if let Some(storage) = local_storage() {
        if let Ok(Some(raw)) = storage.get_item(STORAGE_KEY) {
            if let Ok(mut profiles) = serde_json::from_str::<Vec<SiteProfile>>(&raw) {
                if !profiles.is_empty() {
                    if migrate_stale_self_seed(&mut profiles) {
                        save_profiles(&profiles);
                    }
                    return profiles;
                }
            }
        }
    }
    let seeded = default_profiles();
    save_profiles(&seeded);
    seeded
}

pub fn save_profiles(profiles: &[SiteProfile]) {
    if let Some(storage) = local_storage() {
        if let Ok(raw) = serde_json::to_string(profiles) {
            let _ = storage.set_item(STORAGE_KEY, &raw);
        }
    }
}

pub fn active_profile_id() -> Option<String> {
    local_storage().and_then(|s| s.get_item(ACTIVE_KEY).ok().flatten())
}

pub fn set_active_profile_id(id: &str) {
    if let Some(storage) = local_storage() {
        let _ = storage.set_item(ACTIVE_KEY, id);
    }
}

pub fn active_profile_name() -> String {
    let profiles = load_profiles();
    let active_id = active_profile_id();
    active_id
        .as_deref()
        .and_then(|id| profiles.iter().find(|p| p.id == id))
        .or_else(|| profiles.first())
        .map(|p| {
            let name = if p.name.trim().is_empty() { "アプリ未登録" } else { p.name.trim() };
            format!("{name} ( {} )", p.host)
        })
        .unwrap_or_else(|| "(未設定)".to_string())
}

fn new_id() -> String {
    format!("site-{}", js_sys::Date::now() as u64)
}

/// サイト管理画面の一覧+フォームを再描画する。
pub fn render_site_manager() {
    let profiles = load_profiles();
    let active_id = active_profile_id().unwrap_or_default();

    let mut list_html = String::new();
    if profiles.is_empty() {
        list_html.push_str("<p class=\"muted\">登録済みサイトはありません。下のフォームから追加してください。</p>");
    }
    for p in &profiles {
        let is_active = p.id == active_id
            || (active_id.is_empty() && profiles.first().map(|f| f.id.clone()) == Some(p.id.clone()));
        // アプリ名 ( ドメイン名 ) の形式で表示する(例: "open-easy-web ( easy-web.tokyo )")。
        // 名前が未入力の場合は「アプリ未登録」と表示する。
        let display_name = if p.name.trim().is_empty() { "アプリ未登録" } else { p.name.trim() };
        list_html.push_str(&format!(
            r#"<div class="site-card{active_class}">
  <div class="site-card-main">
    <div class="site-card-title">{display_name} ( {host} ) {badge}</div>
    <div class="site-card-meta muted">{purpose_label} ・ {url}</div>
    <div class="site-card-stack muted">スタック: {stack} ・ エンジン: {engine} ・ アプリサーバー: {app_server}</div>
  </div>
  <div class="site-card-actions">
    <button class="site-select" data-id="{id}">選択(切替)</button>
    <button class="site-test" data-id="{id}">接続テスト</button>
    <button class="site-edit" data-id="{id}">編集</button>
    <button class="site-delete" data-id="{id}">削除</button>
    {register_button}
    <span class="test-result muted" id="test-result-{id}"></span>
    <span class="register-result muted" id="register-result-{id}"></span>
  </div>
</div>"#,
            active_class = if is_active { " active" } else { "" },
            display_name = esc(display_name),
            host = esc(&p.host),
            badge = if is_active { "<span class=\"badge\">選択中</span>" } else { "" },
            purpose_label = if p.purpose == "self" { "このサイト" } else { "他のサイト" },
            register_button = if p.app_server != "none" && !p.shared_appserver_endpoint.trim().is_empty() {
                format!(r#"<button class="site-register-appserver" data-id="{}">🔗 共有バックエンドへ登録</button>"#, esc(&p.id))
            } else {
                String::new()
            },
            url = esc(&p.url()),
            stack = esc(&p.backend_stack),
            engine = esc(&p.webserver_engine),
            app_server = esc(&p.app_server),
            id = esc(&p.id),
        ));
    }

    if let Some(el) = try_by_id("site-list") {
        el.set_inner_html(&list_html);
    }
    wire_site_list_buttons();
}

fn wire_site_list_buttons() {
    use wasm_bindgen::prelude::*;
    use web_sys::{Event, HtmlButtonElement};

    let doc = document();

    type Handler = fn(String);
    let wiring: [(&str, Handler); 5] = [
        ("site-select", on_select_site),
        ("site-test", on_test_site),
        ("site-edit", on_edit_site),
        ("site-delete", on_delete_site),
        ("site-register-appserver", on_register_appserver),
    ];
    for (class, handler) in wiring {
        if let Ok(nodes) = doc.query_selector_all(&format!(".{class}")) {
            for i in 0..nodes.length() {
                if let Some(node) = nodes.get(i) {
                    if let Ok(btn) = node.dyn_into::<HtmlButtonElement>() {
                        let id = btn.get_attribute("data-id").unwrap_or_default();
                        let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                            handler(id.clone());
                        });
                        btn.set_onclick(Some(closure.as_ref().unchecked_ref()));
                        closure.forget();
                    }
                }
            }
        }
    }
}

fn on_select_site(id: String) {
    set_active_profile_id(&id);
    render_site_manager();
    crate::dom::set_status(&format!(
        "「{}」を選択しました。",
        active_profile_name()
    ));
    sync_active_site_label();
}

/// カードの「接続テスト」ボタン。選択中のサイトを変えずに疎通確認だけ行う。
/// GraphQL等の特定プロトコルには依存せず、単純なHTTP到達性のみ確認する。
fn on_test_site(id: String) {
    let profiles = load_profiles();
    let Some(profile) = profiles.iter().find(|p| p.id == id).cloned() else {
        return;
    };
    let result_id = format!("test-result-{}", profile.id);
    if let Some(el) = try_by_id(&result_id) {
        el.set_text_content(Some("確認中…"));
    }
    wasm_bindgen_futures::spawn_local(async move {
        let message = match check_reachable(&profile.url()).await {
            Ok(()) => "✅ 到達可能".to_string(),
            Err(e) => format!("❌ 到達不可: {e}"),
        };
        if let Some(el) = try_by_id(&result_id) {
            el.set_text_content(Some(&message));
        }
    });
}

/// `url` へ軽量なHTTPリクエストを送り、到達可能かどうかだけを確認する。
/// 任意の外部サイト(CORSヘッダを持たない可能性が高い)が対象のため、
/// `no-cors` モードで送信し、レスポンス内容は読まずネットワークエラーの
/// 有無のみで判定する。
async fn check_reachable(url: &str) -> Result<(), String> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::NoCors);

    let request =
        Request::new_with_str_and_init(url, &opts).map_err(|e| format!("request build failed: {e:?}"))?;

    JsFuture::from(crate::dom::window().fetch_with_request(&request))
        .await
        .map(|_| ())
        .map_err(|e| format!("fetch failed: {e:?}"))
}

/// カードの「🔗 共有バックエンドへ登録」ボタン(2026-07-16、「分身の術」
/// 構想の仕上げ)。この`open-easy-web-server`自身の
/// `POST /api/sites/:name/register-appserver`を呼び、既に稼働中の
/// 共有`open-web-server`/`poem-cosmo-tauri`インスタンスへこのサイトの
/// ドメイン(`p.host`)を動的登録する——新しいバックエンドプロセスを
/// 個別インストールする必要は無い。
fn on_register_appserver(id: String) {
    let profiles = load_profiles();
    let Some(profile) = profiles.iter().find(|p| p.id == id).cloned() else {
        return;
    };
    let result_id = format!("register-result-{}", profile.id);

    let Some(kind) = appserver_kind_for(&profile.app_server) else {
        if let Some(el) = try_by_id(&result_id) {
            el.set_text_content(Some("❌ アプリケーションサーバーが未選択です。"));
        }
        return;
    };
    if kind == "open_runo" && profile.shared_appserver_db_uri.trim().is_empty() {
        if let Some(el) = try_by_id(&result_id) {
            el.set_text_content(Some("❌ open-web-server向け登録にはdb_uriが必須です。"));
        }
        return;
    }

    if let Some(el) = try_by_id(&result_id) {
        el.set_text_content(Some("登録中…"));
    }

    wasm_bindgen_futures::spawn_local(async move {
        let message = match register_appserver_request(&profile, kind).await {
            Ok(()) => "✅ 共有バックエンドへ登録しました。".to_string(),
            Err(e) => format!("❌ 登録失敗: {e}"),
        };
        if let Some(el) = try_by_id(&result_id) {
            el.set_text_content(Some(&message));
        }
    });
}

/// `SiteProfile.app_server`("open-runo"/"poem-cosmo-tauri"/"aruaru-llm")を、
/// サーバー側`appserver_registration::AppServerKind`のJSON表現
/// ("open_runo"/"poem_cosmo_tauri"/"aruaru_llm")へ変換する。
/// "none"はどれでもない。
fn appserver_kind_for(app_server: &str) -> Option<&'static str> {
    match app_server {
        "open-runo" => Some("open_runo"),
        "poem-cosmo-tauri" => Some("poem_cosmo_tauri"),
        "aruaru-llm" => Some("aruaru_llm"),
        _ => None,
    }
}

/// `POST /api/sites/{host}/register-appserver`を実際に叩く。相対パスで
/// 送るため、このWASMバンドル自身を配信している`open-easy-web-server`
/// (同一オリジン)が対象になる——共有バックエンドへは
/// `open-easy-web-server`側がサーバー間で中継する
/// (`appserver_registration`モジュール参照)。
async fn register_appserver_request(profile: &SiteProfile, kind: &str) -> Result<(), String> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Headers, Request, RequestInit, RequestMode};

    let body = serde_json::json!({
        "shared_endpoint": profile.shared_appserver_endpoint,
        "kind": kind,
        "backend_addr": profile.app_server_upstream,
        "admin_key": if profile.shared_appserver_admin_key.trim().is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(profile.shared_appserver_admin_key.clone())
        },
        "db_uri": if profile.shared_appserver_db_uri.trim().is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::Value::String(profile.shared_appserver_db_uri.clone())
        },
    });
    let body_str = serde_json::to_string(&body).map_err(|e| format!("failed to build request body: {e}"))?;

    let headers = Headers::new().map_err(|e| format!("headers init failed: {e:?}"))?;
    headers.set("Content-Type", "application/json").map_err(|e| format!("header set failed: {e:?}"))?;
    if !profile.shared_appserver_session_token.trim().is_empty() {
        headers
            .set("Authorization", &format!("Bearer {}", profile.shared_appserver_session_token))
            .map_err(|e| format!("header set failed: {e:?}"))?;
    }

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::SameOrigin);
    opts.set_headers(&headers);
    opts.set_body(&JsValue::from_str(&body_str));

    let url = format!("/api/sites/{}/register-appserver", js_sys::encode_uri_component(&profile.host));
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("request build failed: {e:?}"))?;

    let resp_value = JsFuture::from(crate::dom::window().fetch_with_request(&request))
        .await
        .map_err(|e| format!("fetch failed: {e:?}"))?;
    let resp: web_sys::Response = resp_value.dyn_into().map_err(|_| "unexpected fetch response type".to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(format!("server responded with status {}", resp.status()))
    }
}

fn on_delete_site(id: String) {
    let profiles = load_profiles();
    let Some(target) = profiles.iter().find(|p| p.id == id).cloned() else { return };
    let name = if target.name.trim().is_empty() { "このサイト".to_string() } else { target.name.clone() };
    let confirmed = crate::dom::window()
        .confirm_with_message(&format!("「{name}」を削除します。よろしいですか?"))
        .unwrap_or(true);
    if !confirmed {
        return;
    }

    let mut profiles = profiles;
    profiles.retain(|p| p.id != id);
    save_profiles(&profiles);
    if active_profile_id().as_deref() == Some(id.as_str()) {
        if let Some(first) = profiles.first() {
            set_active_profile_id(&first.id);
        }
    }
    render_site_manager();
    sync_active_site_label();

    // "他のサイト"(実際のドメイン/サブドメイン)の場合は、ローカルの
    // ブックマーク削除に加えて、バックエンドのドメイン登録(nginx vhost)も
    // 実際に取り消す。アップロード済みファイル・証明書は保持される
    // (`vhost::remove`の設計どおり)。
    if target.purpose == "other" {
        let host = target.host.clone();
        crate::dom::set_status(&format!("ドメイン登録を削除中… / Removing domain registration ({host})…"));
        wasm_bindgen_futures::spawn_local(async move {
            match crate::api_upload::delete_domain(&host).await {
                Ok(_) => crate::dom::set_status(&format!("ドメイン登録を削除しました({host})。ローカルの一覧からも削除済みです。")),
                Err(e) => crate::dom::set_status(&format!(
                    "⚠️ ローカルの一覧からは削除しましたが、サーバー側のドメイン登録削除に失敗しました({host}): {e}"
                )),
            }
        });
    }
}

fn on_edit_site(id: String) {
    let profiles = load_profiles();
    if let Some(p) = profiles.iter().find(|p| p.id == id) {
        fill_form(p);
        if let Some(el) = try_by_id("site-form-id") {
            el.set_attribute("value", &p.id).ok();
        }
    }
}

fn fill_form(p: &SiteProfile) {
    let set_val = |id: &str, v: &str| {
        if let Some(el) = try_by_id(id) {
            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                input.set_value(v);
            }
        }
    };
    set_val("site-name", &p.name);
    set_val("site-host", &p.host);
    set_val("site-port", &p.port.to_string());
    set_val("site-path", &p.path);
    set_val("site-stack", &p.backend_stack);

    if let Some(el) = try_by_id("site-purpose") {
        if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
            select.set_value(&p.purpose);
        }
    }
    if let Some(el) = try_by_id("site-protocol") {
        if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
            select.set_value(&p.protocol);
        }
    }
    if let Some(el) = try_by_id("site-engine") {
        if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
            select.set_value(&p.webserver_engine);
        }
    }
    if let Some(el) = try_by_id("site-app-server") {
        if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
            select.set_value(&p.app_server);
        }
    }
    set_val("site-app-server-upstream", &p.app_server_upstream);
    set_val("site-shared-endpoint", &p.shared_appserver_endpoint);
    set_val("site-shared-admin-key", &p.shared_appserver_admin_key);
    set_val("site-shared-db-uri", &p.shared_appserver_db_uri);
    set_val("site-shared-session-token", &p.shared_appserver_session_token);
}

pub fn clear_form() {
    if let Some(el) = try_by_id("site-form-id") {
        el.set_attribute("value", "").ok();
    }
    let set_val = |id: &str, v: &str| {
        if let Some(el) = try_by_id(id) {
            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                input.set_value(v);
            }
        }
    };
    set_val("site-name", "");
    set_val("site-host", "");
    set_val("site-port", "443");
    set_val("site-path", "/");
    set_val("site-stack", "");
    if let Some(el) = try_by_id("site-engine") {
        if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
            select.set_value("nginx");
        }
    }
    if let Some(el) = try_by_id("site-app-server") {
        if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
            select.set_value("none");
        }
    }
    set_val("site-app-server-upstream", "");
    set_val("site-shared-endpoint", "");
    set_val("site-shared-admin-key", "");
    set_val("site-shared-db-uri", "");
    set_val("site-shared-session-token", "");
}

/// 「保存」ボタン押下時のハンドラ。新規追加/既存編集の両方を扱う。
pub fn on_save_site() {
    let get_val = |id: &str| -> String {
        by_id(id)
            .dyn_into::<HtmlInputElement>()
            .map(|i| i.value())
            .unwrap_or_default()
    };
    let get_select = |id: &str| -> String {
        by_id(id)
            .dyn_into::<HtmlSelectElement>()
            .map(|s| s.value())
            .unwrap_or_default()
    };

    let existing_id = get_val("site-form-id");
    let name = get_val("site-name");
    let host = get_val("site-host");
    let path_raw = get_val("site-path");
    let port_raw = get_val("site-port");
    let stack = get_val("site-stack");
    let purpose = get_select("site-purpose");
    let protocol = get_select("site-protocol");
    let engine = get_select("site-engine");
    let app_server = get_select("site-app-server");
    let app_server_upstream = get_val("site-app-server-upstream");
    let shared_appserver_endpoint = get_val("site-shared-endpoint");
    let shared_appserver_admin_key = get_val("site-shared-admin-key");
    let shared_appserver_db_uri = get_val("site-shared-db-uri");
    let shared_appserver_session_token = get_val("site-shared-session-token");

    if name.trim().is_empty() || host.trim().is_empty() {
        crate::dom::set_status("サイト名と接続先ホスト(IP/ドメイン)は必須です。");
        return;
    }
    let port: u16 = match port_raw.trim().parse::<u32>() {
        Ok(p) if (1..=65535).contains(&p) => p as u16,
        _ => {
            crate::dom::set_status(&format!(
                "ポート番号が不正です(1〜65535の数値を入力してください): \"{port_raw}\""
            ));
            return;
        }
    };
    let path = if path_raw.trim().is_empty() {
        "/".to_string()
    } else if path_raw.starts_with('/') {
        path_raw
    } else {
        format!("/{path_raw}")
    };

    let host_for_backend = host.clone();
    let purpose_for_backend = purpose.clone();

    let mut profiles = load_profiles();
    if !existing_id.is_empty() {
        if let Some(p) = profiles.iter_mut().find(|p| p.id == existing_id) {
            p.name = name;
            p.purpose = purpose;
            p.protocol = protocol;
            p.host = host;
            p.port = port;
            p.path = path;
            p.backend_stack = stack;
            p.webserver_engine = engine;
            p.app_server = app_server;
            p.app_server_upstream = app_server_upstream;
            p.shared_appserver_endpoint = shared_appserver_endpoint;
            p.shared_appserver_admin_key = shared_appserver_admin_key;
            p.shared_appserver_db_uri = shared_appserver_db_uri;
            p.shared_appserver_session_token = shared_appserver_session_token;
        }
    } else {
        let new_profile = SiteProfile {
            id: new_id(),
            name,
            purpose,
            protocol,
            host,
            port,
            path,
            backend_stack: stack,
            webserver_engine: engine,
            app_server,
            app_server_upstream,
            shared_appserver_endpoint,
            shared_appserver_admin_key,
            shared_appserver_db_uri,
            shared_appserver_session_token,
        };
        if active_profile_id().is_none() {
            set_active_profile_id(&new_profile.id);
        }
        profiles.push(new_profile);
    }
    save_profiles(&profiles);
    clear_form();
    render_site_manager();
    sync_active_site_label();

    // "他のサイト"(実際のドメイン/サブドメイン)の場合は、ローカルの
    // ブックマーク保存に加えて、バックエンドにドメインを実際に登録する
    // (webroot作成 + 既にファイルがあればPHP自動判定・nginx+HTTPS自動構成)。
    // "このサイト(open-easy-web自身)"は登録対象ではないため対象外。
    if purpose_for_backend == "other" {
        crate::dom::set_status(&format!(
            "サイト情報を保存しました。ドメイン登録中… / Saved. Registering domain ({host_for_backend})…"
        ));
        wasm_bindgen_futures::spawn_local(async move {
            match crate::api_upload::create_folder(&host_for_backend).await {
                Ok(_) => match crate::api_upload::detect_and_configure(&host_for_backend).await {
                    Ok(value) => {
                        let msg = value.get("message_ja").and_then(|v| v.as_str()).unwrap_or("");
                        crate::dom::set_status(&format!(
                            "✅ ドメインを登録しました({host_for_backend})。{msg}"
                        ));
                    }
                    Err(e) => crate::dom::set_status(&format!(
                        "サイト情報は保存されましたが、自動構成に失敗しました({host_for_backend}): {e}"
                    )),
                },
                Err(e) => crate::dom::set_status(&format!(
                    "サイト情報は保存されましたが、サーバー側のドメイン登録に失敗しました({host_for_backend}): {e}"
                )),
            }
        });
    } else {
        crate::dom::set_status("サイト情報を保存しました。");
    }
}

/// ヘッダーの「選択中のサイト」表示を更新する。
pub fn sync_active_site_label() {
    if let Some(el) = try_by_id("active-site-name") {
        el.set_text_content(Some(&active_profile_name()));
    }
}

/// 登録済みサイト一覧をJSONファイルとしてダウンロードする(バックアップ・
/// 他ブラウザ/他マシンへの持ち出し用)。
pub fn export_profiles_json() {
    let profiles = load_profiles();
    let Ok(json) = serde_json::to_string_pretty(&profiles) else {
        crate::dom::set_status("サイト一覧のエクスポートに失敗しました。");
        return;
    };
    if crate::dom::trigger_download(
        "open-easy-web-sites.json",
        &json,
        "application/json;charset=utf-8;",
    )
    .is_none()
    {
        crate::dom::set_status("サイト一覧のエクスポートに失敗しました。");
    }
}

/// `<input type="file">` で選択されたJSONファイルを読み込み、
/// 確認の上で登録済みサイト一覧を置き換える。
pub fn import_profiles_from_file(file: web_sys::File) {
    use web_sys::FileReader;

    let Ok(reader) = FileReader::new() else {
        crate::dom::set_status("ファイル読み込みの初期化に失敗しました。");
        return;
    };
    let reader_for_closure = reader.clone();
    let onload = Closure::<dyn FnMut()>::new(move || {
        let text = reader_for_closure.result().ok().and_then(|v| v.as_string());
        if let Some(text) = text {
            apply_imported_json(&text);
        } else {
            crate::dom::set_status("ファイルの内容を読み取れませんでした。");
        }
    });
    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();
    if reader.read_as_text(&file).is_err() {
        crate::dom::set_status("ファイルの読み込みに失敗しました。");
    }
}

fn apply_imported_json(text: &str) {
    match serde_json::from_str::<Vec<SiteProfile>>(text) {
        Ok(profiles) if !profiles.is_empty() => {
            let confirmed = crate::dom::window()
                .confirm_with_message(&format!(
                    "{}件のサイトをインポートします。既存の登録済みサイトは置き換えられます。よろしいですか?",
                    profiles.len()
                ))
                .unwrap_or(false);
            if !confirmed {
                crate::dom::set_status("インポートをキャンセルしました。");
                return;
            }
            let count = profiles.len();
            save_profiles(&profiles);
            if let Some(first) = profiles.first() {
                set_active_profile_id(&first.id);
            }
            render_site_manager();
            sync_active_site_label();
            crate::dom::set_status(&format!("{count}件のサイトをインポートしました。"));
        }
        Ok(_) => crate::dom::set_status("インポートするサイトがありません(空のファイルです)。"),
        Err(e) => crate::dom::set_status(&format!("JSONの読み込みに失敗しました: {e}")),
    }
}
