//! `/admin/lolipop-cron/jobs`(ロリポップ!向け時間指定オートクロール、
//! `server/src/lolipop_cron.rs`参照)への薄い`fetch()`ラッパー。
//! 共通の`call`は`api_auto_update.rs`のものを再利用する。

use serde::Serialize;
use serde_json::Value;

use crate::api_auto_update::call;

const BASE: &str = "/admin/lolipop-cron/jobs";

pub async fn list(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, BASE, "GET", admin_token, None).await
}

#[derive(Serialize)]
pub struct NewJob {
    pub name: String,
    pub url: String,
    pub method: String,
    pub schedule: Schedule,
    pub timeout_secs: u64,
    pub catch_up: bool,
}

#[derive(Serialize)]
pub struct Schedule {
    pub times: Vec<String>,
    pub weekdays: Vec<u8>,
}

pub async fn create(base_url: &str, admin_token: &str, job: &NewJob) -> Result<Value, String> {
    call(base_url, BASE, "POST", admin_token, Some(job)).await
}

pub async fn delete(base_url: &str, admin_token: &str, id: &str) -> Result<Value, String> {
    call::<()>(base_url, &format!("{BASE}/{id}"), "DELETE", admin_token, None).await
}

pub async fn run_now(base_url: &str, admin_token: &str, id: &str) -> Result<Value, String> {
    call::<()>(base_url, &format!("{BASE}/{id}/run"), "POST", admin_token, None).await
}

#[derive(Serialize)]
struct EnabledBody {
    enabled: bool,
}

pub async fn set_enabled(base_url: &str, admin_token: &str, id: &str, enabled: bool) -> Result<Value, String> {
    call(base_url, &format!("{BASE}/{id}/enabled"), "PUT", admin_token, Some(&EnabledBody { enabled })).await
}
