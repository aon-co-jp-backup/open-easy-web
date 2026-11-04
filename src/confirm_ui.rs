//! メールの確認リンク(画面のURL `…/#/confirm-email-change?token=…`)を開いたときの確認画面。
//!
//! - トークンは URL のフラグメント(`#` 以降)にあり、サーバーのログに残らない。読み取ったら、すぐにアドレスバーから消す。
//! - **リンクを開いただけでは何も確定しない**(メールの先読み・プレビューでトークンが消費されないように)。
//!   「確認する」ボタンを押したときだけ、Connect の `Account/ConfirmEmailChange`(POST、RS-JSON)で確定する。

use crate::dom::{by_id, try_by_id};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlButtonElement};

thread_local! {
    static TOKEN: RefCell<Option<String>> = const { RefCell::new(None) };
}

const HASH_PREFIX: &str = "#/confirm-email-change?token=";

fn set_text(id: &str, text: &str) {
    if let Some(el) = try_by_id(id) {
        el.set_text_content(Some(text));
    }
}

/// トークンに使える文字だけ通す(英数字・`-`・`_`・`.`)。
fn clean_token(raw: &str) -> Option<String> {
    let t = raw.trim();
    (!t.is_empty() && t.len() <= 256 && t.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')).then(|| t.to_string())
}

/// `location.hash` から確認用トークンを取り出す(該当しなければ `None`)。
fn token_from_hash(hash: &str) -> Option<String> {
    hash.strip_prefix(HASH_PREFIX).and_then(clean_token)
}

fn on_confirm() {
    let Some(token) = TOKEN.with(|t| t.borrow().clone()) else {
        set_text("confirm-email-change-status", "❌ 確認用のトークンがありません。メールのリンクをもう一度開いてください。 / No token. Please open the link from the email again.");
        return;
    };
    spawn_local(async move {
        set_text("confirm-email-change-status", "確認中… / Confirming…");
        match crate::api_account::confirm_email_change(&token).await {
            Ok(v) => {
                let ja = v.get("message_ja").and_then(|x| x.as_str()).unwrap_or("変更しました。");
                let en = v.get("message_en").and_then(|x| x.as_str()).unwrap_or("Done.");
                set_text("confirm-email-change-status", &format!("✅ {ja} / {en}"));
                TOKEN.with(|t| *t.borrow_mut() = None);
                if let Ok(btn) = by_id("confirm-email-change-btn").dyn_into::<HtmlButtonElement>() {
                    btn.set_disabled(true);
                }
            }
            Err(e) => set_text("confirm-email-change-status", &format!("❌ {e}")),
        }
    });
}

pub fn wire() -> Result<(), JsValue> {
    let btn: HtmlButtonElement = by_id("confirm-email-change-btn").dyn_into()?;
    let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| on_confirm());
    btn.set_onclick(Some(closure.as_ref().unchecked_ref()));
    closure.forget();

    let win = crate::dom::window();
    let hash = win.location().hash().unwrap_or_default();
    if let Some(token) = token_from_hash(&hash) {
        TOKEN.with(|t| *t.borrow_mut() = Some(token));
        // トークンをアドレスバー・履歴から消す(フラグメントなので、サーバーには元から送られていない)。
        if let Ok(history) = win.history() {
            let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&win.location().pathname().unwrap_or_else(|_| "/".to_string())));
        }
        if let Some(sec) = try_by_id("confirm-email-change-section") {
            let _ = sec.class_list().remove_1("hidden");
            sec.scroll_into_view();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_taken_only_from_the_expected_hash_and_charset() {
        assert_eq!(token_from_hash("#/confirm-email-change?token=abc-DEF_123.x"), Some("abc-DEF_123.x".to_string()));
        assert_eq!(token_from_hash("#/other?token=abc"), None);
        assert_eq!(token_from_hash("#/confirm-email-change?token="), None);
        assert_eq!(token_from_hash("#/confirm-email-change?token=a b"), None);
        assert_eq!(token_from_hash("#/confirm-email-change?token=a&x=1"), None);
        assert_eq!(token_from_hash("#/confirm-email-change?token=<script>"), None);
    }
}
