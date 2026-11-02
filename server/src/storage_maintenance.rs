//! ストレージ(HDD/SSD)のメンテナンス(標準機能、2026-11-03新設、ユーザー指示「SMART の健康チェックと、
//! SSD の TRIM・HDD の最適化も入れて。open-easy-web の標準機能に追加して」)。
//!
//! 中身は `vps-tidy` クレート(姉妹リポジトリ)で、ここは「サーバーへの組み込み」(設定の保存・定期実行・管理API)だけ。
//!
//! - **健康チェック**(SMART など): 読み取りだけ。既定で毎日。
//! - **無駄なデータの整理**: 固定の規則だけで決める(AIは使わない)。**既定は試し実行**。`apply` を管理画面で
//!   明示的に有効にしたときだけ、実際に消す。
//! - **最適化**(SSD の TRIM / HDD の最適化): 既定は無効。
//!
//! 管理API(`x-admin-token` 必須)は `main.rs` 側。結果は、直近のものを保存して、画面に出す。

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use vps_tidy::{config as tidy_config, health, optimize, runner};

/// JSON を読むときの上限(RS-JSON の安全な読み込み)。状態ファイルは 1 MiB まで、リクエスト本文は 4 KiB まで。
fn limits(max_bytes: usize) -> rust_json::Limits {
    rust_json::Limits { max_bytes, max_depth: 16, reject_duplicate_keys: true }
}

/// 外から来るリクエスト本文(JSON)を、RS-JSON で、上限つきで読む。
pub fn parse_body<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("本文が UTF-8 ではありません: {e}"))?;
    rust_json::from_str_secure(text, &limits(4096)).map_err(|e| format!("invalid JSON: {e}"))
}

/// 直近の実行結果の文章は、これ以上長いと切る(画面と保存ファイルを大きくしすぎない)。
const MAX_REPORT_CHARS: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// 健康チェックを、毎日(時刻 `hour` 時、サーバーの現地時間)に実行する。
    #[serde(default = "yes")]
    pub health_enabled: bool,
    /// 無駄なデータの整理を、毎日実行する。
    #[serde(default = "yes")]
    pub tidy_enabled: bool,
    /// true のときだけ、整理で**実際に消す**。false(既定)なら、試し実行(報告だけ)。
    #[serde(default)]
    pub tidy_apply: bool,
    /// 最適化(TRIM/最適化)を、毎週日曜に実行する。既定は無効。
    #[serde(default)]
    pub optimize_enabled: bool,
    /// true のときだけ、最適化を**実際に実行**する。false(既定)なら、計画だけ。
    #[serde(default)]
    pub optimize_apply: bool,
    /// 実行する時刻(0〜23時)。
    #[serde(default = "default_hour")]
    pub hour: u8,
}

fn yes() -> bool {
    true
}

fn default_hour() -> u8 {
    4
}

