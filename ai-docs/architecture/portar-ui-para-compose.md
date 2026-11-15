# Guia: portar a UI do jogo (bevy_hui) para Compose

Guia para portar **à mão** o HUD, o inventário e a loja/catálogo de `bevy_hui` para Compose Multiplatform, mantendo as mesmas funções. Tudo aqui foi levantado do código real em `crates/game/src/` na branch `main` (host KMP, ver [kmp-host.md](kmp-host.md)). O que é **proposta** (ainda não existe no código) está marcado assim.

Ponto de partida de código: `crates/game/src/hud_bridge.rs` (lado Rust) e `kmp/shared/src/commonMain/kotlin/br/eng/joaovictor/resort/ComposeHud.kt` + `GameHost.kt` (lado Kotlin). Resultado do teste de viabilidade: [hud-compose-teste.md](hud-compose-teste.md).

## 1. Regra de ouro

- **O servidor é a fonte da verdade** (saldo, preços, inventário, salas, móveis). O Bevy só espelha o que o servidor manda; o Compose só espelha o que o Bevy publica. Nada de calcular saldo ou estoque no Kotlin.
- **O Bevy continua dono de toda a lógica de rede** (`NakamaClientHandle`, reconexão, heartbeat, fluxo de compra). O Compose **não** fala com o Nakama: ele manda *comandos* ao Bevy e desenha *snapshots* recebidos.
- **A cena 3D continua no Bevy** (picking de tile, fantasma de colocação, câmera, avatares). O Compose só dá a intenção ("quero colocar este item"); o toque no chão continua indo ao jogo.

## 2. Como o Compose conversa com o Bevy

Tudo passa pela biblioteca `bevy-kmp` (ver `docs/arquitetura.md` do bevy_kmp). Há dois canais.

### 2.1 Mensagens nomeadas (o canal principal)

| Direção | Rust | Kotlin |
|---|---|---|
| Bevy → Compose | `Outbox::send_app(nome, bytes)` (recurso `Outbox`) | `BevyEvent.App(name, payload)` no `BevyEventListener` (`GameHost.kt`) |
| Compose → Bevy | `MessageReader<HostMessage>` (`HostMessage { name, payload }`) | `host.sendMessage(nome, bytes)` |

O payload é **JSON em UTF-8** por convenção do cliente (`onboarding.rs`, `hud_bridge.rs`). Exemplos que já existem:

- `onboarding_stage` (Bevy → Compose) e `onboarding_google` / `onboarding_nickname {"nickname":".."}` (Compose → Bevy): `onboarding.rs`.
- `hud_state` (Bevy → Compose) e `hud_toggle_navigator` (Compose → Bevy): `hud_bridge.rs`.

### 2.2 Serviços nativos (para o que só o sistema faz)

`request_service` / `request_service_blocking` (Rust) ↔ `BevyEvent.ServiceRequest` + `respond` (Kotlin). Usados hoje para Keychain/Keystore e Google (`secure_get/put/delete`, `google_sign_in`, em `GameHost.handleService`). **Nunca** logue o payload: leva tokens. Use este canal quando o Bevy precisar de algo do SO (ex.: abrir o teclado, vibrar, compartilhar); para telas e dados use as mensagens nomeadas.

### 2.3 Threading e ciclo de vida

- O **Bevy roda numa thread própria** (`bevy-kmp`, loop de render). O `BevyEventListener.onEvent` é chamado **nessa thread**, não na principal. Por isso o `GameHost` guarda tudo em `MutableStateFlow` (seguro para escrever de qualquer thread) e o Compose só lê com `collectAsState()`. Mantenha esse padrão: o listener faz parse e `value = ...`; nunca toque em UI dele.
- `host.sendMessage` pode ser chamado da thread principal; a mensagem entra numa fila e é lida no próximo frame pelo `MessageReader<HostMessage>`.
- **Pausa** (app em segundo plano): o mundo Bevy **não atualiza** enquanto pausado. Mensagens enviadas nesse intervalo só são lidas no primeiro frame depois de retomar. O Compose deve tratar o último snapshot como "possivelmente velho" até chegar um novo.
- **Reconexão**: ao voltar do segundo plano o jogo manda `NakamaCommand::ForceReconnect` sozinho (`net.rs: reconnect_after_background`). Durante a reconexão `ConnectionStatus.reconnecting == true` (`hud_state.reconnecting`); o estado do app continua `InRoom`. O snapshot da sala é recarregado pelo servidor; **o Compose não precisa reconstruir nada**, só reagir ao próximo snapshot.
- Ao sair de `InRoom` (ex.: criador de personagem, "Sair") o `bevy_hui` fecha loja/inventário (`close_*_on_leaving_room`). O Compose deve fazer o mesmo: esconder o HUD quando `in_room == false` (já é o que `ComposeHudTest` faz).

