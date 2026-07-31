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
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlButtonElement, HtmlInputElement};

/// Step 6(深夜バックグラウンド自動アップデート)の設定表示・切り替え。
async fn refresh_auto_update_status(base_url: String) {
    let admin_token = input_value("auto-update-admin-token");
    match crate::api_auto_update::get_status(&base_url, &admin_token).await {
        Ok(value) => {
            let enabled = value.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
            let current_version = value.get("current_version").and_then(|v| v.as_str()).unwrap_or("?");
            if let Some(checkbox) = try_by_id("auto-update-enabled-toggle").and_then(|el| el.dyn_into::<HtmlInputElement>().ok()) {
                checkbox.set_checked(enabled);
            }
            set_text(
                "auto-update-status",
                &format!(
                    "現在のバージョン: {current_version} / 自動アップデート: {} / Current version: {current_version}, auto-update: {}",
                    if enabled { "有効" } else { "無効" },
                    if enabled { "enabled" } else { "disabled" }
                ),
            );
        }
        Err(e) => set_text("auto-update-status", &format!("❌ {e}")),
    }
}

fn on_refresh_auto_update_status() {
    spawn_local(async move {
        refresh_auto_update_status(same_origin_base_url()).await;
    });
}

fn on_toggle_auto_update(evt: Event) {
    let Some(checkbox) = evt.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) else {
        return;
    };
    let enabled = checkbox.checked();
    let admin_token = input_value("auto-update-admin-token");
    let base_url = same_origin_base_url();
    spawn_local(async move {
        match crate::api_auto_update::set_enabled(&base_url, &admin_token, enabled).await {
            Ok(value) => {
                let message = value.get("message_ja").and_then(|v| v.as_str()).unwrap_or("✅ 設定を更新しました。");
                set_text("auto-update-status", message);
            }
            Err(e) => set_text("auto-update-status", &format!("❌ {e}")),
        }
    });
}

/// Step 7(DATABASE暗号化、既定ON)の設定表示・切り替え(2026-07-31新設)。
async fn refresh_db_encryption_status(base_url: String) {
    let admin_token = input_value("db-encryption-admin-token");
    match crate::api_db_encryption::get_status(&base_url, &admin_token).await {
        Ok(value) => {
            let enabled = value.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
            if let Some(checkbox) = try_by_id("db-encryption-enabled-toggle").and_then(|el| el.dyn_into::<HtmlInputElement>().ok()) {
                checkbox.set_checked(enabled);
            }
            set_text(
                "db-encryption-status",
                &format!(
                    "DATABASE暗号化: {} / Database encryption: {}",
                    if enabled { "有効(ON)" } else { "無効(OFF)" },
                    if enabled { "enabled" } else { "disabled" }
                ),
            );
        }
        Err(e) => set_text("db-encryption-status", &format!("❌ {e}")),
    }
}

fn on_refresh_db_encryption_status() {
    spawn_local(async move {
        refresh_db_encryption_status(same_origin_base_url()).await;
    });
}

fn on_toggle_db_encryption(evt: Event) {
    let Some(checkbox) = evt.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) else {
        return;
    };
    let enabled = checkbox.checked();
    let admin_token = input_value("db-encryption-admin-token");
    let base_url = same_origin_base_url();
    spawn_local(async move {
        match crate::api_db_encryption::set_enabled(&base_url, &admin_token, enabled).await {
            Ok(value) => {
                let message = value.get("message_ja").and_then(|v| v.as_str()).unwrap_or("✅ 設定を更新しました。");
                set_text("db-encryption-status", message);
            }
            Err(e) => set_text("db-encryption-status", &format!("❌ {e}")),
        }
    });
}

const COMPAT_MODE_STORAGE_KEY: &str = "openeasyweb_compat_mode_v1";

/// Step 5(分散同期・ディザスタリカバリ)は、この`open-easy-web-server`
/// 自身の管理APIを呼ぶ想定のため、同一オリジン(現在の`Location`)を
/// ベースURLとして使う——`api_free_domain`のような別オリジン入力は不要
/// (このファイルサーバー自身の設定という位置づけのため、コーディネーター
/// 追加指示どおりStep 4と同じ流れの中に置く)。
fn same_origin_base_url() -> String {
    crate::dom::window().location().origin().unwrap_or_default()
}

fn input_value(id: &str) -> String {
    try_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}

fn set_text(id: &str, text: &str) {
    if let Some(el) = try_by_id(id) {
        el.set_text_content(Some(text));
    }
}