impl Default for Settings {
    fn default() -> Self {
        Self { health_enabled: true, tidy_enabled: true, tidy_apply: false, optimize_enabled: false, optimize_apply: false, hour: 4 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Last {
    /// 実行した時刻(Unixの秒)。
    pub at: i64,
    /// `ok` / `warning` / `critical` / `unknown` / `error` など、画面に出す短い状態。
    pub state: String,
    /// 人が読む結果。
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Saved {
    #[serde(default)]
    settings: Settings,
    #[serde(default)]
    last_health: Option<Last>,
    #[serde(default)]
    last_tidy: Option<Last>,
    #[serde(default)]
    last_optimize: Option<Last>,
    /// 最後に自動実行した日(日付の通し番号)。同じ日に二度実行しない。
    #[serde(default)]
    last_auto_day: i64,
}

pub struct StorageMaintenance {
    path: PathBuf,
    tidy_config: PathBuf,
    inner: Mutex<Saved>,
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn truncate(mut s: String) -> String {
    if s.chars().count() > MAX_REPORT_CHARS {
        s = s.chars().take(MAX_REPORT_CHARS).collect::<String>() + "\n…(長いので、ここで切りました)";
    }
    s
}

fn health_state(s: health::Status) -> &'static str {
    match s {
        health::Status::Ok => "ok",
        health::Status::Unknown => "unknown",
        health::Status::Warning => "warning",
        health::Status::Critical => "critical",
    }
}

impl StorageMaintenance {
    /// `path` は設定と直近の結果の保存先(JSON)。`tidy_config` は、整理の規則(`vps-tidy` の設定ファイル)。
    pub fn load(path: PathBuf, tidy_config: PathBuf) -> Self {
        let saved = std::fs::read_to_string(&path).ok().and_then(|t| rust_json::from_str_secure::<Saved>(&t, &limits(1024 * 1024)).ok()).unwrap_or_default();
        Self { path, tidy_config, inner: Mutex::new(saved) }
    }

    fn save(&self, s: &Saved) {
        if let Ok(text) = rust_json::fast::to_string_pretty(s) {
            if let Some(dir) = self.path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let tmp = self.path.with_extension("json.tmp");
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }

    pub fn settings(&self) -> Settings {
        self.inner.lock().map(|g| g.settings.clone()).unwrap_or_default()
    }

    pub fn set_settings(&self, mut s: Settings) -> Result<Settings, String> {
        if s.hour > 23 {
            return Err("hour は 0〜23 です".to_string());
        }
        // 実際に消す/実行するのは、整理・最適化そのものが有効なときだけ意味がある。
        s.hour = s.hour.min(23);
        let mut g = self.inner.lock().map_err(|_| "内部エラー".to_string())?;
        g.settings = s.clone();
        self.save(&g);
        Ok(s)
    }

    pub fn view(&self) -> serde_json::Value {
        let g = self.inner.lock().map(|g| g.clone()).unwrap_or_default();
        serde_json::json!({
            "settings": g.settings,
            "last_health": g.last_health,
            "last_tidy": g.last_tidy,
            "last_optimize": g.last_optimize,
            "tidy_config_path": self.tidy_config.display().to_string(),
            "tidy_config_exists": self.tidy_config.is_file(),
            "platform": std::env::consts::OS,
        })
    }

    /// 健康チェックを実行して、結果を保存する(ブロックする処理なので、呼び出し側は `spawn_blocking` で呼ぶ)。
    pub fn run_health(&self) -> serde_json::Value {
        let disks = health::collect();
        let worst = health::worst(&disks);
        let mut text = String::new();
        for d in &disks {
            text.push_str(&format!("[{}] {} ({})\n", health_state(d.status), d.name, d.model));
            for r in &d.reasons {
                text.push_str(&format!("  - {r}\n"));
            }
        }
        let last = Last { at: now_secs(), state: health_state(worst).to_string(), text: truncate(text) };
        if let Ok(mut g) = self.inner.lock() {
            g.last_health = Some(last.clone());
            self.save(&g);
        }
        serde_json::json!({ "worst": health_state(worst), "disks": disks, "last": last })
    }

    /// 整理を実行する(`apply` が false なら、試し実行)。設定ファイルが無ければ、その旨を返す。
    pub fn run_tidy(&self, apply: bool) -> serde_json::Value {
        let cfg = match tidy_config::load(&self.tidy_config) {
            Ok(c) => c,
            Err(e) => {
                let last = Last { at: now_secs(), state: "error".to_string(), text: format!("整理の規則を読めません: {e:#}\n(設定ファイル: {})", self.tidy_config.display()) };
                if let Ok(mut g) = self.inner.lock() {
                    g.last_tidy = Some(last.clone());
                    self.save(&g);
                }
                return serde_json::json!({ "error": last.text, "last": last });
            }
        };
        let run = runner::run_tidy(&cfg, apply, false);
        let state = if apply { "applied" } else { "dry-run" };
        let last = Last { at: now_secs(), state: state.to_string(), text: truncate(run.report_text.clone()) };
        if let Ok(mut g) = self.inner.lock() {
            g.last_tidy = Some(last.clone());
            self.save(&g);
        }
        serde_json::json!({ "applied": apply, "summary": run.summary, "free_before": run.free_before, "free_after": run.free_after, "last": last })
    }

    pub fn run_optimize(&self, apply: bool) -> serde_json::Value {
        let rep = optimize::run_optimize(apply);
        let mut text = String::new();
        for i in &rep.items {
            let st = match i.ok {
                Some(true) => "成功",
                Some(false) => "失敗",
                None => "予定",
            };
            text.push_str(&format!("[{st}] {} {} {}\n", i.target, i.action, i.message));
        }
        for n in &rep.notes {
            text.push_str(&format!("* {n}\n"));
        }
        let failed = rep.items.iter().any(|i| i.ok == Some(false));
        let last = Last { at: now_secs(), state: if failed { "error" } else if apply { "applied" } else { "dry-run" }.to_string(), text: truncate(text) };
        if let Ok(mut g) = self.inner.lock() {
            g.last_optimize = Some(last.clone());
            self.save(&g);
        }
        serde_json::json!({ "applied": apply, "report": rep, "last": last })
    }

    /// 自動実行の時刻か(その日まだ実行していなくて、設定の時刻を過ぎている)。`local_secs` は、現地時間の通し秒。
    fn due(&self, local_secs: i64) -> Option<(i64, bool)> {
        let g = self.inner.lock().ok()?;
        let day = local_secs.div_euclid(86_400);
        let hour = local_secs.rem_euclid(86_400) / 3600;
        if g.last_auto_day == day || hour < g.settings.hour as i64 {
            return None;
        }
        // 日曜か(1970-01-01 は木曜。日=0)
        let weekday = (day + 4).rem_euclid(7);
        Some((day, weekday == 0))
    }

    fn mark_auto(&self, day: i64) {
        if let Ok(mut g) = self.inner.lock() {
            g.last_auto_day = day;
            self.save(&g);
        }
    }
}

/// 現地時間の通し秒(UTC秒 + 時差)。時差は、環境変数 `OPEN_EASYWEB_STORAGE_TZ_OFFSET_HOURS`(既定 9=日本)。
fn local_secs() -> i64 {
    let off: i64 = std::env::var("OPEN_EASYWEB_STORAGE_TZ_OFFSET_HOURS").ok().and_then(|v| v.parse().ok()).unwrap_or(9);
    now_secs() + off * 3600
}

/// 5分おきに、「自動実行の時刻か」を見て、実行する。起動直後の負荷を避けるため、最初は少し待つ。
pub async fn run_scheduler(m: std::sync::Arc<StorageMaintenance>) {
    tokio::time::sleep(std::time::Duration::from_secs(120)).await;
    loop {
        if let Some((day, is_sunday)) = m.due(local_secs()) {
            let s = m.settings();
            tracing::info!("ストレージのメンテナンスを自動実行します");
            if s.health_enabled {
                let mm = std::sync::Arc::clone(&m);
                let _ = tokio::task::spawn_blocking(move || mm.run_health()).await;
            }
            if s.tidy_enabled {
                let mm = std::sync::Arc::clone(&m);
                let apply = s.tidy_apply;
                let _ = tokio::task::spawn_blocking(move || mm.run_tidy(apply)).await;
            }
            if s.optimize_enabled && is_sunday {
                let mm = std::sync::Arc::clone(&m);
                let apply = s.optimize_apply;
                let _ = tokio::task::spawn_blocking(move || mm.run_optimize(apply)).await;
            }
            m.mark_auto(day);
        }
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> (tempfile::TempDir, StorageMaintenance) {
        let d = tempfile::tempdir().unwrap();
        let m = StorageMaintenance::load(d.path().join("sm.json"), d.path().join("tidy.toml"));
        (d, m)
    }

    #[test]
    fn defaults_are_safe_dry_run_and_no_optimize() {
        let s = Settings::default();
        assert!(s.health_enabled && s.tidy_enabled);
        assert!(!s.tidy_apply, "既定は、消さない(試し実行)");
        assert!(!s.optimize_enabled && !s.optimize_apply);
    }

    #[test]
    fn settings_round_trip_and_are_validated() {
        let (d, m) = tmp();
        let mut s = m.settings();
        s.tidy_apply = true;
        s.hour = 2;
        m.set_settings(s).unwrap();
        let m2 = StorageMaintenance::load(d.path().join("sm.json"), d.path().join("tidy.toml"));
        assert!(m2.settings().tidy_apply);
        assert_eq!(m2.settings().hour, 2);
        let mut bad = m2.settings();
        bad.hour = 30;
        assert!(m2.set_settings(bad).is_err());
    }

    #[test]
    fn scheduler_runs_once_a_day_after_the_hour() {
        let (_d, m) = tmp(); // 既定 4 時
        let day = 20_000i64;
        assert!(m.due(day * 86_400 + 3 * 3600).is_none(), "4時前は、実行しない");
        let (d, _) = m.due(day * 86_400 + 4 * 3600 + 5).unwrap();
        assert_eq!(d, day);
        m.mark_auto(day);
        assert!(m.due(day * 86_400 + 5 * 3600).is_none(), "同じ日に二度は、実行しない");
        assert!(m.due((day + 1) * 86_400 + 4 * 3600).is_some(), "翌日は、また実行する");
    }

    #[test]
    fn sunday_detection_matches_the_calendar() {
        let (_d, m) = tmp();
        // 2026-11-01 は日曜(1970-01-01 から 20_758 日目)
        let sunday = 20_758i64;
        let (_, is_sun) = m.due(sunday * 86_400 + 5 * 3600).unwrap();
        assert!(is_sun);
        let (_, is_sun) = m.due((sunday + 1) * 86_400 + 5 * 3600).unwrap();
        assert!(!is_sun);
    }

    #[test]
    fn request_bodies_are_read_with_rs_json_limits() {
        let ok: Settings = parse_body(br#"{"tidy_apply":true,"hour":3}"#).unwrap();
        assert!(ok.tidy_apply);
        assert_eq!(ok.hour, 3);
        assert!(ok.health_enabled, "省いた項目は、既定値");
        assert!(parse_body::<Settings>(br#"{"hour":3,"hour":4}"#).is_err(), "重複キーは、拒否する");
        assert!(parse_body::<Settings>(&vec![b' '; 5000]).is_err(), "4 KiB を超える本文は、拒否する");
        let deep = format!("{}1{}", "[".repeat(40), "]".repeat(40));
        assert!(parse_body::<Settings>(deep.as_bytes()).is_err(), "深すぎる入れ子は、拒否する");
    }

    #[test]
    fn a_corrupt_state_file_falls_back_to_the_safe_defaults() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("sm.json"), "{ this is not json").unwrap();
        let m = StorageMaintenance::load(d.path().join("sm.json"), d.path().join("tidy.toml"));
        assert!(!m.settings().tidy_apply, "壊れていても、消す設定にはならない");
    }

    #[test]
    fn tidy_without_a_config_reports_an_error_instead_of_deleting() {
        let (_d, m) = tmp();
        let v = m.run_tidy(true);
        assert!(v.get("error").is_some(), "設定が無いなら、何も消さず、エラーを返す");
    }
}
