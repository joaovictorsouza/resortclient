# Spec 0012: Padronização de modais, safe areas e bloqueio de cliques

## Status
Implemented

## Objetivo
Loja, inventário, navegador de salas e configurações da sala seguem o mesmo molde, aproveitam melhor a tela de um celular em paisagem e nunca colocam botões onde o toque não chega (notch, cantos arredondados). A identidade visual (slate/indigo/emerald) não muda; o layout se inspira na divisão de espaço do Habbo.

## Molde de modal
- A janela ocupa a **largura toda** e 92% da altura (centralizada, com o fundo escurecido e a sala visível acima e abaixo). Dentro dela, só o conteúdo respeita a safe area.
- **Cabeçalho** (uma linha, 44 px): título à esquerda, **abas centralizadas** no meio, extras (saldos) e botão fechar à direita. As abas indicam o tipo de catálogo (hoje só "Mobília"); o menu lateral nunca é aba.
- **Corpo**: menu lateral (largura 192 px) com a **busca sempre no topo**, e abaixo as seções; conteúdo (grid) ao centro; painel de detalhe (inventário) à direita.
- **Itens do menu lateral** têm altura fixa (36 px) e texto em uma linha só (corta, nunca quebra). Subseções ficam num **acordeão** dentro da seção: tocar na seção a abre; tocar de novo na seção aberta a fecha (seta indica o estado).
- **Grids** de 5 colunas com cartões compactos (miniatura 48 px, nome em até 2 linhas, preço e botão de 28 px).
- **Detalhe do inventário** compacto: miniatura 56 px e o botão "Colocar" logo abaixo das informações, sempre dentro da área visível.

## Safe areas
- `safe_area.rs`: recurso `SafeArea` (pontos da janela). No iOS vem do crate `bevy_ios_safearea` (`IosSafeAreaPlugin`). O UIKit pode entregar as insets da orientação em que a view foi montada (retrato: notch no topo, laterais zeradas) mesmo com a janela em paisagem; `landscape_insets` detecta isso e move o inset do topo para as duas laterais (como o UIKit faz) e não reserva nada no topo. O valor bruto e o corrigido saem no log (`iOS safe area: raw ...`). Nas outras plataformas fica zerado e só vale a margem base.
- Modais hui: os templates recebem as propriedades `safe_t/r/b/l` (px de layout = inset / `UiScale` + 12) e as usam em nós espaçadores. A janela é reconstruída quando a safe area ou a escala muda.
- HUD: o componente `HudEdge` marca quais bordas de um nó absoluto acompanham a safe area (inset + 16). Substitui a antiga margem fixa `SCREEN_MARGIN`.
- Android: sem leitura de insets por enquanto (só a margem base).

## Cliques
- Toda raiz de modal é um `LegacyButton` (`Interaction` + `FocusPolicy::Block`). Com ela aberta, o `ui_focus` não entrega o clique aos botões do HUD que ficam por baixo; ao fechar, o HUD volta a receber cliques. Antes, a raiz não tinha `Interaction`, então o clique caía no botão do HUD sob o item do modal.

## Como conferir
- `PREVIEW_PHONE=1 cargo run -p game --example shop_preview -- shot.png` (idem `inventory_preview`, `navigator_preview`, `hud_preview`) simula um iPhone em paisagem: janela 852x393, escala de toque e safe area de um iPhone com notch.
- `HUD_PROBE=1 HUD_SETTINGS=1 cargo run -p game --example hud_preview -- shot.png` coloca o cursor sobre o botão da loja e imprime a `Interaction` dele (com a janela de configurações aberta deve ser `None`).
- `python3 tools/ui/build_ui.py --check` e `./scripts/check.sh`.

## Critérios de Aceite
- [x] Modais ocupam a largura toda; conteúdo fora de áreas não clicáveis.
- [x] Busca, abas e menu seguem a mesma ordem em loja e inventário.
- [x] Itens do menu lateral com altura fixa e acordeão para subseções.
- [x] Botão "Colocar na sala" visível no iPhone em paisagem.
- [x] Clique em item do modal não aciona o botão do HUD por baixo.
