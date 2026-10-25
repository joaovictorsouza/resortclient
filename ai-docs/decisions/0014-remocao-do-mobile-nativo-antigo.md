# ADR 0014: Remoção do mobile nativo antigo (winit)

## Status
Aceita (25/10/2026). Substitui o caminho de build mobile antigo descrito em `ai-docs/architecture/platforms.md` (versões anteriores), nas Specs 0009, 0010 e 0011 e no que o [ADR 0012](0012-cliente-publico-e-github-actions-manuais.md) previa para builds de APK/IPA (workflows manuais). Os textos dessas decisões não foram reescritos.

## Contexto
Desde o [ADR 0013](0013-host-kmp-e-onboarding-nativo.md) o jogo roda no mobile dentro de um host Kotlin Multiplatform (`kmp/`, feature `kmp-host`). O app Android (`android/`) e o app iOS (`ios/`) antigos, baseados em winit, ficaram sem uso e duplicavam o onboarding, o login Google e os scripts de build.

## Decisão
- Remover os projetos `android/` e `ios/` e os scripts que só serviam a eles (`build-android-lib.sh`, `build-apk.sh`, `run-android.sh`, `build-ios-device.sh`, `export-ipa.sh`, `gen-ios-project.sh`, `run-ios-sim.sh`, `load-environment.sh`). Android e iOS passam a ser buildados só por `kmp/` e `scripts/kmp-ios-device.sh`.
- **Mantidos por enquanto** (decisão do João, em aberto): os workflows `build-android.yml` e `build-ios.yml` e `scripts/ci/deliver-build.sh`, que chamam os scripts removidos e **deixam de funcionar** até serem reescritos ou apagados; e o código Rust condicionado a `winit-host` em mobile (`android_main`, `ios_window.rs`, `ios_google.rs`, `android_google.rs`, `bevy_ios_*`), que o desktop não usa mas está entrelaçado com `secure_store.rs`, `safe_area.rs` e `Cargo.toml`.
- A feature `winit-host` continua sendo o caminho do **desktop**.

## Consequências
- Um único caminho de build mobile (Gradle/Xcode via KMP). Os valores do Google Sign-In do iOS já estão em `kmp/iosApp/project.yml`.
- Arquivos locais ignorados (`ios/Config/Local.xcconfig`, `xcuserdata`) não foram tocados; ficam soltos na pasta `ios/` do disco do João até ele apagá-los.
- Próximo passo sugerido: um MR separado removendo o Rust morto do mobile antigo e os workflows, com `cargo check` desktop e `kmp-host` (também em `aarch64-apple-ios` e `aarch64-linux-android`).