### 2.4 Parse de JSON no Kotlin

`parseFlatJson` (`Onboarding.kt`) só entende objeto **plano**. Para listas (loja, inventário, salas) use `kotlinx-serialization-json` (hoje não está no catálogo do projeto; **adicione** em `kmp/gradle/libs.versions.toml` e no `shared`). Assim as classes Kotlin espelham os DTOs de `crates/protocol/src/messages.rs`.

## 3. Levantamento do HUD atual (bevy_hui) e função de cada item

O HUD nasce em `navigator.rs: setup_hud` (`OnEnter(InRoom)`) e é limpo em `cleanup_all_ui` (`OnExit(InRoom)`).

| Item | Onde | O que faz hoje |
|---|---|---|
| Moedas prata/ouro (canto sup. direito) | `wallet.rs: spawn_wallet_chips`, `refresh_hud_wallet` | Mostra `WalletState.balance` com `format_amount` (pt-BR, `1.234.567`); "-" até a 1ª resposta (`loaded == false`). |
| Heartbeat de carteira | `wallet.rs: send_heartbeat` | A cada 60 s manda `WalletHeartbeat`; o servidor credita no máx. 120 s por batida. O 1º heartbeat (`request_wallet_on_enter`) paga o bônus diário e já devolve o saldo. |
| Som de ouro | `wallet.rs: apply_wallet_updates` | Toca `audio/gold_received.wav` só quando `delta_gold > 0` (`should_play_gold_sound`). **Fica no Bevy**; o Compose não deve tocar de novo. |
| Botão Loja (`storefront`) | `shop.rs: HudShopBtn`, `toggle_shop_from_hud` | Abre/fecha a loja; ao abrir fecha o inventário e manda `FetchShopCatalog`. |
| Botão Avatar (`user`) | `navigator.rs: handle_hud_appearance_btn` | Abre/fecha o menu do avatar (`AvatarMenuState.is_open`). |
| Menu do avatar | `avatar_menu.rs` | Dois itens: **Visuais** (vai ao `CharacterCreator` com `return_to_room = true`) e **Sair** (`revoke_session`, `sign_out`, remove `NakamaResource`, vai a `Login`). |
| Título da sala (sup. esquerdo) | `navigator.rs: update_hud_display` | `"{nome}  •  [Pública/Oculta/Privada]"`; "The Grand Lobby" se o nome estiver vazio. |
| Botão Construção (`hammer`) | `furniture.rs: HudBuildModeBtn`, `handle_hud_mode_buttons` | Só para o **dono** da sala (`ActiveRoomMetadata.is_owner`). Alterna `BuildModeState.is_active`. |
| Botão Configurações da sala (`gear`) | `navigator.rs: HudEditRoomBtn`, `handle_hud_buttons` | Só para o dono. Abre `RoomSettingsState` (nome, visibilidade, apagar). |
| Botão Inventário (`backpack`) | `furniture.rs: HudInventoryBtn`, `inventory.rs: toggle_inventory_from_hud` | Abre/fecha; ao abrir manda `FetchInventory`. |
| Botão Navegador (`compass`) | `navigator.rs: HudNavigatorBtn`, `handle_hud_buttons` | Abre o navegador de salas; ao abrir fecha as configurações e manda `FetchPopularRooms`, `FetchMyRooms`, `FetchRoomLayouts`. |
| Navegador de salas | `navigator_ui.rs` (tabs `NavigatorTab`) | Abas: Espaços públicos (lobby e "Coffee House" fixos no código, ver `navigator_ui.rs` ~l. 28-40), Salas populares, Minhas salas, Buscar (`SearchRooms{query}`), Criar espaço (`CreateRoom{name, layout_id}`, nome até 50 caracteres, `MAX_NAME_CHARS`). "Entrar" = `RequestJoinRoom{room_id}`. |
| Resposta de entrada em sala | `net.rs: JoinRoomResult` | `allowed` (troca de partida com `SwitchMatch`), `doorbell_pending` (campainha tocada; `DoorbellState.is_waiting_approval`), `denied` (mostra o motivo). |
| Campainha (dono) | `navigator.rs: render_doorbell_popup`, `respond_to_doorbell` | Pop-up "fulano quer entrar"; responde `DoorbellResponse{request_id, allowed}`. |
| Configurações da sala | `navigator.rs: render_settings_overlay`, `handle_settings_interactions` | Renomear + visibilidade (`public`/`hidden`/privada) → `UpdateRoom{room_id, name, visibility}`; apagar com confirmação → `DeleteRoom{room_id}`. |
| Sala fechada pelo dono | `net.rs: RoomClosed` | Volta ao lobby (`RequestJoinRoom{"lobby"}`). |
| Inventário | `inventory.rs` | Grade de itens com categorias (`available_categories`), busca sem acento (`normalize_search`), detalhe, botão "colocar" (só dono). |
| Loja | `shop.rs` | Menu de categorias com subcategorias (acordeão), busca, grade de ofertas, "comprar". |
| Modo construção | `furniture.rs` | Fantasma na grade; clique confirma `PlaceItem` / `MoveItem`; R gira; Esc cancela; toque num móvel abre menu de contexto (girar `RotateItem`, mover, guardar `PickupItem`). |
| Criador de personagem | `character_creator.rs` | 7 categorias (rosto, pele, cabelo, superior, calças, calçados, acessórios), paleta de cores, aleatório, girar o avatar, "Confirmar e entrar" (`UpdateAppearance`). Catálogo vem de `get_catalogs` (cosméticos). |
| Status de conexão | `connection.rs`, `status_panel.rs` | `ConnectionStatus` (último evento, `reconnecting`); tela "Conectando..." e de erro (`friendly_error_message`); painel de **dev** (só fora de produção). |
| Notificações | — | **Não existe** sistema de notificações no HUD. Feedback hoje: linha de status da loja (`ShopStatus`), mensagem do navegador (`status_message`), aviso de campainha. |
| Chat | — | **Não existe** chat no cliente (não há mensagem de chat no `protocol`). |

