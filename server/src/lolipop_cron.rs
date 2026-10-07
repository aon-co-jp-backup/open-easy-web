//! ロリポップ!サーバー向け「時間指定オートクロール(cron相当)」機能
//! (2026-10-07追加、ユーザー指示「LOLIPOPの時間指定可能なオートクロール
//! 設定機能を開発して搭載」。「ロリポップ!サーバーのcron相当」との確認済み)。
//!
//! ロリポップ!の管理画面cronは、プランによって有無・最短間隔・書式が
//! 異なり、外部から設定するAPIも無い。そこで本機能は次の2本立てにする。
//!
//! 1. **サーバー内蔵スケジューラ**: 登録したジョブ(ロリポップ上のPHP等の
//!    URL)を、指定した時刻(`HH:MM`を複数)・曜日で自動的にHTTP呼び出しする。
//!    ロリポップ側のcron機能やプランに依存しない。
//! 2. **ロリポップ管理画面用cron書式の書き出し**: 同じスケジュールを
//!    「分 時 日 月 曜日」形式で出力し、管理画面のcron設定へ貼り付けられる。
//!
//! 時刻は既定でJST(UTC+9、ロリポップ!の運用時刻に合わせる)。
//! 環境変数`OPEN_EASYWEB_LOLIPOP_TZ_OFFSET_HOURS`で変更できる。
//! 設定は`OPEN_EASYWEB_LOLIPOP_CRON_FILE`(既定`/var/www/.open-easy-web-lolipop-cron.json`)
//! へ永続化され、再起動後も保持される。管理APIは`x-admin-token`認証必須
//! (`dist_sync::require_admin_token`)。
//!
//! ## 正直な開示
//! - 内蔵スケジューラはこのサーバープロセスが起動している間だけ動く
//!   (停止中の時刻に当たった分は後追い実行しない)。
//! - 実際のロリポップ!サーバーへは接続して検証していない
//!   (ローカルのモックHTTPサーバーに対する実呼び出しテストのみ)。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_TZ_OFFSET_HOURS: i64 = 9;
const TICK_SECS: u64 = 15;
const MAX_JOBS: usize = 200;
/// 後追い実行の対象とする取りこぼしの最大期間(24時間)。
const CATCH_UP_WINDOW_MINUTES: i64 = 1440;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Schedule {
    /// 実行時刻(`"HH:MM"`、24時間表記)。1つ以上必須。
    pub times: Vec<String>,
    /// 実行する曜日(0=日 … 6=土)。空なら毎日。
    #[serde(default)]
    pub weekdays: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub url: String,
    /// `"GET"`または`"POST"`。
    pub method: String,
    pub schedule: Schedule,
    pub enabled: bool,
    pub timeout_secs: u64,
    /// サーバー停止中に過ぎた時刻の分を、起動後に1回だけ後追い実行するか
    /// (直近`CATCH_UP_WINDOW_MINUTES`分以内の取りこぼしのみ対象)。
    #[serde(default = "default_true")]
    pub catch_up: bool,
    /// 最後に発火したローカル時刻のエポック分(後追い判定用、永続化される)。
    #[serde(default)]
    pub last_fired_minute: Option<i64>,
    #[serde(default)]
    pub last_run_at_unix: Option<u64>,
    #[serde(default)]
    pub last_result: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewJob {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub method: Option<String>,
    pub schedule: Schedule,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub catch_up: Option<bool>,
}

pub struct LolipopCron {
    path: PathBuf,
    jobs: Mutex<Vec<Job>>,
    tz_offset_hours: i64,
}

fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.split_once(':')?;
    if h.len() > 2 || m.len() != 2 {
        return None;
    }
    let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (h < 24 && m < 60).then_some((h, m))
}

/// スケジュールを検証し、時刻を`HH:MM`へ正規化・重複除去・昇順化する。
pub fn normalize_schedule(s: &Schedule) -> Result<Schedule> {
    if s.times.is_empty() {
        return Err(anyhow!("times must contain at least one HH:MM / 実行時刻を1つ以上指定してください"));
    }
    let mut times: Vec<(u32, u32)> = Vec::new();
    for t in &s.times {
        let hm = parse_hhmm(t.trim()).ok_or_else(|| anyhow!("invalid time '{t}' (expected HH:MM) / 時刻の形式が不正です: {t}"))?;
        if !times.contains(&hm) {
            times.push(hm);
        }
    }
    times.sort();
    let mut weekdays = s.weekdays.clone();
    if weekdays.iter().any(|d| *d > 6) {
        return Err(anyhow!("weekdays must be 0(Sun)..6(Sat) / 曜日は0(日)〜6(土)で指定してください"));
    }
    weekdays.sort();
    weekdays.dedup();
    if weekdays.len() == 7 {
        weekdays.clear();
    }
    Ok(Schedule {
        times: times.iter().map(|(h, m)| format!("{h:02}:{m:02}")).collect(),
        weekdays,
    })
}

