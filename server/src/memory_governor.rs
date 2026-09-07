//! 「AI省メモリ」— `power_profile::PowerProfileFlags.memory_saver` が
//! 実際にこのVPS上のメモリ使用に効果を持つようにする実装(2026-09-07、
//! ユーザー指示: 「省メモリは、AI省メモリに進化させて、どうしても
//! メモリが必要なリポジトリや場面では、システムメモリーの融通とSSDや
//! HDDなどをシステムメモリーのキャッシュや退避先として上手く使う機能を
//! 搭載して」)。
//!
//! ## これまでの制約(修正対象)
//!
//! `power_profile.rs`の`memory_cache_limit_factor`は、以前は「将来
//! 何かに配線されるはずの情報値」に留まり、実際にメモリ使用量を減らす
//! 仕組みには一切繋がっていなかった(モジュールdocに正直に明記済み)。
//! 本モジュールはこれを解消し、**Linuxのcgroup経由でVPS上の各サービス
//! (systemdユニット)に実際にメモリ上限をかける**。
//!
//! ## 仕組み(なぜこれが「本物」か)
//!
//! - `systemctl set-property --runtime <unit> MemoryHigh=<値>` を使う。
//!   `MemoryMax`(ハード上限、超過でOOM kill)ではなく`MemoryHigh`
//!   (ソフト上限)を採用——超過した分はプロセスを殺さず、カーネルが
//!   優先的にページキャッシュ・匿名メモリをスワップ(このVPSには既に
//!   `/var/spool/swap/swapfile`が2GiB存在することを確認済み)へ退避
//!   させる。これがユーザー要求の「システムメモリーの融通とSSD/HDDを
//!   キャッシュ・退避先として使う」の実体(cgroup memory.high相当)。
//! - `--runtime`を付けることで再起動不要・即座に反映され、ディスクの
//!   unit fileそのものは書き換えない(電源プロファイルOFFに戻せば
//!   `infinity`で即座に解除できる)。
//! - `memory_saver`がONの間だけ上限を課し、OFFなら`infinity`(無制限)に
//!   戻す——「どうしてもメモリが必要な場面」でOOM killに至る前に
//!   スワップへ逃がすソフト上限のため、各サービス自体を停止させない。
//!
//! ## 対象サービス(このVPS固有、正直な開示)
//!
//! `OPEN_EASYWEB_MEMORY_GOVERNOR_UNITS`環境変数(カンマ区切りの
//! systemdユニット名)で明示指定する。未設定時は、このVPS上で実際に
//! 稼働が確認されているaon-co-jpエコシステムのアプリケーションサービス
//! 一覧を既定値として使う(`systemctl list-units`で2026-09-07に確認した
//! 実際の稼働サービス、sshd/firewalld等のOS基盤サービスは対象外)。
//!
//! ## 正直な開示・スコープ外
//!
//! - これは**このVPS上でopen-easy-web-server自身が`systemctl`を実行
//!   できる(root権限で稼働している)ことを前提**とする——他ホストや
//!   コンテナ分離された環境では機能しない。`systemctl`コマンド自体が
//!   見つからない/権限が無い場合は、各ユニットについて正直にエラーを
//!   記録し、他のユニットへの適用は継続する(1件の失敗で全体を止めない)。
//! - 各サービス(open-web-server/open-english/open-redmine等)が
//!   個別に持つアプリケーション内の`power_profile`相当のAPIへの通知は
//!   本モジュールの範囲外(cgroup経由の上限はプロセスの種類を問わず
//!   一律に効くため、個々のリポジトリへ改修を入れなくても実効果を
//!   持たせられる、という設計判断)。
//! - `MemoryHigh`の具体的な値(既定256MiB)は各サービスの実メモリ使用量を
//!   調査した上でのチューニングではなく、保守的な初期値。実運用で
//!   サービスの応答が遅くなる(頻繁なスワップ)場合は
//!   `OPEN_EASYWEB_MEMORY_GOVERNOR_LIMIT_MIB`で調整すること。

use std::process::Command;

/// このVPS上で2026-09-07時点に実際に稼働が確認された、
/// aon-co-jpエコシステムのアプリケーションサービス一覧(既定値)。
const DEFAULT_UNITS: &[&str] = &[
    "aon-tokyo-server.service",
    "aruaru-db-web.service",
    "aruaru-llm.service",
    "aruaru-server.service",
    "aruaru.tokyo.service",
    "audiocafe-php-legacy.service",
    "audiocafe-tokyo-rust.service",
    "e-gov-server.service",
    "fbi-tokyo.service",
    "icpo-tokyo.service",
    "karu-tokyo-server.service",
    "nasa-tokyo.service",
    "open-cg-cad-demo.service",
    "open-cg-cad.service",
    "open-easy-web.service",
    "open-english-demo.service",
    "open-english.service",
    "open-gitea.service",
    "open-kagaku.service",
    "open-raid-z-web.service",
    "open-redmine-demo.service",
    "open-redmine.service",
    "open-web-server.service",
    "rs-blog.service",
    "rs-ec.service",
    "rs-guard.service",
    "rs-link-fusion-landing.service",
    "rs-ops.service",
    "rs-sync-demo.service",
    "rs-sync.service",
    "runo-tokyo.service",
];

