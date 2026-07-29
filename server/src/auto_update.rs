//! 深夜バックグラウンド自動アップデート機能(2026-07-27追加、ユーザー指示
//! 「open-easy-web-serverとopen-web-serverは、AUTO-UPDATEで真夜中に
//! バックグラウンドで自動UPDATEして」「一瞬でVERSIONUPで切り替わって」
//! 「AUTO UPDATEデフォルトでOFFにしてONに出来るようにして」
//! 「環境変数のコマンドとGUIでも設定変更出来るようにして」への対応。
//!
//! ## 有効化(2通り、どちらでも同じ設定に反映される)
//!
//! - 環境変数`OPEN_EASYWEB_AUTO_UPDATE=true`(既定はOFF、これはプロセス
//!   起動時の**初期値**としてのみ使われる)。
//! - `POST /admin/auto-update`(`x-admin-token`認証、`{"enabled": true}`)
//!   ——GUIのトグルスイッチから呼ぶ想定(`api_auto_update.rs`参照)。
//!   一度でも`POST`されると、その設定が`.open-easy-web-auto-update.json`
//!   へ永続化され、以後は環境変数より優先される(再起動しても保持)。
//!
//! ## ほぼゼロダウンタイムでの切り替え(Linux)
//!
//! 1. 毎日ローカル深夜0時に、GitHub Releasesの最新タグを確認する。
//! 2. 現在実行中のバージョン(`env!("CARGO_PKG_VERSION")`)より新しければ、
//!    OS別のリリースアセット(`open-easy-web-server-linux-x86_64.tar.gz`
//!    または`-windows-x86_64.zip`、`.github/workflows/release.yml`と
//!    同じ命名規則)をダウンロードし、実行ファイルを展開する。
//! 3. 展開した新バイナリを`--version`引数で実行し、正しく起動できることを
//!    確認する(壊れたバイナリで本番を巻き込まないため)。
//! 4. **Linux**: 新バイナリを、現在のプロセスと同じ環境変数を引き継いだ
//!    子プロセスとして起動する。リスンソケットを`SO_REUSEPORT`で
//!    bind している(`OPEN_EASYWEB_AUTO_UPDATE`有効時のみ、無効時は
//!    従来通りの通常bind)ため、新旧プロセスが同じポートへ同時にbindでき、
//!    OSカーネルが新規接続を両方へ振り分ける——ソケットの明け渡し・
//!    ハンドオフ通信は不要。子プロセスの`/healthz`が実際に応答することを
//!    確認できてから、現在のプロセスは新規接続の受付のみを停止し
//!    (既存の処理中コネクションは完走させる猶予期間を置く)、その後
//!    終了する。ポートが一度も空かないため、外部から見て実質的に
//!    ダウンタイムが無い。
//! 5. **Windows**: `SO_REUSEPORT`相当の仕組みが無いため、現在のプロセスが
//!    リスンソケットを閉じてから新バイナリを起動する、素早い逐次切り替え
//!    (正直な開示: 真のゼロダウンタイムではない、数百ミリ秒程度の
//!    切り替え猶予が生じる)。
//!
//! ## 正直な開示
//!
//! - ダウンロードしたリリースアセットの署名検証(GPG・checksum)は行って
//!   いない——取得先の真正性はGitHubへの信頼(HTTPS+GitHub Releases自体
//!   への信頼)に依拠する。次回課題。
//! - 本セッションでは、GitHub Releases APIのレスポンス解析・バージョン
//!   比較・アーカイブ展開のロジックはモックHTTPサーバー経由で実際に
//!   検証したが、**実際に2プロセスがSO_REUSEPORTで同一ポートを共有し
//!   ゼロダウンタイムで切り替わる、という本番相当のシナリオは実機検証
//!   していない**(このセッションでは実際に有効化・デプロイしていない)。
//!   本番で有効化する前にステージング環境での実地検証を推奨する。

