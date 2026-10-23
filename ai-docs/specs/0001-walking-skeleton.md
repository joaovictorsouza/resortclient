# Spec 0001: Walking Skeleton (Client)

## Status
Implemented

## Contexto e Objetivo
Criar o esqueleto funcional mínimo do cliente Bevy capaz de autenticar via device ID, entrar na sala pública "lobby", renderizar os tiles 3D e movimentar o avatar por clique em tiles com sincronização multiplayer em tempo real.

## Escopo
- **Dentro do escopo**:
  - Conexão e autenticação com Nakama via `crates/nakama_client`.
  - Construção do grid 3D de tiles a partir do `RoomSnapshot`.
  - Câmera isométrica inclinada 3D e iluminação cinematográfica.
  - Spawn do avatar do jogador (cápsula ciano) e avatares remotos (cápsula âmbar).
  - Raycast no clique/touch do chão para emitir `MOVE_REQUEST`.
  - Interpolação suave do avatar ao receber `AVATAR_PATH`.
- **Fora do escopo**:
  - Customização cosmética de avatares (roupas, cabelos).
  - Posicionamento de mobília.
  - Chat de texto na tela.

## Critérios de Aceite
- [x] O cliente conecta ao Nakama e transiciona para `AppState::InRoom`.
- [x] O grid de tiles da sala é renderizado com visual checkerboard moderno.
- [x] Ao clicar em um tile do chão, o avatar caminha suavemente até o destino.
- [x] Outros clientes conectados na mesma sala veem o avatar se movimentando em tempo real.
- [x] Ao desconectar um cliente, seu avatar desaparece nos outros clientes.