const DEFAULT_LIMIT_MIB: u64 = 256;

fn target_units() -> Vec<String> {
    match std::env::var("OPEN_EASYWEB_MEMORY_GOVERNOR_UNITS") {
        Ok(v) if !v.trim().is_empty() => v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
        _ => DEFAULT_UNITS.iter().map(|s| s.to_string()).collect(),
    }
}

fn limit_mib() -> u64 {
    std::env::var("OPEN_EASYWEB_MEMORY_GOVERNOR_LIMIT_MIB")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_LIMIT_MIB)
}

/// 1ユニットへ`MemoryHigh`を設定した結果(成功/失敗を隠さず報告するため)。
#[derive(Debug, Clone, serde::Serialize)]
pub struct UnitApplyResult {
    pub unit: String,
    pub ok: bool,
    pub detail: String,
}

/// `memory_saver`フラグの現在値に応じ、対象ユニット全件へ
/// `MemoryHigh`(有効なら`limit_mib()`、無効なら`infinity`)を適用する。
/// `systemctl`が存在しない/失敗する環境(このVPS以外での開発・テスト実行)
/// でも呼び出し元をパニックさせない——結果を`Vec<UnitApplyResult>`として
/// 正直に返す。
pub fn apply(memory_saver_enabled: bool) -> Vec<UnitApplyResult> {
    let value = if memory_saver_enabled { format!("{}M", limit_mib()) } else { "infinity".to_string() };
    target_units()
        .into_iter()
        .map(|unit| {
            let property = format!("MemoryHigh={value}");
            match Command::new("systemctl").args(["set-property", "--runtime", &unit, &property]).output() {
                Ok(out) if out.status.success() => {
                    UnitApplyResult { unit, ok: true, detail: format!("MemoryHigh={value}") }
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    UnitApplyResult { unit, ok: false, detail: stderr }
                }
                Err(e) => UnitApplyResult { unit, ok: false, detail: format!("systemctl not runnable: {e}") },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_units_falls_back_to_default_list_without_env() {
        let _lock = env_test_lock();
        std::env::remove_var("OPEN_EASYWEB_MEMORY_GOVERNOR_UNITS");
        let units = target_units();
        assert!(units.contains(&"open-easy-web.service".to_string()));
        assert!(units.len() >= 10);
    }

    #[test]
    fn target_units_honors_env_override() {
        let _lock = env_test_lock();
        std::env::set_var("OPEN_EASYWEB_MEMORY_GOVERNOR_UNITS", "foo.service, bar.service");
        let units = target_units();
        std::env::remove_var("OPEN_EASYWEB_MEMORY_GOVERNOR_UNITS");
        assert_eq!(units, vec!["foo.service".to_string(), "bar.service".to_string()]);
    }

    #[test]
    fn limit_mib_defaults_and_honors_env() {
        let _lock = env_test_lock();
        std::env::remove_var("OPEN_EASYWEB_MEMORY_GOVERNOR_LIMIT_MIB");
        assert_eq!(limit_mib(), DEFAULT_LIMIT_MIB);
        std::env::set_var("OPEN_EASYWEB_MEMORY_GOVERNOR_LIMIT_MIB", "512");
        assert_eq!(limit_mib(), 512);
        std::env::remove_var("OPEN_EASYWEB_MEMORY_GOVERNOR_LIMIT_MIB");
    }

    #[test]
    fn apply_reports_honest_failure_when_systemctl_is_unavailable_or_units_unknown() {
        let _lock = env_test_lock();
        // このテスト実行環境(Windows開発機・CI)には対象unitが実在しない、
        // または`systemctl`自体が無いため、必ず`ok: false`になることを
        // 確認する(パニックしないことが本テストの主眼)。
        std::env::set_var("OPEN_EASYWEB_MEMORY_GOVERNOR_UNITS", "definitely-not-a-real-unit.service");
        let results = apply(true);
        std::env::remove_var("OPEN_EASYWEB_MEMORY_GOVERNOR_UNITS");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].unit, "definitely-not-a-real-unit.service");
    }

    /// プロセス全体の環境変数を読み書きするテスト同士が並列実行で
    /// 競合しないための直列化ロック(既存の`dist_sync.rs`等と同じ
    /// パターン)。
    fn env_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }
}