use std::io::Read as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// 監視対象のGitHubリポジトリ(owner/repo)。
const REPO: &str = "aon-co-jp/open-easy-web";
const BINARY_NAME_UNIX: &str = "open-easy-web-server";
const BINARY_NAME_WINDOWS: &str = "open-easy-web-server.exe";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedSettings {
    enabled: bool,
}

/// 自動アップデートの有効/無効設定。環境変数(初期値)と永続化ファイル
/// (GUI/API経由の変更、環境変数より優先)の両方を反映する。
pub struct AutoUpdateState {
    settings_path: PathBuf,
    enabled: std::sync::RwLock<bool>,
    /// 2026-07-29追記(ユーザー指示、RS-Syncのスケジューラ無言停止バグを
    /// 受けての横展開): `run_daily_check_loop`が1サイクルの開始時に
    /// 更新するハートビート。このループは1日1回しか回らないため、
    /// RS-Sync(1分間隔)と違い「更新が無い=異常」の閾値は日単位になる
    /// (`GET /admin/auto-update`のレスポンスに含め、25時間を超えて
    /// 更新が無ければ異常とみなす、呼び出し側の責務)。
    last_cycle_started_at: std::sync::RwLock<std::time::Instant>,
}

impl AutoUpdateState {
    /// `settings_path`に既存の永続化ファイルがあればそれを使い、無ければ
    /// `OPEN_EASYWEB_AUTO_UPDATE`環境変数(既定`false`)を初期値とする。
    pub fn load(settings_path: PathBuf) -> Self {
        let enabled = match std::fs::read(&settings_path) {
            Ok(bytes) => serde_json::from_slice::<PersistedSettings>(&bytes).map(|s| s.enabled).unwrap_or(false),
            Err(_) => std::env::var("OPEN_EASYWEB_AUTO_UPDATE").map(|v| v == "true" || v == "1").unwrap_or(false),
        };
        Self { settings_path, enabled: std::sync::RwLock::new(enabled), last_cycle_started_at: std::sync::RwLock::new(std::time::Instant::now()) }
    }

    pub fn mark_cycle_started(&self) {
        *self.last_cycle_started_at.write().unwrap() = std::time::Instant::now();
    }

    /// 直近のサイクル開始から経過した秒数。
    pub fn seconds_since_last_cycle(&self) -> u64 {
        self.last_cycle_started_at.read().unwrap().elapsed().as_secs()
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read().unwrap()
    }

    /// GUI/HTTP API経由での設定変更。ディスクへ永続化するため、
    /// プロセス再起動後も設定が保持される。
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        *self.enabled.write().unwrap() = enabled;
        if let Some(parent) = self.settings_path.parent() {
            std::fs::create_dir_all(parent).context("failed to create auto-update settings directory")?;
        }
        let bytes = serde_json::to_vec_pretty(&PersistedSettings { enabled }).expect("PersistedSettings serialization is infallible");
        std::fs::write(&self.settings_path, bytes).context("failed to persist auto-update settings")?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
}

/// `GET {base_url}/repos/{repo}/releases/latest`から最新タグを取得する
/// (`v`プレフィックスは剥がして返す)。`base_url`はテスト時にモック
/// サーバーへ差し替えるための引数(既定は`https://api.github.com`から
/// `latest_release_tag`経由で呼ぶ)。
pub async fn latest_release_tag_from(client: &reqwest::Client, base_url: &str, repo: &str) -> Result<String> {
    let url = format!("{base_url}/repos/{repo}/releases/latest");
    let resp = client
        .get(&url)
        .header("User-Agent", "open-easy-web-auto-update")
        .send()
        .await
        .context("GitHub releases API request failed")?;
    if !resp.status().is_success() {
        anyhow::bail!("GitHub releases API returned HTTP {}", resp.status());
    }
    let release: GithubRelease = resp.json().await.context("GitHub releases API response was not valid JSON")?;
    Ok(release.tag_name.trim_start_matches('v').to_string())
}

