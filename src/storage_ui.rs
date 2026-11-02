//! 「ストレージ(HDD/SSD)のメンテナンス」画面のDOM配線(`server/src/storage_maintenance.rs`の管理APIを呼ぶ)。
//!
//! 安全のため、「実際に消す/実行する」ボタンは、試し実行・計画のボタンと別にして、確認ダイアログを出す。

use crate::dom::{by_id, esc, try_by_id};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlButtonElement, HtmlInputElement};

fn base_url() -> String {
    crate::dom::window().location().origin().unwrap_or_default()
}

fn token() -> String {
    try_by_id("storage-admin-token").and_then(|el| el.dyn_into::<HtmlInputElement>().ok()).map(|el| el.value()).unwrap_or_default()
}

fn checked(id: &str) -> bool {
    try_by_id(id).and_then(|el| el.dyn_into::<HtmlInputElement>().ok()).map(|el| el.checked()).unwrap_or(false)
}

fn set_checked(id: &str, v: bool) {
    if let Some(el) = try_by_id(id).and_then(|el| el.dyn_into::<HtmlInputElement>().ok()) {
        el.set_checked(v);
    }
}

fn set_text(id: &str, text: &str) {
    if let Some(el) = try_by_id(id) {
        el.set_text_content(Some(text));
    }
}

fn set_html(id: &str, html: &str) {
    if let Some(el) = try_by_id(id) {
        el.set_inner_html(html);
    }
}

/// サーバーの状態(`ok`/`warning`/`critical`/`unknown`/`error`/`dry-run`/`applied`)を、画面の言葉と色にする。
fn state_view(state: &str) -> (&'static str, &'static str) {
    match state {
        "ok" => ("正常", "#1a7f37"),
        "warning" => ("注意", "#b7791f"),
        "critical" => ("危険", "#c53030"),
        "unknown" => ("不明", "#6b7280"),
        "error" => ("エラー", "#c53030"),
        "dry-run" => ("試し実行(消していません)", "#2f6fed"),
        "applied" => ("実行しました", "#1a7f37"),
        _ => ("-", "#6b7280"),
    }
}

fn when(at: i64) -> String {
    if at <= 0 {
        return "未実行".to_string();
    }
    let d = js_sys::Date::new(&JsValue::from_f64(at as f64 * 1000.0));
    d.to_locale_string("ja-JP", &JsValue::UNDEFINED).as_string().unwrap_or_default()
}

fn render_last(box_id: &str, last: Option<&serde_json::Value>) {
    let Some(l) = last.filter(|l| !l.is_null()) else {
        set_html(box_id, "<p class=\"muted\">まだ実行していません。</p>");
        return;
    };
    let state = l.get("state").and_then(|v| v.as_str()).unwrap_or("");
    let (label, color) = state_view(state);
    let at = l.get("at").and_then(|v| v.as_i64()).unwrap_or(0);
    let text = l.get("text").and_then(|v| v.as_str()).unwrap_or("");
    set_html(
        box_id,
        &format!("<p><strong style=\"color:{color}\">{label}</strong> <span class=\"muted\">{}</span></p><pre style=\"white-space:pre-wrap;max-height:16em;overflow:auto\">{}</pre>", esc(&when(at)), esc(text)),
    );
}

fn render_overview(v: &serde_json::Value) {
    let s = v.get("settings");
    let flag = |k: &str| s.and_then(|s| s.get(k)).and_then(|x| x.as_bool()).unwrap_or(false);
    set_checked("storage-health-enabled", flag("health_enabled"));
    set_checked("storage-tidy-enabled", flag("tidy_enabled"));
    set_checked("storage-tidy-apply", flag("tidy_apply"));
    set_checked("storage-optimize-enabled", flag("optimize_enabled"));
    set_checked("storage-optimize-apply", flag("optimize_apply"));
    let hour = s.and_then(|s| s.get("hour")).and_then(|x| x.as_u64()).unwrap_or(4);
    if let Some(el) = try_by_id("storage-hour").and_then(|el| el.dyn_into::<HtmlInputElement>().ok()) {
        el.set_value(&hour.to_string());
    }
    render_last("storage-health-result", v.get("last_health"));
    render_last("storage-tidy-result", v.get("last_tidy"));
    render_last("storage-optimize-result", v.get("last_optimize"));
    let cfg_exists = v.get("tidy_config_exists").and_then(|x| x.as_bool()).unwrap_or(false);
    let cfg_path = v.get("tidy_config_path").and_then(|x| x.as_str()).unwrap_or("");
    let platform = v.get("platform").and_then(|x| x.as_str()).unwrap_or("");
    set_text(
        "storage-info",
        &format!("OS: {platform} / 整理の規則ファイル: {cfg_path} {}", if cfg_exists { "(あります)" } else { "(まだありません。整理は、規則ファイルを作ってから)" }),
    );
}

