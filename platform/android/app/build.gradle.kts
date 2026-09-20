// Cranamp's Android build, on the Cranpose Gradle plugin.
//
// The native build, the ABIs, the Cargo profile, the JNI packaging, the
// framework's Java and its manifest contributions all come from the plugin.
// What is left here is what is specific to this product: the store identity,
// the release versioning, and the signing.
plugins {
    id("com.android.application")
    id("dev.cranpose.android")
}

fun releaseVersionName(): String {
    val tag = System.getenv("GITHUB_REF_NAME")?.removePrefix("v")
    return tag?.takeIf { it.isNotBlank() } ?: "0.1.0"
}

fun releaseVersionCode(): Int {
    val version = releaseVersionName()
    val parts = version.split(".").mapNotNull { it.toIntOrNull() }
    if (parts.size != 3) {
        throw GradleException("version name '$version' is not MAJOR.MINOR.PATCH")
    }
    val (major, minor, patch) = parts
    return major * 1_000_000 + minor * 10_000 + patch
}

// Set by CI (decoded from the CRANAMP_RELEASE_KEYSTORE_BASE64 secret). Local
// builds without it sign with the debug keystore for emulator work.
val releaseKeystorePath: String? = System.getenv("CRANAMP_RELEASE_KEYSTORE")

fun requiredSigningEnv(name: String): String =
    System.getenv(name)
        ?: throw GradleException("$name must be set when CRANAMP_RELEASE_KEYSTORE is configured")

val isCiBuild = (System.getenv("CI") ?: "").isNotEmpty() ||
    (System.getenv("GITHUB_ACTIONS") ?: "").isNotEmpty()

cranpose {
    // `workspaceRoot` resolves relative to this Gradle project's own
    // directory (`platform/android/app`), not the root project's: three
    // directories up (app -> android -> platform -> repo root).
    workspaceRoot.set("../../..")
    cargoPackage.set("cranamp")
    features.set(listOf("android", "renderer-wgpu"))
    // The plugin builds debug variants for x86_64, which was the emulator's
    // architecture when every development machine was an Intel one. On an
    // Apple Silicon Mac both the emulator and the phone in the drawer are
    // arm64, and an APK carrying only x86_64 installs on neither: `adb
    // install` fails with INSTALL_FAILED_NO_MATCHING_ABIS.
    debugAbis.set(listOf("arm64-v8a"))
    releaseAbis.set(listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64"))
    label.set("Cranamp")
    // Playback continues with the app off screen, and the visualiser reads
    // analysis samples; the media module contributes the foreground service
    // and the permissions that needs, so the manifest declares neither.
    services.add("media")
    // The in-app updater hands its package to PackageInstaller, which Android
    // refuses without REQUEST_INSTALL_PACKAGES.
    services.add("update")
}

android {
    namespace = "com.cranamp.app"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.cranamp.app"
        minSdk = 26
        targetSdk = 36
        versionCode = releaseVersionCode()
        versionName = releaseVersionName()
    }

    signingConfigs {
        if (releaseKeystorePath != null) {
            create("release") {
                storeFile = file(releaseKeystorePath)
                storePassword = requiredSigningEnv("CRANAMP_RELEASE_KEYSTORE_PASSWORD")
                keyAlias = requiredSigningEnv("CRANAMP_RELEASE_KEY_ALIAS")
                keyPassword = requiredSigningEnv("CRANAMP_RELEASE_KEY_PASSWORD")
            }
        }
    }

    // CI release APKs ship one ABI each (app-<abi>-release.apk); phones never
    // download the emulator architectures. Which architectures those are comes
    // from `cranpose { releaseAbis }`, which the plugin writes into the split.
    splits {
        abi {
            isEnable = isCiBuild
            isUniversalApk = false
        }
    }

    buildTypes {
        debug {
            // A debug build is signed with the debug key, so Android refuses
            // to install it over the released Cranamp, which is signed with
            // the real one: INSTALL_FAILED_UPDATE_INCOMPATIBLE. Under its own
            // application id it goes on beside that copy instead, and neither
            // has to be uninstalled to make room for the other.
            applicationIdSuffix = ".debug"
        }
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
            signingConfig = if (releaseKeystorePath != null) {
                signingConfigs.getByName("release")
            } else {
                signingConfigs.getByName("debug")
            }
        }
    }
}