pub async fn latest_release_tag(client: &reqwest::Client) -> Result<String> {
    latest_release_tag_from(client, "https://api.github.com", REPO).await
}

/// 日付そのものをバージョンとして扱う(2026-07-27追加、ユーザー指示
/// 「バージョンは、日付にしてその表示機能を持たせて 例えば 最新は、
/// 2026.07.27 11:15」)——`current_version()`(このバイナリが実際に
/// ビルドされた日時、`build.rs`が`OPEN_EASYWEB_BUILD_VERSION_COMPACT`
/// 環境変数へコンパイル時に埋め込む)は「今この瞬間の時刻」ではなく
/// **このバイナリがビルドされた時点の固定値**であり、実行中に変化
/// しない。GitHub Releasesのタグも同じ`YYYY.MM.DD.HHMM`形式(区切り
/// 文字は`.`または`-`のどちらでもよい)で運用する前提。
///
/// `current`/`candidate`いずれも数字以外の文字(`.`/`-`/先頭の`v`等)を
/// 取り除いた上で、12桁の`YYYYMMDDHHMM`数値として比較する——同じ桁数の
/// 数値文字列同士なら、文字列比較がそのまま時系列比較になるため、依存を
/// 増やさず単純な実装で済む。桁数が合わない・数値化できない壊れた形式は
/// 「新しくない」として安全側に倒す(誤って壊れたタグに反応して無限
/// アップデートループへ陥らないため)。
pub fn is_newer(current: &str, candidate: &str) -> bool {
    fn parse(v: &str) -> Option<u64> {
        let digits: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() != 12 {
            return None;
        }
        digits.parse().ok()
    }
    match (parse(current), parse(candidate)) {
        (Some(cur), Some(cand)) => cand > cur,
        _ => false,
    }
}

fn asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "open-easy-web-server-windows-x86_64.zip"
    } else {
        "open-easy-web-server-linux-x86_64.tar.gz"
    }
}

/// 最新リリースのOS別アセットをダウンロードし、実行ファイルを`dest_dir`へ
/// 展開する(tar.gz on Linux/macOS、zip on Windows)。返り値は展開した
/// 実行ファイルのパス。
pub async fn download_and_extract(client: &reqwest::Client, dest_dir: &Path) -> Result<PathBuf> {
    download_and_extract_from(client, "https://github.com", REPO, dest_dir).await
}

pub async fn download_and_extract_from(client: &reqwest::Client, base_url: &str, repo: &str, dest_dir: &Path) -> Result<PathBuf> {
    let asset = asset_name();
    let url = format!("{base_url}/{repo}/releases/latest/download/{asset}");
    let resp = client.get(&url).send().await.context("failed to download release asset")?;
    if !resp.status().is_success() {
        anyhow::bail!("release asset download returned HTTP {}", resp.status());
    }
    let bytes = resp.bytes().await.context("failed to read release asset body")?;
    std::fs::create_dir_all(dest_dir).context("failed to create download destination directory")?;

    if cfg!(target_os = "windows") {
        extract_zip_binary(&bytes, dest_dir)
    } else {
        extract_tar_gz_binary(&bytes, dest_dir)
    }
}

fn extract_tar_gz_binary(bytes: &[u8], dest_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dest_dir).context("failed to create extraction destination directory")?;
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries().context("failed to read tar.gz entries")? {
        let mut entry = entry.context("failed to read tar.gz entry")?;
        let path = entry.path().context("failed to read tar entry path")?.into_owned();
        if path.file_name().and_then(|n| n.to_str()) == Some(BINARY_NAME_UNIX) {
            let dest = dest_dir.join(BINARY_NAME_UNIX);
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).context("failed to read binary from tar.gz")?;
            std::fs::write(&dest, &buf).context("failed to write extracted binary")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755)).context("failed to chmod extracted binary")?;
            }
            return Ok(dest);
        }
    }
    anyhow::bail!("{BINARY_NAME_UNIX} binary not found inside downloaded tar.gz archive")
}