Observação: o tamanho dos botões vem de `ui_scale.rs: HUD_SIZE_FACTOR` e as margens de `safe_area.rs`. No Compose isso **desaparece**: use `safeDrawingPadding()` e `dp`; os insets chegam ao Bevy por `setInsets` (já ligado em `BevyHostEffects`).

## 4. O que já existe na ponte × lacunas

### 4.1 Já existe (`hud_bridge.rs`, `onboarding.rs`)

| Nome | Direção | Payload |
|---|---|---|
| `hud_state` | Bevy → Compose | `{"name","silver","gold","in_room","reconnecting"}`, só quando muda |
| `hud_toggle_navigator` | Compose → Bevy | vazio; **só inverte `RoomNavigatorState.is_open`** |
| `onboarding_stage` e comandos `onboarding_*` | ambos | ver `onboarding.rs` |

Limitações do que existe: `hud_state` não tem o flag `loaded` da carteira (o Compose mostraria "0" antes do 1º saldo), nem título/visibilidade da sala nem `is_owner`; e `hud_toggle_navigator` **não** dispara os `Fetch*` que o botão real dispara (`handle_hud_buttons`), então a lista fica vazia/velha. Ao portar, mova a lógica de abrir para uma função compartilhada (ex.: `open_navigator(...)`) chamada pelo botão `bevy_hui` e pela mensagem, para não duplicar.

### 4.2 Lacunas a criar (propostas)

Convenção sugerida: Bevy → Compose publica **snapshots completos por tela** (JSON, só quando muda); Compose → Bevy manda **comandos pequenos** (idempotentes quando possível). Todos os nomes abaixo são propostas.

