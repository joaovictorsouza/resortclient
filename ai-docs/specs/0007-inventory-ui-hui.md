# Spec 0007: Inventário com bevy_hui, Phosphor Icons e Tailwind

## Status
Implemented

## Contexto e Objetivo
O inventário atual (spec 0004) é um modal em `bevy_ui` puro, recriado por completo a cada mudança de estado, com cartões de texto e sem miniaturas. Esta spec redesenha a interface com visual moderno inspirado no inventário do Habbo: grid de itens com miniatura quadrada e título embaixo, abas, busca, categorias e painel de preview com o botão "Colocar na sala".

## Escopo
- Dentro do escopo:
  - Migrar a UI do inventário para templates `bevy_hui` 0.7 (Bevy 0.19).
  - Design em classes Tailwind, traduzidas em build time para atributos do bevy_hui (`tools/ui/build_ui.py`).
  - Ícones Phosphor rasterizados em PNG (`tools/ui/fetch_icons.py`).
  - Miniaturas PNG quadradas por item, geradas por `tools/asset_pipeline/render_item_thumbnails.py` (apenas as que ainda não existem).
  - Catálogo de móveis exportado do servidor para `assets/furniture/catalog.json`.
  - Campo de busca por nome (`bevy_hui_widgets` 0.6, widget `input`), sem diferenciar maiúsculas e acentos, combinado com a categoria.
  - Lógica pura (filtro por categoria e busca, seleção, rótulos, caminho da miniatura) com testes unitários.
- Fora do escopo:
  - Menu de contexto de móveis e HUD (continuam em `bevy_ui`).
  - Abas Pets, Builders Club e "Furni por sala" (a aba "Mobília" é a única funcional).
  - Venda em marketplace.

## Componentes Bevy Envolvidos
- Prévia sem servidor: `cargo run -p game --example inventory_preview [-- arquivo.png]`.
- `InventoryPlugin` (novo, `crates/game/src/inventory.rs`), registrado em `GamePlugin`.
- `HuiPlugin` (bevy_hui) com `HtmlFunctions`: `inv_select`, `inv_close`, `inv_place`, `inv_filter`.
- Recurso `PlayerInventoryState` (continua em `furniture.rs`) e novo `InventoryFilter { category, query }`.
- Templates: `assets/ui/inventory.html`, `inventory_slot.html`, geradas a partir de `assets/ui/src/*.html`.

## Critérios de Aceite
- [x] O inventário abre pelo botão do HUD e fecha com o botão X ou Esc.
- [x] Os itens aparecem em grid; cada slot mostra miniatura quadrada, título embaixo e badge de quantidade.
- [x] Hover e seleção têm feedback visual.
- [x] Todos os slots têm o mesmo tamanho (108x156) e o grid rola com a roda do mouse, com barra de rolagem indicando a posição.
- [x] Filtro por categoria e busca por nome funcionam juntos; Esc sai do campo antes de fechar a janela.
- [x] O painel de preview mostra miniatura, nome, tipo, dimensões e "Colocar na sala (N)" (apenas para o dono da sala).
- [x] O script de miniaturas só gera PNGs que não existem (`--force` refaz).
- [ ] `./scripts/check.sh`, `go test -race` e os testes Python passam.