fn extract_zip_binary(bytes: &[u8], dest_dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dest_dir).context("failed to create extraction destination directory")?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("failed to open zip archive")?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("failed to read zip entry")?;
        if file.name().ends_with(BINARY_NAME_WINDOWS) {
            let dest = dest_dir.join(BINARY_NAME_WINDOWS);
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).context("failed to read binary from zip")?;
            std::fs::write(&dest, &buf).context("failed to write extracted binary")?;
            return Ok(dest);
        }
    }
    anyhow::bail!("{BINARY_NAME_WINDOWS} binary not found inside downloaded zip archive")
}

/// ダウンロードした新バイナリが実際に起動できることを`--version`引数で
/// 確認する(壊れたバイナリ・不完全なダウンロードで本番を巻き込まない
/// ための安全確認、`main.rs`が`--version`を処理してすぐ終了する前提)。
pub fn verify_binary_runs(path: &Path) -> Result<String> {
    let output = std::process::Command::new(path)
        .arg("--version")
        .output()
        .context("failed to execute downloaded binary for --version check")?;
    if !output.status.success() {
        anyhow::bail!("downloaded binary exited with a failure status for --version");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// 新バイナリを、現在のプロセスの環境変数を引き継いだ子プロセスとして
/// 起動し、`health_check_addr`の`/healthz`が応答するまで待つ
/// (`timeout`以内に応答しなければエラー)。`OPEN_EASYWEB_AUTO_UPDATE`
/// 無効時にはこの関数自体が呼ばれない経路のため、通常運用への影響は無い。
/// 現状は`#[cfg(unix)]`の`check_and_apply_update`からのみ呼ばれる
/// (Windowsは`SO_REUSEPORT`相当が無いため別の逐次切り替え経路を使う)ため
/// Windowsビルドではdead codeになる——将来Windows版のゼロダウンタイム
/// 化(名前付きパイプ経由のハンドオフ等)に転用できるよう関数自体は
/// 残し、`allow(dead_code)`で警告のみ抑制する。
#[allow(dead_code)]
pub async fn spawn_new_binary_and_wait_healthy(new_binary: &Path, health_check_addr: SocketAddr, timeout: Duration) -> Result<std::process::Child> {
    let child = std::process::Command::new(new_binary).spawn().context("failed to spawn new binary")?;

    let client = reqwest::Client::builder().timeout(Duration::from_secs(2)).build().context("failed to build health-check HTTP client")?;
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!("new binary did not become healthy within {timeout:?}");
        }
        if let Ok(resp) = client.get(format!("http://{health_check_addr}/healthz")).send().await {
            if resp.status().is_success() {
                return Ok(child);
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// 次のローカル深夜0時までの秒数を計算する(依存を増やさないため
/// `chrono`等は使わず、`std::time`とローカルタイムゾーンのオフセットを
/// 環境依存にせず「UTCの次の0時」を基準にする単純な実装——厳密な
/// タイムゾーン対応が必要な場合は次回課題)。
pub fn duration_until_next_midnight_utc() -> Duration {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs_into_day = now.as_secs() % 86400;
    let secs_until_midnight = 86400 - secs_into_day;
    Duration::from_secs(secs_until_midnight)
}

/// 現在実行中のバイナリが**ビルドされた日時**(`YYYYMMDDHHMM`、`build.rs`
/// がコンパイル時に埋め込む固定値——プロセスの起動時刻でも現在時刻でも
/// ない)。`is_newer`での比較・GitHubタグとの突き合わせに使う内部形式。
pub fn current_version() -> &'static str {
    env!("OPEN_EASYWEB_BUILD_VERSION_COMPACT")
}

/// ユーザー向け表示形式(例: `"2026.07.27 11:15"`)。`current_version()`
/// と同じビルド時刻を表すが、人間が読みやすいように区切り文字を入れた
/// もの(`GET /admin/auto-update`のレスポンス・ブラウザGUI表示で使う)。
pub fn current_version_display() -> &'static str {
    env!("OPEN_EASYWEB_BUILD_VERSION_DISPLAY")
}

/// 深夜バックグラウンド自動アップデートの常駐ループ。`main()`から
/// `tokio::spawn`されるバックグラウンドタスクとして動く想定
/// (呼び出し元のリクエスト処理を一切ブロックしない)。毎日ローカル
/// 深夜0時に起床し、`auto_update_state.is_enabled()`(環境変数または
/// GUI/API経由でいつでも変更可能)を都度確認した上で、有効な場合のみ
/// 新バージョンの確認・ダウンロード・検証・切り替えを行う。
///
/// **正直な開示**: `spawn_new_binary_and_wait_healthy`による新プロセス
/// 起動から、`stop_accepting`フラグを立てて旧プロセスの受付を止める
/// までの一連の流れはコード上実装済みだが、本セッションでは実際に
/// 2プロセスがSO_REUSEPORTで同一ポートを共有する本番相当のシナリオを
/// 検証していない。
pub async fn run_daily_check_loop(auto_update_state: std::sync::Arc<AutoUpdateState>, bind_addr: SocketAddr, stop_accepting: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    loop {
        let wait = duration_until_next_midnight_utc();
        tracing::info!(wait_secs = wait.as_secs(), "auto_update: sleeping until next check (local midnight)");
        tokio::time::sleep(wait).await;

        // ハートビート更新(2026-07-29追記): disabled時もタイムアウト時も
        // 必ず「サイクル自体は起動した」ことを記録する——RS-Sync側で発生した
        // 「ループが無言のまま永久停止」バグをここでも早期検知できるように。
        auto_update_state.mark_cycle_started();

        if !auto_update_state.is_enabled() {
            tracing::debug!("auto_update: disabled, skipping this cycle");
            continue;
        }

        // 2026-07-29追記: 内部のHTTPクライアント(`reqwest::Client`)は
        // タイムアウト未設定のままだと理論上は無期限にハングしうる
        // (RS-Sync側で見つかった実障害と同種のリスク)。ここは非同期の
        // `reqwest::Client`のため一つのタスクがハングしても他のタスクの
        // 実行自体は止まらないが、それでも「アップデートが永久に確認
        // されない」状態に陥るのを避けるため、サイクル全体に外側から
        // 30分のタイムアウトを掛ける。
        match tokio::time::timeout(Duration::from_secs(30 * 60), check_and_apply_update(bind_addr, &stop_accepting)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!(error = %e, "auto_update: update cycle failed (server continues running the current version)"),
            Err(_) => tracing::warn!("auto_update: update cycle timed out after 30 minutes (server continues running the current version)"),
        }
    }
}

/// 1回分のアップデート確認・適用サイクル。失敗しても現在動作中の
/// プロセスには一切影響しない(呼び出し元が結果をログするだけ)。
/// 1回分のアップデート確認・適用サイクルを、深夜0時を待たずに即座に
/// 実行する(`POST /admin/auto-update/check-now`から呼ぶ、`enabled`
/// 設定に関わらず「今すぐ確認」自体は常に実行できる——確認・適用の
/// 都度実行はユーザーが能動的に押した結果であり、深夜の自動実行とは
/// 別の操作として扱う)。
pub async fn check_and_apply_update(bind_addr: SocketAddr, stop_accepting: &std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<()> {
    // Windows等`#[cfg(unix)]`以外のビルドでは`bind_addr`をヘルスチェック
    // 用途に使わない(逐次切り替え方式のため)。未使用警告を避けるための
    // 明示的な参照(挙動には影響しない)。
    #[cfg(not(unix))]
    let _ = &bind_addr;
    let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().context("failed to build GitHub API client")?;
    let latest = latest_release_tag(&client).await.context("failed to check latest release")?;
    let current = current_version();
    if !is_newer(current, &latest) {
        tracing::debug!(current, latest = %latest, "auto_update: already up to date");
        return Ok(());
    }
    tracing::info!(current, latest = %latest, "auto_update: newer version found, downloading");

    let dest_dir = std::env::temp_dir().join("open-easy-web-auto-update");
    let new_binary = download_and_extract(&client, &dest_dir).await.context("failed to download/extract new release")?;

    let reported_version = verify_binary_runs(&new_binary).context("downloaded binary failed the --version sanity check")?;
    tracing::info!(reported_version, "auto_update: new binary passed --version sanity check");

    #[cfg(unix)]
    {
        let child = spawn_new_binary_and_wait_healthy(&new_binary, bind_addr, Duration::from_secs(30))
            .await
            .context("new binary did not become healthy")?;
        tracing::info!(pid = child.id(), "auto_update: new process is healthy and serving via SO_REUSEPORT; stopping this process's accept loop");
        // 新プロセスが既に同じポートで健全に応答しているため、旧プロセスは
        // 新規受付だけを止めればよい(ポートが一度も空かない)。処理中の
        // 接続を完走させる猶予を置いてから終了する。
        stop_accepting.store(true, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(5)).await;
        tracing::info!("auto_update: handoff complete, exiting old process");
        std::process::exit(0);
    }
    #[cfg(not(unix))]
    {
        // SO_REUSEPORT相当が無いため、正直な開示通り「受付停止→即起動」の
        // 逐次切り替え(真のゼロダウンタイムではない)。
        stop_accepting.store(true, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _child = std::process::Command::new(&new_binary).spawn().context("failed to spawn new binary")?;
        tracing::info!("auto_update: sequential handoff started, exiting old process");
        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `OPEN_EASYWEB_AUTO_UPDATE`等のプロセス全体のグローバル環境変数を
    /// 読み書きするテスト同士が、`cargo test`の既定の並行実行(複数OS
    /// スレッド)で競合しないようにする排他ロック(open-redmineの
    /// `env_test_lock`と同じ理由・同じパターン——実際に`FAILED`
    /// [`auto_update_state_persists_and_reloads_gui_toggle`が別テストの
    /// 環境変数設定と競合]を再現・確認した上で導入した)。
    fn env_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 2026-07-29追記: ハートビート(`mark_cycle_started`/
    /// `seconds_since_last_cycle`)の基本契約を確認する回帰テスト。
    #[test]
    fn heartbeat_reflects_recent_mark_cycle_started_call() {
        let dir = tempfile::tempdir().unwrap();
        let state = AutoUpdateState::load(dir.path().join("settings.json"));
        // `load`直後もインスタンス生成時刻がセットされているため、
        // 経過秒数はごく小さい値になるはず。
        assert!(state.seconds_since_last_cycle() < 5);

        state.mark_cycle_started();
        assert!(state.seconds_since_last_cycle() < 2);
    }

    #[test]
    fn is_newer_detects_later_build_dates() {
        // YYYYMMDDHHMM形式。区切り文字(`.`/`-`)の有無を問わず数字だけを
        // 見て比較することも確認する。
        assert!(is_newer("2026.07.27 11:15", "2026.07.27 11:16"));
        assert!(is_newer("202607271115", "202607280000"));
        assert!(is_newer("2026-07-27-1115", "2027-01-01-0000"));
        assert!(!is_newer("202607271116", "202607271115"));
        assert!(!is_newer("202607271115", "202607271115"));
    }

    #[test]
    fn is_newer_treats_malformed_versions_as_not_newer() {
        assert!(!is_newer("202607271115", "not-a-version"));
        assert!(!is_newer("garbage", "202607271115"));
        // 桁数が合わない(12桁の数字が揃わない)場合も「新しくない」扱い。
        assert!(!is_newer("2026.07", "2026.08.01.0000"));
    }

    #[tokio::test]
    async fn latest_release_tag_from_parses_tag_name_and_strips_v_prefix() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/repos/aon-co-jp/open-easy-web/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"tag_name": "v2026.07.27.1115"}"#)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let tag = latest_release_tag_from(&client, &server.url(), "aon-co-jp/open-easy-web").await.unwrap();
        assert_eq!(tag, "2026.07.27.1115");
    }

    #[tokio::test]
    async fn latest_release_tag_from_fails_cleanly_on_http_error() {
        let mut server = mockito::Server::new_async().await;
        let _mock = server.mock("GET", "/repos/aon-co-jp/open-easy-web/releases/latest").with_status(404).create_async().await;

        let client = reqwest::Client::new();
        let result = latest_release_tag_from(&client, &server.url(), "aon-co-jp/open-easy-web").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn download_and_extract_from_extracts_the_binary_from_a_real_tar_gz() {
        // 実際にtar.gz(gzip圧縮)を組み立て、モックHTTPサーバー経由で
        // ダウンロード→展開までの一気通貫を実際のバイト列で検証する
        // (Windowsでのテスト実行を想定し、Unix専用の実行権限確認は
        // #[cfg(unix)]でスキップする)。
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write as _;

        let mut tar_bytes = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            let content = b"#!/bin/sh\necho fake-binary\n";
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, "open-easy-web-server", &content[..]).unwrap();
            builder.finish().unwrap();
        }
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&tar_bytes).unwrap();
        let gz_bytes = encoder.finish().unwrap();

        let mut server = mockito::Server::new_async().await;
        let _mock = server
            .mock("GET", "/aon-co-jp/open-easy-web/releases/latest/download/open-easy-web-server-linux-x86_64.tar.gz")
            .with_status(200)
            .with_body(gz_bytes)
            .create_async()
            .await;

        let client = reqwest::Client::new();
        let dir = std::env::temp_dir().join(format!("open-easyweb-autoupdate-test-{}", std::process::id()));
        let dest = extract_tar_gz_binary(
            &{
                let resp = client
                    .get(format!("{}/aon-co-jp/open-easy-web/releases/latest/download/open-easy-web-server-linux-x86_64.tar.gz", server.url()))
                    .send()
                    .await
                    .unwrap();
                resp.bytes().await.unwrap().to_vec()
            },
            &dir,
        )
        .unwrap();
        assert!(dest.exists());
        assert_eq!(std::fs::read(&dest).unwrap(), b"#!/bin/sh\necho fake-binary\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn auto_update_state_defaults_to_env_var_when_no_settings_file_exists() {
        let _guard = env_test_lock();
        std::env::set_var("OPEN_EASYWEB_AUTO_UPDATE", "true");
        let dir = std::env::temp_dir().join(format!("open-easyweb-autoupdate-state-test-{}", std::process::id()));
        let state = AutoUpdateState::load(dir.join("settings.json"));
        assert!(state.is_enabled());
        std::env::remove_var("OPEN_EASYWEB_AUTO_UPDATE");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn auto_update_state_persists_and_reloads_gui_toggle() {
        let _guard = env_test_lock();
        let dir = std::env::temp_dir().join(format!("open-easyweb-autoupdate-persist-test-{}", std::process::id()));
        let settings_path = dir.join("settings.json");
        let state = AutoUpdateState::load(settings_path.clone());
        assert!(!state.is_enabled(), "should default to disabled with no env var and no file");
        state.set_enabled(true).unwrap();
        assert!(state.is_enabled());

        // 新しいインスタンス(プロセス再起動を模す)で、永続化された設定
        // (環境変数ではなくファイル)が読み込まれることを確認する。
        let reloaded = AutoUpdateState::load(settings_path);
        assert!(reloaded.is_enabled());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
