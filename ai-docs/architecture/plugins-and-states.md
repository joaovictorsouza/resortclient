# Plugins e Máquina de Estados (Bevy)

## Máquina de Estados (`AppState`)
- **`AppState::InitAuth`** → **`Login`** (botão "Entrar com Google", `identity.rs`) → **`ChooseNickname`** (HTML `nickname.html`) → `CharacterCreator` → `Connecting`. Contas Google que já têm apelido no servidor (`get_profile_status`) pulam `ChooseNickname`; em dev ou com perfil de dispositivo, `Login` é pulado. Ver [Spec 0011](../specs/0011-login-google-nativo.md).
- **`AppState::Connecting`**: Estado inicial. Dispara a autenticação de device e solicitação de join via RPC.
- **`AppState::InRoom`**: Conexão WebSocket estabelecida e snapshot de sala recebido. Renderização ativa do grid e dos avatares.
- **`AppState::ConnectionError`**: Erro de rede ou desconexão inesperada.

## Árvore de Plugins
- `NetPlugin`: Ponte entre o worker Tokio e eventos do Bevy ECS.
- `CameraPlugin`: Câmera 3D isométrica inclinada e iluminação de sala.
- `RoomPlugin`: Renderização de tiles, mapeamento de coordenadas e colliders do Avian.
- `AvatarPlugin`: Gestão de entidades de jogadores e interpolação de caminho.
- `InputPlugin`: Raycasting de clique/toque de tela projetado no chão.
