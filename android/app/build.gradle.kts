plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "io.github.muji4.nagi"
    compileSdk = 35

    defaultConfig {
        applicationId = "io.github.muji4.nagi"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        // hack/build-rust-android.sh が生成する .so の ABI に合わせる。
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
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
        compose = true
    }
    // uniffi-bindgen が生成する Kotlin（package uniffi.nagi_ffi）を取り込む。
    // 生成は hack/build-rust-android.sh が build/generated/uniffi へ行う。
    sourceSets["main"].java.srcDir("build/generated/uniffi")
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2024.10.01"))
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    // UniFFI が生成する Kotlin は JNA でネイティブライブラリを読み込む（必須）。
    implementation("net.java.dev.jna:jna:5.14.0@aar")
}
