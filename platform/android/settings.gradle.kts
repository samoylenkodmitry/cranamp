// Cranamp consumes the published Cranpose Android distribution: the Gradle
// plugin and the `dev.cranpose` libraries are resolved by version, the same way
// any application outside the framework's repository resolves them. Nothing
// here points at the framework's source tree.
//
// `mavenLocal()` is listed first because that distribution is not on a remote
// repository yet; publish it once with `<cranpose>/android/gradlew
// publishToMavenLocal` and this build resolves it.
pluginManagement {
    repositories {
        mavenLocal()
        google()
        mavenCentral()
        gradlePluginPortal()
    }
    plugins {
        id("com.android.application") version "9.2.1"
        id("dev.cranpose.android") version "0.1.96"
    }
}

dependencyResolutionManagement {
    repositories {
        mavenLocal()
        google()
        mavenCentral()
    }
}

rootProject.name = "Cranamp"
include(":app")
