package br.eng.joaovictor.resort.views.preview

import br.eng.joaovictor.resort.OnboardingStage
import br.eng.joaovictor.resort.views.hud.HudUiState
import br.eng.joaovictor.resort.views.inventory.ALL_CATEGORIES
import br.eng.joaovictor.resort.views.inventory.InventoryRow
import br.eng.joaovictor.resort.models.ShopCategory
import br.eng.joaovictor.resort.views.inventory.InventoryUiState
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

    val onboardingLogin = OnboardingStage(stage = "login")
    val onboardingNickname = OnboardingStage(stage = "nickname")
}
