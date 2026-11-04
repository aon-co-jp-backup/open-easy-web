//! `/admin/notify/...`(更新のお知らせメール、`server/src/update_notify.rs`参照)への薄い`fetch()`ラッパー。
//! 共通の`call`は`api_auto_update.rs`のものを再利用する。

use serde::Serialize;
use serde_json::Value;

use crate::api_auto_update::call;

/// `GET /admin/notify` — 登録済みの宛先、履歴、SMTPが設定済みか。
pub async fn overview(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, "/admin/notify", "GET", admin_token, None).await
}

#[derive(Serialize)]
struct Recipients<'a> {
    emails: &'a [String],
}

/// `PUT /admin/notify/recipients` — 宛先の一覧を置き換える。
pub async fn save_recipients(base_url: &str, admin_token: &str, emails: &[String]) -> Result<Value, String> {
    call(base_url, "/admin/notify/recipients", "PUT", admin_token, Some(&Recipients { emails })).await
}

/// `POST /admin/notify/test` — 登録済みの全宛先へ、テストのお知らせを送る。
pub async fn send_test(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, "/admin/notify/test", "POST", admin_token, None).await
}
