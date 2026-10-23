# ADR 0006: Modo Construção, Preview Holográfico e Renderização Paramétrica de Mobílias

## Status
Aceito

## Contexto
A experiência do jogador ao decorar sua sala requer alta fidelidade visual, controle intuitivo e resposta imediata (snapping suave às tiles, rotação de 90° e validação visual de áreas bloqueadas/livres). Além disso, o cliente deve renderizar mobílias imediatamente através de volumes cuboides coloridos antes da disponibilidade final dos assets 3D GLTF do Asset Manager.

## Decisão

### 1. Máquina de Estados do Modo Construção
No cliente Bevy, o modo de construção é gerenciado pelo recurso `BuildModeState`:
- **`Idle`**: Construção inativa. O clique esquerdo movimenta o avatar pelo piso.
- **`Placing(catalog_id, rotation)`**: Jogador selecionou um item do inventário para incluir na sala. Um ghost preview segue o ponteiro no grid.
- **`Moving(item_id, rotation)`**: Jogador selecionou uma mobília já existente para reposicionar. O item original fica translúcido enquanto o ghost indica a nova posição alvo.
- **`ItemContextOpen(item_id)`**: Um item colocado no chão foi clicado, abrindo o menu de contexto (Girar / Mover / Recolher).

### 2. Projeção de Cursor e Snapping no Grid
- O sistema converte a coordenada de tela do ponteiro (mouse ou toque móvel) para um raio 3D via câmera ortográfica/perspectiva.
- O raio intersecta o plano `Y = 0.1` (topo das tiles do piso).
- As coordenadas contínuas de impacto são convertidas para o índice discreto de tile `(tile_x, tile_z) = (floor(hit.x), floor(hit.z))`.
- Se o item tiver dimensões maiores que 1x1 (ex: 1x2, 2x2), a rotação atual inverte a largura e a profundidade conforme aplicável, e a bounding box inteira é validada contra o layout da sala e itens existentes.

### 3. Feedback Visual do Ghost Preview
- A entidade do ghost utiliza material semitransparente com alpha blending (`Color::srgba(..., 0.5)`):
  - **Verde / Azul Translúcido**: Posição válida e desimpedida. O clique do usuário envia a mensagem `PLACE_ITEM_REQUEST` ou `MOVE_ITEM_REQUEST`.
  - **Vermelho Translúcido**: Posição inválida (fora do grid, sobre uma tile vazia `'x'`, sobre a porta de entrada ou sobrepondo outro item colidível). O clique é ignorado e um som ou feedback de recusa é disparado.

### 4. Renderização Paramétrica Desacoplada (Asset Manager Ready)
- Cada item possui dimensões `(width, depth, height)` e cor hexadecimal (`initial_color`) no catálogo.
- Na ausência do modelo GLTF definitivo, o cliente instancia um `Cuboid` 3D no centro geométrico das tiles ocupadas com a elevação adequada e anexa um `Collider::cuboid` do Avian 3D caso o item seja colidível.
- A entidade recebe o componente `PlacedFurniture { item_id, catalog_id, x, z, rotation }`.
- Futuramente, o `AssetServer` substituirá a malha do cubóide pela cena 3D GLTF sem alterar a hierarquia de componentes ou a lógica de seleção.

## Consequências

### Positivas
- Feedback visual imediato e altamente responsivo para o jogador ao decorar o espaço.
- Separação clara entre a renderização visual e a lógica autoritativa de rede.
- Fácil transição para modelos 3D complexos no futuro através da arquitetura de componentes do Bevy.

### Negativas / Mitigações
- Raycasting contínuo no `Update` pode gerar custo caso haja milhares de objetos (mitigado por usar interseção analítica com plano `Y = 0.1` e consulta por hash map em memória de tiles).
