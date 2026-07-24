package tokyo.runo.openeasyweb

import android.content.Intent
import android.os.Bundle
import android.widget.Button
import androidx.appcompat.app.AppCompatActivity

/**
 * 起動時の電源プロファイル選択画面(LAUNCHER)。
 * open-web-server/androidの`ProfileSelectActivity`と同じ設計。
 */
class ProfileSelectActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_profile_select)

        findViewById<Button>(R.id.buttonPowerSave).setOnClickListener {
            launchWithProfile(PowerProfile.POWER_SAVE)
        }
        findViewById<Button>(R.id.buttonNormal).setOnClickListener {
            launchWithProfile(PowerProfile.NORMAL)
        }
        findViewById<Button>(R.id.buttonAlwaysOn).setOnClickListener {
            launchWithProfile(PowerProfile.ALWAYS_ON)
        }
    }

    private fun launchWithProfile(profile: PowerProfile) {
        PowerProfile.save(this, profile)
        val intent = Intent(this, MainActivity::class.java)
        intent.putExtra(MainActivity.EXTRA_PROFILE, profile.prefValue)
        startActivity(intent)
        finish()
    }
}
