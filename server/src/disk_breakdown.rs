//! ディスク使用状況を3枚の円グラフへ分解する(2026-09-07新設、
//! ユーザー指示「ディスク使用状況も1つ目はリポジトリかアプリ別、
//! 2つ目は拡張子別、3つ目はカテゴリ・ジャンル別でaruaru-llmのAIを
//! 使用」)。`memory_breakdown.rs`と対になる設計。
//!
//! ## 円グラフ1: アプリ(サービス)別ディスク使用量
//! `memory_governor::target_units()`の各systemdユニットについて
//! `systemctl show <unit> -p WorkingDirectory`で作業ディレクトリを取得し、
//! `du -sb <dir>`(GNU coreutils、実バイト数)で実使用量を取得する。
//!
//! ## 円グラフ2: 「その他」領域のファイル拡張子別内訳
//! 既知アプリの作業ディレクトリに含まれない領域(`/root`直下でアプリ
//! ディレクトリと一致しないもの、既定)を`find <dir> -maxdepth 4 -type f`
//! で走査し、拡張子ごとにファイルサイズを合算する。**正直な開示**:
//! 深さ4・1領域限定のスキャンであり、VPS全体のディスクを網羅する
//! ものではない(本番環境での過大な負荷・長時間実行を避けるための
//! 意図的な制限)。
//!
//! ## 円グラフ3: 「その他」の拡張子をaruaru-llmのAIでカテゴリ分類
//! 円グラフ2で得た拡張子一覧を`aruaru_llm_client::classify_many`へ渡し、
//! 固定カテゴリ(ソースコード/データベース/ログ/メディア/設定/
//! ドキュメント/ビルド成果物/その他)のいずれに属するかを判定してもらう。
//! aruaru-llmへ到達できない場合は全て「分類不能」へ計上し処理は継続する。

use std::process::Command;

use serde::Serialize;

use crate::memory_governor;

#[derive(Debug, Clone, Serialize)]
pub struct AppDiskEntry {
    pub unit: String,
    pub working_directory: Option<String>,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionEntry {
    pub extension: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskCategoryEntry {
    pub category: String,
    pub bytes: u64,
}

pub const DISK_CATEGORIES: &[&str] = &[
    "Source code",
    "Database",
    "Logs",
    "Media (images/audio/video)",
    "Configuration",
    "Documentation",
    "Build artifacts / binaries",
];
pub const UNCLASSIFIED_LABEL: &str = "Unclassified (分類不能)";

/// スキャン対象の「その他」領域の既定(既知アプリの作業ディレクトリを
/// 含まない、一般的な置き場所)。`OPEN_EASYWEB_DISK_OTHER_SCAN_ROOT`で
/// 上書き可能。
const DEFAULT_OTHER_SCAN_ROOT: &str = "/var/www";
const OTHER_SCAN_MAX_DEPTH: &str = "4";

fn working_directory_of(unit: &str) -> Option<String> {
    let out = Command::new("systemctl").args(["show", unit, "-p", "WorkingDirectory", "--value"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if dir.is_empty() { None } else { Some(dir) }
}

fn dir_size_bytes(dir: &str) -> u64 {
    let Ok(out) = Command::new("du").args(["-sb", dir]).output() else { return 0 };
    if !out.status.success() {
        return 0;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.split_whitespace().next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0)
}

pub fn app_disk_usage() -> Vec<AppDiskEntry> {
    memory_governor::target_units()
        .into_iter()
        .map(|unit| match working_directory_of(&unit) {
            Some(dir) => {
                let bytes = dir_size_bytes(&dir);
                AppDiskEntry { unit, working_directory: Some(dir), bytes }
            }
            None => AppDiskEntry { unit, working_directory: None, bytes: 0 },
        })
        .collect()
}

fn other_scan_root() -> String {
    std::env::var("OPEN_EASYWEB_DISK_OTHER_SCAN_ROOT").unwrap_or_else(|_| DEFAULT_OTHER_SCAN_ROOT.to_string())
}

fn extension_of(filename: &str) -> String {
    match filename.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() && ext.len() <= 10 => format!(".{}", ext.to_lowercase()),
        _ => "(no extension)".to_string(),
    }
}

/// `find <root> -maxdepth 4 -type f -printf '%s %f\n'`相当を実行し、
/// 拡張子ごとの合計バイト数を返す。`find`が使えない(非Linux等)環境では
/// 空を返す(パニックしない)。
pub fn other_extension_breakdown() -> Vec<ExtensionEntry> {
    let root = other_scan_root();
    let Ok(out) =
        Command::new("find").args([root.as_str(), "-maxdepth", OTHER_SCAN_MAX_DEPTH, "-type", "f", "-printf", "%s %f\n"]).output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut by_ext: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for line in text.lines() {
        let Some((size_str, name)) = line.split_once(' ') else { continue };
        let Ok(size) = size_str.parse::<u64>() else { continue };
        *by_ext.entry(extension_of(name)).or_insert(0) += size;
    }
    let mut out: Vec<ExtensionEntry> = by_ext.into_iter().map(|(extension, bytes)| ExtensionEntry { extension, bytes }).collect();
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    out
}

/// 拡張子ごとのバイト数一覧+分類結果(拡張子→カテゴリ、失敗時は空)から
/// カテゴリ別合計を組み立てる。分類できなかった拡張子は
/// `UNCLASSIFIED_LABEL`へ計上する。
pub fn build_category_breakdown(extensions: &[ExtensionEntry], classified: &[(String, String)]) -> Vec<DiskCategoryEntry> {
    let category_of: std::collections::HashMap<&str, &str> =
        classified.iter().map(|(ext, cat)| (ext.as_str(), cat.as_str())).collect();
    let mut by_category: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for entry in extensions {
        let category = category_of.get(entry.extension.as_str()).copied().unwrap_or(UNCLASSIFIED_LABEL);
        *by_category.entry(category.to_string()).or_insert(0) += entry.bytes;
    }
    let mut out: Vec<DiskCategoryEntry> =
        by_category.into_iter().map(|(category, bytes)| DiskCategoryEntry { category, bytes }).collect();
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_of_handles_normal_hidden_and_missing_extensions() {
        assert_eq!(extension_of("app.rs"), ".rs");
        assert_eq!(extension_of("archive.tar.gz"), ".gz");
        assert_eq!(extension_of("Makefile"), "(no extension)");
        assert_eq!(extension_of(".gitignore"), "(no extension)");
    }

    #[test]
    fn build_category_breakdown_sums_by_category_and_falls_back_to_unclassified() {
        let extensions = vec![
            ExtensionEntry { extension: ".rs".to_string(), bytes: 100 },
            ExtensionEntry { extension: ".log".to_string(), bytes: 50 },
            ExtensionEntry { extension: ".xyz".to_string(), bytes: 10 },
        ];
        let classified = vec![
            (".rs".to_string(), "Source code".to_string()),
            (".log".to_string(), "Logs".to_string()),
        ];
        let breakdown = build_category_breakdown(&extensions, &classified);
        let total: u64 = breakdown.iter().map(|c| c.bytes).sum();
        assert_eq!(total, 160);
        let unclassified = breakdown.iter().find(|c| c.category == UNCLASSIFIED_LABEL).unwrap();
        assert_eq!(unclassified.bytes, 10);
    }

    #[test]
    fn disk_categories_are_non_empty() {
        assert!(!DISK_CATEGORIES.is_empty());
    }
}
