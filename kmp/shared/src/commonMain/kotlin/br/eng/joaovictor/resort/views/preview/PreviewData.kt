package br.eng.joaovictor.resort.views.preview

import br.eng.joaovictor.resort.OnboardingStage
import br.eng.joaovictor.resort.views.hud.HudUiState
import br.eng.joaovictor.resort.views.inventory.ALL_CATEGORIES
import br.eng.joaovictor.resort.views.inventory.InventoryRow
import br.eng.joaovictor.resort.views.inventory.InventoryUiState

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

    val onboardingLogin = OnboardingStage(stage = "login")
    val onboardingNickname = OnboardingStage(stage = "nickname")
}
