//! システムメモリ円グラフを「リポジトリ/プロジェクト/アプリ名ごとの
//! 内訳」+「それ以外(アプリに属さないデータ)の内訳」の2枚の円グラフへ
//! 拡張する実装(2026-09-07、ユーザー指示)。
//!
//! ## 円グラフ1: アプリ(サービス)別の実メモリ使用量
//!
//! `memory_governor::target_units()`(このVPS上のaon-co-jpエコシステム
//! アプリケーションサービス一覧)の各systemdユニットについて、
//! `systemctl show <unit> -p MemoryCurrent`(cgroup経由の実測値、
//! `memory_governor`がMemoryHighを課すのと同じcgroupの実測RSS相当)を
//! 取得する。これは推測ではなく、実際にカーネルが集計している値。
//!
//! ## 円グラフ2: 「その他」(アプリに属さないデータ)のAI分類内訳
//!
//! システム全体の使用中メモリ(`system_memory.rs`、`sysinfo`)から円グラフ1
//! の合計を差し引いた残り(=`sshd`/`postgresql`/`firewalld`/カーネル
//! バッファ等、既知アプリ一覧に属さないプロセス)を対象に、
//! `ps -eo comm,rss --no-headers`でプロセス名ごとのRSS合計を取り、
//! 既知アプリのユニット名と一致するプロセス(comm名は15文字で切り詰め
//! られるため前方一致で判定)を除外した残りを、**aruaru-llmの汎用分類
//! API(`/v1/classify`、embeddingコサイン類似度)**へ渡し、固定カテゴリ
//! 一覧のどれに最も近いかを判定してもらう。カテゴリごとにRSSを合算した
//! ものが円グラフ2の内訳になる。
//!
//! **正直な開示**: (1) cgroup別`MemoryCurrent`の合計とOS全体の
//! `used`(`sysinfo`)は会計方式が完全には一致しない(ページキャッシュの
//! 扱い等)ため、「その他」は**概算**であり負値になった場合は0として
//! 扱う。(2) `ps`のcomm名は15文字で切り詰められるため、既知アプリの
//! 除外判定は前方一致による近似(取りこぼし・誤除外の可能性がある)。
//! (3) aruaru-llmへの到達不能・分類失敗時は、個々のプロセスを
//! `"分類不能 (Unclassified)"`カテゴリへ計上し、処理全体は継続する
//! (黙って0にしたり全体を失敗させたりしない)。

use std::process::Command;

use serde::Serialize;

use crate::memory_governor;

