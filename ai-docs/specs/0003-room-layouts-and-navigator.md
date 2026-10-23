# Spec 0003: Room Layouts e Room Navigator (Client)

## Status
Approved

## Contexto e Objetivo
Prover ao jogador uma experiência completa de navegação, descoberta, criação e gerenciamento de seus próprios espaços virtuais.
O cliente Bevy deve apresentar uma interface gráfica (UI) visualmente rica, moderna e responsiva — adaptável para desktop e dispositivos móveis — permitindo explorar salas públicas e populares, buscar espaços por nome, criar salas personalizadas a partir de uma galeria de layouts e gerenciar as propriedades de suas próprias salas (nome, privacidade ou exclusão).

---

## Escopo

### Dentro do Escopo
1. **Navegador de Salas (Room Navigator UI)**:
   - Interface em overlay/modal com design moderno, tema escuro, tipografia nítida e abas navegáveis:
     - **Espaços Públicos**: Lista salas oficiais do jogo fornecidas pela staff.
     - **Salas Populares**: Lista salas públicas criadas por usuários, ordenadas pelo número de pessoas online.
     - **Minhas Salas**: Lista todas as salas pertencentes ao jogador logado (com badge de visibilidade e botão de entrada).
     - **Busca**: Campo de busca com debounce para pesquisa em tempo real por nome de sala.
   - Itens de lista/cards de sala contendo: Nome do espaço, Nome do criador/proprietário, Lotação atual / Máxima (`online/max`) e Indicador de Privacidade (`Pública` / `Privada`).
   - Botão de acesso rápido no HUD principal para abrir/fechar o Navegador a qualquer momento.
2. **Criação de Salas**:
   - Tela/modal de criação com campo de texto para o nome (validação de 1 a 50 caracteres).
   - Galeria seletora de layouts com exibição da imagem/preview do mapa de tiles, dimensões e limite de usuários.
   - Indicador claro de que a sala nasce como `Privada` por padrão, podendo ser alterada após a entrada.
   - Transição imediata para a sala recém-criada após o sucesso da requisição.
3. **Painel de Configurações da Sala (Room Settings)**:
   - Botão de engrenagem no HUD, visível exclusivamente para o proprietário dentro de sua sala.
   - Modal com edição de Nome (até 50 caracteres), seleção de Visibilidade (`Pública`, `Privada`, `Oculta`) e botão perigoso de "Excluir Sala" com diálogo de confirmação.
4. **Mecânica de Campainha (Doorbell System)**:
   - **Visão do Solicitante**: Ao clicar em entrar numa sala privada cujo dono está presente, exibe estado "Tocando a campainha... Aguardando autorização do proprietário" com opção de cancelar. Se o dono recusar ou expirar o tempo, exibe mensagem amigável de recusa.
   - **Visão do Proprietário**: Ao receber evento de campainha (`DoorbellRequest`), exibe popup flutuante de autorização com nome do solicitante e botões `[Aceitar]` e `[Recusar]`.
5. **Transição Suave de Salas e Tratamento de Exclusão**:
   - Ao trocar de sala ou ao receber aviso de sala fechada/excluída (`RoomClosed`), o cliente desconecta da match anterior, limpa os avatares e entidades de grid 3D atuais, e conecta na nova match (ou retorna ao lobby público padrão).
6. **Responsividade Cross-Platform (Mobile & Desktop)**:
   - Layout flexível adaptado a telas touch (botões com área de toque mínima de 44x44 dp) e telas com ponteiro de mouse.

### Fora do Escopo
- Editor de móveis ou catálogo de itens decorativos (Feature 2).
- Upload de imagens customizadas pelo usuário para o layout (os layouts usam assets predefinidos no banco).

---

## Componentes Bevy, Recursos e Eventos

### 1. Novos Recursos e Estados
- **`enum NavigatorTab`**: `PublicSpaces`, `PopularRooms`, `MyRooms`, `Search`, `CreateRoom`.
- **`struct NavigatorState`**: controle de visibilidade da janela, aba ativa, texto de pesquisa, listagens em cache e layout selecionado.
- **`struct ActiveRoomMetadata`**: armazena se a sala atual pertence ao jogador logado (`is_owner`), nome da sala e visibilidade atual.
- **`struct PendingDoorbellRequest`**: estado do modal de campainha aguardando resposta do proprietário.

