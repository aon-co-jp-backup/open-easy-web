//! アカウント登録・ログイン(メールOTP)・サイト操作(フォルダー作成・
//! アップロード・AI判定)のDOM配線。`api_auth`/`api_upload`の薄い
//! `fetch()`ラッパーを呼び出し、結果を各パネルの結果表示欄へ反映する。

use crate::dom::{by_id, try_by_id};
use crate::{api_auth, api_upload};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlButtonElement, HtmlInputElement, HtmlSelectElement};

fn input_value(id: &str) -> String {
    try_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value())
        .unwrap_or_default()
}

fn select_value(id: &str) -> String {
    try_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_default()
}

fn set_text(id: &str, text: &str) {
    if let Some(el) = try_by_id(id) {
        el.set_text_content(Some(text));
    }
}

fn bilingual_message(value: &serde_json::Value, fallback: &str) -> String {
    let ja = value.get("message_ja").and_then(|v| v.as_str());
    let en = value.get("message_en").and_then(|v| v.as_str());
    match (ja, en) {
        (Some(ja), Some(en)) => format!("{ja} / {en}"),
        (Some(ja), None) => ja.to_string(),
        (None, Some(en)) => en.to_string(),
        (None, None) => fallback.to_string(),
    }
}

/// ログイン状態に応じて、未ログイン用パネル・ログイン済みパネル・
/// サイト操作パネルの表示/非表示を切り替える。
pub fn sync_auth_visibility() {
    let logged_in = api_auth::saved_token().is_some();
    if let Some(el) = try_by_id("auth-logged-out") {
        el.set_class_name(if logged_in { "hidden" } else { "" });
    }
    if let Some(el) = try_by_id("auth-logged-in") {
        el.set_class_name(if logged_in { "" } else { "hidden" });
    }
    if let Some(el) = try_by_id("site-ops-section") {
        el.set_class_name(if logged_in { "" } else { "hidden" });
    }
    // セキュリティ上、サイト管理画面(登録済みサイト一覧・追加編集フォーム)は
    // ログインするまで表示しない(ユーザー指示、2026-07-16)。
    if let Some(el) = try_by_id("site-mgmt-section") {
        el.set_class_name(if logged_in { "" } else { "hidden" });
    }
    if let Some(email) = api_auth::saved_account_email() {
        set_text("account-email-label", &email);
    }
}

fn on_request_otp() {
    let contact = input_value("login-contact");
    if contact.trim().is_empty() {
        set_text("login-result", "メール1・メール2・電話番号のいずれかを入力してください。 / Enter one of Email 1, Email 2, or phone.");
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        set_text("login-result", "送信中… / Sending…");
        match api_auth::request_otp(&contact).await {
            Ok(value) => set_text("login-result", &bilingual_message(&value, "送信しました。 / Sent.")),
            Err(e) => set_text("login-result", &format!("❌ {e}")),
        }
    });
}

fn on_verify_otp() {
    let contact = input_value("login-contact");
    let code = input_value("login-code");
    let totp_code = input_value("login-totp-code");
    if contact.trim().is_empty() || code.trim().is_empty() {
        set_text("login-result", "連絡先とコードの両方を入力してください。 / Enter both the contact and the code.");
        return;
    }
    let totp_code = if totp_code.trim().is_empty() { None } else { Some(totp_code) };
    wasm_bindgen_futures::spawn_local(async move {
        set_text("login-result", "確認中… / Verifying…");
        match api_auth::verify_otp(&contact, &code, totp_code.as_deref()).await {
            Ok(api_auth::VerifyOtpOutcome::LoggedIn(_token)) => {
                set_text("login-result", "✅ ログインしました。 / Logged in.");
                sync_auth_visibility();
            }
            Ok(api_auth::VerifyOtpOutcome::TotpRequired) => {
                set_text(
                    "login-result",
                    "🔐 このアカウントは認証アプリの2段階認証が有効です。新しいOTPを取得し、\
                     6桁コードと一緒に「認証アプリのコード」欄にも入力してもう一度ログインしてください。 / \
                     This account requires an authenticator app code. Request a fresh OTP and \
                     submit it together with the 6-digit code in the \"Authenticator code\" field.",
                );
                if let Some(el) = try_by_id("login-totp-row") {
                    el.set_class_name("");
                }
            }
            Err(e) => set_text("login-result", &format!("❌ {e}")),
        }
    });
}

fn on_totp_setup() {
    wasm_bindgen_futures::spawn_local(async move {
        set_text("totp-result", "セットアップ中… / Setting up…");
        match api_auth::totp_setup().await {
            Ok(value) => {
                let secret = value.get("secret").and_then(|v| v.as_str()).unwrap_or("");
                let uri = value.get("provisioning_uri").and_then(|v| v.as_str()).unwrap_or("");
                set_text("totp-secret", secret);
                set_text("totp-uri", uri);
                set_text(
                    "totp-result",
                    "認証アプリでシークレットまたはURIを登録し、表示された6桁コードを下に入力して有効化してください。 / \
                     Add the secret or URI to your authenticator app, then enter the displayed 6-digit code below to enable.",
                );
                if let Some(el) = try_by_id("totp-enable-row") {
                    el.set_class_name("");
                }
            }
            Err(e) => set_text("totp-result", &format!("❌ {e}")),
        }
    });
}