/// `local_minute`(ローカル時刻のエポック分)がスケジュールに該当するか。
pub fn is_due(s: &Schedule, local_minute: i64) -> bool {
    let day = local_minute.div_euclid(1440);
    let min_of_day = local_minute.rem_euclid(1440);
    // 1970-01-01は木曜(=4)。
    let weekday = (day + 4).rem_euclid(7) as u8;
    if !s.weekdays.is_empty() && !s.weekdays.contains(&weekday) {
        return false;
    }
    s.times
        .iter()
        .filter_map(|t| parse_hhmm(t))
        .any(|(h, m)| (h * 60 + m) as i64 == min_of_day)
}

/// ロリポップ!管理画面のcron設定へ貼り付けられる「分 時 日 月 曜日」書式。
pub fn to_crontab_lines(s: &Schedule) -> Vec<String> {
    let dow = if s.weekdays.is_empty() {
        "*".to_string()
    } else {
        s.weekdays.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",")
    };
    s.times
        .iter()
        .filter_map(|t| parse_hhmm(t))
        .map(|(h, m)| format!("{m} {h} * * {dow}"))
        .collect()
}

fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl LolipopCron {
    pub fn load(path: PathBuf) -> Self {
        let jobs = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Vec<Job>>(&b).ok())
            .unwrap_or_default();
        let tz_offset_hours = std::env::var("OPEN_EASYWEB_LOLIPOP_TZ_OFFSET_HOURS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| (-12..=14).contains(v))
            .unwrap_or(DEFAULT_TZ_OFFSET_HOURS);
        Self { path, jobs: Mutex::new(jobs), tz_offset_hours }
    }

    fn persist(&self, jobs: &[Job]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(jobs)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn tz_offset_hours(&self) -> i64 {
        self.tz_offset_hours
    }

    pub fn list(&self) -> Vec<Job> {
        self.jobs.lock().unwrap().clone()
    }

    pub fn add(&self, n: NewJob) -> Result<Job> {
        let name = n.name.trim().to_string();
        if name.is_empty() || name.len() > 100 {
            return Err(anyhow!("name must be 1..100 chars / ジョブ名は1〜100文字で指定してください"));
        }
        let url = n.url.trim().to_string();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(anyhow!("url must start with http:// or https:// / URLはhttp(s)://で始めてください"));
        }
        let method = n.method.as_deref().unwrap_or("GET").to_ascii_uppercase();
        if method != "GET" && method != "POST" {
            return Err(anyhow!("method must be GET or POST"));
        }
        let schedule = normalize_schedule(&n.schedule)?;
        let job = Job {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            url,
            method,
            schedule,
            enabled: true,
            timeout_secs: n.timeout_secs.unwrap_or(60).clamp(1, 600),
            catch_up: n.catch_up.unwrap_or(true),
            // 作成時刻より前のスロットは後追いしない。直前の分を基準にして、
            // 作成と同じ分に指定した時刻は通常どおり発火させる。
            last_fired_minute: Some(self.local_minute(now_unix() as i64) - 1),
            last_run_at_unix: None,
            last_result: None,
        };
        let mut jobs = self.jobs.lock().unwrap();
        if jobs.len() >= MAX_JOBS {
            return Err(anyhow!("too many jobs (max {MAX_JOBS})"));
        }
        jobs.push(job.clone());
        self.persist(&jobs)?;
        Ok(job)
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        let mut jobs = self.jobs.lock().unwrap();
        let before = jobs.len();
        jobs.retain(|j| j.id != id);
        let removed = jobs.len() != before;
        if removed {
            self.persist(&jobs)?;
        }
        Ok(removed)
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<bool> {
        let mut jobs = self.jobs.lock().unwrap();
        let Some(j) = jobs.iter_mut().find(|j| j.id == id) else { return Ok(false) };
        j.enabled = enabled;
        self.persist(&jobs)?;
        Ok(true)
    }

    pub fn get(&self, id: &str) -> Option<Job> {
        self.jobs.lock().unwrap().iter().find(|j| j.id == id).cloned()
    }

    fn record_result(&self, id: &str, result: String) {
        let mut jobs = self.jobs.lock().unwrap();
        if let Some(j) = jobs.iter_mut().find(|j| j.id == id) {
            j.last_run_at_unix = Some(now_unix());
            j.last_result = Some(result);
            let _ = self.persist(&jobs);
        }
    }

    /// ジョブを1回実行し、結果を記録して返す(スケジューラ・「今すぐ実行」共通)。
    pub async fn run_job(&self, job: &Job) -> String {
        let result = match call(job).await {
            Ok(status) => format!("HTTP {status}"),
            Err(e) => format!("error: {e}"),
        };
        tracing::info!(job = %job.name, %result, "lolipop_cron: job executed");
        self.record_result(&job.id, result.clone());
        result
    }

    fn local_minute(&self, now_unix_secs: i64) -> i64 {
        (now_unix_secs + self.tz_offset_hours * 3600).div_euclid(60)
    }

    /// 今この分に発火すべき有効ジョブ(および停止中に取りこぼして後追い
    /// 対象のジョブ)を返し、発火済みとして記録・永続化する。
    pub fn due_jobs(&self, now_unix_secs: i64) -> Vec<Job> {
        let local_minute = self.local_minute(now_unix_secs);
        let mut jobs = self.jobs.lock().unwrap();
        let mut out = Vec::new();
        for j in jobs.iter_mut().filter(|j| j.enabled) {
            let last = j.last_fired_minute;
            let fire = if is_due(&j.schedule, local_minute) {
                last != Some(local_minute)
            } else {
                j.catch_up && missed_slot(&j.schedule, last, local_minute).is_some()
            };
            if fire {
                j.last_fired_minute = Some(local_minute);
                out.push(j.clone());
            }
        }
        if !out.is_empty() {
            let _ = self.persist(&jobs);
        }
        out
    }
}

/// `last_fired`より後・`local_minute`より前で、取りこぼした直近のスロットを探す。
pub fn missed_slot(s: &Schedule, last_fired: Option<i64>, local_minute: i64) -> Option<i64> {
    let last = last_fired?;
    let lowest = (last + 1).max(local_minute - CATCH_UP_WINDOW_MINUTES);
    (lowest..local_minute).rev().find(|m| is_due(s, *m))
}

async fn call(job: &Job) -> Result<u16> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(job.timeout_secs)).build()?;
    let req = if job.method == "POST" { client.post(&job.url) } else { client.get(&job.url) };
    Ok(req.send().await?.status().as_u16())
}