### 2. Mensagens e Eventos de Rede
- **`NetDoorbellRequestEvent`**: recebido pelo dono quando um visitante solicita entrada.
- **`NetRoomClosedEvent`**: recebido por todos na sala quando a mesma é excluída.
- Comandos assíncronos no canal Nakama:
  - `NakamaCommand::ListRoomLayouts`
  - `NakamaCommand::CreateRoom { name, layout_id }`
  - `NakamaCommand::ListPopularRooms`
  - `NakamaCommand::ListMyRooms`
  - `NakamaCommand::SearchRooms { query }`
  - `NakamaCommand::UpdateRoom { room_id, name, visibility }`
  - `NakamaCommand::DeleteRoom { room_id }`
  - `NakamaCommand::JoinRoom { room_id }`
  - `NakamaCommand::RespondDoorbell { request_id, allowed }`

### 3. Sistemas Principais (Plugins Bevy)
- `RoomNavigatorPlugin`:
  - `setup_navigator_ui`: instancia a árvore de nós de UI (oculta por padrão).
  - `toggle_navigator_ui`: reage ao clique no botão do HUD ou atalho de teclado (`N`).
  - `handle_tab_selection`: alterna a visualização e dispara o fetch de dados da aba selecionada.
  - `handle_room_item_click`: dispara requisição de entrada no espaço.
  - `handle_create_room_submit`: valida campos e emite comando de criação de sala.
- `RoomSettingsPlugin`:
  - `update_hud_settings_button_visibility`: exibe botão de configurações apenas se `ActiveRoomMetadata.is_owner == true`.
  - `handle_room_settings_submit`: dispara atualização de nome e visibilidade via RPC.
  - `handle_room_delete_confirm`: dispara exclusão e exibe feedback de retorno ao lobby.
- `DoorbellPlugin`:
  - `render_doorbell_popup`: renderiza notificação flutuante com temporizador.
  - `handle_doorbell_action`: envia resposta de permissão ou recusa.

---

## Critérios de Aceite (Testáveis)

- [ ] A interface do Room Navigator pode ser aberta e fechada via botão no HUD ou tecla de atalho.
- [ ] A navegação entre as 4 abas carrega os dados correspondentes sem travamentos na thread de renderização.
- [ ] A aba "Criar Sala" exibe a galeria de layouts disponíveis retornados pelo servidor com nome, imagem e capacidade máxima.
- [ ] A tentativa de criar sala com nome vazio ou maior que 50 caracteres é bloqueada na UI com mensagem de erro amigável.
- [ ] Ao confirmar a criação de uma sala, o cliente recebe o `room_id`, conecta à nova sala e monta os tiles 3D do layout escolhido.
- [ ] Dentro de uma sala de propriedade do jogador, o botão de configurações aparece no HUD; em salas de terceiros ou públicas, o botão permanece oculto.
- [ ] O proprietário consegue alterar o nome e a visibilidade da sala através do modal de configurações.
- [ ] Ao excluir a sala, os ocupantes recebem o evento `RoomClosed` e são automaticamente reconectados ao Lobby público padrão.
- [ ] Ao clicar para entrar em uma sala privada cujo dono está presente, a interface exibe feedback de espera e o dono recebe o popup com opções de aceitar ou recusar.
- [ ] Se o dono aceitar a campainha, o visitante conecta na sala com sucesso; se recusar ou não estiver na sala, exibe notificação amigável de acesso negado.
- [ ] Todos os componentes de UI utilizam dimensionamento responsivo compatível com resolução móvel e desktop.
- [ ] Testes unitários cobrem validações de formulário, transições de abas e mapeamento de mensagens de protocolo.

---

## Plano de Testes e Validação
1. **Testes Unitários no Crate `game`**:
   - Validações de limites de caracteres (nome da sala entre 1 e 50 chars).
   - Teste de deserialização de modelos de RPC de salas e layouts.
2. **Testes Manuais de Multiplayer Local**:
   - Instanciar dois clientes (`Alice` e `Bob`).
   - `Alice` cria uma sala privada.
   - `Alice` altera para pública e valida se aparece na busca de `Bob`.
   - `Alice` define como privada novamente, `Bob` solicita entrada e `Alice` aceita via popup.
   - `Alice` exclui a sala e ambos retornam automaticamente ao lobby.
