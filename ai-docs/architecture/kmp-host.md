# Host Kotlin Multiplatform (mobile)

No mobile o jogo roda dentro de um app **Kotlin Multiplatform + Compose Multiplatform** que hospeda o Bevy numa view nativa, usando a biblioteca [`bevy-kmp`](https://gitlab.com/joaovictorsouza/bevy_kmp) (sem winit). O desktop continua com winit. Decisões: [ADR 0013](../decisions/0013-host-kmp-e-onboarding-nativo.md).

## Duas formas de buildar o crate `game`
| Feature | Onde | O que traz |
|---|---|---|
| `winit-host` (padrão) | desktop e o caminho antigo de mobile | `bevy/default` (winit), `main.rs`, `android_main`, `main_rs`, `bevy_ios_app_delegate`, `ios_window`, Keychain/JNI nativos do crate |
| `kmp-host` | Android e iOS no host KMP | `cdylib` + `staticlib` com `bevy_kmp_ffi::export!`; módulos `kmp_host` e `onboarding`; sem winit |

`cargo build -p game` continua igual para o desktop. O host KMP usa `--no-default-features --features kmp-host`; quem monta isso é o Gradle (Gobley) em `kmp/shared`.

## Estrutura
```
crates/game/src/kmp_host.rs     pre_plugins (assets, GAME_STORAGE_DIR, ponte Google), build_app, gpu_hook
crates/game/src/onboarding.rs   mensagens tipadas do onboarding nativo (ver abaixo)
kmp/shared                      módulo KMP: GameHost (UniFFI), App, Onboarding, GoogleAuth, serviços nativos
kmp/androidApp                  app Android (Activity, assets, dev.env)
kmp/iosApp                      app iOS (xcodegen: project.yml; Swift: ComposeView + GoogleProvider)
```

## O que mudou no jogo
- `cdn_assets::install_with`: o leitor dos assets embutidos é injetável. No Android o shell copia os assets do APK para `files/assets`; no iOS lê do bundle.
- `gpu_compat` virou `gpu_hook` (limites de skinning por uniform para Adreno 650/730).
- `game_log::custom_layer` entra como `log_layer`; os logs INFO+ também chegam ao Kotlin.
- `secure_store` e a ponte Google pedem ao shell por **serviços nativos** (`secure_get/put/delete`, `google_sign_in`) com `request_service_blocking`; o Keychain/Keystore vivem no Kotlin.
- `SafeArea` vem do resource `Insets` do `bevy-kmp` (dp da orientação atual).
- A reconexão do Nakama ao voltar do background usa o `AppLifecycle` do Bevy, que o `bevy-kmp` escreve ao pausar/retomar.
- Todas as telas continuam em `bevy_hui`, exceto o onboarding.

## Telas já portadas para Compose
- As pontes de telas (`wallet_bridge`, `game_state_bridge`, `screens_bridge`) ficam **sempre ligadas** no host KMP; só a barra de teste do HUD (`hud_bridge`) depende de `COMPOSE_HUD=1`.
- **HUD**: `HudView` (título da sala, prata/ouro, loja, avatar, e para o dono construir/configurações, mais inventário e navegador) lê `game_state` + `wallet_state`; só aparece em sala. No host KMP o `ScreensBridgePlugin` insere o marcador `ComposeHud` e o `setup_hud` do bevy_hui não cria a barra (o desktop continua com ela). Os botões usam os mesmos efeitos dos antigos: `shop_open`/`navigator_open` agora chamam `set_shop_open`/`set_navigator_open` (fecham o inventário/configurações e disparam os `Fetch*`), `build_mode_set`, `inventory_open` e dois comandos novos, `hud_avatar_menu` (menu do avatar, ainda bevy_hui) e `hud_room_settings` (configurações da sala, só dono, ainda bevy_hui). Seguem em bevy_hui: loja, navegador, criador, menu do avatar, configurações da sala, campainha e o menu de contexto/ferramentas do modo construção (girar, mover, guardar, Esc/R).
- **Inventário**: `InventoryView` é uma **janela flutuante** (cartão arrastável pelo título, limitado à área segura, sem scrim: toques fora dela chegam ao jogo) com busca, categorias, miniatura, nome e quantidade. Lê `inventory_state`; cada item traz `thumbnail`, o arquivo local da miniatura no cache da CDN (vazio até baixar; o Rust pede o download). Toque num item = `inventory_select` (`inventory::start_placing`: o dono entra no modo construção com o item e a janela fecha). Segurar e arrastar para fora da janela = `inventory_drop {catalog_id,x,y}` com **pixels físicos da superfície** (o mesmo espaço do `pointer` do bevy-kmp); o Rust divide por `window.scale_factor()`, faz o raio no chão (`furniture::ray_floor_tile`), valida com `placement_valid` (dono, piso, sem sobrepor sólido) e manda `PlaceItem`; drop inválido não faz nada. Animais não são arrastáveis.

## Onboarding nativo (Compose)
O estado e as regras continuam em Rust (`identity.rs`: `apply_google_result`, `persist_refresh_token`, `submit_nickname`, `apply_account_check`). O Compose só desenha:
- Rust → Compose: `onboarding_stage {stage: loading|login|nickname|hidden, busy, error, dev}` (muda de estado → evento).
- Compose → Rust: `onboarding_google`, `onboarding_device_login` (fora de produção), `onboarding_nickname {nickname}`, `onboarding_reset` (fora de produção: esquece a sessão).
- O token Google atravessa o serviço `google_sign_in`; o refresh token fica no Keychain/Keystore; nada disso vai para log.
- As telas `bevy_hui` de login e nome não são criadas quando existe o resource `NativeOnboarding`.
- Conta que já tem nome e aparência no servidor (`get_avatar_appearance` diferente da padrão) pula o criador de personagem, mesmo em aparelho novo.

## Como buildar e rodar
```bash
# Android (APK debug; use o perfil sem debuginfo definido no Gradle)
cd kmp && ./gradlew :androidApp:assembleDebug && adb install -r androidApp/build/outputs/apk/debug/androidApp-debug.apk
# iOS (aparelho; a chave vem do ambiente e nunca é impressa)
NAKAMA_SERVER_KEY=... ./scripts/kmp-ios-device.sh <udid> Debug   # INSTALL=0 só compila, LAUNCH=0 não abre
```
Variáveis de runtime (`NAKAMA_HOST`, `NAKAMA_PORT`, `GAME_ENV`, `NAKAMA_SERVER_KEY`, `ASSETS_MANIFEST_URL`) viram `assets/dev.env` no build (Gradle no Android; build phase no iOS) a partir de `environments/<ENVIRONMENT>.env` e do ambiente.
Primeira compilação de um alvo: ~15 min; incrementais de Kotlin/Swift: 1 a 3 min. Cada alvo debug do Bevy ocupa vários GB em `target/`.

## Diagnóstico no aparelho
- `Documents/game.log` e `Documents/status.txt` (fps, memória, GPU, estágio do onboarding), lidos com `xcrun devicectl device copy from --domain-type appDataContainer ...`.
- iOS: `-ResetSession` como argumento de lançamento (ou o botão "Limpar sessão (dev)") apaga Keychain e perfil.

## O que NÃO está validado
Ver `docs/relatorio-final.md`.
