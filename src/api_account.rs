//! アカウント系の Connect 呼び出し(`POST /open_easy_web.admin.v1.Account/<Method>`、POST、RS-JSON)。
//! ログイン前から呼べる(トークン自体が資格情報)。共通の `call` は `api_auto_update.rs` のものを再利用する。

use serde::Serialize;
use serde_json::Value;

use crate::api_auto_update::call;

#[derive(Serialize)]
struct TokenBody<'a> {
    token: &'a str,
}

/// `Account/ConfirmEmailChange` — メールの確認リンクの確認ボタンを押したときに、連絡先の変更を確定する。
pub async fn confirm_email_change(token: &str) -> Result<Value, String> {
    let base = crate::dom::window().location().origin().unwrap_or_default();
    call(&base, "/open_easy_web.admin.v1.Account/ConfirmEmailChange", "POST", "", Some(&TokenBody { token })).await
}