async fn refresh() {
    match crate::api_storage::overview(&base_url(), &token()).await {
        Ok(v) => {
            render_overview(&v);
            set_text("storage-status", "");
        }
        Err(e) => set_text("storage-status", &format!("❌ {e}")),
    }
}

fn confirm(msg: &str) -> bool {
    crate::dom::window().confirm_with_message(msg).unwrap_or(false)
}

fn on_health() {
    spawn_local(async {
        set_text("storage-status", "健康チェックを実行中…");
        match crate::api_storage::run_health(&base_url(), &token()).await {
            Ok(_) => {
                set_text("storage-status", "✅ 健康チェックが終わりました。");
                refresh().await;
            }
            Err(e) => set_text("storage-status", &format!("❌ {e}")),
        }
    });
}

fn run_tidy(apply: bool) {
    if apply && !confirm("無駄なデータを、実際に削除します(規則と、保護パスに従います)。実行しますか?\nReally delete files now?") {
        return;
    }
    spawn_local(async move {
        set_text("storage-status", if apply { "整理を実行中…" } else { "整理の試し実行中(何も消しません)…" });
        match crate::api_storage::run_tidy(&base_url(), &token(), apply).await {
            Ok(v) => {
                let msg = if let Some(e) = v.get("error").and_then(|e| e.as_str()) { format!("❌ {e}") } else { "✅ 終わりました。".to_string() };
                set_text("storage-status", &msg);
                refresh().await;
            }
            Err(e) => set_text("storage-status", &format!("❌ {e}")),
        }
    });
}

fn run_optimize(apply: bool) {
    if apply && !confirm("SSD の TRIM / HDD の最適化を、実際に実行します(Windows では管理者権限が要ります)。実行しますか?\nReally run optimization now?") {
        return;
    }
    spawn_local(async move {
        set_text("storage-status", if apply { "最適化を実行中…" } else { "最適化の計画を作成中(何もしません)…" });
        match crate::api_storage::run_optimize(&base_url(), &token(), apply).await {
            Ok(_) => {
                set_text("storage-status", "✅ 終わりました。");
                refresh().await;
            }
            Err(e) => set_text("storage-status", &format!("❌ {e}")),
        }
    });
}

fn on_save_settings() {
    let hour: u8 = try_by_id("storage-hour").and_then(|el| el.dyn_into::<HtmlInputElement>().ok()).and_then(|el| el.value().trim().parse().ok()).unwrap_or(4);
    let s = crate::api_storage::Settings {
        health_enabled: checked("storage-health-enabled"),
        tidy_enabled: checked("storage-tidy-enabled"),
        tidy_apply: checked("storage-tidy-apply"),
        optimize_enabled: checked("storage-optimize-enabled"),
        optimize_apply: checked("storage-optimize-apply"),
        hour,
    };
    // 「自動で実際に消す/実行する」を有効にするときは、必ず確認する。
    if (s.tidy_apply && s.tidy_enabled || s.optimize_apply && s.optimize_enabled)
        && !confirm("自動実行で、実際に削除/最適化を行う設定にします。よろしいですか?\nEnable automatic real deletion/optimization?")
    {
        return;
    }
    spawn_local(async move {
        match crate::api_storage::save_settings(&base_url(), &token(), &s).await {
            Ok(_) => {
                set_text("storage-status", "✅ 設定を保存しました。");
                refresh().await;
            }
            Err(e) => set_text("storage-status", &format!("❌ {e}")),
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
    wire_click("storage-refresh-btn", || spawn_local(refresh()))?;
    wire_click("storage-health-btn", on_health)?;
    wire_click("storage-tidy-dry-btn", || run_tidy(false))?;
    wire_click("storage-tidy-apply-btn", || run_tidy(true))?;
    wire_click("storage-optimize-dry-btn", || run_optimize(false))?;
    wire_click("storage-optimize-apply-btn", || run_optimize(true))?;
    wire_click("storage-save-btn", on_save_settings)?;
    Ok(())
}
