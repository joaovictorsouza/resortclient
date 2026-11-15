import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.compose.compiler)
}

// Runtime config the game reads from assets/dev.env (see parse_env_file in crates/game/src/lib.rs).
// Variables (never committed): NAKAMA_HOST, NAKAMA_PORT, GAME_DEV_MODE, GAME_ENV, NAKAMA_SERVER_KEY,
// ASSETS_MANIFEST_URL. Defaults come from environments/<ENVIRONMENT>.env.
fun envFile(name: String): Map<String, String> {
    val file = rootProject.file("../environments/$name.env")
    if (!file.exists()) return emptyMap()
    return file.readLines().map { it.trim() }
        .filter { it.isNotEmpty() && !it.startsWith("#") && it.contains('=') }
        .associate { it.substringBefore('=') to it.substringAfter('=') }
}

val gameEnvironment = System.getenv("ENVIRONMENT") ?: "dev"
val defaults = envFile(gameEnvironment)
fun setting(name: String, fallback: String = "") = System.getenv(name) ?: defaults[name] ?: fallback

val devEnvDir = layout.buildDirectory.dir("generated/devenv")
val writeDevEnv = tasks.register("writeDevEnv") {
    val lines = buildString {
        append("NAKAMA_HOST=${setting("NAKAMA_HOST", "127.0.0.1")}\n")
        append("NAKAMA_PORT=${setting("NAKAMA_PORT", "7350")}\n")
        append("GAME_DEV_MODE=${setting("GAME_DEV_MODE", "0")}\n")
        append("GAME_ENV=$gameEnvironment\n")
        setting("NAKAMA_SERVER_KEY").takeIf { it.isNotEmpty() }?.let { append("NAKAMA_SERVER_KEY=$it\n") }
        setting("ASSETS_MANIFEST_URL").takeIf { it.isNotEmpty() }?.let { append("ASSETS_MANIFEST_URL=$it\n") }
        // The Compose HUD test is a debug-only tool: release builds never carry the flag.
        val releaseBuild = gradle.startParameter.taskNames.any { it.contains("Release", ignoreCase = true) }
        System.getenv("EXTRA_DEV_ENV")?.let { extra ->
            append(extra.lineSequence().filterNot { releaseBuild && it.startsWith("COMPOSE_HUD") }.joinToString("\n"))
        }
    }
    inputs.property("content", lines)
    val out = devEnvDir.map { it.file("dev.env") }
    outputs.file(out)
    doLast {
        out.get().asFile.apply { parentFile.mkdirs() }.writeText(lines)
    }
}

android {
    namespace = "br.eng.joaovictor.resort"
    compileSdk = libs.versions.compileSdk.get().toInt()
    defaultConfig {
        applicationId = "br.eng.joaovictor.resort"
        minSdk = libs.versions.minSdk.get().toInt()
        targetSdk = libs.versions.targetSdk.get().toInt()
        versionCode = 2
        versionName = "0.2.0"
        ndk.abiFilters += setOf("arm64-v8a")
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    sourceSets {
        getByName("main") {
            assets.srcDirs("../../assets", devEnvDir)
        }
    }
    androidResources {
        // Template sources (Tailwind) are only needed to generate assets/ui/*.html.
        ignoreAssetsPattern = "!src:!.DS_Store"
    }
    // The Google OAuth Android client is registered for the project keystore's SHA-1, so with the
    // variables below (never committed) both debug and release builds are signed with it; without
    // them the Android debug key is used and Google sign-in is rejected by the Credential Manager.
    val projectKeystore = System.getenv("ANDROID_KEYSTORE")?.takeIf { it.isNotBlank() }
    val projectSigning = projectKeystore?.let {
        signingConfigs.create("project") {
            storeFile = file(it)
            storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
            keyAlias = System.getenv("ANDROID_KEY_ALIAS")
            keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
        }
    }
    buildTypes {
        debug {
            projectSigning?.let { signingConfig = it }
        }
        release {
            signingConfig = projectSigning ?: signingConfigs.getByName("debug")
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
}

tasks.matching { it.name.startsWith("merge") && it.name.endsWith("Assets") }.configureEach {
    dependsOn(writeDevEnv)
}

kotlin { compilerOptions { jvmTarget.set(JvmTarget.JVM_17) } }

dependencies {
    implementation(project(":shared"))
    implementation(libs.androidx.activity.compose)
}
