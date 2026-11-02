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
- **Visibilidade do HUD**: o HUD some (fade + desliza para cima, e sai da composição, então não intercepta toques do jogo) quando `game_state.full_screen_open` é `true`, isto é, quando alguma tela cheia/modal do bevy_hui está aberta: loja, navegador, configurações da sala (backdrop e janela grandes) ou criador de personagem. O menu do avatar e a campainha são popups pequenos sobre a cena e **não** escondem o HUD (`game_state_bridge::full_screen_open`). Vale também fora de sala (aí já some pela regra `in_room`). A janela flutuante do inventário esconde o HUD por conta própria (`HideHudWhileVisible("inventory")`): o HUD some enquanto ela está aberta, inclusive durante o arrasto, e volta assim que ela fecha (botão, colocação ou o próprio jogo). Para uma tela cheia em Compose esconder o HUD no futuro use `HideHudWhileVisible("nome-da-tela")` (ou `game.hudVisibility.hide(dono)` / `show(dono)`): o HUD fica escondido enquanto **qualquer** dono o segura (conjunto de donos; esconder duas vezes ou liberar um dono desconhecido não faz nada). Exemplo:

  ```kotlin
  @Composable
  fun MinhaTelaCheia() {
      HideHudWhileVisible("minha-tela") // HUD some ao entrar, volta ao sair
      // ...conteúdo...
  }
  ```
  Visibilidade final = em sala **e** nenhuma tela bevy_hui cheia **e** nenhum dono Compose segurando. Não coberto: o painel de dev (`status_panel`) e a tela de conexão/erro, que já não existem em sala.
- **Inventário**: `InventoryView` é uma **janela flutuante** (cartão arrastável pelo título, de até 420dp de largura (no máx. 60% da tela) e ~90% da altura da área segura, limitado a ela, sem scrim: toques fora dela chegam ao jogo) com busca, categorias, miniatura, nome e quantidade. Lê `inventory_state`; cada item traz `thumbnail`, o arquivo local da miniatura no cache da CDN (vazio até baixar; o Rust pede o download). Toque num item = `inventory_select` (`inventory::start_placing`: o dono entra no modo construção com o item e a janela fecha). Segurar e arrastar: enquanto o dedo está dentro da janela nada é enviado; ao sair dela a janela fica invisível (continua composta, para o gesto não morrer) e o shell manda `inventory_drag {catalog_id,x,y,phase}` a cada movimento (`move`), com **pixels físicos da superfície** (o espaço do `pointer` do bevy-kmp). O Rust divide por `window.scale_factor()`, entra no modo de colocação (`inventory::begin_placing`, sem fechar a janela) e alimenta o recurso `furniture::ExternalPointer`, que o `handle_ghost_preview` usa no lugar do ponteiro real (o Bevy não vê o dedo, que é do Compose): a caixa-fantasma (verde/vermelha) segue o dedo com a mesma validação da colocação normal. Soltar fora (`end`) confirma: coloca no tile sob o dedo se válido e fecha o inventário; se inválido, o jogo continua em modo de colocação com o item e um toque num tile válido coloca. Depois de sair, a saída é definitiva até soltar: a área da janela (agora invisível) também serve para posicionar, então dá para colocar no meio da tela. Só um gesto cancelado pelo sistema manda `cancel` e restaura o modo construção anterior. Só o dono (o snapshot traz `can_place`; para quem não é dono a janela avisa e desliga o arrastar); animais não são arrastáveis. Se o Rust recusar o arrasto (não é dono ou item inexistente) registra `inventory_drag ignored ... owner=.. item_in_inventory=..`. O Rust registra cada passo (`inventory_drag ... px=(..) logical=(..)`) no log do jogo.

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
