package tokyo.runo.openeasyweb

import android.app.ActivityManager
import android.content.ActivityNotFoundException
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.os.Bundle
import android.os.PowerManager
import android.widget.Button
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import java.io.BufferedReader
import java.io.File
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * open-easy-web Android版シェル(2026-07-24新規実装)。
 *
 * 参照実装: `open-web-server/android`の`MainActivity.kt`(3電源プロファイル
 * + 電源抜き差し監視ダイアログの設計をそのまま踏襲)。このActivity自体は
 * open-easy-webの機能(フォルダー作成・アップロード・AI判定・vhost自動
 * 構成)を一切実装しない。クロスコンパイル済みの`open-easy-web-server`
 * ネイティブ実行ファイル(`jniLibs/<abi>/libopeneasywebserver.so`——
 * open-web-server版と同じ、実行ファイルを`.so`の皮を被せてnativeLibraryDir
 * 配下に同梱する手法)を`ProcessBuilder`で起動し、自分自身へ`GET /healthz`
 * を投げて実際に応答することを画面上で確認できるようにする。
 *
 * **正直な開示(WASM UIについて)**: `open-easy-web-server`自体は
 * `GET /`で`OPEN_EASYWEB_STATIC_DIR`(既定`.`)配下の`index.html`を配信する
 * 設計だが、このAndroidアプリはWASM UIバンドル(`index.html`/`pkg/`)を
 * 同梱しない(過剰実装を避けるため——ビルド成果物を都度同梱する仕組みは
 * 今回のスコープ外)。そのため「ブラウザで開く」ボタンでサーバーの`/`を
 * 開いても、`OPEN_EASYWEB_STATIC_DIR`を別途配置していない限り404になる。
 * REST API(`/healthz`・`/api/...`)自体は同梱バイナリだけで完全に機能する。
 *
 * スコープ(意図的に今回含めない): フォアグラウンドサービス化、APK署名・
 * 配布、WASM UIバンドルの同梱、Doze中のネットワークI/O制限自体の回避。
 */
class MainActivity : AppCompatActivity() {

    companion object {
        const val EXTRA_PROFILE = "profile"
    }

    private var serverProcess: Process? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private val bindPort = 18090

    /**
     * 定期ヘルスチェックのポーリング間隔(open-web-server版と同じ施策:
     * 省電力版は間隔を大きく延ばしDoze/App Standbyへの影響を最小化し、
     * 常時電源接続版は短い間隔で即応性を優先する)。
     */
    private fun healthPollIntervalMs(profile: PowerProfile): Long = when (profile) {
        PowerProfile.POWER_SAVE -> 5 * 60_000L // 5分
        PowerProfile.NORMAL -> 60_000L // 1分
        PowerProfile.ALWAYS_ON -> 5_000L // 5秒
    }

    private var healthPollJob: Job? = null
    private var powerConnectionReceiver: BroadcastReceiver? = null

    /**
     * 別端末/別ホストで動くopen-easy-webサーバーへ接続するための設定
     * (ローカル同梱バイナリの代わりにリモートサーバーを使いたい場合の
     * 導線)。`SharedPreferences`に保存し、次回起動時も保持する。
     */
    private fun remoteServerUrlPrefs() = getSharedPreferences("open_easy_web_prefs", Context.MODE_PRIVATE)

    private fun serverBaseUrl(): String {
        val remote = remoteServerUrlPrefs().getString("remote_server_url", null)
        return if (!remote.isNullOrBlank()) remote.trimEnd('/') else "http://127.0.0.1:$bindPort"
    }

    private lateinit var currentProfile: PowerProfile

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        currentProfile = resolveProfile()
        PowerProfile.save(this, currentProfile)

        val statusText = findViewById<TextView>(R.id.statusText)
        val logText = findViewById<TextView>(R.id.logText)
        val startButton = findViewById<Button>(R.id.startButton)
        val openBrowserButton = findViewById<Button>(R.id.openBrowserButton)
        val changeProfileButton = findViewById<Button>(R.id.changeProfileButton)