| Área | Mensagem | Dir. | Payload proposto |
|---|---|---|---|
| Carteira | `hud_state` (estender) | B→C | + `"wallet_loaded":bool`, `"room":{"id","name","visibility","is_owner"}`, `"build_mode":bool` |
| Carteira | `wallet_delta` | B→C | `{"silver":0,"gold":0,"reason":"daily_login"}` quando o servidor reporta ganho (para animação; o som continua no Bevy) |
| Navegador | `navigator_open` / `navigator_close` | C→B | vazio; abrem **e** disparam os `Fetch*` |
| Navegador | `navigator_state` | B→C | `{"loading":bool,"layouts":[RoomLayoutDto],"popular":[RoomSummaryDto],"mine":[RoomSummaryDto],"search":[RoomSummaryDto],"searching":bool,"message":null}` (campos de `RoomNavigatorState`) |
| Navegador | `navigator_search` | C→B | `{"query":".."}` → `SearchRooms` |
| Navegador | `room_create` | C→B | `{"name":"..","layout_id":".."}` → `CreateRoom` (o Bevy já entra na sala criada) |
| Navegador | `room_join` | C→B | `{"room_id":".."}` → `RequestJoinRoom` |
| Navegador | `join_status` | B→C | `{"status":"doorbell_pending"\|"denied","message":".."}` (de `DoorbellState`) |
| Sala (dono) | `room_update` / `room_delete` | C→B | `{"name","visibility"}` / vazio (usa a sala ativa) |
| Campainha | `doorbell_request` | B→C | `{"request_id","requester_name"}` ou `null` |
| Campainha | `doorbell_answer` | C→B | `{"request_id","allowed":bool}` |
| Menu avatar | `avatar_open_creator` / `sign_out` | C→B | vazio (reaproveitam a lógica de `handle_menu_buttons`) |
| Inventário | `inventory_open` | C→B | vazio → `FetchInventory` |
| Inventário | `inventory_state` | B→C | `{"loading":bool,"items":[InventoryItemDto]}` |
| Inventário | `build_place_start` | C→B | `{"catalog_id":".."}` → entra no modo construção com `BuildTool::Placing` (só dono) |
| Construção | `build_mode` | C→B | `{"active":bool}` |
| Construção | `build_rotate` / `build_cancel` | C→B | vazio (equivalentes a R e Esc) |
| Construção | `furniture_selected` | B→C | `{"item_id","catalog_id","name","x","y"}` (posição em dp do toque) ou `null` |
| Construção | `furniture_action` | C→B | `{"item_id":"..","action":"rotate"\|"move"\|"pickup"}` |
| Construção | `build_rejected` | B→C | `{"reason":".."}` (de `NetBuildModeRejectedEvent`) |
| Loja | `shop_open` | C→B | vazio → `FetchShopCatalog` |
| Loja | `shop_state` | B→C | `{"loading":bool,"categories":[ShopCategoryDto],"offers":[ShopOfferDto],"pending":null,"status":{"ok":bool,"text":".."}}` |
| Loja | `shop_buy` | C→B | `{"catalog_id":".."}` (quantidade 1, como hoje) |
| Miniaturas | `thumb_request` / `thumb_ready` | C→B / B→C | ver seção 7.3 |
| Criador | `creator_state` e comandos de escolha | ambos | ver seção 6.7 (a mais cara; deixe para o fim) |

Mantenha cada tela/snapshot independente: o Compose só assina o que está aberto, e o Bevy só publica enquanto a tela está aberta (ou publica sempre, mas só quando `is_changed()`).

## 5. Padrão de implementação

### 5.1 Rust: publicar um snapshot e processar um comando

Siga `hud_bridge.rs`. Um sistema publica só quando o valor muda, outro lê comandos:

```rust
// crates/game/src/hud_bridge.rs (padrão para cada tela nova)
#[derive(Serialize, PartialEq, Clone)]
struct ShopState {
    loading: bool,
    categories: Vec<ShopCategoryDto>, // DTOs do `protocol` já são Serialize
    offers: Vec<ShopOfferDto>,
    pending: Option<String>,
    status: Option<ShopStatusJson>,
}

fn publish_shop(state: Res<crate::shop::ShopState>, outbox: Res<Outbox>, mut last: Local<Option<ShopState>>) {
    if !state.is_changed() { return; }
    let snap = ShopState { /* copie os campos de `state` */ };
    if last.as_ref() != Some(&snap) {
        if let Ok(bytes) = serde_json::to_vec(&snap) { outbox.send_app("shop_state", bytes); }
        *last = Some(snap);
    }
}

fn handle_shop_commands(
    mut messages: MessageReader<HostMessage>,
    mut shop: ResMut<crate::shop::ShopState>,
    net: Option<Res<NakamaResource>>,
) {
    for msg in messages.read() {
        match msg.name.as_str() {
            "shop_open" => {
                shop.is_open = true; shop.is_loading = true; shop.status = None; shop.pending = None;
                if let Some(net) = &net { net.handle.send_command(NakamaCommand::FetchShopCatalog); }
            }
            "shop_buy" => { /* reutilize a lógica de `buy_offer` (extraia para função pura) */ }
            _ => {}
        }
    }
}
```

Duas regras: (1) **extraia a lógica dos handlers de botão `bevy_hui`** (`buy_offer`, `toggle_shop_from_hud`, `place_selected_item`...) para funções que o botão e a mensagem chamam, em vez de copiar; (2) registre os plugins da ponte só quando o HUD Compose estiver ligado (`hud_bridge::enabled`), como hoje.

### 5.2 Kotlin: repositório + ViewModel + Composable

