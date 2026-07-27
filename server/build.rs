//! 深夜バックグラウンド自動アップデート(`src/auto_update.rs`)向けの
//! ビルド時バージョン埋め込み(2026-07-27追加、ユーザー指示「バージョンは
//! 日付にしてその表示機能を持たせて 例えば 最新は、2026.07.27 11:15」)。
//!
//! `Cargo.toml`の`version`(セマンティックバージョン、crates.io/cargo
//! ツール向けの慣習をそのまま維持するため変更しない)とは別に、実際の
//! ビルド時刻(UTC)を「日付そのものがバージョン」という表示形式で
//! コンパイル時に環境変数として埋め込む:
//! - `OPEN_EASYWEB_BUILD_VERSION_COMPACT`: `"202607271115"`
//!   (`YYYYMMDDHHMM`、12桁固定幅の数値文字列——同じ桁数同士なら文字列
//!   比較がそのまま数値比較・時系列比較になるため、`is_newer`の実装を
//!   単純に保てる)。
//! - `OPEN_EASYWEB_BUILD_VERSION_DISPLAY`: `"2026.07.27 11:15"`
//!   (ユーザー向け表示形式)。

use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let (y, mo, d, h, mi) = civil_from_unix_seconds(now);

    println!("cargo:rustc-env=OPEN_EASYWEB_BUILD_VERSION_COMPACT={y:04}{mo:02}{d:02}{h:02}{mi:02}");
    println!("cargo:rustc-env=OPEN_EASYWEB_BUILD_VERSION_DISPLAY={y:04}.{mo:02}.{d:02} {h:02}:{mi:02}");
    println!("cargo:rerun-if-changed=build.rs");
}

/// UNIXエポック秒(UTC)から、年・月・日・時・分を計算する(依存を増やさない
/// ため`chrono`等は使わず、Howard Hinnantの`civil_from_days`アルゴリズム
/// [http://howardhinnant.github.io/date_algorithms.html]を自前実装——
/// うるう年・月末日数を正しく扱う、広く知られた実装)。
fn civil_from_unix_seconds(secs: u64) -> (i64, u32, u32, u32, u32) {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let hour = (rem / 3600) as u32;
    let minute = ((rem % 3600) / 60) as u32;

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };

    (y, m, d, hour, minute)
}
