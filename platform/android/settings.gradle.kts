// Cranamp's Android build runs the Cranpose Gradle plugin straight out of
// the `cranpose` crate source that Cargo already resolved for this workspace
// (registry cache, git checkout, or workspace path) -- the plugin is never
// published to Maven, so it is included as a composite build rather than
// resolved by a plugin id/version pair.
pluginManagement {
    val cranposePackage = (groovy.json.JsonSlurper().parseText(
        providers.exec { commandLine("cargo", "metadata", "--format-version=1") }
            .standardOutput.asText.get()
    ) as Map<*, *>)["packages"].let { it as List<*> }
        .map { it as Map<*, *> }
        .firstOrNull { it["name"] == "cranpose" }
        ?: error("cargo metadata reports no `cranpose` package; add it as a dependency first")
    val cranposeDir = java.io.File(cranposePackage["manifest_path"] as String).parentFile
    includeBuild(cranposeDir.resolve("android/cranpose-gradle-plugin"))

    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }

    plugins {
        id("com.android.application") version "9.2.1"
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "Cranamp"
include(":app")