```kotlin
// GameHost já expõe StateFlow; faça um repositório por tela e parse tipado.
@Serializable data class ShopOffer(val catalog_id: String, val category_id: String, val name: String,
    val width: Int, val depth: Int, val walkable: Boolean, val price_silver: Long, val price_gold: Long)
@Serializable data class ShopState(val loading: Boolean = false, val offers: List<ShopOffer> = emptyList(),
    val pending: String? = null /* ... */)

// No BevyEventListener do GameHost (thread do Bevy): só parse + value =
"shop_state" -> _shop.value = Json.decodeFromString(event.payload.decodeToString())

class ShopViewModel(private val game: GameHost) : ViewModel() {
    val state = game.shop            // StateFlow<ShopState>
    fun open() = game.send("shop_open")
    fun buy(id: String) = game.send("shop_buy", """{"catalog_id":"$id"}""")
}

@Composable fun ShopScreen(vm: ShopViewModel) {
    val s by vm.state.collectAsState()
    // grade de s.offers; botão desabilitado se s.pending != null ou saldo < preço
}
```

`game.send(name, json)` é um helper trivial sobre `host.sendMessage(name, json.encodeToByteArray())`. Use `ViewModel` multiplataforma (`androidx.lifecycle`, já usado pela lib) ou um plain class com `StateFlow`; o importante é **não** acessar `BevyHost` direto dos Composables.

## 6. Mapeamento por tela

Para cada tela: estado que o Bevy publica, comandos do Compose, eventos de volta, observações.

### 6.1 Carteira / HUD básico
- Estado: `hud_state` (estender com `wallet_loaded`, `room`, `build_mode`). Mostre "-" enquanto `wallet_loaded == false`, e formate com o mesmo `format_amount` (pt-BR, ponto de milhar).
- Comandos: nenhum (heartbeat e `FetchWallet` continuam no Bevy).
- Evento: `wallet_delta` opcional para animar ganho. O som de ouro permanece no Bevy.
- Visibilidade dos botões: Construção e Configurações só quando `room.is_owner`.

### 6.2 Menu do avatar
- Compose: `DropdownMenu` ancorado no botão (ele abre num `Popup`, que é janela própria no Android: teste se as barras do sistema voltam; ver armadilhas). Itens: Visuais → `avatar_open_creator`; Sair → `sign_out`.
- Bevy: `avatar_open_creator` = `creator_state.return_to_room = true; next_state.set(CharacterCreator)`. `sign_out` = o corpo de `handle_menu_buttons` (revogar sessão, `sign_out`, remover `NakamaResource`, `Login`); o Compose então volta a mostrar o onboarding via `onboarding_stage`.

### 6.3 Navegador de salas e configurações
- Estado: `navigator_state` (campos de `RoomNavigatorState`). Públicas (lobby/cafe) são fixas no código hoje; publique-as no snapshot (`public`) em vez de duplicar no Kotlin.
- Comandos: `navigator_open/close`, `navigator_search`, `room_create`, `room_join`, `room_update`, `room_delete`.
- Eventos: `join_status` (campainha pendente/negada), `doorbell_request` (dono). `RoomCreated` já entra na sala sozinho (`net.rs`).
- Teclado: campos de texto são `TextField` do Compose. **A ponte de teclado do bevy_hui deixa de ser necessária** nessas telas (vantagem: resolve o ponto "teclado nos `TextInput`" da lista de pendências do MR). Valide `MAX_NAME_CHARS = 50` no Compose também, mas o servidor continua validando.

### 6.4 Campainha
- `doorbell_request` (B→C) abre um `AlertDialog`/card; resposta `doorbell_answer`. O Bevy limpa `incoming_request` ao responder.

### 6.5 Inventário e modo construção
Ver seção 7. Resumo: `inventory_open` → `inventory_state`; "Colocar" → `build_place_start {catalog_id}` e **fecha a janela Compose**; o toque no chão confirma (Bevy) e manda `PlaceItem`.

### 6.6 Conexão, erros e notificações
- `hud_state.reconnecting`: mostre um indicador discreto (como o `ComposeHudTest` já faz).
- Erro fatal da conexão (`AppState::ConnectionError`) hoje é uma tela `bevy_hui` (`connection.rs`). Proposta: `connection_state {"phase":"connecting"|"error","message":".."}` e comando `connection_retry`; o texto vem de `friendly_error_message`.
- Painel de dev: continua no Bevy (só fora de produção); não porte.
- Notificações: não existem. Se criar, use uma fila de `toast {"text","kind"}` (B→C) em vez de muitos estados.

### 6.7 Criador de personagem (última coisa a portar)
É a tela mais acoplada ao 3D: tem pré-visualização do avatar em render (`AvatarPreviewRoot`) e arrasto para girar. Sugestão: o Compose desenha as abas/paleta/itens e **o Bevy continua desenhando o avatar** num retângulo reservado (o overlay deixa essa área sem componente para o toque passar). Estado: `creator_state {"category","appearance":AvatarAppearance,"items":[CatalogItem da categoria],"editing":bool}`. Comandos: `creator_category`, `creator_pick_item {id}`, `creator_pick_color {hex}`, `creator_random`, `creator_confirm`, `creator_close`.

