package tokyo.runo.openeasyweb

import android.content.Context

/**
 * 3電源プロファイル(open-web-server/android の`PowerProfile.kt`と同じ
 * 設計、2026-07-24 open-easy-web向けに新規移植)。
 *
 * - [POWER_SAVE] 省電力版: バックグラウンドでの常時稼働を避け、Android
 *   Doze/App Standbyに逆らわない(=`WakeLock`を一切取得しない)。
 * - [NORMAL] 通常版: 上記2つの中間。バランス型(既定値)。
 * - [ALWAYS_ON] 常時電源接続版: 充電器に繋ぎっぱなしのサーバー専用機
 *   向け。`PARTIAL_WAKE_LOCK`を保持し、画面消灯・Doze移行後もサーバー
 *   プロセスが確実に生き続けるようにする。
 *
 * **正直な開示**: open-web-server版と同様、「3電源プロファイルの最小
 * 実装」であり、Doze中のネットワークI/O制限自体の回避・バッテリー
 * 最適化ホワイトリスト登録UI・詳細な電力測定は含まない。
 */
enum class PowerProfile(val prefValue: String, val label: String, val emoji: String) {
    POWER_SAVE("power_save", "省電力", "🔋⚡️✕"),
    NORMAL("normal", "通常", "⚖️"),
    ALWAYS_ON("always_on", "常時電源接続", "🔌");

    companion object {
        private const val PREFS_NAME = "open_easy_web_prefs"
        private const val KEY_PROFILE = "power_profile"

        fun fromPrefValue(value: String?): PowerProfile =
            values().firstOrNull { it.prefValue == value } ?: NORMAL

        fun load(context: Context): PowerProfile {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            return fromPrefValue(prefs.getString(KEY_PROFILE, null))
        }

        fun save(context: Context, profile: PowerProfile) {
            val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            prefs.edit().putString(KEY_PROFILE, profile.prefValue).apply()
        }
    }
}
