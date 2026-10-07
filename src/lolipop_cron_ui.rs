//! 「ロリポップ!時間指定オートクロール」画面のDOM配線
//! (`server/src/lolipop_cron.rs`の管理APIを呼ぶ)。

use crate::dom::{by_id, esc, try_by_id};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlButtonElement, HtmlInputElement, HtmlSelectElement};

fn base_url() -> String {
    crate::dom::window().location().origin().unwrap_or_default()
}

fn input_value(id: &str) -> String {
    try_by_id(id).and_then(|el| el.dyn_into::<HtmlInputElement>().ok()).map(|el| el.value()).unwrap_or_default()
}

fn checked(id: &str) -> bool {
    try_by_id(id).and_then(|el| el.dyn_into::<HtmlInputElement>().ok()).map(|el| el.checked()).unwrap_or(false)
}

fn set_text(id: &str, text: &str) {
    if let Some(el) = try_by_id(id) {
        el.set_text_content(Some(text));
    }
}

const WEEKDAYS_JA: [&str; 7] = ["日", "月", "火", "水", "木", "金", "土"];

fn str_list(v: Option<&serde_json::Value>, sep: &str) -> String {
    v.and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>().join(sep))
        .unwrap_or_default()
}

fn render_jobs(value: &serde_json::Value) {
    let tz = value.get("tz_offset_hours").and_then(|v| v.as_i64()).unwrap_or(9);
    let jobs = value.get("jobs").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut html = format!("<p class=\"muted\">時刻基準: UTC{tz:+} / {} 件のジョブ</p>", jobs.len());
    for j in &jobs {
        let id = esc(j.get("id").and_then(|v| v.as_str()).unwrap_or(""));
        let name = j.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let url = j.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let method = j.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
        let enabled = j.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let catch_up = j.get("catch_up").and_then(|v| v.as_bool()).unwrap_or(false);
        let times = str_list(j.pointer("/schedule/times"), ", ");
        let days = j
            .pointer("/schedule/weekdays")
            .and_then(|v| v.as_array())
            .filter(|a| !a.is_empty())
            .map(|a| {
                a.iter()
                    .filter_map(|d| d.as_u64())
                    .map(|d| WEEKDAYS_JA.get(d as usize).copied().unwrap_or("?"))
                    .collect::<Vec<_>>()
                    .join("・")
            })
            .unwrap_or_else(|| "毎日".to_string());
        let last = j.get("last_result").and_then(|v| v.as_str()).unwrap_or("未実行");
        let cron = str_list(j.get("lolipop_crontab"), "\n");
        html.push_str(&format!(
            "<div class=\"lolipop-cron-job\"><strong>{}</strong> <code>{} {}</code><br>\
             時刻: {} / 曜日: {} / 後追い: {} / 状態: {} / 最終結果: {}<br>\
             <details><summary>ロリポップ管理画面用cron書式</summary><pre>{}</pre></details>\
             <button data-action=\"run\" data-id=\"{id}\">今すぐ実行</button> \
             <button data-action=\"toggle\" data-id=\"{id}\" data-enabled=\"{}\">{}</button> \
             <button data-action=\"delete\" data-id=\"{id}\">削除</button></div>",
            esc(name),
            esc(method),
            esc(url),
            esc(&times),
            esc(&days),
            if catch_up { "あり" } else { "なし" },
            if enabled { "有効" } else { "無効" },
            esc(last),
            esc(&cron),
            !enabled,
            if enabled { "無効にする" } else { "有効にする" },
        ));
    }
    if let Some(el) = try_by_id("lolipop-cron-list") {
        el.set_inner_html(&html);
    }
}

async fn refresh() {
    match crate::api_lolipop_cron::list(&base_url(), &input_value("lolipop-cron-admin-token")).await {
        Ok(v) => render_jobs(&v),
        Err(e) => set_text("lolipop-cron-status", &format!("❌ {e}")),
    }
}

fn on_refresh() {
    spawn_local(refresh());
}

fn on_create() {
    let times: Vec<String> = input_value("lolipop-cron-times")
        .split(|c: char| c == ',' || c == '、' || c.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let weekdays: Vec<u8> = (0u8..7).filter(|d| checked(&format!("lolipop-cron-wd-{d}"))).collect();
    let method = try_by_id("lolipop-cron-method")
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "GET".into());
    let job = crate::api_lolipop_cron::NewJob {
        name: input_value("lolipop-cron-name"),
        url: input_value("lolipop-cron-url"),
        method,
        schedule: crate::api_lolipop_cron::Schedule { times, weekdays },
        timeout_secs: input_value("lolipop-cron-timeout").trim().parse().unwrap_or(60),
        catch_up: checked("lolipop-cron-catch-up"),
    };
    spawn_local(async move {
        match crate::api_lolipop_cron::create(&base_url(), &input_value("lolipop-cron-admin-token"), &job).await {
            Ok(_) => {
                set_text("lolipop-cron-status", "✅ ジョブを登録しました。 / Job added.");
                refresh().await;
            }
            Err(e) => set_text("lolipop-cron-status", &format!("❌ {e}")),
        }
    });
}

/// ジョブ一覧内のボタン(実行・有効/無効・削除)をイベント委譲で処理する。
fn on_list_click(evt: Event) {
    let Some(btn) = evt
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .and_then(|el| el.closest("button[data-action]").ok().flatten())
    else {
        return;
    };
    let action = btn.get_attribute("data-action").unwrap_or_default();
    let id = btn.get_attribute("data-id").unwrap_or_default();
    let enabled = btn.get_attribute("data-enabled").as_deref() == Some("true");
    spawn_local(async move {
        let (b, t) = (base_url(), input_value("lolipop-cron-admin-token"));
        let result = match action.as_str() {
            "run" => crate::api_lolipop_cron::run_now(&b, &t, &id).await,
            "toggle" => crate::api_lolipop_cron::set_enabled(&b, &t, &id, enabled).await,
            "delete" => {
                if !crate::dom::window().confirm_with_message("このジョブを削除しますか? / Delete this job?").unwrap_or(false) {
                    return;
                }
                crate::api_lolipop_cron::delete(&b, &t, &id).await
            }
            _ => return,
        };
        match result {
            Ok(v) => {
                let msg = v
                    .get("result")
                    .and_then(|r| r.as_str())
                    .map(|r| format!("実行結果: {r}"))
                    .unwrap_or_else(|| "✅ 完了しました。".into());
                set_text("lolipop-cron-status", &msg);
            }
            Err(e) => set_text("lolipop-cron-status", &format!("❌ {e}")),
        }
        refresh().await;
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
    wire_click("lolipop-cron-refresh-btn", on_refresh)?;
    wire_click("lolipop-cron-add-btn", on_create)?;
    let list = by_id("lolipop-cron-list");
    let closure = Closure::<dyn FnMut(Event)>::new(on_list_click);
    list.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}
