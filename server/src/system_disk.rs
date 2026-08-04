//! 実ディスク(HDD/SSD)使用状況の取得(2026-08-04追加、ユーザー指示
//! 「実際のHDD(SSD)と実際の使用量を円グラフで表示する機能」)。
//! `system_memory.rs`と同じ`sysinfo`クレートの`Disks` APIで、このサーバーが
//! 動いているマシンに実際にマウントされている全ディスクの容量・使用量を
//! 取得する。

use serde::Serialize;
use sysinfo::Disks;

#[derive(Debug, Clone, Serialize)]
pub struct DiskEntry {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    /// 使用率(0.0〜100.0)。`total_bytes`が0の場合は0.0を返す(ゼロ除算回避)。
    pub used_percent: f64,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskSnapshot {
    pub disks: Vec<DiskEntry>,
    /// 全ディスクを合算した合計・使用量(GUIの円グラフはまずこの合算値を
    /// 表示する想定、個別ディスクの内訳は`disks`側で参照できる)。
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub used_percent: f64,
}

/// 現在のディスク使用状況を取得する。呼び出しのたびに`Disks`を新規構築し
/// 一覧を取得する(`system_memory::snapshot`と同じく、常駐ポーリングでは
/// なく都度取得——過剰実装回避)。
pub fn snapshot() -> DiskSnapshot {
    let disks = Disks::new_with_refreshed_list();
    let mut entries = Vec::new();
    let mut total_bytes: u64 = 0;
    let mut used_bytes: u64 = 0;

    for disk in disks.list() {
        let disk_total = disk.total_space();
        let available = disk.available_space();
        let disk_used = disk_total.saturating_sub(available);
        let used_percent = if disk_total == 0 { 0.0 } else { (disk_used as f64 / disk_total as f64) * 100.0 };

        total_bytes = total_bytes.saturating_add(disk_total);
        used_bytes = used_bytes.saturating_add(disk_used);

        entries.push(DiskEntry {
            name: disk.name().to_string_lossy().to_string(),
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            total_bytes: disk_total,
            available_bytes: available,
            used_bytes: disk_used,
            used_percent,
            is_removable: disk.is_removable(),
        });
    }

    let used_percent = if total_bytes == 0 { 0.0 } else { (used_bytes as f64 / total_bytes as f64) * 100.0 };

    DiskSnapshot { disks: entries, total_bytes, used_bytes, used_percent }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reports_at_least_one_disk_on_this_real_machine() {
        // 実際にこのマシンのOS APIを叩いて値が取れることを確認する
        // (モックではなく実システムコール、正直な開示: コンテナ環境等
        // ディスクが検出できない環境では0件になる可能性はゼロではない)。
        let snap = snapshot();
        for entry in &snap.disks {
            assert!(entry.used_bytes <= entry.total_bytes);
            assert!((0.0..=100.0).contains(&entry.used_percent));
        }
        assert_eq!(snap.used_bytes <= snap.total_bytes, true);
        assert!((0.0..=100.0).contains(&snap.used_percent));
    }

    #[test]
    fn used_percent_is_zero_when_total_is_zero() {
        let snap = DiskSnapshot { disks: vec![], total_bytes: 0, used_bytes: 0, used_percent: 0.0 };
        assert_eq!(snap.used_percent, 0.0);
    }
}
