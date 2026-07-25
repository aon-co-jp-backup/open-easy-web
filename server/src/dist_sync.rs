//! 分散同期(distributed sync)+ディザスタ復旧のBUILT-IN機能(2026-07-25新設)。
//!
//! ユーザー要望: (1) 他のVPSへローカルDBを継続的に自動レプリケーションする
//! 「簡単設定の分散クローンDB」機能、(2) ネットワーク切断・緊急時に
//! メール/Googleドライブへ自動フェイルオーバー(自動圧縮/自動展開込み)、
//! (3) CPU/GPU/NPUのハードウェアアクセラレーション。
//!
//! **再利用方針(CLAUDE.md指示どおり、車輪の再発明をしない)**: 切断耐性
//! ジャーナル・再接続時自動復旧・退避先(Email/Googleドライブ/SFTP)・
//! 圧縮アクセラレーションは、姉妹リポジトリ`open-raid-z`が実装・テスト
//! 済みの`open_raid_z_core`(`journal`/`disaster_recovery`/`offsite_backup`/
//! `accel`)をpath依存として再利用し、このファイルでは**再実装しない**。
//! このモジュールが新規に持つのは、(a) VPS同期先の登録・削除・一覧という
//! 「open-easy-web固有の管理API」(`appserver_registration.rs`と同じ
//! `TenantRegistry`的パターン)、(b) 登録された同期先を
//! `open_raid_z_core::offsite_backup::SftpBackupTarget`
//! (VPSへのSFTP経由レプリケーション先。「VPSへの分散同期」も
//! 「SFTPオフサイト退避」も同じ`SftpBackupTarget`で表現できるため、
//! 要求どおり両者は同一の抽象を共有する)へマッピングし
//! `DisasterRecoveryManager`を構築する橋渡し、の2点のみ。
//!
//! **正直な開示**: (1) 実際にこのサーバーが管理するデータ(アップロード
//! 済みサイトファイル等)を`DisasterRecoveryManager::protect_write`経由で
//! 保護する配線までは、このパスでは行っていない
//! (`upload.rs`/`main.rs`のファイル書き込み経路を変更するのは影響範囲が
//! 大きいため——今回は「同期先の登録・退避先の設定・再接続時自動復旧の
//! 起動」という管理APIとバックグラウンドループの土台を提供する)。
//! (2) GPU/NPU圧縮は`open_raid_z_core::accel::AccelBackend`がそのまま
//! 常にCPUへ安全にフォールバックする(open-raid-z側で2026-07時点の
//! 既知の制約として文書化済み、このリポジトリ側で新たに実装したものでは
//! ない)。(3) 実クラウドアカウント・実VPSへの結合テストはこのパスでも
//! 行っていない(open-raid-z側の`tests/offsite_backup_integration.rs`と
//! 同じ「ローカルモックのみ」方針を踏襲、下記テスト参照)。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use open_raid_z_core::accel::AccelBackend;
use open_raid_z_core::disaster_recovery::{DisasterRecoveryConfig, DisasterRecoveryManager, FirstTimeSetupReport};
use open_raid_z_core::offsite_backup::{EmailBackupTargetConfig, GoogleDriveBackupTargetConfig, SftpBackupTargetConfig};

type BoxBody = Full<Bytes>;

/// `POST /admin/dist-sync/targets`のリクエストボディ。
/// VPS同期先(=SFTP経由でジャーナルセグメントを複製する宛先)を登録する。
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterTargetRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    /// SFTPパスワードを保持する環境変数名(値そのものはリクエストに含めない
    /// ——このエコシステムの既存方針どおり、秘密情報は環境変数経由)。
    pub password_env: String,
    /// このVPS上でジャーナルセグメントを退避するディレクトリ。
    pub remote_backup_dir: String,
    /// 任意のラベル(管理画面での表示用)。
    #[serde(default)]
    pub label: Option<String>,
}

/// 登録済みVPS同期先(レスポンス表示用、パスワード環境変数名は伏せない
/// ——値そのものではなく変数名なので秘密情報の漏洩にはならない)。
#[derive(Debug, Clone, Serialize)]
pub struct DistSyncTarget {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub remote_backup_dir: String,
    pub label: Option<String>,
}

/// ディザスタ用オフサイト退避先(Email/Googleドライブ/スキップ)の設定。
#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct DisasterFallbackRequest {
    #[serde(default)]
    pub email: Option<EmailBackupTargetConfig>,
    #[serde(default)]
    pub google_drive: Option<GoogleDriveBackupTargetConfig>,
}

