# Teste de HUD em Compose sobre a cena 3D

Teste de viabilidade feito no host KMP (ver [kmp-host.md](kmp-host.md)), **sem remover** o HUD em bevy_hui. Fica desligado por padrão.

## Como ligar
- Android (somente Debug; builds Release descartam o flag): `EXTRA_DEV_ENV=$'COMPOSE_HUD=1\n' ./gradlew :androidApp:assembleDebug` dentro de `kmp/`.
- Sem `COMPOSE_HUD=1`, o plugin Rust (`hud_bridge`) nem é registrado e o Compose não desenha nada (`GameHost.composeHud`). Teste: `hud_bridge::tests::the_hud_test_is_off_by_default`.
- iOS: o build phase não escreve `COMPOSE_HUD`, então fica desligado; o teste não foi feito lá.

## O que existe
- Bevy → Compose: `hud_state {name, silver, gold, in_room, reconnecting}` (`crates/game/src/hud_bridge.rs`), só quando muda.
- Compose → Bevy: `hud_toggle_navigator` abre/fecha o navegador de salas.
- Compose: barra translúcida, botões, `AlertDialog` e `ModalBottomSheet` (`kmp/shared/.../ComposeHud.kt`).

## Resultado (Galaxy S22, Debug)
| Item | Resultado |
|---|---|
| Overlay translúcido sobre o `SurfaceView` | funciona |
| `AlertDialog` | funciona, jogo rodando atrás |
| `ModalBottomSheet` | funciona, mas abre em janela própria que **traz as barras do sistema de volta** e muda o layout enquanto está aberto |
| Comando Compose → Bevy | funciona |
| FPS | 37–45 com overlay, 41–49 sem (amostras de `status.txt`, log de debug ligado) |
| Memória | RSS cerca de 430 MB nos dois casos |

## Decisão
O `ModalBottomSheet` **não será usado** no jogo. O problema das barras foi contornado em um teste no S22 (esconder as barras na janela do diálogo, como a Activity), mas o ajuste não foi mantido. O guia genérico está no bevy_kmp (`docs/sobrepor-compose.md`).