fn on_totp_enable() {
    let code = input_value("totp-confirm-code");
    if code.trim().is_empty() {
        set_text("totp-result", "認証アプリの6桁コードを入力してください。 / Enter the 6-digit code from your authenticator app.");
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        match api_auth::totp_enable(&code).await {
            Ok(value) => set_text("totp-result", &bilingual_message(&value, "有効化しました。 / Enabled.")),
            Err(e) => set_text("totp-result", &format!("❌ {e}")),
        }
    });
}

fn on_totp_disable() {
    wasm_bindgen_futures::spawn_local(async move {
        match api_auth::totp_disable().await {
            Ok(value) => set_text("totp-result", &bilingual_message(&value, "無効化しました。 / Disabled.")),
            Err(e) => set_text("totp-result", &format!("❌ {e}")),
        }
    });
}

fn on_logout() {
    wasm_bindgen_futures::spawn_local(async move {
        let _ = api_auth::logout().await;
        sync_auth_visibility();
        set_text("login-result", "ログアウトしました。 / Logged out.");
    });
}

fn on_request_contact_change() {
    let field = select_value("change-email-field");
    let new_value = input_value("change-email-new");
    if new_value.trim().is_empty() {
        set_text("change-email-result", "新しい値を入力してください。 / Enter a new value.");
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        set_text("change-email-result", "送信中… / Sending…");
        match api_auth::request_contact_change(&field, &new_value).await {
            Ok(value) => set_text("change-email-result", &bilingual_message(&value, "送信しました。 / Sent.")),
            Err(e) => set_text("change-email-result", &format!("❌ {e}")),
        }
    });
}

fn on_create_folder() {
    let site = input_value("site-ops-name");
    if site.trim().is_empty() {
        set_text("site-ops-result", "サイト名を入力してください。 / Enter a site name.");
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        set_text("site-ops-result", "作成中… / Creating…");
        match api_upload::create_folder(&site).await {
            Ok(value) => set_text("site-ops-result", &bilingual_message(&value, "作成しました。 / Created.")),
            Err(e) => set_text("site-ops-result", &format!("❌ {e}")),
        }
    });
}

fn on_upload_files() {
    let site = input_value("site-ops-name");
    if site.trim().is_empty() {
        set_text("site-ops-result", "サイト名を入力してください。 / Enter a site name.");
        return;
    }
    let Some(files_el) = try_by_id("site-ops-files").and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
    else {
        return;
    };
    let Some(files) = files_el.files() else {
        set_text("site-ops-result", "ファイルを選択してください。 / Select files first.");
        return;
    };
    if files.length() == 0 {
        set_text("site-ops-result", "ファイルを選択してください。 / Select files first.");
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        set_text("site-ops-result", "アップロード中… / Uploading…");
        match api_upload::upload_files(&site, &files).await {
            Ok(value) => set_text("site-ops-result", &bilingual_message(&value, "アップロードしました。 / Uploaded.")),
            Err(e) => set_text("site-ops-result", &format!("❌ {e}")),
        }
    });
}

fn on_detect_and_configure() {
    let site = input_value("site-ops-name");
    if site.trim().is_empty() {
        set_text("site-ops-result", "サイト名を入力してください。 / Enter a site name.");
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        set_text("site-ops-result", "🤖 判定中… / Detecting…");
        match api_upload::detect_and_configure(&site).await {
            Ok(value) => {
                let confidence = value.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let pct = (confidence * 100.0).round();
                let mut msg = format!("🤖 確信度 / confidence: {pct}%\n");
                msg.push_str(&bilingual_message(&value, ""));
                set_text("site-ops-result", &msg);
                if let Some(el) = try_by_id("site-ops-correction") {
                    el.set_class_name("");
                }
            }
            Err(e) => set_text("site-ops-result", &format!("❌ {e}")),
        }
    });
}

fn on_correct(is_php: bool) {
    let site = input_value("site-ops-name");
    if site.trim().is_empty() {
        return;
    }
    wasm_bindgen_futures::spawn_local(async move {
        match api_upload::correct_detection(&site, is_php).await {
            Ok(value) => {
                set_text("site-ops-result", &bilingual_message(&value, "訂正を記録しました。 / Correction recorded."));
                if let Some(el) = try_by_id("site-ops-correction") {
                    el.set_class_name("hidden");
                }
            }
            Err(e) => set_text("site-ops-result", &format!("❌ {e}")),
        }
    });
}

/// クリックイベントを`f`に配線する共通ヘルパー。
fn wire_click(id: &str, f: impl Fn() + 'static) -> Result<(), JsValue> {
    let btn: HtmlButtonElement = by_id(id).dyn_into()?;
    let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| f());
    btn.set_onclick(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
    Ok(())
}

pub fn wire() -> Result<(), JsValue> {
    wire_click("login-request-otp", on_request_otp)?;
    wire_click("login-verify-otp", on_verify_otp)?;
    wire_click("logout-btn", on_logout)?;
    wire_click("change-email-submit", on_request_contact_change)?;
    wire_click("totp-setup-btn", on_totp_setup)?;
    wire_click("totp-enable-btn", on_totp_enable)?;
    wire_click("totp-disable-btn", on_totp_disable)?;
    wire_click("site-ops-create-folder", on_create_folder)?;
    wire_click("site-ops-upload", on_upload_files)?;
    wire_click("site-ops-detect", on_detect_and_configure)?;
    wire_click("site-ops-correct-yes", || on_correct(true))?;
    wire_click("site-ops-correct-no", || on_correct(false))?;

    sync_auth_visibility();
    Ok(())
}