## 7. Inventário e loja/catálogo em detalhe

### 7.1 De onde vêm os dados

| Dado | Fonte | Quando | Código |
|---|---|---|---|
| Saldo | RPC Nakama `get_wallet` / heartbeat | ao entrar na sala e a cada 60 s | `wallet.rs`, `NetWalletUpdatedEvent` |
| Catálogo da loja (`categories`, `offers`) | RPC da loja (`FetchShopCatalog`) | **ao abrir a loja** | `shop.rs`, `NetShopCatalogLoadedEvent` |
| Inventário | RPC do inventário (`FetchInventory`) | ao abrir; também após compra e após guardar/colocar móvel | `furniture.rs` ~l. 487, `shop.rs: apply_purchase_results` |
| Cosméticos e móveis (`get_catalogs`) | `FetchCatalogs{if_version}` | ao conectar; cache em disco por host (`catalog_sync.rs`) | `NetCatalogsLoadedEvent` |
| Miniatura de item | CDN: `ui/thumbs/{catalog_id}.png` | baixada pelo Bevy (tier e TTL: `ai-docs/decisions/0011-assets-via-cdn.md` e `ai-docs/specs/0010-assets-cdn.md`) | `inventory.rs: thumbnail_path`, `cdn_assets.rs` |
| Ícones de categoria/HUD | embutidos no app: `assets/ui/icons/*.png` | sempre | `shop.rs: category_icon_path`, `inventory.rs: category_icon` |
| Modelo 3D do móvel | CDN: `furniture/*.glb` | quando a sala precisa | `furniture.rs: furniture_model` |

DTOs (`crates/protocol/src/messages.rs`):
- `InventoryItemDto { id, catalog_id, name, category, quantity, width, depth, walkable, color, asset_id }`.
- `ShopCategoryDto { id, name, icon, parent }` (`parent` vazio = nível superior); `ShopOfferDto { catalog_id, category_id, name, width, depth, walkable, price_silver, price_gold }`.
- `PurchaseItemResponse { catalog_id, quantity, silver, gold }` (já traz o saldo novo).

### 7.2 Regras de negócio que o Compose precisa reproduzir (ou delegar)

- **Categorias do inventário**: `all` + as categorias presentes, na ordem em que aparecem (`available_categories`). Rótulos: `collider` → "Móveis", `floor_decoration` → "Pisos", outras: o próprio id (`category_label`). Tipo: `walkable` → "Piso decorativo", senão "Móvel sólido" (`type_label`).
- **Busca**: minúsculas e sem acento (`normalize_search`: á→a, ç→c…). Faça no Kotlin igual, ou deixe o Bevy filtrar e publique a lista já filtrada (mais simples e sem divergência). Recomendação: **o Bevy filtra** (a função já está testada) e o Compose manda `inventory_filter {"category","query"}`.
- **Seleção**: `resolve_selection` mantém o item selecionado se ainda visível, senão o primeiro. Faça igual no ViewModel.
- **Colocar**: só se `is_owner` (`place_selected_item` ignora o clique caso contrário). Esconda/desabilite o botão para quem não é dono. A quantidade vem de `quantity`; o servidor decide se pode colocar.
- **Loja, menu**: categorias de nível superior com subcategorias em acordeão (`menu_rows`, `press_category`, `holds_selection`); uma seção fica aberta quando contém a seleção. Busca com `filter_offers` (mesma normalização).
- **Comprar**: `can_afford = wallet.covers(price_silver, price_gold)` (preço 0 = não cobra naquela moeda). É só atalho de UX: o servidor revalida tudo. Falta de saldo mostra "Saldo insuficiente." sem ir à rede. Durante a compra `pending = Some(catalog_id)` bloqueia clique duplo.
- **Depois da compra**: sucesso → "`{nome}` adicionado ao inventário!", `FetchInventory` automático, saldo atualizado pela própria resposta. Falha → mensagem do servidor com a 1ª letra maiúscula (`capitalize`).

### 7.3 Como o Compose carrega as miniaturas

As miniaturas **não** estão no APK/IPA: ficam no CDN e o Bevy as baixa para o cache (`CdnAssets` → arquivos em disco, índice `CdnIndex`: caminho lógico → arquivo). O Compose não deve baixar do CDN por conta própria (duplicaria cache, hash e TTL). Proposta:

