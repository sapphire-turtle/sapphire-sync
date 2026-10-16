// The Android shell around libsapphire_sync_mobile.so: GameActivity, and nothing of its
// own. `../build-android.sh` builds the library into src/main/jniLibs first.
plugins {
    id("com.android.application")
}

android {
    namespace = "io.github.sapphireturtle.sync"
    compileSdk = 35

    defaultConfig {
        applicationId = "io.github.sapphireturtle.sync"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
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
}

dependencies {
    // AndroidX pulls Kotlin's stdlib at two versions (the jdk7/jdk8 split and the merged
    // one); the BOM lines them up.
    implementation(platform("org.jetbrains.kotlin:kotlin-bom:1.8.22"))
    // The version android-activity 0.6's glue is written against (its README).
    implementation("androidx.games:games-activity:4.4.0")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core:1.13.1")
}
