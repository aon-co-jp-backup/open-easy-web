//! `/admin/storage/...`(ストレージ(HDD/SSD)のメンテナンス: 健康チェック・無駄なデータの整理・TRIM/最適化、
//! `server/src/storage_maintenance.rs`参照)への薄い`fetch()`ラッパー。共通の`call`は`api_auto_update.rs`のものを再利用する。

use serde::Serialize;
use serde_json::Value;

use crate::api_auto_update::call;

/// `GET /admin/storage/maintenance` — 設定と、直近の実行結果。
pub async fn overview(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, "/admin/storage/maintenance", "GET", admin_token, None).await
}

/// `POST /admin/storage/health` — 健康チェックを、今すぐ実行(読み取りだけ)。
pub async fn run_health(base_url: &str, admin_token: &str) -> Result<Value, String> {
    call::<()>(base_url, "/admin/storage/health", "POST", admin_token, None).await
}

#[derive(Serialize)]
struct ApplyBody {
    apply: bool,
}

/// `POST /admin/storage/tidy` — 無駄なデータの整理。`apply` が false なら、試し実行(何も消さない)。
pub async fn run_tidy(base_url: &str, admin_token: &str, apply: bool) -> Result<Value, String> {
    call(base_url, "/admin/storage/tidy", "POST", admin_token, Some(&ApplyBody { apply })).await
}

/// `POST /admin/storage/optimize` — SSD の TRIM / HDD の最適化。`apply` が false なら、計画だけ。
pub async fn run_optimize(base_url: &str, admin_token: &str, apply: bool) -> Result<Value, String> {
    call(base_url, "/admin/storage/optimize", "POST", admin_token, Some(&ApplyBody { apply })).await
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

/// `PUT /admin/storage/maintenance/settings` — 自動実行の設定。
pub async fn save_settings(base_url: &str, admin_token: &str, s: &Settings) -> Result<Value, String> {
    call(base_url, "/admin/storage/maintenance/settings", "PUT", admin_token, Some(s)).await
}
