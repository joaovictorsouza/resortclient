package br.eng.joaovictor.resort.views.preview

import br.eng.joaovictor.resort.OnboardingStage
import br.eng.joaovictor.resort.views.hud.HudUiState
import br.eng.joaovictor.resort.views.inventory.ALL_CATEGORIES
import br.eng.joaovictor.resort.views.inventory.InventoryRow
import br.eng.joaovictor.resort.models.DoorbellInfo
import br.eng.joaovictor.resort.models.SettingsInfo
import br.eng.joaovictor.resort.models.ShopCategory
import br.eng.joaovictor.resort.views.inventory.InventoryUiState
import br.eng.joaovictor.resort.views.creator.CreatorOption
import br.eng.joaovictor.resort.views.creator.CreatorSwatch
import br.eng.joaovictor.resort.views.creator.CreatorTab
import br.eng.joaovictor.resort.views.creator.CreatorUiState
import br.eng.joaovictor.resort.views.navigator.LayoutChoice
import br.eng.joaovictor.resort.views.navigator.NavTab
import br.eng.joaovictor.resort.views.navigator.NavigatorUiState
import br.eng.joaovictor.resort.views.navigator.RoomCard
import br.eng.joaovictor.resort.views.shop.ShopTile
import br.eng.joaovictor.resort.views.shop.ShopUiState

/**
 * Fake UI states for `@Preview`s, in one place so a new screen copies the pattern: build the plain UI
 * state here (never a ViewModel, repository or the game) and `.copy(...)` it per preview state.
 */
internal object PreviewData {
    val hudInRoom = HudUiState(
        visible = true,
        roomTitle = "Casa do João  •  [Pública]",
        silver = "1.234",
        gold = "56",
    )

    private fun row(id: String, name: String, category: String = "collider", quantity: Int? = 1, selected: Boolean = false) =
        InventoryRow(id, name, category, quantity, selected, thumbnail = "") // empty thumbnail => placeholder

    val inventoryRows = listOf(
        row("chair", "Poltrona azul", quantity = 3),
        row("rug", "Tapete redondo", "floor_decoration", selected = true),
        row("lamp", "Luminária de chão com um nome bem comprido para testar quebra de linha", quantity = 12),
        row("sofa", "Sofá", quantity = null),
        row("cat", "Gato", "pet"),
    )

    val inventoryLoaded = InventoryUiState(
        open = true,
        rows = inventoryRows,
        categories = listOf(ALL_CATEGORIES, "collider", "floor_decoration", "pet"),
        canPlace = true,
    )

    /** Enough tiles to fill several grid rows (one selected, mixed quantities). */
    val inventoryMany = inventoryLoaded.copy(
        rows = List(30) { i ->
            row("item$i", "Item número $i", if (i % 3 == 0) "floor_decoration" else "collider", quantity = 1 + i % 4, selected = i == 4)
        },
    )

    private fun tile(i: Int, silver: Long, gold: Long, affordable: Boolean, selected: Boolean = false) =
        ShopTile("offer$i", if (i == 2) "Sofá de três lugares com um nome bem comprido" else "Móvel $i", "", silver, gold, affordable, selected)

    val shopLoaded = ShopUiState(
        open = true,
        topCategories = listOf(ShopCategory("furniture", "Móveis"), ShopCategory("floors", "Pisos"), ShopCategory("pets", "Animais")),
        subCategories = listOf(ShopCategory("seats", "Assentos", parent = "furniture"), ShopCategory("tables", "Mesas", parent = "furniture")),
        selectedCategory = "seats",
        tiles = List(12) { tile(it, silver = 50L * (it + 1), gold = if (it % 4 == 3) 2 else 0, affordable = true, selected = it == 1) },
        selected = tile(1, silver = 100, gold = 0, affordable = true, selected = true),
        silver = 1234,
        gold = 5,
        walletLoaded = true,
    )

    val shopPoor = shopLoaded.copy(
        selected = tile(7, silver = 5000, gold = 20, affordable = false, selected = true),
        silver = 10,
        gold = 0,
    )

    val navigatorPublic = NavigatorUiState(
        open = true,
        tab = NavTab.Public,
        cards = listOf(
            RoomCard("lobby", "The Grand Lobby", "Espaço público central do hotel para interagir e conversar.  •  Capacidade 100"),
            RoomCard("cafe", "Coffee House", "Cafeteria aconchegante para relaxar com amigos.  •  Capacidade 40"),
        ),
    )

    fun navigatorRooms(tab: NavTab) = navigatorPublic.copy(
        tab = tab,
        cards = List(6) { RoomCard("room$it", if (it == 2) "Sala com um nome bem comprido para testar a quebra de linha" else "Sala $it", "Criado por Ana  •  Ocupação ${it + 1}/20  •  Pública") },
    )

    val navigatorCreate = navigatorPublic.copy(
        tab = NavTab.Create,
        cards = emptyList(),
        createName = "Minha sala",
        layouts = listOf(
            LayoutChoice("small", "Pequena", "8x8 (máx. 10)", selected = true),
            LayoutChoice("big", "Grande", "16x16 (máx. 40)", selected = false),
        ),
    )

    private fun tabs(selected: String) = listOf("body" to "Corpo", "face" to "Rosto", "skin" to "Pele", "hair" to "Cabelo", "top" to "Superior", "shoes" to "Calçados")
        .map { (k, l) -> CreatorTab(k, l, selected = k == selected) }

    private val skinTones = listOf("#FFE0BD", "#E8BEAC", "#C68642", "#8D5524", "#4A2C0F")

    val creatorHair = CreatorUiState(
        active = true,
        confirmLabel = "Confirmar e entrar no jogo",
        tabs = tabs("hair"),
        swatches = listOf("#1C1C1C", "#5A3825", "#B55239", "#D6B370", "#E91E63", "#2196F3").mapIndexed { i, h -> CreatorSwatch(h, selected = i == 2) },
        options = List(10) { CreatorOption("hair$it", "Penteado $it", "", selected = it == 3) },
    )

    val creatorSkin = creatorHair.copy(
        tabs = tabs("skin"),
        swatches = skinTones.mapIndexed { i, h -> CreatorSwatch(h, selected = i == 1) },
        options = emptyList(),
    )

    val creatorBody = creatorHair.copy(
        tabs = tabs("body"),
        swatches = emptyList(),
        options = listOf(CreatorOption("female", "Feminino", "", selected = true), CreatorOption("male", "Masculino", "", selected = false)),
    )

    val roomSettings = SettingsInfo(name = "Casa do João", visibility = "hidden")
    val doorbell = DoorbellInfo(requestId = "r1", requesterName = "Ana")

    val onboardingLogin = OnboardingStage(stage = "login")
    val onboardingNickname = OnboardingStage(stage = "nickname")
}
