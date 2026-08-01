//! 実行時に切り替え可能な省メモリ/省電力プロファイル(デスクトップ
//! [Windows/Linux]版、2026-07-26追加、同日中に単一選択→**組み合わせ可能な
//! 独立フラグ**へ再設計)。
//!
//! **背景(再設計前)**: 当初はAndroid版と同じく4値排他選択(enum一つ)
//! だった。ユーザー指示「4つのモードを組み合わせで選択出来るようにして」
//! を受け、**省メモリ・省電力・常時電源接続の3つを独立したON/OFFフラグ**
//! として持てるように再設計する(「通常」の扱いは下記参照)。
//!
//! ## 設計判断: 「通常」は独立フラグではなく「他のフラグが1つも立っていない状態」
//!
//! 「通常」は他の3つ(省メモリ/省電力/常時電源接続)のどれとも同時に選択
//! する意味がない(例: 「通常+省電力」は「省電力」と区別する実益が無い)。
//! これを独立フラグにすると「通常かつ省電力」のような無意味な組み合わせを
//! 型として許してしまい、かえって呼び出し側の判定が複雑になる。そのため
//! 「通常」は**フラグを持たない**設計とし、`PowerProfileFlags`の3フィールド
//! (`memory_saver`/`power_save`/`always_on`)が全て`false`の状態を「通常」
//! として扱う(`PowerProfileFlags::default()`と一致、既定値)。API上も
//! `profiles: []`(空配列)を送れば明示的に「通常(＝フラグ無し)」を意味する。
//!
//! ## 組み合わせ時の挙動
//!
//! - **省メモリ + 省電力(両方の効果が合成される)**: 各数値設定は
//!   「有効な各フラグのうちより保守的(制限が厳しい)な値」を採用する
//!   (`effective_settings()`参照)。省電力のポーリング間隔延長(3倍)と
//!   省メモリのキャッシュ縮小(倍率0.25)は別軸の設定値なので、両方が
//!   そのまま同時に適用される——「上書き」ではなく「合成」。
//! - **省電力 + 常時電源接続(意味論的に矛盾する組み合わせ)**: 常時電源
//!   接続は「バッテリー残量を気にする必要が無い」という前提そのものが
//!   省電力の存在理由と矛盾するため、**常時電源接続が有効な場合は
//!   ポーリング間隔に関する省電力固有の効果(間隔延長)を無効化**し、
//!   常時電源接続自身の値(間隔短縮=0.2倍)を優先する。ただし、これは
//!   「常時電源接続が省電力の"バッテリー節約"軸だけを無効化する」という
//!   意味であり、**省メモリが同時に有効な場合の「メモリを減らす」軸は
//!   独立した別軸のため、常時電源接続の影響を受けず引き続き適用される**
//!   (`memory_saver`は`poll_interval_multiplier`の計算式に一切登場しない
//!   ことに注意)。この優先順位はユーザーの工学的判断による設計選択であり、
//!   「充電しっぱなしの機器ではバッテリー節約より即応性を優先する方が
//!   実用上有益」という考え方に基づく。

use std::sync::RwLock;
use std::time::Duration;

/// 個別のプロファイル・フラグを指し示す識別子(Android版`PowerProfile.kt`
/// と同じ`prefValue`文字列・同じ日本語ラベルを維持する)。**「通常」だけは
/// 独立フラグではないため、この列挙型には含まれない**——「通常」は
/// `PowerProfileFlags`が全フラグ`false`の状態として表現される(上記
/// モジュールdoc参照)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerProfileFlag {
    MemorySaver,
    PowerSave,
    AlwaysOn,
}

impl PowerProfileFlag {
    pub fn pref_value(&self) -> &'static str {
        match self {
            PowerProfileFlag::MemorySaver => "memory_saver",
            PowerProfileFlag::PowerSave => "power_save",
            PowerProfileFlag::AlwaysOn => "always_on",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PowerProfileFlag::MemorySaver => "省メモリ",
            PowerProfileFlag::PowerSave => "省電力",
            PowerProfileFlag::AlwaysOn => "常時電源接続",
        }
    }

    pub fn from_pref_value(value: &str) -> Option<Self> {
        match value {
            "memory_saver" => Some(PowerProfileFlag::MemorySaver),
            "power_save" => Some(PowerProfileFlag::PowerSave),
            "always_on" => Some(PowerProfileFlag::AlwaysOn),
            // 後方互換: 旧来の単一選択API/Android版が送る"normal"は、
            // 「フラグ無し」を意味する明示値として受理する(このどのフラグ
            // にもマッチしないため、呼び出し側で「該当フラグ無し=通常」と
            // して扱う。from_pref_valuesを参照)。
            _ => None,
        }
    }
}

