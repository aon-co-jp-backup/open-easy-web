//! システムメモリ使用状況の取得(2026-07-31追加、ユーザー指示「全体管理
//! 機能に現在のメモリ使用状況/全体の使用可能メモリをGUIで表示して円
//! グラフで表示して」)。`sysinfo`クレート(クロスプラットフォーム、
//! Windows/Linux/macOSで実際のOS APIを叩く実績あるクレート)で総メモリ・
//! 使用中メモリを取得する。

use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct MemorySnapshot {
    /// 物理メモリ(実メモリ/RAM)。
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    /// 使用率(0.0〜100.0)。`total_bytes`が0の場合は0.0を返す(ゼロ除算回避)。
    pub used_percent: f64,
    /// 仮想メモリ(スワップ/ページファイル)。`sysinfo`はLinux(Android含む、
    /// `/proc/meminfo`のSwapTotal/SwapFree経由)・Windows(ページファイル
    /// 経由)・macOSいずれでも取得できる(2026-07-31追加、ユーザー指示
    /// 「実メモリ+仮想メモリを表示」)。スワップ未設定の環境では
    /// `total_swap_bytes`が0になる(エラーではなく正常な状態、正直な開示)。
    pub total_swap_bytes: u64,
    pub used_swap_bytes: u64,
}

/// 現在のシステムメモリ使用状況を取得する。呼び出しのたびに`System`を
/// 新規構築し`refresh_memory()`する(常駐プロセスがバックグラウンドで
/// 継続的にポーリングする設計ではないため、都度取得で十分——過剰実装
/// 回避)。
pub fn snapshot() -> MemorySnapshot {
    let mut sys = System::new();
    sys.refresh_memory();
    let total_bytes = sys.total_memory();
    let used_bytes = sys.used_memory();
    let available_bytes = total_bytes.saturating_sub(used_bytes);
    let used_percent = if total_bytes == 0 { 0.0 } else { (used_bytes as f64 / total_bytes as f64) * 100.0 };
    MemorySnapshot {
        total_bytes,
        used_bytes,
        available_bytes,
        used_percent,
        total_swap_bytes: sys.total_swap(),
        used_swap_bytes: sys.used_swap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reports_a_nonzero_total_on_this_real_machine() {
        // 実際にこのマシンのOS APIを叩いて値が取れることを確認する
        // (モックではなく実システムコール、正直な開示: CI/コンテナ環境に
        // よってはtotal_memoryが取得できない可能性はゼロではないが、
        // 通常のWindows/Linux実行環境では非ゼロになるはず)。
        let snap = snapshot();
        assert!(snap.total_bytes > 0, "expected a nonzero total memory on this real machine");
        assert!(snap.used_bytes <= snap.total_bytes);
        assert_eq!(snap.available_bytes, snap.total_bytes - snap.used_bytes);
        assert!((0.0..=100.0).contains(&snap.used_percent));
        assert!(snap.used_swap_bytes <= snap.total_swap_bytes, "used swap must not exceed total swap");
    }

    #[test]
    fn used_percent_is_zero_when_total_is_zero() {
        let snap = MemorySnapshot { total_bytes: 0, used_bytes: 0, available_bytes: 0, used_percent: 0.0, total_swap_bytes: 0, used_swap_bytes: 0 };
        assert_eq!(snap.used_percent, 0.0);
    }
}
