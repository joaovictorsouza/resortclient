# Design system do Resort Game (cliente Compose)

Fonte da verdade do visual da UI Compose do host KMP. O plano completo (identidade, telas, fases) está em `design-system-do-jogo.md`; este arquivo guarda **as regras e onde está o código**. Fase 1 entregue: fundação, HUD e painel adaptativo. Fase 2 entregue: Inventário e Loja no `AdaptivePanel`. Navegador, Criador e RoomUi ainda usam `GameWindow` (herdam só as cores, pelo mapeamento do Material3).

## Onde fica

Pacote `br.eng.joaovictor.resort.ds` no módulo `shared` (`commonMain`). **Regra de fronteira: nada em `ds/` importa `views/`, `repositories/`, `game/` ou `models/`.** Se outro app passar a usar, vira o módulo `:ds` (é só mover a pasta).

```
ds/theme/       ResortColors, ResortTypography, ResortDimens (Space, Radius, Size, Motion), ResortTheme, GameFeedback, Contrast
ds/icons/       GameIcons (vetores próprios, sem emoji e sem material-icons-extended)
ds/layout/      PanelSpec, AdaptivePanel, SidePanel, BottomSheet, PanelParts (header, footer, seção)
ds/components/  Tiles (ItemTile, OfferTile, PriceTag), CategoryRail, SearchField, Skeleton, GameButton, GameIconButton, CurrencyPill, Badge, GameTabs/CategoryChips, EmptyState,
                ConfirmDialog, Toast/ToastHost, ActionDock, RoomPill, SolidShadow
ds/format/      formatGroupedAmount
```

## Regras

1. **Tema**: `ResortTheme { }` (em `App` e no `PreviewScene`) fornece cores, tipografia e feedback e mapeia os tokens para o `MaterialTheme`. Use `ResortTheme.colors` / `.type` / `.feedback`; **não escreva `Color(0x...)` fora de `ds/theme`** (a fase 8 põe um `grep` no `check.sh`).
2. **Texto em botões coloridos é escuro** (`onAccent`): branco sobre coral dá 2,6:1. `DesignTokensTest` calcula o contraste WCAG dos pares usados e falha abaixo de 4,5:1.
3. **Sem efeitos caros**: nada de `Modifier.blur`, `RenderEffect` ou `shadow` de raio grande. Profundidade com retângulos sólidos (`drawSolidShadow`, lábio do botão). Animação lida em `graphicsLayer`/`layout`/`draw` para não recompor.
4. **Componentes sem estado**: estado de UI entra, callbacks saem; `modifier` primeiro opcional; ícone sozinho exige `contentDescription` em português; alvo mínimo 48dp.
5. **Painéis não bloqueiam a cena**: `AdaptivePanel` não tem scrim nem `pointerInput` fora dos próprios limites; tocar na cena com o painel aberto continua andando. O único componente com scrim é o `ConfirmDialog`.
6. **Lógica em função pura** e testada sem janela: `panelSpecFor`, `resolveHudMode`, `dockActions`, `currencyDelta`, `contrastRatio`, `ToastHostState`.
7. Cada componente tem `@Preview` no fim do arquivo (padrão do repo, ver `previews-compose.md`), usando `DsPreviewScene` dentro de `ds/` e `PreviewScene` nas telas.

## Painel adaptativo

`panelSpecFor(width, height)` escolhe a forma:

| Janela | Forma |
|---|---|
| Paisagem com largura ≥ 560dp, ou largura ≥ 840dp | `SidePanel` à direita, 42% da largura (340 a 480dp, no máximo 55%) |
| Retrato | `BottomSheet` com 3 detents: Peek 136dp, Half 52%, Full 92% da altura |

`SidePanel` e `BottomSheet` ficam fora do caminho dos toques da cena. O `BottomSheet` usa `anchoredDraggable` do `foundation` (não o `ModalBottomSheet`, que tem scrim) e é o **único arquivo que importa essa API**; a altura do sheet segue o arrasto na fase de layout, então o rodapé fica sempre na borda visível. Rolagem aninhada própria: arrastar para cima move o sheet antes de rolar a lista, a lista só rola em Full. `tucked = true` recolhe o painel sem fechar (aba de 24dp em paisagem, Peek em retrato).

## HUD

`HudVisibility` tem três modos (`HudMode`): **Full**, **Compact** (painel aberto: fica só a pill da sala e as moedas) e **Hidden** (criador, onboarding, diálogo). Quem precisa pede `compact(owner)` ou `hide(owner)` e solta com `release(owner)`; vale o mais restritivo. `hide`/`show` continuam funcionando para as telas antigas. `HudUiState` só ganhou campos (`roomName`, `visibility`, `silverAmount`, `goldAmount`, `mode`); `HudActions` não mudou. O dock vem de `dockActions(ui)`: visitante (Salas, Itens, Loja), dono (+ Construir) e dono em modo construção (Itens, Concluir). Configurações da sala saíram do dock: tocar na pill abre as configurações (dono) ou o Navegador (visitante) até o menu da pill da fase 5.

## Inventário e Loja (fase 2)

Os dois são `AdaptivePanel`: painel lateral com `CategoryRail` em paisagem, bottom sheet com `CategoryChips` em retrato. A busca é um botão no cabeçalho que abre um `SearchField` abaixo dele (`toggleSearch` continua valendo). A Loja mostra o saldo (`CurrencyPill` compacta) no cabeçalho e a barra de compra no rodapé fixo (`GameButton` com `loading` enquanto compra). Os dois pedem `HudMode.Compact` (`CompactHudWhileVisible`), não escondem mais o HUD.

- `ItemTile` e `OfferTile` recebem `TileState` (selecionado, bloqueado, novo, raridade, quantidade) e a miniatura por slot, para `ds/` não depender de `components/`. Bloqueado continua tocável (mostra o motivo); preço sem saldo em `dangerText` com cadeado.
- Arrasto do Inventário: o toque longo fica na tela (`dragModifier`); ao sair do painel ele recolhe (`tucked`) e a cena inteira fica livre para posicionar o móvel, como a janela sumia antes. Funções puras: `leftWindow`, `dragPhase`, `inventoryTileState`, `shopRail`, `offerTileState`, `buyLabel`, `tileDescription`.

## Como adicionar um componente

1. Arquivo em `ds/components/`, composable sem estado, cores e tipos só do `ResortTheme`.
2. `@Preview` no fim do arquivo cobrindo os estados (normal, desativado, carregando, selecionado...).
3. Regra de apresentação não trivial vira função pura com teste em `commonTest`.
4. Não importe nada de fora de `ds/`.

## Fora da fase 1 (decisões em aberto)

Fonte própria, raridade/"novo" nos dados, `QuantityStepper`, `PanelHost`, `ContextActionBar`, `Popover`, sons (`ui_sound`), `ui_occlusion` para a câmera e testes de UI com `compose.uiTest` seguem o plano nas fases 2 a 8.
