# Arquitetura Multi-Plataforma: Desktop, Android e iOS

O cliente foi projetado para compilar e executar em **Desktop** (macOS, Windows, Linux) e **Mobile** (Android e iOS).

## Estratégia de Compilação
- `crates/game/src/lib.rs`: Configurado no `Cargo.toml` como `crate-type = ["lib", "cdylib"]`.
  - No Android e no iOS, a biblioteca (`libgame.so` / `libgame.a`) é carregada pelo host KMP (feature `kmp-host`).
- `crates/game/src/main.rs`: Ponto de entrada padrão para builds Desktop.

## Abstração de Entradas (Mouse e Toque)
O sistema `handle_pointer_click_or_tap` abstrai o input:
- No Desktop: lê `ButtonInput<MouseButton>::just_pressed(MouseButton::Left)` e a posição do cursor.
- No Mobile: lê `Touches::iter_just_pressed()` e a posição de toque na tela.
Ambos projetam o mesmo raio 3D através da câmera para identificar o tile tocado.

## iOS e Android (host KMP)

O Android e o iOS nativos antigos (winit, pastas `android/` e `ios/`, scripts `run-android.sh`, `build-apk.sh`, `run-ios-sim.sh`, `build-ios-device.sh`, `export-ipa.sh`, `gen-ios-project.sh`) foram removidos no [ADR 0014](../decisions/0014-remocao-do-mobile-nativo-antigo.md). Hoje o mobile roda dentro do host Kotlin Multiplatform; veja [kmp-host.md](kmp-host.md) e [rodando-do-zero.md](rodando-do-zero.md).

- Nas Adrenos 650 e 730 há um workaround de skinning (`gpu_compat.rs`); ver [investigacao-android-skinning.md](../process/investigacao-android-skinning.md).
- O servidor local é alcançado no Android por `adb reverse tcp:7350 tcp:7350` (USB) ou `NAKAMA_HOST=<ip-do-mac>`.

## Host KMP (branch feat/bevy-kmp): toque no `bevy_picking` (resolvido)

No host KMP, os botões que dependem de `bevy_picking` (templates `bevy_hui`) não recebiam toque, enquanto a UI baseada em `Interaction` do `bevy_ui` (status panel) recebia. Causa: o `bevy-kmp` 0.1.2 só escrevia `TouchInput`, e o `bevy_picking` lê `WindowEvent`. Resolvido no `bevy-kmp` 0.1.3 (escreve os dois e não sintetiza mouse). A validação das telas `bevy_hui` no aparelho vem do teste manual do João.
