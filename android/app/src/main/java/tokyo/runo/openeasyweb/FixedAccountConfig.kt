package tokyo.runo.openeasyweb

import android.content.Context
import android.content.SharedPreferences

/**
 * `open-easy-web-server`が起動時に要求する固定アカウント
 * (`OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL`、未設定だとサーバーがpanicして
 * 起動できない設計、`server/src/main.rs::fixed_account_email()`参照)を
 * Android側から設定するための最小限の永続化(2026-08-04新設)。
 *
 * **発見の経緯**: 外付けHDD主ストレージ対応機能の実装中に、
 * `startServerProcess()`がこの必須環境変数を一切設定していないため、
 * 現状のAndroid版は`open-easy-web-server`を一度も正常起動できない
 * (起動直後に確実にpanicする)ことが判明した。実機検証を行う前提として
 * 解消が必要なため、この機能追加と合わせて対応した。
 */
object FixedAccountConfig {
    private const val PREFS_NAME = "open_easy_web_fixed_account_prefs"
    private const val KEY_EMAIL = "email"

    private fun prefs(context: Context): SharedPreferences =
        context.applicationContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun getEmail(context: Context): String? =
        prefs(context).getString(KEY_EMAIL, null)?.takeIf { it.isNotBlank() }

    fun setEmail(context: Context, email: String) {
        prefs(context).edit().putString(KEY_EMAIL, email.trim()).apply()
    }
}
