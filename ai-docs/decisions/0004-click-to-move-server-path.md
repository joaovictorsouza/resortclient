# ADR 0004: Movimentação por Clique com Caminho Autoritativo

## Status
Aceita

## Contexto
O jogo segue a estética e dinâmica do Habbo Hotel em 3D, onde o jogador clica em um tile do chão e seu avatar caminha contornando obstáculos.

## Decisão
O cliente realiza raycast da câmera contra o plano dos tiles e envia apenas `MOVE_REQUEST {x, z}` ao servidor. O cliente nunca calcula a rota definitiva localmente para movimentar o avatar; ele aguarda o evento `AVATAR_PATH` do servidor e então interpola suavemente a cápsula ao longo dos waypoints recebidos.

## Consequências
- Impossibilidade de descompasso de rota entre múltiplos clientes conectados.
- Feedback imediato caso o tile seja inválido através de `MOVE_REJECTED`.