/// 分散同期先+ディザスタ退避先のレジストリ(`appserver_registration.rs`の
/// `TenantRegistry`的パターン——`RwLock`で実行時更新できる、単純な
/// メモリ常駐registry)。
pub struct DistSyncRegistry {
    targets: RwLock<HashMap<String, RegisterTargetRequest>>,
    disaster_fallback: RwLock<DisasterFallbackRequest>,
    /// ジャーナル(WAL)の保存先ディレクトリ。
    journal_dir: PathBuf,
    /// 圧縮/展開に使うハードウェアバックエンド(既定CPU、GPU/NPUは
    /// `open_raid_z_core::accel`が安全にCPUへフォールバックする)。
    accel_backend: AccelBackend,
}

impl DistSyncRegistry {
    pub fn new(journal_dir: PathBuf) -> Self {
        Self {
            targets: RwLock::new(HashMap::new()),
            disaster_fallback: RwLock::new(DisasterFallbackRequest::default()),
            journal_dir,
            accel_backend: AccelBackend::Cpu,
        }
    }

    pub fn register(&self, req: RegisterTargetRequest) -> DistSyncTarget {
        let id = Uuid::new_v4().to_string();
        let target = DistSyncTarget {
            id: id.clone(),
            host: req.host.clone(),
            port: req.port,
            username: req.username.clone(),
            remote_backup_dir: req.remote_backup_dir.clone(),
            label: req.label.clone(),
        };
        self.targets.write().unwrap().insert(id, req);
        target
    }

    pub fn list(&self) -> Vec<DistSyncTarget> {
        self.targets
            .read()
            .unwrap()
            .iter()
            .map(|(id, req)| DistSyncTarget {
                id: id.clone(),
                host: req.host.clone(),
                port: req.port,
                username: req.username.clone(),
                remote_backup_dir: req.remote_backup_dir.clone(),
                label: req.label.clone(),
            })
            .collect()
    }

    pub fn remove(&self, id: &str) -> bool {
        self.targets.write().unwrap().remove(id).is_some()
    }

    pub fn set_disaster_fallback(&self, req: DisasterFallbackRequest) {
        *self.disaster_fallback.write().unwrap() = req;
    }

    /// 現在の登録内容から`DisasterRecoveryManager`を構築する。
    /// VPS同期先は全て`SftpBackupTarget`(要件どおり「VPSへの分散同期」と
    /// 「SFTPオフサイト退避」は同一の抽象を共有)、加えてEmail/Googleドライブの
    /// ディザスタ用退避先を設定に含める。
    pub fn build_manager(&self) -> anyhow::Result<DisasterRecoveryManager> {
        let sftp: Vec<SftpBackupTargetConfig> = self
            .targets
            .read()
            .unwrap()
            .values()
            .map(|t| SftpBackupTargetConfig {
                host: t.host.clone(),
                port: t.port,
                username: t.username.clone(),
                password_env: Some(t.password_env.clone()),
                remote_backup_dir: t.remote_backup_dir.clone(),
            })
            .collect();

        let fallback = self.disaster_fallback.read().unwrap().clone();
        let config = DisasterRecoveryConfig {
            journal_dir: self.journal_dir.clone(),
            fallback_dirs: Vec::new(),
            auto_recover_on_reconnect: true,
            cleanup_offsite_after_replay: true,
            auto_recovery_throttle_ms: 500,
            accel_backend: self.accel_backend,
            email: fallback.email.into_iter().collect(),
            google_drive: fallback.google_drive.into_iter().collect(),
            sftp,
        };
        Ok(DisasterRecoveryManager::new(config)?)
    }

    /// 初回セットアップ(ウィザード完了時、またはサーバー起動時)に呼び、
    /// 全同期先/退避先へ`ensure_ready`を試みる。個々の失敗は
    /// スキップとして扱われ、使用開始自体は妨げない
    /// (`open_raid_z_core::disaster_recovery`の既存方針どおり)。
    pub fn run_first_time_setup(&self) -> anyhow::Result<FirstTimeSetupReport> {
        Ok(self.build_manager()?.run_first_time_setup())
    }
}

fn json_response(status: StatusCode, value: &impl serde::Serialize) -> Response<BoxBody> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .expect("static response headers are always valid")
}

fn error_response(status: StatusCode, message: impl Into<String>) -> Response<BoxBody> {
    json_response(status, &serde_json::json!({ "error": message.into() }))
}

/// `x-admin-token`ヘッダによる簡易認証(`appserver_registration.rs`が
/// 呼び出し先へ送るのと同じヘッダ名——このサーバー自身が受け取る側になる
/// 場合も同じ慣習に揃える)。`OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN`が
/// 未設定の場合、この管理APIは無効化する(意図せず無認証で公開しない
/// ための安全側デフォルト)。
pub fn require_admin_token(req: &Request<Incoming>) -> Result<(), Response<BoxBody>> {
    let Ok(expected) = std::env::var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN") else {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "dist-sync admin API is disabled (OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN not set) / \
             分散同期の管理APIは無効です(OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN未設定)",
        ));
    };
    let provided = req.headers().get("x-admin-token").and_then(|v| v.to_str().ok());
    if provided == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(error_response(StatusCode::UNAUTHORIZED, "invalid or missing x-admin-token"))
    }
}