1. Compose manda `thumb_request {"catalog_ids":[".."]}` ao exibir uma célula (de-duplicado; só os visíveis, como `LazyVerticalGrid` faz).
2. Bevy chama `CdnAssets::request("ui/thumbs/{id}.png")` (já faz o dedupe/retry de 30 s) e, quando o arquivo entra no `CdnIndex` (`SyncEvent::FileReady`), publica `thumb_ready {"catalog_id","path":"/…/arquivo.png"}`.
3. Kotlin lê os bytes do `path` (no app, é arquivo local do próprio processo) e decodifica para `ImageBitmap` (ex.: `decodeToImageBitmap` do Compose Multiplatform), guardando num `LruCache` pelo `catalog_id`. Placeholder (ícone de categoria embutido) enquanto não chega.

Os ícones de `assets/ui/icons/*.png` são embutidos: copie-os para `composeResources` do `shared` (a fonte única continua `assets/ui/icons`; automatize a cópia no Gradle para não divergir) ou use `Icons` do Material onde servir.

### 7.4 Estados de carregando e erro
- `loading` true entre o comando (`*_open`) e a resposta. Mostre esqueleto/spinner, e **timeout visual** (ex.: 10 s) com botão "Tentar de novo" que reenvia `*_open`: hoje o `bevy_hui` fica em "Carregando" se a rede cair.
- Sem conexão (`reconnecting`): desabilite "Comprar" e "Colocar" e mostre o aviso; comandos enviados nesse intervalo podem ser perdidos.
- Erros de compra e de construção vêm como texto do servidor; mostre no lugar do `ShopStatus` e como toast.

### 7.5 Colocação de móveis (a parte que mistura Compose e 3D)
1. Compose: "Colocar" → `build_place_start {catalog_id}`; **fecha a janela** (como `place_selected_item`, que faz `inventory.is_open = false`).
2. Bevy: entra no modo construção (`BuildTool::Placing{rotation:0,...}`, `skip_first_click = true`) e desenha o fantasma sob o dedo.
3. O jogador toca no chão: o Bevy confirma e manda `PlaceItem{catalog_id,x,z,rotation}`. **O toque tem de chegar ao jogo**: não deixe o overlay Compose cobrir o chão (veja armadilhas).
4. Rotação: botão Compose → `build_rotate` (`rotation = (rotation+1)%4`); Cancelar → `build_cancel`.
5. Toque num móvel já colocado → o Bevy publica `furniture_selected` (com posição em dp); o Compose mostra o menu de contexto e responde `furniture_action`. Resultados vêm por `ItemPlaced/Moved/Rotated/Removed` (broadcast do servidor) e `build_rejected` em caso de recusa.

## 8. Ordem recomendada (passos pequenos e testáveis)

Cada passo: uma mensagem nova, uma tela mínima, teste no S22 com a flag e um teste Rust de serialização (como `state_serialises_to_the_documented_keys`).

1. **Carteira completa**: estender `hud_state` (`wallet_loaded`, `room`, `build_mode`) e mostrar título, moedas e visibilidade dos botões por `is_owner`. Sem comandos novos.
2. **Infra Kotlin**: `kotlinx-serialization`, `game.send(name, json)`, um repositório/VM por tela, parse tipado. Refatore `parseHud` para isso.
3. **Menu do avatar** (`avatar_open_creator`, `sign_out`): poucos comandos, valida o padrão comando→estado.
4. **Navegador de salas** (abrir/fechar com `Fetch*`, buscar, entrar, criar). Corrija a lacuna do `hud_toggle_navigator`. Teste: entrar no Coffee House.
5. **Configurações da sala + campainha** (dono). Teste com 2 contas: pedir entrada em sala privada e responder.
6. **Loja** (`shop_open/state/buy`), primeiro sem miniaturas (ícone de categoria). Teste: comprar e ver saldo e inventário mudarem.
7. **Miniaturas** (`thumb_request/ready` + cache de bitmap).
8. **Inventário** + `build_place_start`, depois menu de contexto de móvel.
9. **Conexão/erros** (`connection_state`) e toasts.
10. **Criador de personagem** por último.
11. Só quando tudo estiver estável: desligar o HUD `bevy_hui` (hoje a flag só **adiciona** o Compose por cima; não existe flag para esconder o antigo). Proposta: `HUD_BEVY=0` que não registra `setup_hud` e as janelas, mantendo as funções puras (`shop.rs`, `inventory.rs`).

Mantenha o HUD de teste **desligado por padrão** em todos os passos: a ponte Rust só é registrada com `COMPOSE_HUD=1` (`hud_bridge::enabled`) e builds Release descartam o flag.

