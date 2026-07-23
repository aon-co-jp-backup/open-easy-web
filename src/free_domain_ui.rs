//! 「簡単ドメイン設定」ウィザードのDOM配線(無料DDNS、DuckDNS)。
//!
//! 4ステップ: (a) DuckDNSアカウント作成への外部リンク案内、(b) サブ
//! ドメイン名+トークン入力、(c) `open-web-server`側の
//! `POST /admin/ddns/setup-free-domain`を呼んで即時疎通確認(1インスタンス
//! につき最大20ドメインまで登録可能)、(d) 成功したらSFTP接続コマンド例も
//! 一緒に表示(`GET /admin/sftp/connection-info`、複数ドメイン登録時は
//! どれを使うか選択できる)。加えて、登録済みドメイン一覧+残り枠の表示・
//! 個別削除にも対応する(`GET`/`DELETE /admin/ddns/domains`)。
//! 過剰実装を避け、1画面で完結するシンプルな構成として実装する。

use crate::api_free_domain;
use crate::dom::{by_id, esc, try_by_id};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlButtonElement, HtmlInputElement, HtmlSelectElement};

fn input_value(id: &str) -> String {
    try_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value())
        .unwrap_or_default()
}

fn set_text(id: &str, text: &str) {
    if let Some(el) = try_by_id(id) {
        el.set_text_content(Some(text));
    }
}

fn server_and_token() -> (String, String) {
    (input_value("freedomain-server-url"), input_value("freedomain-admin-token"))
}

/// 登録済みドメイン一覧を取得し、一覧カード+SFTPドメイン選択`<select>`の
/// 両方を再描画する。
fn on_refresh_domain_list() {
    let (base_url, admin_token) = server_and_token();
    if base_url.trim().is_empty() || admin_token.trim().is_empty() {
        set_text("freedomain-list-result", "❌ open-web-serverのURL・管理トークンを入力してください。");
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        set_text("freedomain-list-result", "取得中… / Fetching…");
        match api_free_domain::list_domains(&base_url, &admin_token).await {
            Ok(value) => {
                render_domain_list(&value);
                set_text("freedomain-list-result", "");
            }
            Err(e) => set_text("freedomain-list-result", &format!("❌ {e}")),
        }
    });
}

fn render_domain_list(value: &serde_json::Value) {
    let domains = value.get("domains").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let count = value.get("count").and_then(|v| v.as_u64()).unwrap_or(domains.len() as u64);
    let capacity = value.get("capacity").and_then(|v| v.as_u64()).unwrap_or(20);
    let remaining = value.get("remaining_capacity").and_then(|v| v.as_u64()).unwrap_or(capacity.saturating_sub(count));

    if let Some(container) = try_by_id("freedomain-domain-list") {
        if domains.is_empty() {
            container.set_inner_html(
                "<p class=\"muted\">登録済みドメインはありません。 / No domains registered yet.</p>",
            );
        } else {
            let rows: String = domains
                .iter()
                .filter_map(|d| d.get("full_hostname").and_then(|v| v.as_str()).zip(d.get("domain").and_then(|v| v.as_str())))
                .map(|(full_hostname, domain)| {
                    format!(
                        "<div class=\"site-card\"><div><div class=\"site-card-title\">{}</div></div>\
                         <div class=\"site-card-actions\"><button class=\"secondary freedomain-remove-btn\" data-domain=\"{}\">削除 / Remove</button></div></div>",
                        esc(full_hostname),
                        esc(domain),
                    )
                })
                .collect();
            container.set_inner_html(&rows);
        }
    }
    set_text(
        "freedomain-list-result",
        &format!("登録数: {count} / {capacity}(残り{remaining}件登録可能) / Registered: {count} of {capacity} (capacity remaining: {remaining})"),
    );

    // SFTP接続に使うドメインの選択肢も同期する。
    if let Some(select_el) = try_by_id("freedomain-sftp-host-select") {
        if let Ok(select) = select_el.dyn_into::<HtmlSelectElement>() {
            select.set_inner_html("<option value=\"\">(自動選択 / auto-select)</option>");
            for d in &domains {
                if let (Some(full_hostname), Some(domain)) =
                    (d.get("full_hostname").and_then(|v| v.as_str()), d.get("domain").and_then(|v| v.as_str()))
                {
                    let option = crate::dom::document().create_element("option").ok();
                    if let Some(option) = option.and_then(|o| o.dyn_into::<web_sys::HtmlOptionElement>().ok()) {
                        option.set_value(domain);
                        option.set_text(full_hostname);
                        let _ = select.add_with_html_option_element(&option);
                    }
                }
            }
        }
    }
}