/// `POST /admin/dist-sync/targets`
pub async fn register_target(
    registry: &Arc<DistSyncRegistry>,
    req: Request<Incoming>,
) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: RegisterTargetRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };
    let target = registry.register(payload);
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "target": target,
            "message_ja": "分散同期先を登録しました。次回の自動同期から反映されます。",
            "message_en": "Distributed sync target registered. It will be included starting from the next automatic sync.",
        }),
    )
}

/// `GET /admin/dist-sync/targets`
pub async fn list_targets(registry: &Arc<DistSyncRegistry>) -> Response<BoxBody> {
    json_response(StatusCode::OK, &serde_json::json!({ "targets": registry.list() }))
}

/// `DELETE /admin/dist-sync/targets/:id`
pub async fn delete_target(registry: &Arc<DistSyncRegistry>, id: &str) -> Response<BoxBody> {
    if registry.remove(id) {
        json_response(
            StatusCode::OK,
            &serde_json::json!({
                "message_ja": "分散同期先を削除しました。",
                "message_en": "Distributed sync target removed.",
            }),
        )
    } else {
        error_response(StatusCode::NOT_FOUND, "unknown target id")
    }
}

/// `POST /admin/dist-sync/disaster-fallback` — ディザスタ用オフサイト退避先
/// (Email/Googleドライブ/どちらもスキップ)を設定する。
pub async fn set_disaster_fallback(
    registry: &Arc<DistSyncRegistry>,
    req: Request<Incoming>,
) -> Response<BoxBody> {
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("failed to read body: {e}")),
    };
    let payload: DisasterFallbackRequest = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };
    registry.set_disaster_fallback(payload);
    json_response(
        StatusCode::OK,
        &serde_json::json!({
            "message_ja": "ディザスタ用退避先を設定しました。",
            "message_en": "Disaster fallback destination configured.",
        }),
    )
}

/// `POST /admin/dist-sync/first-time-setup` — ウィザード完了時に呼び、
/// 登録済みの全同期先/退避先へ`ensure_ready`を試みる(失敗はスキップ扱い、
/// 使用開始自体は妨げない)。
pub async fn run_first_time_setup(registry: &Arc<DistSyncRegistry>) -> Response<BoxBody> {
    match registry.run_first_time_setup() {
        Ok(report) => json_response(StatusCode::OK, &report),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, format!("first-time setup failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_list_and_remove_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let registry = DistSyncRegistry::new(tmp.path().join("journal"));

        let target = registry.register(RegisterTargetRequest {
            host: "vps1.example.tokyo".into(),
            port: 22,
            username: "sync-user".into(),
            password_env: "EASYWEB_VPS1_SFTP_PASSWORD".into(),
            remote_backup_dir: "/home/sync-user/easyweb-sync".into(),
            label: Some("VPS1".into()),
        });

        let listed = registry.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, target.id);
        assert_eq!(listed[0].host, "vps1.example.tokyo");

        assert!(registry.remove(&target.id));
        assert!(registry.list().is_empty());
        // 存在しないIDの削除は false を返す(冪等な失敗、panicしない)。
        assert!(!registry.remove(&target.id));
    }

    #[test]
    fn build_manager_maps_registered_targets_to_sftp_backup_targets() {
        let tmp = tempfile::tempdir().unwrap();
        let registry = DistSyncRegistry::new(tmp.path().join("journal"));
        registry.register(RegisterTargetRequest {
            host: "vps2.example.tokyo".into(),
            port: 2222,
            username: "sync-user2".into(),
            password_env: "EASYWEB_VPS2_SFTP_PASSWORD".into(),
            remote_backup_dir: "/backup/easyweb".into(),
            label: None,
        });

        // 実SFTP接続は行わない(構築できること自体の確認)。
        let manager = registry.build_manager().expect("manager should build from registered targets");
        // 退避先が1件も"準備完了"にならないこと(パスワード環境変数未設定+
        // 到達不能ホストのため)を確認しつつ、パニックしないことを検証する。
        let report = manager.run_first_time_setup();
        assert!(!report.any_offsite_target_ready);
        assert_eq!(report.outcomes.len(), 1);
    }

    #[test]
    fn disaster_fallback_defaults_to_no_targets_when_unset() {
        let tmp = tempfile::tempdir().unwrap();
        let registry = DistSyncRegistry::new(tmp.path().join("journal"));
        let manager = registry.build_manager().unwrap();
        let report = manager.run_first_time_setup();
        // ディザスタ退避先を1件も設定していない場合、"退避先なし"の
        // 正直な報告が1件だけ返る(ローカル完結フォールバックのみで運用)。
        assert_eq!(report.outcomes.len(), 1);
        assert!(!report.any_offsite_target_ready);
    }
}
