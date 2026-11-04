//! ロリポップ!向け時間指定オートクロールの管理API(`LolipopCron/*`、`server/src/lolipop_cron.rs`参照)への薄い呼び出し。
//! 方式は Connect の単純呼び出し(`POST /open_easy_web.admin.v1.LolipopCron/<Method>`、JSON)で、REST ではない。
//! 共通の`rpc`は`api_auto_update.rs`のものを再利用する。

use serde::Serialize;
use serde_json::Value;

use crate::api_auto_update::{rpc, rpc_empty};

pub async fn list(base_url: &str, admin_token: &str) -> Result<Value, String> {
    rpc_empty(base_url, "LolipopCron/ListJobs", admin_token).await
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
    rpc(base_url, "LolipopCron/CreateJob", admin_token, job).await
}

pub async fn delete(base_url: &str, admin_token: &str, id: &str) -> Result<Value, String> {
    rpc(base_url, "LolipopCron/DeleteJob", admin_token, &IdBody { id }).await
}

pub async fn run_now(base_url: &str, admin_token: &str, id: &str) -> Result<Value, String> {
    rpc(base_url, "LolipopCron/RunJob", admin_token, &IdBody { id }).await
}

#[derive(Serialize)]
struct IdBody<'a> {
    id: &'a str,
}

#[derive(Serialize)]
struct EnabledBody<'a> {
    id: &'a str,
    enabled: bool,
}

pub async fn set_enabled(base_url: &str, admin_token: &str, id: &str, enabled: bool) -> Result<Value, String> {
    rpc(base_url, "LolipopCron/SetJobEnabled", admin_token, &EnabledBody { id, enabled }).await
}
