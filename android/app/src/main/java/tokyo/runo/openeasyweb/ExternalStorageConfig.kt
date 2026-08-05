package tokyo.runo.openeasyweb

import android.content.Context
import android.content.SharedPreferences
import android.os.Build
import android.os.storage.StorageManager

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

    /**
     * 検知された外部ストレージ候補1件(2026-08-05新設、ユーザー指示
     * 「マイクロSDや外付けUSB HDD/SSD/nVME SSDなどを簡単接続後に簡単に
     * 選択可能にする」への対応)。`path`は`ExternalStorageConfig.save()`が
     * 保存するマウントパスとして直接使える値、`label`はダイアログに
     * 表示する人間向けの説明文(種別判別はベストエフォート)。
     */
    data class DetectedCandidate(val path: String, val label: String)

    /**
     * `StorageManager.getStorageVolumes()`(root不要、Android標準API)経由で
     * リムーバブルボリューム(SDカード・USBマスストレージ等)を列挙する。
     * 例外(権限不足・APIレベル差異等)は握りつぶして空リストへ
     * フォールバックする——検知機能自体の失敗でアプリ起動を止めない
     * 既存方針(root到達不可時の起動拒否とは別物)を踏襲。
     */
    fun detectViaStorageManager(context: Context): List<DetectedCandidate> {
        return try {
            val sm = context.applicationContext.getSystemService(Context.STORAGE_SERVICE) as? StorageManager
                ?: return emptyList()
            val volumes = sm.storageVolumes
            volumes.mapNotNull { volume ->
                try {
                    if (!volume.isRemovable) return@mapNotNull null
                    val path = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                        volume.directory?.absolutePath
                    } else {
                        @Suppress("DEPRECATION")
                        volume::class.java.getMethod("getPath").invoke(volume) as? String
                    }
                    if (path.isNullOrBlank()) return@mapNotNull null
                    val description = try {
                        volume.getDescription(context)
                    } catch (e: Exception) {
                        null
                    }
                    val kind = classifyPath(path)
                    val label = buildString {
                        append(kind)
                        if (!description.isNullOrBlank()) {
                            append(" ($description)")
                        }
                        append(": $path")
                    }
                    DetectedCandidate(path = path, label = label)
                } catch (e: Exception) {
                    null
                }
            }
        } catch (e: Exception) {
            emptyList()
        }
    }

    /**
     * root権限がある場合に、`/proc/partitions`・`/dev/block/`から追加の
     * ブロックデバイス候補を収集する(rootで直接マウント/フォーマットする
     * 対象になりうるデバイスパス)。`isRootAvailable`は呼び出し側
     * (`MainActivity.isRootAvailable()`)の判定結果を渡してもらう
     * ——このオブジェクト自体はroot昇格の可否を判定しない。
     * 例外・コマンド失敗は握りつぶして空リストへフォールバックする。
     */
    fun detectViaRootBlockDevices(isRootAvailable: Boolean): List<DetectedCandidate> {
        if (!isRootAvailable) return emptyList()
        return try {
            val process = ProcessBuilder("su", "-c", "ls /dev/block/ 2>/dev/null").start()
            val finished = process.waitFor(3, java.util.concurrent.TimeUnit.SECONDS)
            if (!finished) return emptyList()
            val output = process.inputStream.bufferedReader().readText()
            output.lineSequence()
                .map { it.trim() }
                .filter { it.isNotBlank() && !it.startsWith("by-name") && !it.startsWith("platform") }
                .filter { name -> name.matches(Regex("(mmcblk\\d+p?\\d*|sd[a-z]\\d*|nvme\\d+n\\d+p?\\d*)")) }
                .map { name ->
                    val devicePath = "/dev/block/$name"
                    DetectedCandidate(path = devicePath, label = "${classifyPath(name)}(root): $devicePath")
                }
                .toList()
        } catch (e: Exception) {
            emptyList()
        }
    }

    /**
     * 検知した候補をまとめて取得する(`StorageManager`+root経由の合算)。
     * どちらか片方が失敗しても、もう片方の結果は活かす。
     */
    fun detectAllCandidates(context: Context, isRootAvailable: Boolean): List<DetectedCandidate> {
        val viaStorageManager = try {
            detectViaStorageManager(context)
        } catch (e: Exception) {
            emptyList()
        }
        val viaRoot = try {
            detectViaRootBlockDevices(isRootAvailable)
        } catch (e: Exception) {
            emptyList()
        }
        return (viaStorageManager + viaRoot).distinctBy { it.path }
    }

    /**
     * パス名のパターンからデバイス種別をベストエフォートで推測する。
     * 判別できない場合は「外部ストレージ候補」として一括りにする
     * (過剰な作り込みを避ける、ユーザー指示どおり)。
     */
    private fun classifyPath(path: String): String {
        return when {
            path.contains("mmcblk") -> "SDカード"
            path.contains("nvme") -> "NVMe SSD"
            Regex("(^|/)sd[a-z]").containsMatchIn(path) -> "USBストレージ"
            else -> "外部ストレージ候補"
        }
    }
}
