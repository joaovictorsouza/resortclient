# ADR 0017: Telas do jogo migradas de bevy_hui para Compose no host KMP

## Status
Aceita

## Contexto
O mobile roda dentro de um host Kotlin Multiplatform ([ADR 0013](./0013-host-kmp-e-onboarding-nativo.md)), mas as telas do jogo (HUD, inventário, loja, navegador, criador, popups) eram `bevy_hui` (HTML + Tailwind em build time, [ADR 0009](./0009-bevy-hui-tailwind-tokens.md)). No toque isso custa: teclado e campos de texto sob medida, rolagem e arrastar reimplementados, previews só rodando o jogo, e um look difícil de ajustar. O servidor continua sendo a fonte da verdade e o Bevy dono da rede ([guia de portar a UI](../architecture/portar-ui-para-compose.md)).

## Decisão
No host KMP (feature `kmp-host`) as telas do jogo são Compose, no padrão Repository -> ViewModel -> View:
- O Bevy publica **snapshots JSON por tela** (`screens_bridge.rs`, `game_state_bridge.rs`, `wallet_bridge.rs`) e recebe **comandos pequenos** que fazem o mesmo que o botão `bevy_hui` fazia (a lógica foi extraída para funções compartilhadas: `set_shop_open`, `set_navigator_open`, `start_placing`, `save_room_settings`, `rotate_placed`...).
- Cada tela portada tem um recurso marcador (`ComposeShop`, `ComposeNavigator`, ...) inserido pelo `ScreensBridgePlugin`; com ele a versão `bevy_hui` não é criada. O desktop mantém o `bevy_hui`.
- A tela com estado só coleta o estado do ViewModel; um composable **sem estado** (`XxxContent(ui, actions)`) desenha, com `@Preview` no fim do mesmo arquivo ([previews](../process/previews-compose.md)).
- Janelas flutuantes compartilham o `GameWindow`; o HUD é escondido por quem abre uma janela (`HudVisibility`).
- Coisas que dependem do mundo 3D ficam no Bevy: a prévia do avatar no criador, o fantasma de colocação e o picking de tile; o Compose dá a intenção (arrastar um item para a sala manda a posição do dedo; o jogo valida e posiciona).

## Alternativas Consideradas
- **Manter `bevy_hui` no mobile**: um só stack de UI, mas teclado/rolagem/gestos ruins no toque e nenhuma ferramenta de preview.
- **UI nativa por plataforma (SwiftUI/Compose Android)**: duplica telas; o Compose Multiplatform já está no host.
- **Material3 puro**: rápido, mas fora do estilo in-game; o plano é um kit próprio no futuro (decisão do dono do projeto), por isso as telas usam componentes em `components/`.

## Consequências
- **Impactos Positivos**: telas de toque nativas (teclado, rolagem, arrastar), previews na IDE, lógica de UI testável em `commonTest`, bevy_hui do mobile reduzido a quase nada.
- **Impactos Negativos / Trade-offs**: duas implementações de cada tela (desktop `bevy_hui` e mobile Compose) até o desktop também ser migrado; contrato de mensagens entre Rust e Kotlin para manter; miniaturas vêm do cache local da CDN que o Rust expõe por caminho de arquivo.
