//! 「更新のお知らせメール」画面のDOM配線(`server/src/update_notify.rs`の管理APIを呼ぶ)。
//! 宛先は1行1件のテキストで編集する。

use crate::dom::{by_id, esc, try_by_id};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlButtonElement, HtmlInputElement, HtmlTextAreaElement};

fn base_url() -> String {
    crate::dom::window().location().origin().unwrap_or_default()
}

fn token() -> String {
    try_by_id("notify-admin-token").and_then(|el| el.dyn_into::<HtmlInputElement>().ok()).map(|el| el.value()).unwrap_or_default()
}

fn set_text(id: &str, text: &str) {
    if let Some(el) = try_by_id(id) {
        el.set_text_content(Some(text));
    }
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "update-needed" => "更新が必要",
        "needs-claude" => "Claudeへの依頼が必要",
        "heavy-test" => "重い試験(手元で)",
        _ => "お知らせ",
    }
}

fn when(at: i64) -> String {
    let d = js_sys::Date::new(&JsValue::from_f64(at as f64 * 1000.0));
    d.to_locale_string("ja-JP", &JsValue::UNDEFINED).as_string().unwrap_or_default()
}

fn render(v: &serde_json::Value) {
    let emails: Vec<String> = v.get("recipients").and_then(|r| r.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    if let Some(el) = try_by_id("notify-recipients").and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok()) {
        el.set_value(&emails.join("\n"));
    }
    let smtp = v.get("smtp_configured").and_then(|x| x.as_bool()).unwrap_or(false);
    set_text(
        "notify-info",
        if smtp { "メール送信(SMTP)は設定済みです。" } else { "⚠ メール送信(SMTP)が未設定です。サーバーの OPEN_EASYWEB_SMTP_* を設定してください。" },
    );
    let mut html = String::new();
    if let Some(h) = v.get("history").and_then(|h| h.as_array()) {
        for e in h.iter().rev().take(20) {
            let at = e.get("at").and_then(|x| x.as_i64()).unwrap_or(0);
            let kind = e.get("kind").and_then(|x| x.as_str()).unwrap_or("");
            let subject = e.get("subject").and_then(|x| x.as_str()).unwrap_or("");
            let sent = e.get("sent").and_then(|x| x.as_u64()).unwrap_or(0);
            let failed = e.get("failed").and_then(|x| x.as_u64()).unwrap_or(0);
            let sup = e.get("suppressed").and_then(|x| x.as_bool()).unwrap_or(false);
            let result = if sup { "同じ件名のため送信を見送り".to_string() } else { format!("送信 {sent} 件 / 失敗 {failed} 件") };
            html.push_str(&format!("<li><span class=\"muted\">{}</span> [{}] {} — {}</li>", esc(&when(at)), esc(kind_label(kind)), esc(subject), esc(&result)));
        }
    }
    if let Some(el) = try_by_id("notify-history") {
        el.set_inner_html(&if html.is_empty() { "<li class=\"muted\">まだお知らせを送っていません。</li>".to_string() } else { html });
    }
}

async fn refresh() {
    match crate::api_notify::overview(&base_url(), &token()).await {
        Ok(v) => {
            render(&v);
        }
        Err(e) => set_text("notify-status", &format!("❌ {e}")),
    }
}

fn on_save() {
    let text = try_by_id("notify-recipients").and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok()).map(|el| el.value()).unwrap_or_default();
    let emails: Vec<String> = text.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    spawn_local(async move {
        match crate::api_notify::save_recipients(&base_url(), &token(), &emails).await {
            Ok(_) => {
                set_text("notify-status", "✅ 宛先を保存しました。");
                refresh().await;
            }
            Err(e) => set_text("notify-status", &format!("❌ {e}")),
        }
    });
}

fn on_test() {
    spawn_local(async {
        set_text("notify-status", "テストのお知らせを送信中…");
        match crate::api_notify::send_test(&base_url(), &token()).await {
            Ok(v) => {
                let sent = v.get("sent").and_then(|x| x.as_u64()).unwrap_or(0);
                let failed = v.get("failed").and_then(|x| x.as_u64()).unwrap_or(0);
                set_text("notify-status", &format!("✅ 送信 {sent} 件 / 失敗 {failed} 件"));
                refresh().await;
            }
            Err(e) => set_text("notify-status", &format!("❌ {e}")),
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

pub fn wire() -> Result<(), JsValue> {
    wire_click("notify-refresh-btn", || spawn_local(refresh()))?;
    wire_click("notify-save-btn", on_save)?;
    wire_click("notify-test-btn", on_test)?;
    Ok(())
}
