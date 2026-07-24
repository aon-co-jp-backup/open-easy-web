//! open-easy-web: 「第二のKUSANAGI」を目指す運用ツールのWeb UI。
//!
//! Rust を `wasm32-unknown-unknown` へコンパイルし、`wasm-bindgen` + `web-sys`
//! で DOM 操作と `fetch()` を行う。TypeScript/Node.js のビルドチェーンは
//! 使用しない(詳細は CLAUDE.md / README.md 参照)。
//!
//! ## できること
//! - 「サイト管理」画面で、open-easy-web自身・WordPress・Laravel・FastAPIなど
//!   任意のバックエンドスタックのデプロイ先(IPアドレス/ドメイン/サブドメイン/
//!   ポート)を複数登録し、`localStorage` に保存してワンクリックで選択・
//!   疎通確認できる(KUSANAGIのサイト一覧に相当する最小限の管理UI)。
//! - 登録済みサイト一覧のJSONエクスポート/インポート(バックアップ・
//!   他ブラウザへの持ち出し用)。
//! - 実際のドメイン取得・DNS登録はここでは行わない
//!   (`deploy/` の vhost テンプレート・`scripts/gen-vhost.sh` を参照)。
//! - DB(aruaru-db等)への接続機能は持たない(意図的にスコープ外)。

mod api_auth;
mod api_free_domain;
mod api_upload;
mod auth_ui;
mod dom;
mod free_domain_ui;
mod profiles;
mod setup_wizard_ui;
mod shell;
pub mod view_bridge;

use dom::{by_id, log, set_status};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlButtonElement};

/// エントリポイント。ページ読み込み時に一度だけ呼ばれる。
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook_set();
    log("open-easy-web starting");

    let root = by_id("app-root");
    root.set_inner_html(shell::SHELL_HTML);

    // 「サイトを保存」ボタン
    let save_site_btn: HtmlButtonElement = by_id("save-site").dyn_into()?;
    let save_site_closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
        profiles::on_save_site();
    });
    save_site_btn.set_onclick(Some(save_site_closure.as_ref().unchecked_ref()));
    save_site_closure.forget();

    // 「クリア」ボタン
    let clear_btn: HtmlButtonElement = by_id("clear-site-form").dyn_into()?;
    let clear_closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
        profiles::clear_form();
    });
    clear_btn.set_onclick(Some(clear_closure.as_ref().unchecked_ref()));
    clear_closure.forget();

    // 「サイト一覧をエクスポート(JSON)」ボタン
    let site_export_btn: HtmlButtonElement = by_id("site-export").dyn_into()?;
    let site_export_closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
        profiles::export_profiles_json();
    });
    site_export_btn.set_onclick(Some(site_export_closure.as_ref().unchecked_ref()));
    site_export_closure.forget();

    // 「サイト一覧をインポート(JSON)」ボタン → 隠しファイル入力をクリック
    let site_import_trigger: HtmlButtonElement = by_id("site-import-trigger").dyn_into()?;
    let site_import_file: web_sys::HtmlInputElement = by_id("site-import-file").dyn_into()?;
    let import_file_for_trigger = site_import_file.clone();
    let site_import_trigger_closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
        import_file_for_trigger.click();
    });
    site_import_trigger.set_onclick(Some(site_import_trigger_closure.as_ref().unchecked_ref()));
    site_import_trigger_closure.forget();

    let import_file_for_change = site_import_file.clone();
    let site_import_change_closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
        if let Some(files) = import_file_for_change.files() {
            if let Some(file) = files.get(0) {
                profiles::import_profiles_from_file(file);
            }
        }
        import_file_for_change.set_value("");
    });
    site_import_file
        .set_onchange(Some(site_import_change_closure.as_ref().unchecked_ref()));
    site_import_change_closure.forget();

    profiles::render_site_manager();
    profiles::sync_active_site_label();
    auth_ui::wire()?;
    free_domain_ui::wire()?;
    setup_wizard_ui::wire()?;

    set_status("準備完了。サイトを登録・選択・接続テストできます。");
    Ok(())
}

/// panic 時にブラウザの console に stacktrace 相当を出す(デバッグ用)。
fn console_error_panic_hook_set() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&JsValue::from_str(&info.to_string()));
    }));
}
