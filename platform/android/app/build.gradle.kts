import groovy.json.JsonSlurper

plugins {
    id("com.android.application")
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

fun cargoPackageDir(packageName: String): File {
    val output = providers.exec {
        commandLine(
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--manifest-path",
            rootProject.file("../../Cargo.toml").absolutePath
        )
        isIgnoreExitValue = true
    }
    val result = output.result.get()

    if (result.exitValue != 0) {
        throw GradleException("failed to resolve Cargo metadata for $packageName")
    }

    val metadata = JsonSlurper().parseText(output.standardOutput.asText.get()) as Map<*, *>
    val packages = metadata["packages"] as List<*>
    val manifestPath = packages
        .filterIsInstance<Map<*, *>>()
        .firstOrNull { it["name"] == packageName }
        ?.get("manifest_path") as? String
        ?: throw GradleException("Cargo metadata did not include package $packageName")

    return file(manifestPath).parentFile
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

    buildTypes {
        debug {
            ndk {
                abiFilters.add("x86_64")
            }
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

            ndk {
                abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
            }
        }
    }

    sourceSets {
        getByName("main") {
            java.directories.add(cargoPackageDir("cranpose").resolve("android/java").absolutePath)
        }
        getByName("debug") {
            jniLibs.directories.add("../target/android")
        }
        getByName("release") {
            jniLibs.directories.add("../target/android")
        }
    }
}

fun checkCargoNdk() {
    val result = providers.exec {
        commandLine("cargo", "ndk", "--version")
        isIgnoreExitValue = true
    }.result.get()

    if (result.exitValue != 0) {
        throw GradleException(
            "cargo-ndk is not installed. Install it with: cargo install cargo-ndk"
        )
    }
}

tasks.register<Exec>("buildRustDebug") {
    description = "Build Cranamp Rust library for Android debug."
    group = "rust"

    doFirst {
        checkCargoNdk()
    }

    workingDir = rootProject.projectDir

    commandLine("sh", "-c", """
        cargo ndk \
            --platform 26 \
            -t x86_64 \
            -o target/android \
            build \
            --manifest-path ../../Cargo.toml \
            --lib \
            --features android,renderer-wgpu \
            --no-default-features
    """)
}

tasks.register<Exec>("buildRustRelease") {
    description = "Build Cranamp Rust library for Android release."
    group = "rust"

    doFirst {
        checkCargoNdk()
    }

    workingDir = rootProject.projectDir

    commandLine("sh", "-c", """
        cargo ndk \
            --platform 26 \
            -t arm64-v8a \
            -t armeabi-v7a \
            -t x86 \
            -t x86_64 \
            -o target/android \
            build \
            --release \
            --manifest-path ../../Cargo.toml \
            --lib \
            --features android,renderer-wgpu \
            --no-default-features
    """)
}

afterEvaluate {
    tasks.matching { it.name.startsWith("merge") && it.name.contains("NativeLibs") }.configureEach {
        if (name.contains("Debug", ignoreCase = true)) {
            dependsOn("buildRustDebug")
        } else if (name.contains("Release", ignoreCase = true)) {
            dependsOn("buildRustRelease")
        }
    }
}
