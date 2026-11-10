// App module: packages the pre-built Rust native library into an APK that
// hosts the GPUI application through Android's NativeActivity.
//
// The Rust library is compiled separately with cargo-ndk and placed into
// app/src/main/jniLibs/<abi>/ (see scripts/build-android.ps1):
//
//   cargo ndk -t x86_64 -o android/app/src/main/jniLibs --platform 26 build
//   cd android && gradlew assembleDebug

plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "com.example.my_gpui_mobile"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.example.my_gpui_mobile"
        minSdk = 26          // Vulkan 1.0 is mandatory from API 26+
        targetSdk = 36
        versionCode = 1
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        // Emulator (x86_64) and physical devices (arm64-v8a).
        ndk {
            abiFilters += listOf("x86_64", "arm64-v8a")
        }

        // Consumed by AndroidManifest's `android.app.lib_name` meta-data.
        // Must match the cdylib name without the `lib` prefix / `.so` suffix.
        manifestPlaceholders["nativeLibraryName"] = "my_gpui_mobile"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            // Dev/perf-test app: sign release builds with the debug keystore
            // so `assembleRelease` produces an installable `app-release.apk`.
            signingConfig = signingConfigs.getByName("debug")
        }
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    // No CMake/ndk-build: the native library comes from cargo-ndk.
    externalNativeBuild {
        // Intentionally left empty.
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    packaging {
        // cargo already strips release builds; stripping again breaks
        // backtraces in logcat.
        jniLibs {
            keepDebugSymbols += listOf(
                "*/x86_64/libmy_gpui_mobile.so",
                "*/arm64-v8a/libmy_gpui_mobile.so"
            )
            // Load the .so straight from the APK (it is stored uncompressed
            // and page-aligned): no extraction means the install needs ~100 MB
            // less free space on /data.
            useLegacyPackaging = false
        }
    }

    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }
}

// Some androidx artifacts pull kotlin-stdlib-jdk7/jdk8 1.6.x while others
// bring kotlin-stdlib 1.8.x — the classes overlap and checkDuplicateClasses
// fails. Pin every kotlin-stdlib flavor to one version.
configurations.all {
    resolutionStrategy {
        force("org.jetbrains.kotlin:kotlin-stdlib:1.8.22")
        force("org.jetbrains.kotlin:kotlin-stdlib-jdk7:1.8.22")
        force("org.jetbrains.kotlin:kotlin-stdlib-jdk8:1.8.22")
    }
}

dependencies {
    // GpuiNotifications / GpuiActivity helpers
    implementation("androidx.core:core:1.12.0")
    // SplashScreen API used by GpuiActivity
    implementation("androidx.core:core-splashscreen:1.0.1")
    // BiometricPrompt used by GpuiAuthActivity
    implementation("androidx.biometric:biometric:1.1.0")
    // MediaSessionCompat used by GpuiMediaSession / GpuiInputActivity
    implementation("androidx.media:media:1.7.1")
    // FragmentActivity base class of the helper activities
    implementation("androidx.fragment:fragment:1.8.5")

    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
}
