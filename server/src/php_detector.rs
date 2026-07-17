//! 「AI自動判定」— 外部LLM・契約不要の純Rust統計学習によるPHPサイト検出。
//!
//! `poem-cosmo-tauri` の `CachePredictor`(EWMA によるオンライン統計学習、
//! 外部LLM不要)と同じ設計思想: シグネチャ(拡張子・マーカーファイル・
//! `<?php`タグ)を加重スコアリングし、閾値超えでPHPと判定する。
//! ユーザーの手動訂正を受けると、`prev*(1-α)+new*α` の EWMA 式で
//! 重みを補正・永続化する——固定ルールベースではなく実際に学習が
//! 起きる、という主張を実装で裏付ける。

use serde::{Deserialize, Serialize};
use std::path::Path;

const LEARNING_RATE: f64 = 0.2;
const DEFAULT_THRESHOLD: f64 = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhpSignalWeights {
    pub php_ext: f64,
    pub php_open_tag: f64,
    pub wp_config: f64,
    pub composer_json: f64,
    pub artisan: f64,
    pub htaccess: f64,
}

impl Default for PhpSignalWeights {
    fn default() -> Self {
        Self {
            php_ext: 0.5,
            php_open_tag: 0.2,
            wp_config: 0.9,
            composer_json: 0.4,
            artisan: 0.6,
            htaccess: 0.1,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Signals {
    php_ext: bool,
    php_open_tag: bool,
    wp_config: bool,
    composer_json: bool,
    artisan: bool,
    htaccess: bool,
}

/// ディレクトリを再帰走査してシグネチャの有無を集める。
fn scan(dir: &Path) -> std::io::Result<Signals> {
    let mut signals = Signals::default();
    scan_into(dir, &mut signals)?;
    Ok(signals)
}

fn scan_into(dir: &Path, signals: &mut Signals) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_into(&path, signals)?;
            continue;
        }
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.eq_ignore_ascii_case("wp-config.php") {
            signals.wp_config = true;
        }
        if file_name.eq_ignore_ascii_case("composer.json") {
            signals.composer_json = true;
        }
        if file_name == "artisan" {
            signals.artisan = true;
        }
        if file_name.eq_ignore_ascii_case(".htaccess") {
            signals.htaccess = true;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("php") {
            signals.php_ext = true;
            if !signals.php_open_tag {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.contains("<?php") {
                        signals.php_open_tag = true;
                    }
                }
            }
        }
    }
    Ok(())
}

/// 各シグネチャの重みを「PHPである確率」とみなし、ノイズOR結合
/// (`1 - Π(1 - w_i)`、該当したシグネチャのみ)でスコアを求める。
/// 加重平均(合計/最大合計)だと、WordPress/Laravel特有のマーカーが
/// 無い素のPHPサイト(拡張子と`<?php`タグだけ)が他の強いシグネチャで
/// 薄められて閾値を割ってしまう問題があったため、独立事象の確率的
/// 合成として扱う方式に変更した。
fn score_with_weights(signals: &Signals, weights: &PhpSignalWeights) -> f64 {
    let mut miss_product = 1.0_f64;
    let mut apply = |present: bool, w: f64| {
        if present {
            miss_product *= 1.0 - w.clamp(0.0, 1.0);
        }
    };
    apply(signals.php_ext, weights.php_ext);
    apply(signals.php_open_tag, weights.php_open_tag);
    apply(signals.wp_config, weights.wp_config);
    apply(signals.composer_json, weights.composer_json);
    apply(signals.artisan, weights.artisan);
    apply(signals.htaccess, weights.htaccess);
    (1.0 - miss_product).clamp(0.0, 1.0)
}

#[derive(Debug, Serialize)]
pub struct DetectionResult {
    pub is_php: bool,
    pub confidence: f64,
}

/// `dir` を走査し、`weights` に基づくPHP判定を行う。
pub fn detect(dir: &Path, weights: &PhpSignalWeights) -> std::io::Result<DetectionResult> {
    let signals = scan(dir)?;
    let confidence = score_with_weights(&signals, weights);
    Ok(DetectionResult {
        is_php: confidence >= DEFAULT_THRESHOLD,
        confidence,
    })
}

/// ユーザーの手動訂正を受け、シグネチャの重みをEWMA式で補正する。
/// `is_php_actual` が実際の正解、`dir` は判定対象だったディレクトリ
/// (シグネチャを再走査し、実際に立っていた信号だけを補正対象にする)。
pub fn correct(
    dir: &Path,
    weights: &mut PhpSignalWeights,
    is_php_actual: bool,
) -> std::io::Result<()> {
    let signals = scan(dir)?;
    let target = if is_php_actual { 1.0 } else { 0.0 };
    let nudge = |w: &mut f64, present: bool| {
        if present {
            *w = *w * (1.0 - LEARNING_RATE) + target * LEARNING_RATE;
        }
    };
    nudge(&mut weights.php_ext, signals.php_ext);
    nudge(&mut weights.php_open_tag, signals.php_open_tag);
    nudge(&mut weights.wp_config, signals.wp_config);
    nudge(&mut weights.composer_json, signals.composer_json);
    nudge(&mut weights.artisan, signals.artisan);
    nudge(&mut weights.htaccess, signals.htaccess);
    Ok(())
}

pub fn load_weights(state_path: &Path) -> PhpSignalWeights {
    std::fs::read_to_string(state_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_weights(state_path: &Path, weights: &PhpSignalWeights) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(weights).unwrap_or_default();
    std::fs::write(state_path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn wordpress_site_scores_above_threshold() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "wp-config.php", "<?php\n// config\n");
        write(dir.path(), "index.php", "<?php echo 'hi'; ?>");
        let weights = PhpSignalWeights::default();
        let result = detect(dir.path(), &weights).unwrap();
        assert!(result.is_php, "confidence={}", result.confidence);
    }

    #[test]
    fn static_html_site_scores_below_threshold() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "index.html", "<html></html>");
        write(dir.path(), "style.css", "body{}");
        let weights = PhpSignalWeights::default();
        let result = detect(dir.path(), &weights).unwrap();
        assert!(!result.is_php, "confidence={}", result.confidence);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn bare_php_without_markers_still_detected_via_extension_and_tag() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "index.php", "<?php echo 'audiocafe'; ?>");
        let weights = PhpSignalWeights::default();
        let result = detect(dir.path(), &weights).unwrap();
        assert!(result.is_php, "confidence={}", result.confidence);
    }

    #[test]
    fn correction_nudges_weights_toward_actual_label() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "index.php", "<?php ?>");
        let mut weights = PhpSignalWeights::default();
        let before = weights.php_ext;
        // ユーザーが「これはPHPではない」と訂正した場合、php_extの重みは下がるはず。
        correct(dir.path(), &mut weights, false).unwrap();
        assert!(weights.php_ext < before);
        // htaccessは今回のディレクトリに存在しないので、無関係な重みは変化しない。
        assert_eq!(weights.htaccess, PhpSignalWeights::default().htaccess);
    }

    #[test]
    fn weights_round_trip_through_json_file() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("weights.json");
        let mut weights = PhpSignalWeights::default();
        weights.php_ext = 0.77;
        save_weights(&state_path, &weights).unwrap();
        let loaded = load_weights(&state_path);
        assert_eq!(loaded.php_ext, 0.77);
    }
}