/// 現在アクティブな組み合わせ(3つの独立ブールフラグ)。全て`false`が
/// 「通常」(既定値、`Default`実装参照)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct PowerProfileFlags {
    pub memory_saver: bool,
    pub power_save: bool,
    pub always_on: bool,
}

impl PowerProfileFlags {
    /// 文字列配列(例: `["power_save", "memory_saver"]`)からフラグ集合を
    /// 組み立てる。空配列は「通常」(全フラグfalse)を意味する。旧来の
    /// 単一値`"normal"`が混じっていても無視する(＝フラグを追加しない、
    /// 後方互換のための緩和)。未知の値が1つでもあれば`Err`でその値を返す。
    pub fn from_pref_values<S: AsRef<str>>(values: &[S]) -> Result<Self, String> {
        let mut flags = PowerProfileFlags::default();
        for v in values {
            let v = v.as_ref();
            if v == "normal" {
                continue;
            }
            match PowerProfileFlag::from_pref_value(v) {
                Some(PowerProfileFlag::MemorySaver) => flags.memory_saver = true,
                Some(PowerProfileFlag::PowerSave) => flags.power_save = true,
                Some(PowerProfileFlag::AlwaysOn) => flags.always_on = true,
                None => return Err(v.to_string()),
            }
        }
        Ok(flags)
    }

    /// アクティブなフラグの`pref_value`一覧(順序は固定: memory_saver→
    /// power_save→always_on)。全てfalseなら空配列(＝通常)。
    pub fn active_pref_values(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.memory_saver {
            out.push(PowerProfileFlag::MemorySaver.pref_value());
        }
        if self.power_save {
            out.push(PowerProfileFlag::PowerSave.pref_value());
        }
        if self.always_on {
            out.push(PowerProfileFlag::AlwaysOn.pref_value());
        }
        out
    }

    /// アクティブなフラグの日本語ラベル一覧。空なら`["通常"]`を返す
    /// (UI表示用、「通常」というラベル自体はフラグではないがユーザー
    /// 向け表示のために合成する)。
    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.memory_saver {
            out.push(PowerProfileFlag::MemorySaver.label());
        }
        if self.power_save {
            out.push(PowerProfileFlag::PowerSave.label());
        }
        if self.always_on {
            out.push(PowerProfileFlag::AlwaysOn.label());
        }
        if out.is_empty() {
            out.push("通常");
        }
        out
    }

    pub fn is_normal(&self) -> bool {
        !self.memory_saver && !self.power_save && !self.always_on
    }
}

/// 現在の組み合わせから導かれる実際の数値設定(合成ロジックの実体)。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct EffectivePowerSettings {
    /// バックグラウンド定期ポーリング(DDNS/free_domain)の基準間隔への
    /// 倍率。省電力=3.0(延長)/常時電源接続=0.2(短縮)/通常・省メモリ単独
    /// =1.0(変更無し)。**省電力+常時電源接続が同時に有効な場合は、
    /// 常時電源接続がこの軸を上書きする**(モジュールdoc「組み合わせ時の
    /// 挙動」参照、電源に困らない機器ではバッテリー節約よりも即応性を
    /// 優先するという設計判断)。
    pub poll_interval_multiplier: f64,
    /// メモリ使用量に関する仮想的な「保守度」係数(1.0=制限なし、値が
    /// 小さいほど厳しく制限)。省メモリが有効なら0.25、それ以外は1.0。
    /// **常時電源接続・省電力の状態に関わらず、省メモリが有効なら常に
    /// この値になる**(独立した別軸であることの直接的な表現)。
    /// **正直な開示**: このバイナリには「実際にキャッシュ/バッファ上限を
    /// 持つ具体的な仕組み」がまだ無いため(旧実装からの既知の制約、
    /// 過去のdocコメント参照)、この係数は現時点では実際のメモリ確保箇所
    /// へは配線されていない情報値に留まる——将来そのような仕組みが
    /// 追加された際にこの係数をそのまま使える設計として先取りしてある。
    pub memory_cache_limit_factor: f64,
}