fn on_register_dist_sync_target() {
    let admin_token = input_value("dist-sync-admin-token");
    let host = input_value("dist-sync-host");
    let port: u16 = input_value("dist-sync-port").parse().unwrap_or(22);
    let username = input_value("dist-sync-username");
    let password_env = input_value("dist-sync-password-env");
    let remote_dir = input_value("dist-sync-remote-dir");
    let label = input_value("dist-sync-label");

    if host.trim().is_empty() || username.trim().is_empty() || password_env.trim().is_empty() {
        set_text(
            "dist-sync-result",
            "❌ host / username / password env var は必須です。 / host, username, and password env var are required.",
        );
        return;
    }

    let base_url = same_origin_base_url();
    spawn_local(async move {
        let label_opt = if label.trim().is_empty() { None } else { Some(label.as_str()) };
        match crate::api_dist_sync::register_target(
            &base_url,
            &admin_token,
            &host,
            port,
            &username,
            &password_env,
            &remote_dir,
            label_opt,
        )
        .await
        {
            Ok(_) => {
                set_text(
                    "dist-sync-result",
                    "✅ 同期先を登録しました(VPS/レンタルサーバー/PC/タブレット/スマホ/NAS等)。 / Sync target registered (VPS/rented server/PC/tablet/phone/NAS/etc).",
                );
                refresh_dist_sync_targets(base_url).await;
            }
            Err(e) => set_text("dist-sync-result", &format!("❌ {e}")),
        }
    });
}

async fn refresh_dist_sync_targets(base_url: String) {
    let admin_token = input_value("dist-sync-admin-token");
    match crate::api_dist_sync::list_targets(&base_url, &admin_token).await {
        Ok(value) => {
            let targets = value.get("targets").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let html = if targets.is_empty() {
                "<p class=\"muted\">登録済みの分散同期先はありません。 / No distributed sync targets registered yet.</p>".to_string()
            } else {
                let mut rows = String::new();
                for t in &targets {
                    let host = t.get("host").and_then(|v| v.as_str()).unwrap_or("?");
                    let port = t.get("port").and_then(|v| v.as_u64()).unwrap_or(0);
                    let label = t.get("label").and_then(|v| v.as_str()).unwrap_or("");
                    let id = t.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    rows.push_str(&format!(
                        "<div class=\"site-card\" data-target-id=\"{id}\"><strong>{host}:{port}</strong> {label} \
                         <button class=\"dist-sync-remove-btn\" data-target-id=\"{id}\">Remove (削除)</button></div>"
                    ));
                }
                rows
            };
            if let Some(el) = try_by_id("dist-sync-target-list") {
                el.set_inner_html(&html);
            }
        }
        Err(e) => set_text("dist-sync-result", &format!("❌ list failed: {e}")),
    }
}

fn on_refresh_dist_sync_targets() {
    spawn_local(async move {
        refresh_dist_sync_targets(same_origin_base_url()).await;
    });
}

fn on_set_email_fallback() {
    let admin_token = input_value("dist-sync-admin-token");
    let smtp_host = input_value("dist-sync-smtp-host");
    let smtp_port: u16 = input_value("dist-sync-smtp-port").parse().unwrap_or(587);
    let smtp_username = input_value("dist-sync-smtp-username");
    let smtp_password_env = input_value("dist-sync-smtp-password-env");
    let from_address = input_value("dist-sync-smtp-from");
    let to_address = input_value("dist-sync-smtp-to");

    if smtp_host.trim().is_empty() || to_address.trim().is_empty() {
        set_text(
            "dist-sync-fallback-result",
            "❌ SMTP host / To address は必須です。 / SMTP host and To address are required.",
        );
        return;
    }

    let base_url = same_origin_base_url();
    spawn_local(async move {
        match crate::api_dist_sync::set_disaster_fallback_email(
            &base_url,
            &admin_token,
            &smtp_host,
            smtp_port,
            &smtp_username,
            &smtp_password_env,
            &from_address,
            &to_address,
        )
        .await
        {
            Ok(_) => set_text(
                "dist-sync-fallback-result",
                "✅ メール退避先を設定しました。 / Email fallback destination configured.",
            ),
            Err(e) => set_text("dist-sync-fallback-result", &format!("❌ {e}")),
        }
    });
}

fn on_set_gdrive_fallback() {
    let admin_token = input_value("dist-sync-admin-token");
    let folder = input_value("dist-sync-gdrive-folder");
    let client_id_env = input_value("dist-sync-gdrive-client-id-env");
    let client_secret_env = input_value("dist-sync-gdrive-client-secret-env");
    let refresh_token_env = input_value("dist-sync-gdrive-refresh-token-env");

    if folder.trim().is_empty() || refresh_token_env.trim().is_empty() {
        set_text(
            "dist-sync-fallback-result",
            "❌ Backup folder name / Refresh token env var は必須です。 / \
             Backup folder name and refresh token env var are required.",
        );
        return;
    }

    let base_url = same_origin_base_url();
    spawn_local(async move {
        match crate::api_dist_sync::set_disaster_fallback_google_drive(
            &base_url,
            &admin_token,
            &folder,
            &client_id_env,
            &client_secret_env,
            &refresh_token_env,
        )
        .await
        {
            Ok(_) => set_text(
                "dist-sync-fallback-result",
                "✅ Googleドライブ退避先を設定しました。 / Google Drive fallback destination configured.",
            ),
            Err(e) => set_text("dist-sync-fallback-result", &format!("❌ {e}")),
        }
    });
}