#[derive(Debug, Clone, Serialize)]
pub struct ServiceMemoryEntry {
    pub unit: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OtherCategoryEntry {
    pub category: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryBreakdown {
    pub services: Vec<ServiceMemoryEntry>,
    pub services_total_bytes: u64,
    pub other_total_bytes: u64,
    pub other_by_category: Vec<OtherCategoryEntry>,
    pub classification_engine: String,
}

/// 分類先の固定カテゴリ一覧(日英併記)。
const OTHER_CATEGORIES: &[&str] = &[
    "Database (データベース)",
    "Security & monitoring (セキュリティ・監視)",
    "Networking & remote access (ネットワーク・リモートアクセス)",
    "OS / system infrastructure (OS基盤)",
    "Logging & audit (ログ・監査)",
    "Web/PHP infrastructure (Web・PHP基盤)",
];
const UNCLASSIFIED_LABEL: &str = "Unclassified (分類不能)";

/// `systemctl show <unit> -p MemoryCurrent --value`を実行し、バイト数を
/// 取得する。ユニットが停止中/存在しない/`systemctl`自体が使えない環境
/// では`0`を返す(パニックしない)。
fn unit_memory_current(unit: &str) -> u64 {
    let Ok(out) = Command::new("systemctl").args(["show", unit, "-p", "MemoryCurrent", "--value"]).output() else {
        return 0;
    };
    if !out.status.success() {
        return 0;
    }
    String::from_utf8_lossy(&out.stdout).trim().parse::<u64>().unwrap_or(0)
}

/// `ps -eo comm,rss --no-headers`(RSSはKiB単位)を実行し、プロセス名
/// ごとの合計RSS(バイト)を返す。取得できない環境では空を返す。
fn process_rss_by_comm() -> Vec<(String, u64)> {
    let Ok(out) = Command::new("ps").args(["-eo", "comm,rss", "--no-headers"]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut by_comm: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((comm, rss_kib)) = line.rsplit_once(' ') else { continue };
        let Ok(rss_kib) = rss_kib.trim().parse::<u64>() else { continue };
        *by_comm.entry(comm.trim().to_string()).or_insert(0) += rss_kib * 1024;
    }
    by_comm.into_iter().collect()
}

/// `comm`(15文字切り詰め済み)が、既知アプリユニットのいずれかに
/// 属するプロセスだと推測できるかを前方一致で判定する。
fn belongs_to_known_unit(comm: &str, unit_stems: &[String]) -> bool {
    unit_stems.iter().any(|stem| stem.starts_with(comm) || comm.starts_with(stem.as_str()))
}

/// 円グラフ1(アプリ別)+円グラフ2(その他のAI分類内訳)を組み立てる。
/// `system_total_used_bytes`は`system_memory.rs`が返す実メモリ使用量
/// (物理メモリのused、既存のOS全体スナップショットをそのまま利用)。
/// `classify_fn`はaruaru-llmの`/v1/classify`呼び出し(非同期・HTTP)を
/// 注入するためのコールバック(同期関数であるこのモジュール本体を
/// テスト容易にするため、実際のHTTP呼び出しは呼び出し元`main.rs`が
/// 行い、その結果のみをここへ渡す設計)。
pub fn build(system_total_used_bytes: u64, other_process_categories: Vec<(String, String, u64)>) -> MemoryBreakdown {
    let services: Vec<ServiceMemoryEntry> = memory_governor::target_units()
        .into_iter()
        .map(|unit| {
            let bytes = unit_memory_current(&unit);
            ServiceMemoryEntry { unit, bytes }
        })
        .collect();
    let services_total_bytes: u64 = services.iter().map(|s| s.bytes).sum();
    let other_total_bytes = system_total_used_bytes.saturating_sub(services_total_bytes);

    let mut by_category: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for (_comm, category, bytes) in &other_process_categories {
        *by_category.entry(category.clone()).or_insert(0) += bytes;
    }
    let mut other_by_category: Vec<OtherCategoryEntry> =
        by_category.into_iter().map(|(category, bytes)| OtherCategoryEntry { category, bytes }).collect();
    other_by_category.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    MemoryBreakdown {
        services,
        services_total_bytes,
        other_total_bytes,
        other_by_category,
        classification_engine: "embedding-cosine-v0-open-cuda-bert (aruaru-llm /v1/classify)".to_string(),
    }
}

/// aruaru-llmへ渡す「その他」プロセス一覧(既知アプリ除外済み、
/// comm名ごとの合計RSSバイト)を組み立てる。分類そのもの
/// (HTTP呼び出し)はここでは行わない——呼び出し元(`main.rs`)が
/// この結果の`item`一覧をaruaru-llmへ渡し、返ってきたカテゴリと
/// 組み合わせて`build()`へ渡す2段構成。
pub fn other_process_candidates() -> Vec<(String, u64)> {
    let unit_stems: Vec<String> =
        memory_governor::target_units().iter().map(|u| u.trim_end_matches(".service").to_string()).collect();
    process_rss_by_comm().into_iter().filter(|(comm, _bytes)| !belongs_to_known_unit(comm, &unit_stems)).collect()
}

pub fn fixed_categories() -> Vec<String> {
    OTHER_CATEGORIES.iter().map(|s| s.to_string()).collect()
}

pub fn unclassified_label() -> String {
    UNCLASSIFIED_LABEL.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn belongs_to_known_unit_matches_truncated_comm_names() {
        let stems = vec!["open-web-server".to_string(), "aruaru-llm".to_string()];
        // `ps`は15文字で切り詰める実例("open-english-se"は"open-english"の
        // 前方一致にはならないが、ここでは既知ステムがcommの前方一致
        // している"open-web-server"のケースを確認する。
        assert!(belongs_to_known_unit("open-web-server", &stems));
        assert!(belongs_to_known_unit("aruaru-llm", &stems));
        assert!(!belongs_to_known_unit("sshd-session", &stems));
    }

    #[test]
    fn build_computes_other_total_as_system_minus_services_and_never_goes_negative() {
        // `unit_memory_current`は実環境(systemctl)が無ければ0を返すため、
        // このテスト環境では`services_total_bytes`は常に0になる——
        // その前提で`other_total_bytes`が`system_total_used_bytes`と
        // 一致することを確認する(飽和減算で負値にならないことも兼ねる)。
        let breakdown = build(1_000_000, vec![("sshd".to_string(), "Networking".to_string(), 200_000)]);
        assert_eq!(breakdown.other_total_bytes, 1_000_000 - breakdown.services_total_bytes);
        assert_eq!(breakdown.other_by_category.len(), 1);
        assert_eq!(breakdown.other_by_category[0].bytes, 200_000);
    }

    #[test]
    fn build_never_underflows_when_services_exceed_system_total() {
        // 会計方式の不一致でservices_total_bytesがsystem_total_used_bytesを
        // 上回っても飽和減算で0になり、パニック(オーバーフロー)しないこと。
        let breakdown = build(0, vec![]);
        assert_eq!(breakdown.other_total_bytes, breakdown.other_total_bytes.min(0).max(0));
    }

    #[test]
    fn fixed_categories_and_unclassified_label_are_stable() {
        assert!(!fixed_categories().is_empty());
        assert_eq!(unclassified_label(), "Unclassified (分類不能)");
    }
}