/// フラグの組み合わせから実際の設定値を合成する(「各数値設定について、
/// アクティブな全フラグのうちより保守的[制限が厳しい]な値を採用する」
/// という組み合わせルールの実装本体)。
pub fn effective_settings(flags: PowerProfileFlags) -> EffectivePowerSettings {
    // ポーリング間隔軸: 省電力(延長したい)と常時電源接続(短縮したい)は
    // 直接対立するため、常時電源接続を優先する(モジュールdoc参照)。
    // どちらも無効なら1.0(通常・省メモリ単独)。
    let poll_interval_multiplier = if flags.always_on {
        0.2
    } else if flags.power_save {
        3.0
    } else {
        1.0
    };

    // メモリ軸: 省電力・常時電源接続の状態に一切依存しない独立軸。
    let memory_cache_limit_factor = if flags.memory_saver { 0.25 } else { 1.0 };

    EffectivePowerSettings {
        poll_interval_multiplier,
        memory_cache_limit_factor,
    }
}

/// プロセス全体で共有する現在のフラグ集合(`RwLock`、`AppState`から
/// `Arc`で共有)。管理APIから書き換えられ、バックグラウンドループは
/// **毎回のイテレーションでこの値を読み直す**(起動時に一度だけ固定値を
/// キャプチャするのではない——これが「途中からでも切替可能」の実体、
/// 組み合わせ選択に再設計した後もこの性質は維持する)。
#[derive(Debug, Default)]
pub struct PowerProfileRegistry {
    current: RwLock<PowerProfileFlags>,
}

impl PowerProfileRegistry {
    pub fn new() -> Self {
        Self {
            current: RwLock::new(PowerProfileFlags::default()),
        }
    }

    pub fn get(&self) -> PowerProfileFlags {
        *self.current.read().expect("power profile lock poisoned")
    }

    pub fn set(&self, flags: PowerProfileFlags) {
        *self.current.write().expect("power profile lock poisoned") = flags;
    }
}

