package tokyo.runo.openeasyweb

import android.content.Context
import android.content.SharedPreferences

/**
 * root化した端末で、USB OTG接続の外付けHDDをサーバーの主ストレージ
 * (`OPEN_EASYWEB_SITES_ROOT`配下のサイトファイル・`.open-easy-web-
 * users.json`・DB暗号化キー・AI学習状態の実データ保存先)として使う
 * ための設定(2026-08-04新設、`open-web-server/android`版の
 * `ExternalStorageConfig.kt`をそのまま移植)。
 *
 * **正直な前提(誇張しない)**: Android 10+のScoped Storage制限により、
 * root化していない端末では外部USBストレージへネイティブバイナリが
 * 直接POSIXファイルパスで読み書きできない(SAF経由の`content://`URIしか
 * 得られず`std::fs`は使えない)。この機能は**root化済みの端末専用**で
 * あり、非root端末では`MainActivity`側が`su`の到達性チェックで明確に
 * 検出し、有効化されていても起動を拒否して理由を表示する(黙って内部
 * ストレージへフォールバックしない)。
 */
object ExternalStorageConfig {
    private const val PREFS_NAME = "open_easy_web_external_storage_prefs"
    private const val KEY_ENABLED = "enabled"
    private const val KEY_MOUNT_PATH = "mount_path"

    /** サーバーの実データを置くサブディレクトリ名(マウントパス配下)。 */
    const val DATA_SUBDIR = "open-easy-web-data"

    private fun prefs(context: Context): SharedPreferences =
        context.applicationContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun isEnabled(context: Context): Boolean = prefs(context).getBoolean(KEY_ENABLED, false)

    fun getMountPath(context: Context): String? =
        prefs(context).getString(KEY_MOUNT_PATH, null)?.takeIf { it.isNotBlank() }

    fun save(context: Context, enabled: Boolean, mountPath: String) {
        prefs(context).edit()
            .putBoolean(KEY_ENABLED, enabled)
            .putString(KEY_MOUNT_PATH, mountPath.trim())
            .apply()
    }

    /** `mountPath`配下の実データディレクトリの絶対パス。 */
    fun dataDirPath(mountPath: String): String {
        val trimmed = mountPath.trim().trimEnd('/')
        return "$trimmed/$DATA_SUBDIR"
    }
}
