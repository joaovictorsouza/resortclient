package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.views.hud.DockId
import br.eng.joaovictor.resort.views.hud.HudUiState
import br.eng.joaovictor.resort.views.hud.dockActions
import kotlin.test.Test
import kotlin.test.assertEquals

class DockActionsTest {
    private val visitor = HudUiState(visible = true)

    @Test
    fun aVisitorSeesRoomsItemsAndShop() {
        assertEquals(listOf(DockId.Navigator, DockId.Inventory, DockId.Shop), dockActions(visitor))
    }

    @Test
    fun anOwnerAlsoSeesBuild() {
        assertEquals(
            listOf(DockId.Navigator, DockId.Inventory, DockId.Shop, DockId.Build),
            dockActions(visitor.copy(isOwner = true)),
        )
    }

    @Test
    fun inBuildModeTheDockIsItemsAndDone() {
        assertEquals(listOf(DockId.Inventory, DockId.BuildDone), dockActions(visitor.copy(isOwner = true, buildMode = true)))
    }

    @Test
    fun buildModeWithoutOwnershipDoesNotChangeTheVisitorDock() {
        assertEquals(dockActions(visitor), dockActions(visitor.copy(buildMode = true)))
    }
}