fn on_verify_dist_sync() {
    let admin_token = input_value("dist-sync-admin-token");
    let base_url = same_origin_base_url();
    spawn_local(async move {
        match crate::api_dist_sync::run_first_time_setup(&base_url, &admin_token).await {
            Ok(report) => {
                let ready = report.get("any_offsite_target_ready").and_then(|v| v.as_bool()).unwrap_or(false);
                let msg = if ready {
                    "✅ 1つ以上の同期先/退避先が準備できました。 / One or more sync/fallback targets are ready."
                } else {
                    "ℹ️ 準備できた同期先/退避先はまだありません(接続情報を確認してください、または未設定でもファイルサーバー自体は使用できます)。 / \
                     No sync/fallback target is ready yet (check connection details, or continue without one — this does not block normal use)."
                };
                set_text("dist-sync-fallback-result", msg);
            }
            Err(e) => set_text("dist-sync-fallback-result", &format!("❌ {e}")),
        }
    });
}

fn on_skip_dist_sync() {
    set_text(
        "dist-sync-fallback-result",
        "⏭️ スキップしました。後からいつでもこの画面で設定できます。 / \
         Skipped. You can configure this at any time from this screen later.",
    );
}

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

/// チェックボックス等の`change`イベント配線(`wire_click`のチェック
/// ボックス版、`on_toggle_auto_update`のようにイベント自体〈チェック後の
/// 状態〉を必要とするコールバック向け)。
fn wire_change(id: &str, f: impl Fn(Event) + 'static) -> Result<(), JsValue> {
    let input: HtmlInputElement = by_id(id).dyn_into()?;
    let closure = Closure::<dyn FnMut(Event)>::new(move |evt: Event| f(evt));
    input.set_onchange(Some(closure.as_ref().unchecked_ref()));
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

    wire_click("dist-sync-register-btn", on_register_dist_sync_target)?;
    wire_click("dist-sync-refresh-btn", on_refresh_dist_sync_targets)?;
    wire_click("dist-sync-set-email-fallback-btn", on_set_email_fallback)?;
    wire_click("dist-sync-set-gdrive-fallback-btn", on_set_gdrive_fallback)?;
    wire_click("dist-sync-verify-btn", on_verify_dist_sync)?;
    wire_click("dist-sync-skip-btn", on_skip_dist_sync)?;
    wire_dist_sync_remove_delegation()?;

    wire_click("auto-update-refresh-status-btn", on_refresh_auto_update_status)?;
    wire_change("auto-update-enabled-toggle", on_toggle_auto_update)?;

    wire_click("db-encryption-refresh-status-btn", on_refresh_db_encryption_status)?;
    wire_change("db-encryption-enabled-toggle", on_toggle_db_encryption)?;
    Ok(())
}

/// 動的生成される「削除」ボタンを、コンテナ1つのイベント委譲で処理する
/// (`free_domain_ui.rs`と同じ設計方針——ボタンごとに`forget()`し続ける
/// クロージャがメモリを増やし続けないため)。
fn wire_dist_sync_remove_delegation() -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    let container = by_id("dist-sync-target-list");
    let closure = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        let Some(target) = evt.target() else { return };
        let Ok(el) = target.dyn_into::<web_sys::Element>() else { return };
        let Some(btn) = el.closest(".dist-sync-remove-btn").ok().flatten() else { return };
        let Some(id) = btn.get_attribute("data-target-id") else { return };
        if id.is_empty() {
            return;
        }
        let admin_token = input_value("dist-sync-admin-token");
        let base_url = same_origin_base_url();
        spawn_local(async move {
            match crate::api_dist_sync::delete_target(&base_url, &admin_token, &id).await {
                Ok(_) => {
                    set_text("dist-sync-result", "🗑️ 削除しました。 / Removed.");
                    refresh_dist_sync_targets(base_url).await;
                }
                Err(e) => set_text("dist-sync-result", &format!("❌ {e}")),
            }
        });
    });
    container
        .dyn_ref::<web_sys::HtmlElement>()
        .ok_or_else(|| JsValue::from_str("dist-sync-target-list is not an HtmlElement"))?
        .set_onclick(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
    Ok(())
}