/// 常駐スケジューラ。`TICK_SECS`ごとに該当ジョブを確認して発火する。
pub async fn run_scheduler(cron: Arc<LolipopCron>) {
    loop {
        for job in cron.due_jobs(now_unix() as i64) {
            let cron = Arc::clone(&cron);
            tokio::spawn(async move {
                cron.run_job(&job).await;
            });
        }
        tokio::time::sleep(Duration::from_secs(TICK_SECS)).await;
    }
}

/// 一覧レスポンス用: ジョブ+ロリポップ管理画面用cron書式。
pub fn job_view(j: &Job) -> serde_json::Value {
    let mut v = serde_json::to_value(j).unwrap_or_default();
    v["lolipop_crontab"] = serde_json::json!(to_crontab_lines(&j.schedule));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sched(times: &[&str], weekdays: &[u8]) -> Schedule {
        Schedule { times: times.iter().map(|s| s.to_string()).collect(), weekdays: weekdays.to_vec() }
    }

    #[test]
    fn normalize_sorts_dedups_and_validates() {
        let s = normalize_schedule(&sched(&["9:05", "03:00", "09:05"], &[3, 1, 1])).unwrap();
        assert_eq!(s.times, vec!["03:00", "09:05"]);
        assert_eq!(s.weekdays, vec![1, 3]);
        assert!(normalize_schedule(&sched(&[], &[])).is_err());
        assert!(normalize_schedule(&sched(&["24:00"], &[])).is_err());
        assert!(normalize_schedule(&sched(&["10:5"], &[])).is_err());
        assert!(normalize_schedule(&sched(&["10:00"], &[7])).is_err());
        assert!(normalize_schedule(&sched(&["10:00"], &[0, 1, 2, 3, 4, 5, 6])).unwrap().weekdays.is_empty());
    }

    #[test]
    fn is_due_respects_time_and_weekday() {
        // 1970-01-05(月曜)10:30 = 4日目 → minute = 4*1440 + 630
        let monday_1030 = 4 * 1440 + 630;
        assert!(is_due(&sched(&["10:30"], &[]), monday_1030));
        assert!(is_due(&sched(&["10:30"], &[1]), monday_1030));
        assert!(!is_due(&sched(&["10:30"], &[2]), monday_1030));
        assert!(!is_due(&sched(&["10:31"], &[]), monday_1030));
        // 1970-01-01は木曜(4)
        assert!(is_due(&sched(&["00:00"], &[4]), 0));
    }

    #[test]
    fn crontab_export() {
        assert_eq!(to_crontab_lines(&sched(&["03:00", "15:30"], &[1, 5])), vec!["0 3 * * 1,5", "30 15 * * 1,5"]);
        assert_eq!(to_crontab_lines(&sched(&["00:07"], &[])), vec!["7 0 * * *"]);
    }

    #[tokio::test]
    async fn due_jobs_fire_once_per_minute_and_call_real_http() {
        let mut server = mockito::Server::new_async().await;
        let m = server.mock("GET", "/cron.php").with_status(200).expect(1).create_async().await;
        let dir = tempfile::tempdir().unwrap();
        let cron = LolipopCron::load(dir.path().join("c.json"));
        // 実時刻に依存させないため、現在のローカル時刻の分をそのまま指定する。
        let now = now_unix() as i64;
        let local = now + cron.tz_offset_hours() * 3600;
        let min_of_day = local.div_euclid(60).rem_euclid(1440);
        let t = format!("{:02}:{:02}", min_of_day / 60, min_of_day % 60);
        let job = cron
            .add(NewJob {
                name: "t".into(),
                url: format!("{}/cron.php", server.url()),
                method: None,
                schedule: sched(&[&t], &[]),
                timeout_secs: Some(5),
                catch_up: None,
            })
            .unwrap();
        let due = cron.due_jobs(now);
        assert_eq!(due.len(), 1);
        assert!(cron.due_jobs(now).is_empty(), "same minute must not fire twice");
        assert_eq!(cron.run_job(&due[0]).await, "HTTP 200");
        m.assert_async().await;
        // 永続化→再読込
        let reloaded = LolipopCron::load(dir.path().join("c.json"));
        assert_eq!(reloaded.get(&job.id).unwrap().last_result.as_deref(), Some("HTTP 200"));
        // 無効化すると発火しない
        cron.set_enabled(&job.id, false).unwrap();
        assert!(cron.due_jobs(now + 3600 * 24).is_empty());
        assert!(cron.delete(&job.id).unwrap());
    }

    #[test]
    fn missed_slot_finds_latest_slot_within_window() {
        let s = sched(&["10:00"], &[]);
        let day = 5 * 1440;
        // 09:00に最後に発火→11:00時点で10:00を取りこぼし
        assert_eq!(missed_slot(&s, Some(day + 540), day + 660), Some(day + 600));
        // 10:00以降に発火済みなら取りこぼしなし
        assert_eq!(missed_slot(&s, Some(day + 600), day + 660), None);
        // 最終発火不明は後追いしない
        assert_eq!(missed_slot(&s, None, day + 660), None);
        // 24時間より古い取りこぼしは対象外(3日前に発火、3日後の09:00時点で見える最古は前日10:00)
        let now_min = day + 3 * 1440 + 540;
        assert_eq!(missed_slot(&s, Some(day), now_min), Some(day + 2 * 1440 + 600));
        assert_eq!(missed_slot(&sched(&["09:30"], &[]), Some(day), day + 3 * 1440 + 570 + 1), Some(day + 3 * 1440 + 570));
    }

    #[test]
    fn catch_up_fires_once_after_downtime_and_respects_opt_out() {
        let dir = tempfile::tempdir().unwrap();
        let cron = LolipopCron::load(dir.path().join("c.json"));
        let now = now_unix() as i64;
        let slot = cron.local_minute(now + 3600).rem_euclid(1440);
        let t = format!("{:02}:{:02}", slot / 60, slot % 60);
        let mk = |catch_up| NewJob {
            name: "x".into(),
            url: "http://127.0.0.1:9/x".into(),
            method: None,
            schedule: sched(&[&t], &[]),
            timeout_secs: Some(1),
            catch_up,
        };
        let on = cron.add(mk(None)).unwrap();
        let off = cron.add(mk(Some(false))).unwrap();
        // 2時間後に起動したとする(1時間前に予定時刻を過ぎている)
        let due = cron.due_jobs(now + 7200);
        assert_eq!(due.iter().map(|j| j.id.as_str()).collect::<Vec<_>>(), vec![on.id.as_str()]);
        assert!(cron.due_jobs(now + 7200 + 60).is_empty(), "catch-up runs only once");
        // 永続化されている
        let reloaded = LolipopCron::load(dir.path().join("c.json"));
        assert!(reloaded.get(&on.id).unwrap().last_fired_minute.is_some());
        let _ = off;
    }

    #[test]
    fn add_rejects_bad_input() {
        let dir = tempfile::tempdir().unwrap();
        let cron = LolipopCron::load(dir.path().join("c.json"));
        let mk = |url: &str, method: Option<&str>| NewJob {
            name: "x".into(),
            url: url.into(),
            method: method.map(String::from),
            schedule: sched(&["01:00"], &[]),
            timeout_secs: None,
            catch_up: None,
        };
        assert!(cron.add(mk("ftp://x", None)).is_err());
        assert!(cron.add(mk("https://x", Some("DELETE"))).is_err());
        assert!(cron.add(mk("https://x", Some("post"))).is_ok());
    }
}
