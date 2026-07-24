package tokyo.runo.openeasyweb

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

        registerPowerConnectionReceiver()
    }

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

            val pb = ProcessBuilder(binaryPath.absolutePath)
            pb.directory(filesDir)
            pb.environment()["OPEN_EASYWEB_SERVER_BIND"] = "127.0.0.1:$bindPort"
            // WASM UIバンドルは同梱していないため既定の"."のままで良い
            // (「/」は404になるが `/healthz`・`/api/...` は機能する、doc参照)。
            pb.redirectErrorStream(true)
            val process = pb.start()
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
