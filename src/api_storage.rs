//! ストレージ(HDD/SSD)のメンテナンス(健康チェック・無駄なデータの整理・TRIM/最適化、`server/src/storage_maintenance.rs`)への
//! 薄い`fetch()`ラッパー。方式は Connect(POST、RS-JSON)(`POST /open_easy_web.admin.v1.Storage/<Method>`、本文は JSON)。
//! REST は使わない(2026-11-04 方針: REST 全廃、`CLAUDE.md`「API 方針」参照)。共通の`call`は`api_auto_update.rs`のものを再利用する。

use serde::Serialize;
use serde_json::{json, Value};

use crate::api_auto_update::call;

const SVC: &str = "/open_easy_web.admin.v1.Storage/";

/// `Storage/GetMaintenance` — 設定と、直近の実行結果。
pub async fn overview(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call(base_url, &format!("{SVC}GetMaintenance"), "POST", admin_token, Some(&json!({}))).await
}

/// `Storage/RunHealth` — 健康チェックを、今すぐ実行(読み取りだけ)。
pub async fn run_health(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call(base_url, &format!("{SVC}RunHealth"), "POST", admin_token, Some(&json!({}))).await
}

#[derive(Serialize)]
struct ApplyBody {
    apply: bool,
}

/// `Storage/RunTidy` — 無駄なデータの整理。`apply` が false なら、試し実行(何も消さない)。
pub async fn run_tidy(base_url: &str, admin_token: &str, apply: bool) -> Result<Value, String> {
    call(base_url, &format!("{SVC}RunTidy"), "POST", admin_token, Some(&ApplyBody { apply })).await
}

/// `Storage/RunOptimize` — SSD の TRIM / HDD の最適化。`apply` が false なら、計画だけ。
pub async fn run_optimize(base_url: &str, admin_token: &str, apply: bool) -> Result<Value, String> {
    call(base_url, &format!("{SVC}RunOptimize"), "POST", admin_token, Some(&ApplyBody { apply })).await
}

#[derive(Serialize)]
pub struct Settings {
    pub health_enabled: bool,
    pub tidy_enabled: bool,
    pub tidy_apply: bool,
    pub optimize_enabled: bool,
    pub optimize_apply: bool,
    pub hour: u8,
}

/// `Storage/SetSettings` — 自動実行の設定。
pub async fn save_settings(base_url: &str, admin_token: &str, s: &Settings) -> Result<Value, String> {
    call(base_url, &format!("{SVC}SetSettings"), "POST", admin_token, Some(s)).await
}
