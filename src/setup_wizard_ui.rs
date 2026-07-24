//! 「初回セットアップガイド」画面のDOM配線(2026-07-24新設)。
//!
//! ユーザー要望のフロー: VPSを借りたら (1) 現在アクセスしているIPアドレスを
//! 確認し、(2) SFTPクライアントで`open-easy-web`フォルダを作りアップロード
//! し、(3) Apache互換/Nginx互換のどちらでopen-web-serverを動かすかを選び、
//! (4) （まだインストールしていなければ）`open-web-server`の`install.sh`を
//! 呼ぶワンライナーコマンドを表示する。
//!
//! **安全設計上の意図的な制約(正直な開示)**: (a) SFTPアップロード自体は
//! ユーザーがSFTPクライアント上で手動操作するものであり、この画面から
//! 自動化することはしない(できない)。(b) インストールコマンドは
//! 画面に表示してコピー&ペーストしてもらうだけで、このアプリ自身が
//! VPS上で任意のシェルコマンドを実行することは絶対に行わない
//! (サーバーサイドからの任意コマンド実行機能そのものを実装しない)。
//!
//! **open-web-serverは1台のVPSにつき1回だけインストールする常駐サーバー**
//! という前提(tenant_routerによる1プロセス内マルチテナント振り分け)を
//! 踏まえ、Step 4では「未インストールならこのコマンドで導入」「既に
//! インストール済みなら、この画面の上にあるサイト管理(共有バックエンドへ
//! 登録)または下の簡単ドメイン設定ウィザードから追加登録するだけでよい」
//! という案内文言をHTML側(`shell.rs`)に明記している——稼働判定を新規に
//! 自動検知する機能は過剰実装として今回は追加しない。

use crate::dom::{by_id, set_status, try_by_id};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlButtonElement};

const COMPAT_MODE_STORAGE_KEY: &str = "openeasyweb_compat_mode_v1";

fn local_storage() -> Option<web_sys::Storage> {
    crate::dom::window().local_storage().ok().flatten()
}

/// 現在のURL(location)からホスト名(IPアドレスまたはドメイン)を取得し、
/// `#setup-wizard-current-host` へ表示する。
fn render_current_host() {
    let host = crate::dom::window()
        .location()
        .host()
        .unwrap_or_else(|_| "(不明 / unknown)".to_string());
    if let Some(el) = try_by_id("setup-wizard-current-host") {
        el.set_text_content(Some(&host));
    }
}

/// Apache互換/Nginx互換モードの選択を`localStorage`へ保存し、結果メッセージを
/// 表示する。実際のopen-web-server側`web_vhosts.toml`/管理APIへの反映は
/// このモード名(`"apache"`/`"nginx"`)をvhost登録時の`compat_mode`フィールドへ
/// 指定することで行う(サイト管理画面・簡単ドメイン設定ウィザードの
/// 既存の登録フローと組み合わせて使う想定、過剰な自動連携は今回追加しない)。
fn on_choose_compat_mode(mode: &'static str) {
    if let Some(storage) = local_storage() {
        let _ = storage.set_item(COMPAT_MODE_STORAGE_KEY, mode);
    }

    let label_ja = if mode == "apache" {
        "Apache互換"
    } else {
        "Nginx互換"
    };
    let label_en = if mode == "apache" { "Apache-compatible" } else { "Nginx-compatible" };

    if let Some(el) = try_by_id("setup-wizard-mode-result") {
        el.set_text_content(Some(&format!(
            "✅ {label_ja}モードを選択しました({label_en}選択済み)。open-web-serverへ\
             このサイトを登録する際、compat_mode=\"{mode}\"を指定してください\
             (サイト管理画面の「共有バックエンドへ登録」、または簡単ドメイン設定\
             ウィザードと組み合わせて使います)。 / Selected {label_en} mode. When \
             registering this site with open-web-server, specify \
             compat_mode=\"{mode}\" (combine with the site manager's \"register with \
             shared backend\" option, or the Easy Free-Domain Setup wizard)."
        )));
    }
    set_status(&format!("{label_ja}モードを選択しました。"));
}

fn wire_click(id: &str, f: impl Fn() + 'static) -> Result<(), JsValue> {
    let btn: HtmlButtonElement = by_id(id).dyn_into()?;
    let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| f());
    btn.set_onclick(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
    Ok(())
}

/// 直前に選択したモードを`localStorage`から読み出す(テスト・他モジュールの
/// 参照用に公開)。
pub fn selected_compat_mode() -> Option<String> {
    local_storage().and_then(|s| s.get_item(COMPAT_MODE_STORAGE_KEY).ok().flatten())
}

pub fn wire() -> Result<(), JsValue> {
    render_current_host();
    // 前回セッションで選択済みのモードがあれば、その旨を先に表示しておく
    // (localStorageに保存した値を実際に読み出して使う経路、dead_code回避)。
    if let Some(previous) = selected_compat_mode() {
        crate::dom::log(&format!("open-easy-web: previously selected compat mode = {previous}"));
    }
    wire_click("setup-wizard-apache-btn", || on_choose_compat_mode("apache"))?;
    wire_click("setup-wizard-nginx-btn", || on_choose_compat_mode("nginx"))?;
    Ok(())
}
