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
//! **正直な開示**: (1) 2026-07-26に`protect_site_write`を新設し、
//! `main.rs`の`upload_files`から実際のサイトファイル書き込みを
//! `DisasterRecoveryManager::protect_write`(切断耐性ジャーナル)経由に
//! 配線した(ジャーナルのfsync+`std::fs::write`はブロッキングI/Oのため
//! `tokio::task::spawn_blocking`へ退避)。書き込み成功後は登録済みの
//! Email/Googleドライブ退避先へベストエフォートで複製される
//! (`best_effort_offsite_backup`)。ただし実クラウドアカウント(実Email
//! SMTP・実Googleドライブ)への結合テストは依然として行っていない
//! ——下記テストもローカルモックのみ。
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
use open_raid_z_core::error::BridgeError;
use open_raid_z_core::offsite_backup::{
    EmailBackupTargetConfig, GoogleDriveBackupTargetConfig, OffsiteBackupTarget, SftpBackupTarget, SftpBackupTargetConfig,
};

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
            .iter()
            .map(|(id, t)| SftpBackupTargetConfig {
                host: t.host.clone(),
                port: t.port,
                username: t.username.clone(),
                password_env: Some(t.password_env.clone()),
                remote_backup_dir: t.remote_backup_dir.clone(),
                // TOFU方式のホスト鍵検証(2026-07-26新設): 分散同期先ごとに
                // 独立したknown_hosts風ファイルをjournal_dir配下へ持たせる
                // (VPS間で鍵を混同しないよう、同期先idをファイル名に含める)。
                known_hosts_path: Some(self.journal_dir.join("dist-sync-known-hosts").join(format!("{id}.txt"))),
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

    /// 登録済みVPS同期先が1件でもあるか(実サイトファイル書き込み経路が
    /// 複製処理を起動するかどうかの判定に使う——0件なら複製処理自体を
    /// 一切スケジュールしない、既存動作を完全に保つための早期リターン)。
    pub fn has_sync_targets(&self) -> bool {
        !self.targets.read().unwrap().is_empty()
    }

    /// 登録済みVPS同期先を`SftpBackupTargetConfig`として複製する
    /// (`build_manager`と同じマッピング、複製処理側から個別に使うため公開)。
    fn sftp_target_configs(&self) -> Vec<SftpBackupTargetConfig> {
        self.targets
            .read()
            .unwrap()
            .iter()
            .map(|(id, t)| SftpBackupTargetConfig {
                host: t.host.clone(),
                port: t.port,
                username: t.username.clone(),
                password_env: Some(t.password_env.clone()),
                remote_backup_dir: t.remote_backup_dir.clone(),
                // TOFU方式のホスト鍵検証(2026-07-26新設): 分散同期先ごとに
                // 独立したknown_hosts風ファイルをjournal_dir配下へ持たせる
                // (VPS間で鍵を混同しないよう、同期先idをファイル名に含める)。
                known_hosts_path: Some(self.journal_dir.join("dist-sync-known-hosts").join(format!("{id}.txt"))),
            })
            .collect()
    }
}

/// 実際に書き込まれたサイトファイル1件を、登録済みのVPS同期先
/// (`SftpBackupTarget`)へ複製する。**この関数自体は呼び出し元の処理
/// (アップロードのHTTPレスポンス)を一切ブロックしない**——呼び出し側は
/// `tokio::spawn`でこの関数の実行そのものをデタッチすること
/// (`main.rs`の`upload_files`参照)。
///
/// - 同期先が1件も登録されていない場合は**何もしない**(早期リターン、
///   `has_sync_targets()`で判定)——未設定時は既存動作から一切変化しない
///   ことを保証する設計。
/// - `SftpBackupTarget::upload_segment`はブロッキングI/O(内部で専用の
///   使い捨てtokioランタイムを起動する同期関数)のため、
///   `tokio::task::spawn_blocking`へ退避してから呼ぶ
///   (非同期ランタイムのワーカースレッドを塞がないため)。
/// - 個々の同期先への複製が失敗しても他の同期先への複製は継続する
///   (1つの不達VPSが他の同期を道連れにしない)。失敗は`tracing::warn!`で
///   ログするのみで、アップロード自体の成否には一切影響しない
///   (ユーザーがアップロードした時点で書き込みは既に成功しているため)。
pub async fn replicate_written_file(registry: &Arc<DistSyncRegistry>, label: String, data: Bytes) {
    if !registry.has_sync_targets() {
        return;
    }
    let configs = registry.sftp_target_configs();
    for config in configs {
        let label_for_task = label.clone();
        let data_for_task = data.clone();
        let target_desc = format!("{}@{}:{}", config.username, config.host, config.remote_backup_dir);
        let result = tokio::task::spawn_blocking(move || {
            let target = SftpBackupTarget::new(config);
            target.upload_segment(&label_for_task, &data_for_task)
        })
        .await;
        match result {
            Ok(Ok(())) => {
                tracing::info!("dist-sync: replicated {label} to {target_desc}");
            }
            Ok(Err(e)) => {
                tracing::warn!("dist-sync: failed to replicate {label} to {target_desc}: {e}");
            }
            Err(e) => {
                tracing::warn!("dist-sync: replication task for {label} to {target_desc} panicked: {e}");
            }
        }
    }
}

/// 実際にアップロードされたサイトファイルの書き込みを
/// `DisasterRecoveryManager::protect_write`(切断耐性ジャーナル)経由で
/// 保護する。`apply`には実際のファイル書き込み(`std::fs::write`)を渡す
/// ——本体への書き込みが電源断/ディスク切断で失敗しても、ジャーナル自体は
/// 既にディスク上へ永続化済みのため、再起動後の`replay_pending`で復旧
/// できる(`open_raid_z_core::disaster_recovery`の既存契約どおり)。
///
/// `dataset`には`"{site}/{相対パス}"`を渡す(ジャーナルエントリの識別に
/// 使うだけで、実際の書き込み先パスは`dest`が持つ)。
///
/// ジャーナルのfsyncと`std::fs::write`はいずれもブロッキングI/Oのため、
/// 呼び出し側は`tokio::task::spawn_blocking`へ退避してから呼ぶこと
/// (`main.rs`の`upload_files`参照)。
pub fn protect_site_write(registry: &DistSyncRegistry, dataset: &str, data: Vec<u8>, dest: PathBuf) -> anyhow::Result<()> {
    let manager = registry.build_manager()?;
    manager.protect_write(dataset, 0, data, |bytes| {
        std::fs::write(&dest, bytes)
            .map_err(|e| BridgeError::JournalFailed(format!("site file write failed for {dest:?}: {e}")))
    })?;
    Ok(())
}

/// アップロード完了後に呼ぶ、非ブロッキングの複製起動ヘルパー。
/// `tokio::spawn`で[`replicate_written_file`]をデタッチ実行するため、
/// 呼び出し側(`upload_files`ハンドラ)はこの関数を呼んでも一切待たされない
/// ——遅い/到達不能な同期先があっても、アップロードしたユーザーへの
/// レスポンスが遅延することはない。
pub fn spawn_replication(registry: &Arc<DistSyncRegistry>, label: String, data: Bytes) {
    if !registry.has_sync_targets() {
        return;
    }
    let registry = Arc::clone(registry);
    tokio::spawn(async move {
        replicate_written_file(&registry, label, data).await;
    });
}

/// 起動時に呼ぶ、未committedなジャーナルエントリの自動リプレイ。
/// 前回の終了(クラッシュ・電源断・ディスク切断)が`protect_write`の
/// `apply`失敗時に発生していた場合、ジャーナルには残っているが本体
/// (webroot上の実ファイル)には反映されていない可能性のある書き込みを
/// 再生する。`entry.dataset`は`protect_site_write`が
/// `"{site}/{サイト内相対パス}"`の形式で書き込んでいるため、その形式を
/// 前提に`sites_root`からの絶対パスへ復元する
/// (`entry.dataset`にスラッシュが1つも無い壊れたエントリはスキップし
/// `tracing::warn!`で記録するのみ、パニックしない)。
///
/// ジャーナルのfsync・実ファイル書き込みはいずれもブロッキングI/Oのため、
/// 呼び出し側は`tokio::task::spawn_blocking`へ退避してから呼ぶこと
/// (`main.rs`の起動シーケンス参照)。
pub fn replay_pending_writes(registry: &DistSyncRegistry, sites_root: &std::path::Path) -> anyhow::Result<usize> {
    let manager = registry.build_manager()?;
    let sites_root = sites_root.to_path_buf();
    let replayed = manager.replay_local_journal(|entry| {
        let Some((site, rel)) = entry.dataset.split_once('/') else {
            tracing::warn!(dataset = %entry.dataset, "起動時ジャーナルリプレイ: datasetの形式が不正なためスキップします");
            return Ok(());
        };
        let Some(safe_site) = crate::upload::safe_relative_path(site) else {
            tracing::warn!(dataset = %entry.dataset, "起動時ジャーナルリプレイ: サイト名が不正なためスキップします");
            return Ok(());
        };
        let Some(safe_rel) = crate::upload::safe_relative_path(rel) else {
            tracing::warn!(dataset = %entry.dataset, "起動時ジャーナルリプレイ: 相対パスが不正なためスキップします");
            return Ok(());
        };
        let dest = sites_root.join(safe_site).join(safe_rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| BridgeError::JournalFailed(format!("リプレイ先ディレクトリ作成失敗 {dest:?}: {e}")))?;
        }
        std::fs::write(&dest, &entry.data).map_err(|e| BridgeError::JournalFailed(format!("リプレイ書き込み失敗 {dest:?}: {e}")))?;
        tracing::info!(dataset = %entry.dataset, "起動時ジャーナルリプレイ: 未反映だった書き込みを復元しました");
        Ok(())
    })?;
    if replayed > 0 {
        tracing::info!(count = replayed, "起動時ジャーナルリプレイ完了");
    }
    Ok(replayed)
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
///
/// **2026-08-01追記(実バグ修正)**: この関数は分散同期(dist-sync)専用
/// ではなく、システムメモリ・電源プロファイル・自動アップデート等
/// 全ての`/admin/*`管理APIで共有されている単一のゲートだが、無効時の
/// エラー文言が「dist-sync admin API is disabled」と分散同期専用の
/// 表現に固定されていたため、メモリ使用状況の「更新」ボタンのような
/// 無関係な機能を押しただけのユーザーにも紛らわしい分散同期関連の
/// エラーが表示されてしまっていた(ユーザー報告で発覚)。全管理API
/// 共通の汎用的な文言に変更。
pub fn require_admin_token(req: &Request<Incoming>) -> Result<(), Response<BoxBody>> {
    let Ok(expected) = std::env::var("OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN") else {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "admin API is disabled on this server (OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN not set) / \
             このサーバーでは管理API機能が無効です(OPEN_EASYWEB_DIST_SYNC_ADMIN_TOKEN未設定)",
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

    #[tokio::test]
    async fn spawn_replication_is_a_no_op_when_no_targets_are_registered() {
        // 実サイトファイル書き込み経路への配線が「同期先未設定なら
        // 既存動作から一切変化しない」ことの回帰テスト——`has_sync_targets()`
        // がfalseの間は、`spawn_replication`がtokio::spawn自体を行わない
        // (=バックグラウンドタスクが一切生成されない)ことを確認する。
        let tmp = tempfile::tempdir().unwrap();
        let registry = Arc::new(DistSyncRegistry::new(tmp.path().join("journal")));
        assert!(!registry.has_sync_targets());

        // パニックしない・エラーを返さないことのみが契約(戻り値は無い)。
        spawn_replication(&registry, "example.tokyo/index.html".to_string(), Bytes::from_static(b"<html></html>"));
        // デタッチされたタスクが仮にスケジュールされていたとしても、
        // 同期先が無い場合はreplicate_written_file内で即座にreturnする
        // ため、少し待っても登録状態自体は変化しない(副作用が無いことの
        // 間接的な確認)。
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(!registry.has_sync_targets());
    }

    #[test]
    fn protect_site_write_writes_file_and_commits_journal_entry() {
        // 実際に`protect_site_write`(main.rsのupload_filesが呼ぶ配線経路)
        // を通した書き込みが、(a) 実ファイルへ実際に反映され、(b) ジャーナル
        // エントリがcommitted扱い(=pendingディレクトリから削除済み)に
        // なることを実際のファイルシステム上で確認する(型チェックのみに
        // 留めない)。
        let tmp = tempfile::tempdir().unwrap();
        let journal_dir = tmp.path().join("journal");
        let registry = DistSyncRegistry::new(journal_dir.clone());

        let dest = tmp.path().join("webroot").join("example.tokyo").join("index.html");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();

        protect_site_write(&registry, "example.tokyo/index.html", b"<html>hello</html>".to_vec(), dest.clone())
            .expect("protect_site_write should succeed when the destination is writable");

        let written = std::fs::read(&dest).expect("file should have been written to disk");
        assert_eq!(written, b"<html>hello</html>");

        let pending_dir = journal_dir.join("pending");
        let remaining: Vec<_> = std::fs::read_dir(&pending_dir).unwrap().collect();
        assert!(remaining.is_empty(), "journal entry should be marked committed (removed from pending) after a successful write");
    }

    #[test]
    fn protect_site_write_keeps_journal_entry_pending_when_apply_fails() {
        // 本体への書き込みが失敗した場合(ここでは親ディレクトリが存在しない
        // 無効な書き込み先で意図的に失敗させる、電源断/ディスク切断の代替)、
        // ジャーナルエントリは committed にならず pending のまま残ること
        // (=再接続後のリプレイ対象として復旧可能であること)を確認する。
        let tmp = tempfile::tempdir().unwrap();
        let journal_dir = tmp.path().join("journal");
        let registry = DistSyncRegistry::new(journal_dir.clone());

        // 親ディレクトリを作らない、存在しないネスト先 = std::fs::write が失敗する。
        let dest = tmp.path().join("no-such-parent-dir").join("index.html");

        let result = protect_site_write(&registry, "example.tokyo/index.html", b"<html>hello</html>".to_vec(), dest.clone());
        assert!(result.is_err(), "protect_site_write should surface the underlying write failure to the caller");
        assert!(!dest.exists(), "the file should not exist since the underlying write failed");

        let pending_dir = journal_dir.join("pending");
        let remaining: Vec<_> = std::fs::read_dir(&pending_dir).unwrap().collect();
        assert_eq!(remaining.len(), 1, "the journal entry should remain pending (not committed) so it can be replayed after reconnect");
    }

    #[test]
    fn replay_pending_writes_restores_uncommitted_entry_after_simulated_crash() {
        // 「クラッシュ時にジャーナルへは記録されたが本体への書き込みが
        // まだ行われていない」状態を、`protect_write`のapplyクロージャで
        // わざと失敗させることで再現し(電源断/ディスク切断の代替)、
        // その後`replay_pending_writes`を呼んだ際に実際にファイルが
        // 復元されることを確認する(main.rsの起動時配線と同じ経路)。
        let tmp = tempfile::tempdir().unwrap();
        let journal_dir = tmp.path().join("journal");
        let sites_root = tmp.path().join("sites");
        std::fs::create_dir_all(&sites_root).unwrap();
        let registry = DistSyncRegistry::new(journal_dir.clone());

        // apply失敗を意図的に起こす(存在しない親ディレクトリの下へ書く)。
        let bogus_dest = tmp.path().join("no-such-parent").join("index.html");
        let manager = registry.build_manager().unwrap();
        let result = manager.protect_write("example.tokyo/index.html", 0, b"<html>restored</html>".to_vec(), |bytes| {
            std::fs::write(&bogus_dest, bytes).map_err(|e| BridgeError::JournalFailed(e.to_string()))
        });
        assert!(result.is_err(), "the simulated crash write should fail");

        // ジャーナルにはまだpendingエントリが残っているはず。
        let pending_dir = journal_dir.join("pending");
        assert_eq!(std::fs::read_dir(&pending_dir).unwrap().count(), 1);

        // サーバー再起動を模して、新しいregistryインスタンス(同じjournal_dir)
        // から`replay_pending_writes`を呼ぶ。今度は正しい`sites_root`配下へ
        // 実際に復元されることを確認する。
        let restarted_registry = DistSyncRegistry::new(journal_dir.clone());
        let replayed = replay_pending_writes(&restarted_registry, &sites_root).expect("replay should succeed");
        assert_eq!(replayed, 1);

        let restored = std::fs::read(sites_root.join("example.tokyo").join("index.html"))
            .expect("the file should have been restored under sites_root/example.tokyo/index.html");
        assert_eq!(restored, b"<html>restored</html>");

        // リプレイ後、ジャーナルのpendingは空になっている(committed済み)。
        assert_eq!(std::fs::read_dir(&pending_dir).unwrap().count(), 0);
    }

    /// `open_raid_z_core`側`tests/offsite_backup_integration.rs`と同じ
    /// パターン(インプロセスrussh SFTPサーバー、実クラウド/実VPSには
    /// 一切接続しない)を、このリポジトリのdist_sync複製経路向けに
    /// 再利用した最小限のフェイクSFTPサーバー。
    mod fake_sftp_server {
        use russh::keys::{Algorithm, PrivateKey};
        use russh::server::{Auth, ChannelOpenHandle, Msg, Server as _, Session};
        use russh::{Channel, ChannelId};
        use russh_sftp::protocol::{Attrs, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version};
        use std::collections::HashMap as StdHashMap;
        use std::path::PathBuf;
        use std::sync::Arc;
        use tokio::io::{AsyncSeekExt, AsyncWriteExt};
        use tokio::sync::Mutex;

        #[derive(Clone)]
        pub struct FakeServer {
            pub root: PathBuf,
        }

        impl russh::server::Server for FakeServer {
            type Handler = FakeSshSession;

            fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self::Handler {
                FakeSshSession { root: self.root.clone(), clients: Arc::new(Mutex::new(StdHashMap::new())) }
            }
        }

        pub struct FakeSshSession {
            root: PathBuf,
            clients: Arc<Mutex<StdHashMap<ChannelId, Channel<Msg>>>>,
        }

        impl russh::server::Handler for FakeSshSession {
            type Error = anyhow::Error;

            async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<Auth, Self::Error> {
                Ok(Auth::Accept)
            }

            async fn channel_open_session(
                &mut self,
                channel: Channel<Msg>,
                reply: ChannelOpenHandle,
                _session: &mut Session,
            ) -> Result<(), Self::Error> {
                self.clients.lock().await.insert(channel.id(), channel);
                reply.accept().await;
                Ok(())
            }

            async fn subsystem_request(&mut self, channel_id: ChannelId, name: &str, session: &mut Session) -> Result<(), Self::Error> {
                if name == "sftp" {
                    let channel = self.clients.lock().await.remove(&channel_id).unwrap();
                    let sftp = FakeSftpSession { root: self.root.clone(), files: StdHashMap::new() };
                    session.channel_success(channel_id)?;
                    russh_sftp::server::run(channel.into_stream(), sftp).await;
                } else {
                    session.channel_failure(channel_id)?;
                }
                Ok(())
            }
        }

        struct FakeSftpSession {
            root: PathBuf,
            files: StdHashMap<String, tokio::fs::File>,
        }

        impl FakeSftpSession {
            fn resolve(&self, path: &str) -> PathBuf {
                self.root.join(path.trim_start_matches('/'))
            }
        }

        impl russh_sftp::server::Handler for FakeSftpSession {
            type Error = StatusCode;

            fn unimplemented(&self) -> Self::Error {
                StatusCode::OpUnsupported
            }

            async fn init(&mut self, _version: u32, _extensions: StdHashMap<String, String>) -> Result<Version, Self::Error> {
                Ok(Version::new())
            }

            async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
                Ok(Name { id, files: vec![File::dummy(&path)] })
            }

            async fn mkdir(&mut self, id: u32, path: String, _attrs: FileAttributes) -> Result<Status, Self::Error> {
                let resolved = self.resolve(&path);
                std::fs::create_dir_all(&resolved).map_err(|_| StatusCode::Failure)?;
                Ok(ok_status(id))
            }

            async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
                let resolved = self.resolve(&path);
                if resolved.exists() {
                    Ok(Attrs { id, attrs: FileAttributes::default() })
                } else {
                    Err(StatusCode::NoSuchFile)
                }
            }

            async fn open(&mut self, id: u32, filename: String, _pflags: OpenFlags, _attrs: FileAttributes) -> Result<Handle, Self::Error> {
                let resolved = self.resolve(&filename);
                if let Some(parent) = resolved.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&resolved)
                    .await
                    .map_err(|_| StatusCode::Failure)?;
                self.files.insert(filename.clone(), file);
                Ok(Handle { id, handle: filename })
            }

            async fn write(&mut self, id: u32, handle: String, offset: u64, data: Vec<u8>) -> Result<Status, Self::Error> {
                let file = self.files.get_mut(&handle).ok_or(StatusCode::Failure)?;
                file.seek(std::io::SeekFrom::Start(offset)).await.map_err(|_| StatusCode::Failure)?;
                file.write_all(&data).await.map_err(|_| StatusCode::Failure)?;
                Ok(ok_status(id))
            }

            async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
                self.files.remove(&handle);
                Ok(ok_status(id))
            }
        }

        fn ok_status(id: u32) -> Status {
            Status { id, status_code: StatusCode::Ok, error_message: "Ok".to_string(), language_tag: "en-US".to_string() }
        }

        pub async fn spawn(root: PathBuf) -> u16 {
            let config = russh::server::Config {
                keys: vec![PrivateKey::random(&mut rand_for_test_keys::rng(), Algorithm::Ed25519).unwrap()],
                ..Default::default()
            };
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let mut server = FakeServer { root };

            tokio::spawn(async move {
                let _ = server.run_on_socket(Arc::new(config), &listener).await;
            });

            port
        }
    }

    #[tokio::test]
    async fn spawn_replication_actually_uploads_written_file_to_registered_mock_sftp_target() {
        // 「登録済みの同期先があれば、実ファイル書き込みが実際に複製先へ
        // 届く」ことを、実VPSではなくローカルのインプロセスrussh SFTP
        // サーバー(モック)で検証する——`open_raid_z_core`側の既存検証
        // 方針(`tests/offsite_backup_integration.rs`)をそのまま踏襲。
        let sftp_root = tempfile::tempdir().unwrap();
        let port = fake_sftp_server::spawn(sftp_root.path().to_path_buf()).await;
        // サーバーのacceptループが立ち上がるまで少し待つ。
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let journal_tmp = tempfile::tempdir().unwrap();
        let registry = Arc::new(DistSyncRegistry::new(journal_tmp.path().join("journal")));

        std::env::set_var("TEST_EASYWEB_DIST_SYNC_SFTP_PASSWORD", "unit-test-password");
        registry.register(RegisterTargetRequest {
            host: "127.0.0.1".into(),
            port,
            username: "easyweb-sync".into(),
            password_env: "TEST_EASYWEB_DIST_SYNC_SFTP_PASSWORD".into(),
            remote_backup_dir: "backup".into(),
            label: Some("mock VPS".into()),
        });
        assert!(registry.has_sync_targets());

        replicate_written_file(&registry, "example.tokyo/index.html".to_string(), Bytes::from_static(b"<html>hello</html>")).await;

        // 実際にモックSFTPサーバー側のディスクへファイルが複製されたことを
        // 直接確認する(型チェック・モックの呼び出し回数確認に留めず、
        // 実際に書き込まれたバイト列そのものを検証する)。
        let replicated_path = sftp_root.path().join("backup").join("example.tokyo/index.html");
        let replicated = std::fs::read(&replicated_path).expect("replicated file should exist on the mock SFTP target");
        assert_eq!(replicated, b"<html>hello</html>");
    }
}
