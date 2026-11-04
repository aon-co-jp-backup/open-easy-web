//! 分散同期(dist-sync)+ディザスタリカバリの管理API(`DistSync/*`、`server/src/dist_sync.rs`参照)への薄い呼び出し。
//! 方式は Connect(POST、RS-JSON)(`POST /open_easy_web.admin.v1.DistSync/<Method>`、JSON)で、REST ではない。
//! 共通の`rpc`は`api_auto_update.rs`のものを再利用する。

use serde::Serialize;
use serde_json::Value;

use crate::api_auto_update::{rpc, rpc_empty};


#[derive(Serialize)]
struct RegisterTargetBody<'a> {
    host: &'a str,
    port: u16,
    username: &'a str,
    password_env: &'a str,
    remote_backup_dir: &'a str,
    label: Option<&'a str>,
}

/// `DistSync/RegisterTarget` — 他VPSへの分散同期クローンDB先を1件登録。
#[allow(clippy::too_many_arguments)]
pub async fn register_target(
    base_url: &str,
    admin_token: &str,
    host: &str,
    port: u16,
    username: &str,
    password_env: &str,
    remote_backup_dir: &str,
    label: Option<&str>,
) -> Result<Value, String> {
    rpc(base_url, "DistSync/RegisterTarget", admin_token, &RegisterTargetBody { host, port, username, password_env, remote_backup_dir, label })
    .await
}

/// `DistSync/ListTargets` — 登録済みの分散同期先一覧を取得。
pub async fn list_targets(base_url: &str, admin_token: &str) -> Result<Value, String> {
    rpc_empty(base_url, "DistSync/ListTargets", admin_token).await
}

/// `DistSync/DeleteTarget` — 分散同期先を1件削除。
pub async fn delete_target(base_url: &str, admin_token: &str, id: &str) -> Result<Value, String> {
    rpc(base_url, "DistSync/DeleteTarget", admin_token, &serde_json::json!({"id": id})).await
}

#[derive(Serialize, Default)]
struct DisasterFallbackBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    google_drive: Option<Value>,
}

/// `DistSync/SetDisasterFallback` — メール宛先を設定
/// (「スキップ」の場合はこの関数自体を呼ばない、非ブロッキング設計)。
pub async fn set_disaster_fallback_email(
    base_url: &str,
    admin_token: &str,
    smtp_host: &str,
    smtp_port: u16,
    smtp_username: &str,
    smtp_password_env: &str,
    from_address: &str,
    to_address: &str,
) -> Result<Value, String> {
    let email = serde_json::json!({
        "smtp_host": smtp_host,
        "smtp_port": smtp_port,
        "smtp_username": smtp_username,
        "smtp_password_env": smtp_password_env,
        "from_address": from_address,
        "to_address": to_address,
        "allow_plaintext_for_testing": false,
    });
    rpc(base_url, "DistSync/SetDisasterFallback", admin_token, &DisasterFallbackBody { email: Some(email), google_drive: None })
    .await
}

/// `DistSync/SetDisasterFallback` — Googleドライブ宛先を設定。
/// **正直な開示**: サーバー側APIは実装済みだが、ウィザードUI
/// (`setup_wizard_ui.rs`)側はこのパスでは時間の制約によりメール退避先の
/// フォームのみ配線した(次回課題)。この関数自体はAPIクライアントとして
/// 完成しており、UI配線を追加すればすぐ使える。
#[allow(dead_code)]
pub async fn set_disaster_fallback_google_drive(
    base_url: &str,
    admin_token: &str,
    backup_folder_name: &str,
    client_id_env: &str,
    client_secret_env: &str,
    refresh_token_env: &str,
) -> Result<Value, String> {
    let google_drive = serde_json::json!({
        "backup_folder_name": backup_folder_name,
        "client_id_env": client_id_env,
        "client_secret_env": client_secret_env,
        "refresh_token_env": refresh_token_env,
    });
    rpc(base_url, "DistSync/SetDisasterFallback", admin_token, &DisasterFallbackBody { email: None, google_drive: Some(google_drive) })
    .await
}

/// `DistSync/FirstTimeSetup` — 登録済みの全同期先/退避先へ
/// 疎通確認(`ensure_ready`)を試み、結果レポートを返す。
pub async fn run_first_time_setup(base_url: &str, admin_token: &str) -> Result<Value, String> {
    rpc_empty(base_url, "DistSync/FirstTimeSetup", admin_token).await
}
