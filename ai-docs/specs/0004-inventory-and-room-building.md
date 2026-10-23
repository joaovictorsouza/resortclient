# Spec 0004: Interface de Inventário e Modo Construção de Salas

## Status
Approved

## Contexto e Objetivo
Permitir que o jogador acesse visualmente seu inventário dentro de qualquer sala através do HUD, visualize itens em 3D e, quando estiver dentro de uma sala de sua propriedade, ative o Modo Construção para posicionar, rotacionar, mover e recolher móveis com preview holográfico semitransparente (ghost) e menu de contexto interativo.

## Escopo
- **Dentro do escopo**:
  - Botão de acesso ao inventário no HUD (`🎒 Inventário`).
  - Botão de alternância do Modo Construção no HUD (`🔨 Modo Construção` / `Sair do Modo Construção`), exibido exclusivamente para o proprietário da sala.
  - Painel/Janela moderna de Inventário:
    - Lista de itens agrupados por tipo (`collider` ou `floor_decoration`) com contagem de unidades.
    - Seleção de item com visualização de detalhes e botão `Colocar na Sala`.
  - Preview Holográfico Semitransparente (Ghost Preview):
    - Ao selecionar `Colocar na Sala`, ativa o modo construção e projeta um modelo 3D semitransparente sobre a tile apontada pelo cursor.
    - Validação visual em tempo real: ghost azul/verde se a área estiver desimpedida; vermelho se a tile for inválida, ocupada ou a porta da sala.
    - Rotação rápida do preview (tecla `R` ou botão na interface) antes de colocar.
  - Colocação do item com clique esquerdo no grid e envio da mensagem `PLACE_ITEM_REQUEST`.
  - Menu de Contexto de Itens no Modo Construção:
    - Clicar sobre um item já existente na sala abre um menu flutuante suspenso sobre o item com botões:
      - 🔄 **Girar** (envia `ROTATE_ITEM_REQUEST`).
      - ✥ **Mover** (inicia ghost preview para reposicionar via `MOVE_ITEM_REQUEST`).
      - 📦 **Recolher** (envia `PICKUP_ITEM_REQUEST`).
  - Renderização 3D de mobílias:
    - Representação inicial como caixas/cuboides coloridos proporcionais às dimensões do catálogo (ex: 1x1, 1x2, 2x2) sobre o grid da sala.
    - Arquitetura desacoplada e compatível com futuro Asset Manager (troca transparente para meshes GLTF).
- **FORA do escopo**:
  - Pintura de paredes e papéis de parede customizados.
  - Física rígida de corpos livres em movimento (móveis são fixados no grid discreto).

## Componentes Bevy Envolvidos

### Recursos (`Resource`)
- `PlayerInventory`: Lista de itens possuídos, item selecionado e status de carregamento.
- `BuildModeState`: Estado de construção (`active: bool`, `active_tool: None | Placing(catalog_id, rotation) | Moving(item_id, rotation)`, `hovered_tile: Option<(i32, i32)>`, `is_valid_placement: bool`, `selected_placed_item: Option<String>`).
- `CatalogRegistry`: Cache local das definições de itens e materiais padrão.

### Componentes (`Component`)
- `PlacedFurniture`: Marcador em entidades 3D de mobílias colocadas com `item_id`, `catalog_id`, coordenadas `(x, z)` e rotação.
- `GhostPreview`: Marcador para a entidade 3D semitransparente de preview durante a construção.
- `HudInventoryBtn`: Marcador para o botão de inventário no HUD.
- `HudBuildModeBtn`: Marcador para o botão de modo construção no HUD.
- `InventoryWindowRoot`: Janela modal do inventário.
- `ItemContextMenuRoot`: Menu de contexto flutuante do item selecionado na sala.

## Critérios de Aceite (Testáveis)
- [ ] O botão do inventário está visível no HUD durante a permanência em qualquer sala.
- [ ] O botão do Modo Construção aparece no HUD apenas quando o usuário é dono da sala atual (`active_room.is_owner == true`).
- [ ] Ao clicar em `Colocar na Sala` em um item do inventário, o modo construção é ativado automaticamente e o preview fantasma semitransparente é gerado.
- [ ] Mover o mouse sobre o piso da sala posiciona o ghost perfeitamente alinhado às tiles do grid.
- [ ] O ghost indica visualmente cor de aviso (vermelho) ao tentar posicionar sobre uma tile vazia ou já ocupada por collider.
- [ ] Ao clicar com o mouse sobre uma tile válida, a mensagem de colocação é enviada e o item é fixado na cena com sua cor e dimensões.
- [ ] A sala permanece em Modo Construção até que o jogador aperte explicitamente "Sair do Modo Construção" no HUD.
- [ ] No Modo Construção, clicar em um item existente no chão exibe as opções de Girar, Mover e Recolher.
- [ ] Clicar em "Recolher" remove a mobília da cena 3D e devolve a quantidade correspondente ao inventário.
- [ ] A renderização inicial em caixas coloridas utiliza estrutura preparada para integração direta com carregamento de modelos GLTF do Asset Manager.

## Plano de Testes
- Testes unitários para cálculo de bounding box rotacionada e offset de grid (`furniture_pure_test.rs`).
- Testes unitários para o redutor de estado do inventário local e build mode.
- Validação com `./scripts/check.sh` no client.