## 9. Como testar

- Android Debug: `cd kmp && EXTRA_DEV_ENV=$'COMPOSE_HUD=1\n' ./gradlew :androidApp:assembleDebug` e instale com `adb install -r`. Em `GAME_DEV_MODE`/DEV use "Entrar sem Google (dev)". Para ler FPS/memória, `files/status.txt` do app (`adb shell run-as br.eng.joaovictor.resort cat files/status.txt`).
- iOS: o build phase **não** escreve `COMPOSE_HUD`; para testar no iPhone/simulador adicione a linha ao `dev.env` gerado (ainda não suportado pelo script; está na lista de pendências). Avise o João antes de usar o iPhone físico.
- Unidade: cada snapshot e comando tem teste Rust (`cargo test -p game --no-default-features --features kmp-host hud_bridge`); a lógica pura (`filter_items`, `can_afford`, `menu_rows`) já tem testes, reaproveite-os.
- Não grave tokens em log nem em arquivo; as mensagens do HUD **nunca** devem carregar tokens (só dados de tela).

## 10. Armadilhas conhecidas

- **`ModalBottomSheet` e `Dialog`** abrem em janela própria no Android: trazem as barras do sistema de volta e mudam os insets/layout do jogo enquanto abertos. A decisão do projeto é **não usar `ModalBottomSheet`**. `AlertDialog` funciona (testado), mas com o mesmo cuidado. Prefira cartões/painéis desenhados no mesmo `Box` sobre o `BevyView`. Detalhes e o contorno testado no S22: `docs/sobrepor-compose.md` no bevy_kmp. Menus (`DropdownMenu`) e `Popup` também são janelas: teste.
- **Toque**: um `Box(Modifier.fillMaxSize())` clicável ou com `pointerInput` consome o toque e **o jogo para de receber** o dedo (sem picking, sem câmera, sem colocar móvel). Overlays devem ocupar só o espaço dos controles. Telas "cheias" (loja, inventário) devem consumir o toque de propósito (e o jogo fica pausado visualmente por baixo, como hoje).
- **Translucidez**: funciona (`Color.Black.copy(alpha=...)`); cuidado com a legibilidade sobre cena clara.
- **Performance**: o Compose roda na thread principal e o Bevy na dele; o custo do overlay no S22 foi pequeno no teste, mas medimos pouco (Debug). Publique snapshots **só quando mudam** (comparar com `Local<Option<T>>`), nunca por frame; listas grandes (inventário) em `LazyVerticalGrid`; miniaturas com cache de bitmap.
- **Dois HUDs ao mesmo tempo** duplicam botões: durante a migração, esconda no Compose o que o `bevy_hui` ainda desenha (ou o contrário) por flag, para não ter duas cópias do mesmo botão.
- **Pausa**: mensagens enviadas com o app pausado só são lidas depois de retomar; não presuma resposta imediata.
- **Estado duplicado**: `ShopState`, `PlayerInventoryState` etc. são recursos do Bevy; o Compose **não** mantém cópia própria além do último snapshot. Qualquer regra nova de negócio entra no Rust (ou no servidor), não no Kotlin.
- **Texto em pt-BR e acentos**: strings de UI hoje estão no Rust (`category_label`, `purchase_success_text`, `visibility_label`). Decida (ideal: o Compose traduz a partir de códigos e o Bevy manda só códigos/ids), senão o texto fica nos dois lados.
- **Segredos**: nada de `NAKAMA_SERVER_KEY`, tokens ou senhas de keystore em mensagens, logs, docs ou commits.

## 11. Referências no código

- Ponte atual: `crates/game/src/hud_bridge.rs`, `crates/game/src/onboarding.rs`, `crates/game/src/kmp_host.rs`.
- Kotlin: `kmp/shared/src/commonMain/kotlin/br/eng/joaovictor/resort/{GameHost,App,ComposeHud,Onboarding}.kt`.
- Lógica do HUD: `wallet.rs`, `navigator.rs`, `navigator_ui.rs`, `avatar_menu.rs`, `furniture.rs`, `inventory.rs`, `shop.rs`, `character_creator.rs`, `connection.rs`, `net.rs`, `catalog_sync.rs`, `cdn_assets.rs`.
- Contrato de rede (dono: servidor): `contract/` e `crates/protocol/src/messages.rs`; comandos/eventos do cliente de rede: `crates/nakama_client/src/events.rs`.
- Biblioteca: `docs/arquitetura.md`, `docs/guia-de-integracao.md` e `docs/sobrepor-compose.md` no repositório bevy_kmp.