/// 基準のポーリング間隔(例: DDNS/free_domainループの既定5分)に、現在の
/// フラグ組み合わせから合成された倍率を適用した実際の待機時間を返す。
/// 呼び出し側はループの毎イテレーションでこれを呼ぶこと(起動時に一度だけ
/// 計算した値をキャプチャして使い回さない)。
pub fn effective_poll_interval(registry: &PowerProfileRegistry, base: Duration) -> Duration {
    let settings = effective_settings(registry.get());
    base.mul_f64(settings.poll_interval_multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pref_values_match_android_power_profile_kt() {
        assert_eq!(PowerProfileFlag::MemorySaver.pref_value(), "memory_saver");
        assert_eq!(PowerProfileFlag::PowerSave.pref_value(), "power_save");
        assert_eq!(PowerProfileFlag::AlwaysOn.pref_value(), "always_on");
    }

    #[test]
    fn from_pref_value_round_trips() {
        for f in [
            PowerProfileFlag::MemorySaver,
            PowerProfileFlag::PowerSave,
            PowerProfileFlag::AlwaysOn,
        ] {
            assert_eq!(PowerProfileFlag::from_pref_value(f.pref_value()), Some(f));
        }
        assert_eq!(PowerProfileFlag::from_pref_value("bogus"), None);
    }

    #[test]
    fn default_is_normal_with_no_flags() {
        let reg = PowerProfileRegistry::new();
        let flags = reg.get();
        assert!(flags.is_normal());
        assert_eq!(flags.active_pref_values(), Vec::<&str>::new());
        assert_eq!(flags.active_labels(), vec!["通常"]);
    }

    #[test]
    fn empty_array_explicitly_selects_normal() {
        let flags = PowerProfileFlags::from_pref_values::<&str>(&[]).unwrap();
        assert!(flags.is_normal());
    }

    #[test]
    fn normal_string_is_treated_as_no_flags_for_back_compat() {
        let flags = PowerProfileFlags::from_pref_values(&["normal"]).unwrap();
        assert!(flags.is_normal());
    }

    #[test]
    fn unknown_value_is_rejected() {
        let err = PowerProfileFlags::from_pref_values(&["bogus"]).unwrap_err();
        assert_eq!(err, "bogus");
    }

    #[test]
    fn set_then_get_reflects_new_value_immediately_without_restart() {
        let reg = PowerProfileRegistry::new();
        reg.set(PowerProfileFlags::from_pref_values(&["power_save"]).unwrap());
        assert!(reg.get().power_save);
        reg.set(PowerProfileFlags::from_pref_values(&["always_on"]).unwrap());
        assert!(reg.get().always_on);
        assert!(!reg.get().power_save);
    }

    // --- 個別フラグの既存効果がそのまま維持されていること ---

    #[test]
    fn individual_power_save_triples_poll_interval() {
        let flags = PowerProfileFlags::from_pref_values(&["power_save"]).unwrap();
        let s = effective_settings(flags);
        assert_eq!(s.poll_interval_multiplier, 3.0);
        assert_eq!(s.memory_cache_limit_factor, 1.0);
    }

    #[test]
    fn individual_always_on_shortens_poll_interval() {
        let flags = PowerProfileFlags::from_pref_values(&["always_on"]).unwrap();
        let s = effective_settings(flags);
        assert_eq!(s.poll_interval_multiplier, 0.2);
        assert_eq!(s.memory_cache_limit_factor, 1.0);
    }

    #[test]
    fn individual_memory_saver_only_affects_memory_axis() {
        let flags = PowerProfileFlags::from_pref_values(&["memory_saver"]).unwrap();
        let s = effective_settings(flags);
        assert_eq!(s.poll_interval_multiplier, 1.0);
        assert_eq!(s.memory_cache_limit_factor, 0.25);
    }

    #[test]
    fn normal_has_no_effect_on_either_axis() {
        let s = effective_settings(PowerProfileFlags::default());
        assert_eq!(s.poll_interval_multiplier, 1.0);
        assert_eq!(s.memory_cache_limit_factor, 1.0);
    }

    // --- 組み合わせ(合成)ロジック ---

    #[test]
    fn memory_saver_plus_power_save_composes_both_effects_simultaneously() {
        let flags = PowerProfileFlags::from_pref_values(&["memory_saver", "power_save"]).unwrap();
        let s = effective_settings(flags);
        // 両方の効果が"合成"される: ポーリング間隔は省電力の3倍のまま、
        // かつメモリ係数も省メモリの0.25のまま——どちらかが失われたり
        // 上書きされたりしない。
        assert_eq!(s.poll_interval_multiplier, 3.0);
        assert_eq!(s.memory_cache_limit_factor, 0.25);
    }

    #[test]
    fn power_save_plus_always_on_always_on_wins_the_poll_interval_axis() {
        let flags = PowerProfileFlags::from_pref_values(&["power_save", "always_on"]).unwrap();
        let s = effective_settings(flags);
        // 矛盾する組み合わせ: 常時電源接続がバッテリー節約軸(ポーリング
        // 間隔の延長)を無効化し、自身の値(短縮)を優先する。
        assert_eq!(s.poll_interval_multiplier, 0.2);
    }

    #[test]
    fn always_on_does_not_override_independent_memory_saver_axis() {
        let flags =
            PowerProfileFlags::from_pref_values(&["power_save", "always_on", "memory_saver"])
                .unwrap();
        let s = effective_settings(flags);
        assert_eq!(s.poll_interval_multiplier, 0.2); // always_onがpower_saveに勝つ
        assert_eq!(s.memory_cache_limit_factor, 0.25); // だがmemory_saverは独立して有効なまま
    }

    #[test]
    fn all_three_flags_active_simultaneously() {
        let flags =
            PowerProfileFlags::from_pref_values(&["memory_saver", "power_save", "always_on"])
                .unwrap();
        assert!(flags.memory_saver && flags.power_save && flags.always_on);
        let labels = flags.active_labels();
        assert_eq!(labels, vec!["省メモリ", "省電力", "常時電源接続"]);
    }

    #[test]
    fn poll_interval_reflects_current_combination_live() {
        let reg = PowerProfileRegistry::new();
        let base = Duration::from_secs(300);

        assert_eq!(effective_poll_interval(&reg, base), base); // 通常既定

        reg.set(PowerProfileFlags::from_pref_values(&["power_save"]).unwrap());
        assert_eq!(effective_poll_interval(&reg, base), Duration::from_secs(900));

        reg.set(PowerProfileFlags::from_pref_values(&["always_on"]).unwrap());
        assert_eq!(effective_poll_interval(&reg, base), Duration::from_secs(60));

        // 組み合わせでも即座に反映される(再起動不要の既存性質の維持)。
        reg.set(PowerProfileFlags::from_pref_values(&["power_save", "memory_saver"]).unwrap());
        assert_eq!(effective_poll_interval(&reg, base), Duration::from_secs(900));
    }
}
