// Root Gradle entry so IntelliJ can open the repository root and edit the Rust crates and the
// Kotlin code in one project. The real Kotlin Multiplatform build lives in kmp/ (included below);
// build and run from kmp/ as usual (`cd kmp && ./gradlew :androidApp:assembleDebug`).
rootProject.name = "resortclient"

includeBuild("kmp")
