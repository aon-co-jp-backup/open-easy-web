//! 更新のお知らせメール(`server/src/update_notify.rs`)への薄い`fetch()`ラッパー。
//! 方式は Connect の単純呼び出し(`POST /open_easy_web.admin.v1.Notify/<Method>`、本文は JSON)。REST は使わない
//! (2026-11-04 方針: REST 全廃、`CLAUDE.md`「API 方針」参照)。共通の`call`は`api_auto_update.rs`のものを再利用する。

use serde::Serialize;
use serde_json::{json, Value};

use crate::api_auto_update::call;

const SVC: &str = "/open_easy_web.admin.v1.Notify/";

/// `Notify/Get` — 登録済みの宛先、履歴、SMTPが設定済みか。
pub async fn overview(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call(base_url, &format!("{SVC}Get"), "POST", admin_token, Some(&json!({}))).await
}

#[derive(Serialize)]
struct Recipients<'a> {
    emails: &'a [String],
}

/// `Notify/SetRecipients` — 宛先の一覧を置き換える。
pub async fn save_recipients(base_url: &str, admin_token: &str, emails: &[String]) -> Result<Value, String> {
    call(base_url, &format!("{SVC}SetRecipients"), "POST", admin_token, Some(&Recipients { emails })).await
}

/// `Notify/SendTest` — 登録済みの全宛先へ、テストのお知らせを送る。
pub async fn send_test(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call(base_url, &format!("{SVC}SendTest"), "POST", admin_token, Some(&json!({}))).await
}