fn on_remove_domain(domain: String) {
    let (base_url, admin_token) = server_and_token();
    if base_url.trim().is_empty() || admin_token.trim().is_empty() {
        set_text("freedomain-list-result", "❌ open-web-serverのURL・管理トークンを入力してください。");
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        set_text("freedomain-list-result", "削除中… / Removing…");
        match api_free_domain::remove_domain(&base_url, &admin_token, &domain).await {
            Ok(_) => {
                on_refresh_domain_list();
            }
            Err(e) => set_text("freedomain-list-result", &format!("❌ {e}")),
        }
    });
}

fn on_setup_free_domain() {
    let (base_url, admin_token) = server_and_token();
    let duckdns_domain = input_value("freedomain-duckdns-domain");
    let duckdns_token = input_value("freedomain-duckdns-token");

    if base_url.trim().is_empty() || admin_token.trim().is_empty() || duckdns_domain.trim().is_empty() || duckdns_token.trim().is_empty() {
        set_text(
            "freedomain-result",
            "❌ open-web-serverのURL・管理トークン・サブドメイン名・DuckDNSトークンを\
             すべて入力してください。 / Fill in the open-web-server URL, admin token, \
             subdomain name, and DuckDNS token.",
        );
        return;
    }

    wasm_bindgen_futures::spawn_local(async move {
        set_text("freedomain-result", "疎通確認中… / Verifying…");
        match api_free_domain::setup_free_domain(&base_url, &admin_token, &duckdns_domain, &duckdns_token).await {
            Ok(value) => {
                let full_hostname = value.get("full_hostname").and_then(|v| v.as_str()).unwrap_or("(不明)");
                let verified = value.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
                let message = value.get("message").and_then(|v| v.as_str()).unwrap_or("");
                if verified {
                    set_text(
                        "freedomain-result",
                        &format!("✅ '{full_hostname}' の疎通確認に成功しました。\n{message}"),
                    );
                    if let Some(el) = try_by_id("freedomain-sftp-step") {
                        el.set_class_name("");
                    }
                    on_refresh_domain_list();
                } else {
                    set_text("freedomain-result", &format!("⚠️ 疎通確認に失敗しました。 {message}"));
                }
            }
            Err(e) => set_text("freedomain-result", &format!("❌ {e}")),
        }
    });
}

fn on_fetch_sftp_info() {
    let (base_url, admin_token) = server_and_token();
    if base_url.trim().is_empty() || admin_token.trim().is_empty() {
        set_text("freedomain-sftp-result", "❌ open-web-serverのURL・管理トークンを入力してください。");
        return;
    }
    let selected_domain = try_by_id("freedomain-sftp-host-select")
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .filter(|v| !v.is_empty());

    wasm_bindgen_futures::spawn_local(async move {
        set_text("freedomain-sftp-result", "取得中… / Fetching…");
        let base_url = match &selected_domain {
            Some(domain) => format!("{}?host={}", base_url.trim_end_matches('/'), js_sys::encode_uri_component(domain)),
            None => base_url,
        };
        match api_free_domain::sftp_connection_info(&base_url, &admin_token).await {
            Ok(value) => {
                let example = value
                    .get("example_command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(未取得。open-web-server側でSFTP_BINDが設定されているか確認してください)");
                let sftp_enabled = value.get("sftp_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                if sftp_enabled {
                    set_text("freedomain-sftp-result", &format!("✅ SFTP接続コマンド例: {example}"));
                } else {
                    set_text(
                        "freedomain-sftp-result",
                        "⚠️ open-web-server側でSFTPサーバーが有効化されていません\
                         (OPEN_WEB_SERVER_SFTP_BINDが未設定)。",
                    );
                }
            }
            Err(e) => set_text("freedomain-sftp-result", &format!("❌ {e}")),
        }
    });
}

fn wire_click(id: &str, f: impl Fn() + 'static) -> Result<(), JsValue> {
    let btn: HtmlButtonElement = by_id(id).dyn_into()?;
    let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| f());
    btn.set_onclick(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
    Ok(())
}

/// 動的に生成される「削除」ボタン群を、コンテナへのイベント委譲で1つの
/// リスナーだけで処理する(ボタンごとにクロージャを`forget`し続けて
/// メモリを増やし続けないようにするため)。
fn wire_domain_list_delegation() -> Result<(), JsValue> {
    let container = by_id("freedomain-domain-list");
    let closure = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        if let Some(target) = evt.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
            if let Some(domain) = target.get_attribute("data-domain") {
                on_remove_domain(domain);
            }
        }
    });
    container.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

pub fn wire() -> Result<(), JsValue> {
    wire_click("freedomain-setup-btn", on_setup_free_domain)?;
    wire_click("freedomain-sftp-fetch-btn", on_fetch_sftp_info)?;
    wire_click("freedomain-list-fetch-btn", on_refresh_domain_list)?;
    wire_domain_list_delegation()?;
    Ok(())
}
