import gobley.gradle.cargo.dsl.android
import gobley.gradle.cargo.dsl.appleMobile
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlin.multiplatform)
    alias(libs.plugins.android.library)
    alias(libs.plugins.compose.multiplatform)
    alias(libs.plugins.compose.compiler)
    alias(libs.plugins.kotlin.atomicfu)
    alias(libs.plugins.gobley.cargo)
    alias(libs.plugins.gobley.uniffi)
}

// The game crate (cdylib + staticlib) is the one Rust artifact per platform. The `kmp-host`
// feature replaces the winit entry points with the bevy-kmp host.
cargo {
    packageDirectory = rootProject.layout.projectDirectory.dir("../crates/game")
    builds.android {
        variants {
            buildTaskProvider.configure {
                features.add("kmp-host")
                extraArguments.add("--no-default-features")
                // Debuginfo makes the debug libgame.so gigabytes large (an unsigned 2.7 GB APK).
                additionalEnvironment.put("CARGO_PROFILE_DEV_DEBUG", "0")
                additionalEnvironment.put("CARGO_PROFILE_DEV_STRIP", "debuginfo")
            }
        }
    }
    builds.appleMobile {
        variants { buildTaskProvider.configure { features.add("kmp-host"); extraArguments.add("--no-default-features") } }
    }
}

uniffi {
    generateFromLibrary {
        packageName = "br.eng.joaovictor.resort.ffi"
        cdylibName = "game"
    }
}

kotlin {
    androidTarget { compilerOptions { jvmTarget = JvmTarget.JVM_17 } }
    listOf(iosArm64(), iosSimulatorArm64()).forEach {
        it.binaries.framework {
            baseName = "Shared"
            isStatic = true
        }
    }
    sourceSets {
        commonMain.dependencies {
            api("br.eng.joaovictor:bevy-kmp:0.1.5")
            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.material3)
            implementation(compose.ui)
            implementation(libs.coroutines.core)
        }
        androidMain.dependencies {
            implementation("androidx.credentials:credentials:1.3.0")
            implementation("androidx.credentials:credentials-play-services-auth:1.3.0")
            implementation("com.google.android.libraries.identity.googleid:googleid:1.1.1")
            implementation(libs.coroutines.android)
        }
    }
}

android {
    namespace = "br.eng.joaovictor.resort.shared"
    compileSdk = libs.versions.compileSdk.get().toInt()
    defaultConfig {
        minSdk = libs.versions.minSdk.get().toInt()
        ndk.abiFilters += setOf("arm64-v8a")
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