        statusText.text =
            "open-easy-web [${currentProfile.emoji} ${currentProfile.label}モード] (not started)"

        startButton.setOnClickListener {
            startButton.isEnabled = false
            CoroutineScope(Dispatchers.Main).launch {
                val log = StringBuilder()
                log.appendLine("profile: ${currentProfile.label} (${currentProfile.prefValue})")
                statusText.text = "[${currentProfile.emoji} ${currentProfile.label}] starting..."
                val startResult = withContext(Dispatchers.IO) { startServerProcess(log) }
                if (!startResult) {
                    statusText.text = "[${currentProfile.emoji} ${currentProfile.label}] failed to start (see log)"
                    logText.text = log.toString()
                    startButton.isEnabled = true
                    return@launch
                }

                applyProfilePowerBehavior(log)

                val healthOk = withContext(Dispatchers.IO) { pollHealthz(log) }
                statusText.text = if (healthOk) {
                    "[${currentProfile.emoji} ${currentProfile.label}] RUNNING: GET /healthz responded 200"
                } else {
                    "[${currentProfile.emoji} ${currentProfile.label}] started, but /healthz did not respond (see log)"
                }
                logText.text = log.toString()
                startButton.isEnabled = true

                if (healthOk) {
                    startPeriodicHealthPoll(statusText)
                }
            }
        }

        openBrowserButton.setOnClickListener {
            openInBrowser()
        }

        changeProfileButton.setOnClickListener {
            startActivity(Intent(this, ProfileSelectActivity::class.java))
            finish()
        }

        val memoryInfoButton = findViewById<Button>(R.id.memoryInfoButton)
        memoryInfoButton.setOnClickListener {
            showMemoryInfoDialog()
        }

        val uninstallButton = findViewById<Button>(R.id.uninstallButton)
        uninstallButton.setOnClickListener {
            requestUninstall()
        }

        val externalStorageButton = findViewById<Button>(R.id.externalStorageButton)
        externalStorageButton.setOnClickListener {
            showExternalStorageDialog()
        }

        val fixedAccountButton = findViewById<Button>(R.id.fixedAccountButton)
        fixedAccountButton.setOnClickListener {
            showFixedAccountDialog()
        }

