// open-easy-web Android shell: single-Activity Kotlin app that launches the
// cross-compiled `open-easy-web-server` native binary via ProcessBuilder.
//
// 参照実装: ../../open-web-server/android (同じ設計思想、パッケージ名のみ
// tokyo.runo.openeasyweb として区別)。3電源プロファイル(省電力/通常/
// 常時電源接続)+ 電源抜き差し監視ダイアログを同じパターンで実装する。
plugins {
    id("com.android.application") version "8.7.2" apply false
    id("org.jetbrains.kotlin.android") version "2.0.21" apply false
}
