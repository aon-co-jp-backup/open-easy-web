plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "tokyo.runo.openeasyweb"
    compileSdk = 35

    defaultConfig {
        applicationId = "tokyo.runo.openeasyweb"
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        // open-web-server/android と同じ理由(実機arm64-v8a+この開発機の
        // x86_64エミュレータ両対応)でこの2 ABIのみ同梱する。
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        viewBinding = false
    }

    // open-web-server/android と同じ理由: ネイティブライブラリをAPK内から
    // 直接実行させず、旧来通りnativeLibraryDir配下に展開させる
    // (ProcessBuilderで実ファイルパスとして起動する必要があるため)。
    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
}