        registerPowerConnectionReceiver()
    }

    /**
     * 固定アカウントのメールアドレス設定ダイアログ(2026-08-04新設)。
     * `OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL`が未設定だとサーバーがpanicして
     * 起動できないため、この値を保存してもらうまでは
     * `startServerProcess()`側で明確に起動を拒否する(`FixedAccountConfig`
     * のdoc参照)。
     */
    private fun showFixedAccountDialog() {
        val container = android.widget.LinearLayout(this)
        container.orientation = android.widget.LinearLayout.VERTICAL
        val pad = (16 * resources.displayMetrics.density).toInt()
        container.setPadding(pad, pad, pad, pad)

        val messageView = TextView(this)
        messageView.text = getString(R.string.fixed_account_dialog_message)
        container.addView(messageView)

        val emailInput = android.widget.EditText(this)
        emailInput.hint = getString(R.string.fixed_account_email_hint)
        emailInput.inputType = android.text.InputType.TYPE_CLASS_TEXT or
            android.text.InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS
        emailInput.setText(FixedAccountConfig.getEmail(this) ?: "")
        container.addView(emailInput)

        AlertDialog.Builder(this)
            .setTitle(R.string.fixed_account_dialog_title)
            .setView(container)
            .setPositiveButton(R.string.external_storage_save_button) { _, _ ->
                val email = emailInput.text.toString().trim()
                if (email.isEmpty() || !email.contains("@")) {
                    Toast.makeText(this, "有効なメールアドレスを入力してください", Toast.LENGTH_LONG).show()
                    return@setPositiveButton
                }
                FixedAccountConfig.setEmail(this, email)
                Toast.makeText(this, "保存しました(次回サーバー起動から反映)", Toast.LENGTH_LONG).show()
            }
            .setNegativeButton("キャンセル", null)
            .show()
    }

    /**
     * 外付けHDD(root化端末専用)設定ダイアログ(2026-08-04新設、
     * `open-web-server/android`版と同一設計)。
     */
    private fun showExternalStorageDialog() {
        val container = android.widget.LinearLayout(this)
        container.orientation = android.widget.LinearLayout.VERTICAL
        val pad = (16 * resources.displayMetrics.density).toInt()
        container.setPadding(pad, pad, pad, pad)

        val messageView = TextView(this)
        messageView.text = getString(R.string.external_storage_dialog_message)
        container.addView(messageView)

        val pathInput = android.widget.EditText(this)
        pathInput.hint = getString(R.string.external_storage_path_hint)
        pathInput.setText(ExternalStorageConfig.getMountPath(this) ?: "")
        container.addView(pathInput)

        val enableCheckbox = android.widget.CheckBox(this)
        enableCheckbox.text = getString(R.string.external_storage_enable_checkbox)
        enableCheckbox.isChecked = ExternalStorageConfig.isEnabled(this)
        container.addView(enableCheckbox)

        AlertDialog.Builder(this)
            .setTitle(R.string.external_storage_dialog_title)
            .setView(container)
            .setPositiveButton(R.string.external_storage_save_button) { _, _ ->
                val path = pathInput.text.toString().trim()
                if (enableCheckbox.isChecked && path.isEmpty()) {
                    Toast.makeText(this, "マウントパスを入力してください", Toast.LENGTH_LONG).show()
                    return@setPositiveButton
                }
                ExternalStorageConfig.save(this, enableCheckbox.isChecked, path)
                Toast.makeText(this, "保存しました(次回サーバー起動から反映)", Toast.LENGTH_LONG).show()
            }
            .setNegativeButton("キャンセル", null)
            .show()
    }

    /**
     * `su`(root権限昇格)へ実際に到達できるか同期的に確認する。
     */
    private fun isRootAvailable(): Boolean {
        return try {
            val process = ProcessBuilder("su", "-c", "id").start()
            val finished = process.waitFor(3, java.util.concurrent.TimeUnit.SECONDS)
            finished && process.exitValue() == 0
        } catch (e: Exception) {
            false
        }
    }

    /**
     * `su -c`へ渡すシェルコマンド文字列組み立て用のシングルクォート
     * エスケープ(コマンドインジェクション対策)。
     */
    private fun shellQuote(value: String): String =
        "'" + value.replace("'", "'\\''") + "'"

    /**
     * 電源の抜き差しを監視する(open-web-server版と同じ設計)。
     */
    private fun registerPowerConnectionReceiver() {
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context, intent: Intent) {
                when (intent.action) {
                    Intent.ACTION_POWER_DISCONNECTED -> onPowerDisconnected()
                    Intent.ACTION_POWER_CONNECTED -> onPowerConnected()
                }
            }
        }
        powerConnectionReceiver = receiver
        val filter = IntentFilter().apply {
            addAction(Intent.ACTION_POWER_DISCONNECTED)
            addAction(Intent.ACTION_POWER_CONNECTED)
        }
        registerReceiver(receiver, filter)
    }

    /**
     * 実メモリ(物理RAM)+仮想メモリ(スワップ)の使用状況を表示する
     * (2026-07-31追加、ユーザー指示「スマホとタブレットは実メモリ+仮想
     * メモリを表示する機能も搭載」)。日英Web検索で裏取り済みのAndroid標準
     * API: 実メモリは`ActivityManager.getMemoryInfo()`(`totalMem`/
     * `availMem`、システム全体の値、Android全バージョンで利用可能な公式
     * API)。仮想メモリ(スワップ)はAndroidに`ActivityManager`経由の直接API
     * が無いため、Linuxカーネル標準の`/proc/meminfo`
     * (`SwapTotal`/`SwapFree`、Androidも内部はLinuxカーネルのため同じ
     * ファイルが存在する——rootや特別な権限は不要、一般アプリから読み取り
     * 可能)を直接パースする。**正直な開示**: 一部の非常に制限された
     * カスタムROM/SELinuxポリシーでは`/proc/meminfo`が読めない場合が
     * あり得るため、読み取り失敗時は例外を投げずN/Aと表示する。
     */
    private fun showMemoryInfoDialog() {
        val am = getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
        val memInfo = ActivityManager.MemoryInfo()
        am.getMemoryInfo(memInfo)
        val totalRealMb = memInfo.totalMem / (1024 * 1024)
        val availRealMb = memInfo.availMem / (1024 * 1024)
        val usedRealMb = totalRealMb - availRealMb
        val usedRealPercent = if (totalRealMb > 0) usedRealMb * 100.0 / totalRealMb else 0.0

        val (totalSwapMb, freeSwapMb) = readProcMeminfoSwap()
        val swapLine = if (totalSwapMb == null) {
            "Virtual memory / swap (仮想メモリ/スワップ): N/A (could not read /proc/meminfo / 読み取れませんでした)"
        } else if (totalSwapMb == 0L) {
            "Virtual memory / swap (仮想メモリ/スワップ): N/A (not configured / 未設定)"
        } else {
            val usedSwapMb = totalSwapMb - (freeSwapMb ?: 0L)
            "Virtual memory / swap (仮想メモリ/スワップ): $usedSwapMb / $totalSwapMb MB"
        }

        AlertDialog.Builder(this)
            .setTitle("Memory info (メモリ情報)")
            .setMessage(
                "Physical memory (実メモリ) — Used (使用中): $usedRealMb MB / " +
                    "Total (合計): $totalRealMb MB (${"%.1f".format(usedRealPercent)}%)\n" +
                    "Available (空き): $availRealMb MB\n" +
                    "Low memory (低メモリ状態): ${memInfo.lowMemory}\n\n$swapLine"
            )
            .setPositiveButton("OK (閉じる)", null)
            .show()
    }

    /** `/proc/meminfo`から`SwapTotal`/`SwapFree`(いずれもkB単位、MBへ変換して
     * 返す)を読み取る。読み取り失敗時は`Pair(null, null)`を返す(例外を
     * 投げない、呼び出し側でN/A表示にする)。 */
    private fun readProcMeminfoSwap(): Pair<Long?, Long?> {
        return try {
            var totalKb: Long? = null
            var freeKb: Long? = null
            File("/proc/meminfo").bufferedReader().useLines { lines ->
                for (line in lines) {
                    if (line.startsWith("SwapTotal:")) {
                        totalKb = line.filter { it.isDigit() }.toLongOrNull()
                    } else if (line.startsWith("SwapFree:")) {
                        freeKb = line.filter { it.isDigit() }.toLongOrNull()
                    }
                    if (totalKb != null && freeKb != null) break
                }
            }
            Pair(totalKb?.div(1024), freeKb?.div(1024))
        } catch (e: Exception) {
            Pair(null, null)
        }
    }

    /**
     * アプリのアンインストールを要求する(2026-07-31追加、ユーザー指示
     * 「アプリのアンインストールも可能にして」)。日英Web検索で裏取り
     * 済みの標準Android API: `Intent.ACTION_DELETE`+`package:`Uriで
     * システム標準のアンインストール確認ダイアログを開く(このアプリ
     * コード自体がサイレントにアンインストールすることは不可能——
     * Android OSの仕様上、必ずユーザーの明示的な確認ダイアログを経由する
     * 設計になっており、これはセキュリティ上意図された制約であり本アプリ
     * の実装上の制限ではない)。確認後の実削除自体はOSが行う。
     */
    private fun requestUninstall() {
        AlertDialog.Builder(this)
            .setTitle("Uninstall (アンインストール)")
            .setMessage(
                "Open the system uninstall confirmation dialog for this app?\n" +
                    "このアプリのアンインストール確認ダイアログを開きますか?"
            )
            .setPositiveButton("Uninstall (アンインストール)") { _, _ ->
                try {
                    val intent = Intent(Intent.ACTION_DELETE, Uri.parse("package:$packageName"))
                    startActivity(intent)
                } catch (e: ActivityNotFoundException) {
                    Toast.makeText(this, "Could not open uninstall dialog (アンインストール画面を開けませんでした)", Toast.LENGTH_LONG).show()
                }
            }
            .setNegativeButton("Cancel (キャンセル)", null)
            .show()
    }

    private fun onPowerDisconnected() {
        if (currentProfile != PowerProfile.ALWAYS_ON) return
        if (isFinishing || isDestroyed) return
        AlertDialog.Builder(this)
            .setTitle("電源が外れました")
            .setMessage(
                "常時電源接続モードで動作中に電源が外れました。\n" +
                    "省電力モードに切り替えますか?それとも通常モードの" +
                    "ままにしますか?\n(推奨: 省電力モード)"
            )
            .setPositiveButton("省電力モードへ切替") { _, _ ->
                switchProfileAndRestart(PowerProfile.POWER_SAVE)
            }
            .setNegativeButton("通常モードのままにする") { _, _ ->
                switchProfileAndRestart(PowerProfile.NORMAL)
            }
            .setCancelable(false)
            .show()
    }

    private fun onPowerConnected() {
        if (currentProfile == PowerProfile.ALWAYS_ON) return
        if (isFinishing || isDestroyed) return
        AlertDialog.Builder(this)
            .setTitle("電源が接続されました")
            .setMessage("常時電源接続モードに切り替えますか?")
            .setPositiveButton("常時電源接続へ切替") { _, _ ->
                switchProfileAndRestart(PowerProfile.ALWAYS_ON)
            }
            .setNegativeButton("このままにする", null)
            .show()
    }

    private fun switchProfileAndRestart(newProfile: PowerProfile) {
        PowerProfile.save(this, newProfile)
        Toast.makeText(
            this,
            "${newProfile.emoji} ${newProfile.label}モードへ切り替えます",
            Toast.LENGTH_SHORT
        ).show()
        val intent = Intent(this, MainActivity::class.java)
        intent.putExtra(EXTRA_PROFILE, newProfile.prefValue)
        startActivity(intent)
        finish()
    }

    private fun resolveProfile(): PowerProfile {
        return when (intent?.action) {
            "tokyo.runo.openeasyweb.LAUNCH_POWER_SAVE" -> PowerProfile.POWER_SAVE
            "tokyo.runo.openeasyweb.LAUNCH_NORMAL" -> PowerProfile.NORMAL
            "tokyo.runo.openeasyweb.LAUNCH_ALWAYS_ON" -> PowerProfile.ALWAYS_ON
            else -> {
                val extra = intent?.getStringExtra(EXTRA_PROFILE)
                if (extra != null) PowerProfile.fromPrefValue(extra) else PowerProfile.load(this)
            }
        }
    }

    /**
     * プロファイルごとの電源管理の中身(open-web-server版と同じ):
     * - 省電力/通常: `WakeLock`を一切取得しない。
     * - 常時電源接続: `PARTIAL_WAKE_LOCK`を保持する。
     */
    private fun applyProfilePowerBehavior(log: StringBuilder) {
        when (currentProfile) {
            PowerProfile.ALWAYS_ON -> {
                try {
                    val pm = getSystemService(POWER_SERVICE) as PowerManager
                    val lock = pm.newWakeLock(
                        PowerManager.PARTIAL_WAKE_LOCK,
                        "OpenEasyWeb::AlwaysOnWakeLock"
                    )
                    lock.acquire()
                    wakeLock = lock
                    log.appendLine("power: acquired PARTIAL_WAKE_LOCK (always-on profile)")
                } catch (e: Exception) {
                    log.appendLine("power: failed to acquire WakeLock: ${e.message}")
                }
            }
            PowerProfile.POWER_SAVE -> {
                log.appendLine("power: no WakeLock acquired (power-save profile, Doze-friendly)")
            }
            PowerProfile.NORMAL -> {
                log.appendLine("power: no WakeLock acquired (normal profile)")
            }
        }
    }

    private fun openInBrowser() {
        try {
            val intent = Intent(Intent.ACTION_VIEW, Uri.parse(serverBaseUrl() + "/"))
            startActivity(intent)
        } catch (e: ActivityNotFoundException) {
            Toast.makeText(this, "ブラウザが見つかりません: ${serverBaseUrl()}", Toast.LENGTH_LONG).show()
        }
    }

    private fun startServerProcess(log: StringBuilder): Boolean {
        return try {
            val binaryPath = File(applicationInfo.nativeLibraryDir, "libopeneasywebserver.so")
            log.appendLine("binary path: ${binaryPath.absolutePath}")
            log.appendLine("binary exists: ${binaryPath.exists()}")
            if (!binaryPath.exists()) {
                log.appendLine("ERROR: native binary not found — was the app built with jniLibs populated by cargo ndk?")
                return false
            }

            // `open-easy-web-server`は`OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL`
            // 未設定だと起動直後にpanicする設計(server/src/main.rs
            // `fixed_account_email()`)。外部ストレージ機能の実装中に
            // この必須環境変数がAndroid版から一切設定されていないことが
            // 判明したため、ここで明確に検査し起動を拒否する
            // (`FixedAccountConfig`のdoc参照——黙ってpanicさせるより、
            // 起動前に理由を表示するほうが親切なため)。
            val fixedAccountEmail = FixedAccountConfig.getEmail(this)
            if (fixedAccountEmail.isNullOrBlank()) {
                log.appendLine(
                    "ERROR: fixed account email is not configured — " +
                        "open-easy-web-server requires OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL " +
                        "and will panic on startup without it. Use the " +
                        "'固定アカウント設定' button first."
                )
                return false
            }

            // 外付けHDD(root化端末専用)を主ストレージにする設定
            // (2026-08-04新設、`open-web-server/android`版と同一設計)。
            // 有効化されている場合はroot到達性を実際に確認し、確認できな
            // ければ黙って内部ストレージへフォールバックせず起動を中止する。
            val useExternalStorage = ExternalStorageConfig.isEnabled(this)
            if (useExternalStorage) {
                val mountPath = ExternalStorageConfig.getMountPath(this)
                if (mountPath.isNullOrBlank()) {
                    log.appendLine("ERROR: external storage is enabled but no mount path is configured")
                    return false
                }
                log.appendLine("external storage requested: $mountPath (checking root access...)")
                if (!isRootAvailable()) {
                    log.appendLine(
                        "ERROR: root access ('su') is not available on this device — " +
                            "external HDD storage requires a rooted device (Android Scoped Storage " +
                            "blocks direct file access to USB storage otherwise). " +
                            "Falling back to internal storage was intentionally NOT done."
                    )
                    return false
                }
                log.appendLine("root access confirmed, launching via 'su' with data dir on external storage")
            }

            val process: Process
            if (useExternalStorage) {
                val mountPath = ExternalStorageConfig.getMountPath(this)!!
                val dataDir = ExternalStorageConfig.dataDirPath(mountPath)
                // `su -c`は非rootの起動元プロセス環境を継承しない前提の
                // ため、全て`export`込みの1コマンド文字列として組み立てる
                // (open-web-server/android版と同一パターン)。
                val script = buildString {
                    append("mkdir -p ${shellQuote(dataDir)}; ")
                    append("cd ${shellQuote(dataDir)} && ")
                    append("export OPEN_EASYWEB_SERVER_BIND=${shellQuote("127.0.0.1:$bindPort")}; ")
                    append("export OPEN_EASYWEB_SITES_ROOT=${shellQuote("$dataDir/sites")}; ")
                    append("export OPEN_EASYWEB_USERS_STATE=${shellQuote("$dataDir/.open-easy-web-users.json")}; ")
                    append("export OPEN_EASYWEB_DB_ENCRYPTION_KEY_FILE=${shellQuote("$dataDir/.open-easy-web-db-encryption.key")}; ")
                    append("export OPEN_EASYWEB_AI_STATE=${shellQuote("$dataDir/.open-easy-web-ai-state.json")}; ")
                    append("export OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL=${shellQuote(fixedAccountEmail)}; ")
                    append("exec ${shellQuote(binaryPath.absolutePath)}")
                }
                log.appendLine("data dir on external storage: $dataDir")
                val pb = ProcessBuilder("su", "-c", script)
                pb.redirectErrorStream(true)
                process = pb.start()
            } else {
                val pb = ProcessBuilder(binaryPath.absolutePath)
                pb.directory(filesDir)
                pb.environment()["OPEN_EASYWEB_SERVER_BIND"] = "127.0.0.1:$bindPort"
                pb.environment()["OPEN_EASYWEB_FIXED_ACCOUNT_EMAIL"] = fixedAccountEmail
                // WASM UIバンドルは同梱していないため既定の"."のままで良い
                // (「/」は404になるが `/healthz`・`/api/...` は機能する、doc参照)。
                pb.redirectErrorStream(true)
                process = pb.start()
            }
            serverProcess = process

            Thread {
                try {
                    BufferedReader(InputStreamReader(process.inputStream)).use { reader ->
                        var line: String?
                        while (reader.readLine().also { line = it } != null) {
                            android.util.Log.i("open-easy-web", line ?: "")
                        }
                    }
                } catch (_: Exception) {
                    // プロセス終了時にストリームが閉じるのは正常系。
                }
            }.start()

            log.appendLine("process started (alive=${process.isAlive})")
            true
        } catch (e: Exception) {
            log.appendLine("ERROR launching process: ${e}")
            false
        }
    }

    private fun startPeriodicHealthPoll(statusText: TextView) {
        healthPollJob?.cancel()
        val intervalMs = healthPollIntervalMs(currentProfile)
        healthPollJob = CoroutineScope(Dispatchers.Main).launch {
            while (isActive) {
                delay(intervalMs)
                val ok = withContext(Dispatchers.IO) {
                    try {
                        val url = URL("http://127.0.0.1:$bindPort/healthz")
                        val conn = url.openConnection() as HttpURLConnection
                        conn.connectTimeout = 1000
                        conn.readTimeout = 1000
                        val code = conn.responseCode
                        conn.disconnect()
                        code == 200
                    } catch (_: Exception) {
                        false
                    }
                }
                statusText.text = if (ok) {
                    "[${currentProfile.emoji} ${currentProfile.label}] RUNNING " +
                        "(poll every ${intervalMs / 1000}s)"
                } else {
                    "[${currentProfile.emoji} ${currentProfile.label}] health check failed"
                }
            }
        }
    }

    private fun pollHealthz(log: StringBuilder): Boolean {
        repeat(10) { attempt ->
            try {
                Thread.sleep(300)
                val url = URL("http://127.0.0.1:$bindPort/healthz")
                val conn = url.openConnection() as HttpURLConnection
                conn.connectTimeout = 1000
                conn.readTimeout = 1000
                val code = conn.responseCode
                val body = conn.inputStream.bufferedReader().readText()
                conn.disconnect()
                log.appendLine("attempt ${attempt + 1}: GET /healthz -> $code \"$body\"")
                if (code == 200) return true
            } catch (e: Exception) {
                log.appendLine("attempt ${attempt + 1}: GET /healthz failed: ${e.message}")
            }
        }
        return false
    }

    override fun onDestroy() {
        super.onDestroy()
        healthPollJob?.cancel()
        powerConnectionReceiver?.let {
            try {
                unregisterReceiver(it)
            } catch (_: IllegalArgumentException) {
                // 未登録のまま呼ばれても無視する。
            }
        }
        serverProcess?.destroy()
        if (wakeLock?.isHeld == true) {
            wakeLock?.release()
        }
    }
}
