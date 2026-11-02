package br.eng.joaovictor.resort

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import br.eng.joaovictor.resort.models.InventoryItem
import br.eng.joaovictor.resort.models.InventoryState
import br.eng.joaovictor.resort.repositories.FakeGateway
import br.eng.joaovictor.resort.repositories.InventoryRepository
import br.eng.joaovictor.resort.views.inventory.ALL_CATEGORIES
import br.eng.joaovictor.resort.views.inventory.clampWindow
import br.eng.joaovictor.resort.views.inventory.dragPhase
import br.eng.joaovictor.resort.views.inventory.dropOutside
import br.eng.joaovictor.resort.views.inventory.normalizeSearch
import br.eng.joaovictor.resort.views.inventory.toUi
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class InventoryUiTest {
    private fun state(selected: String? = null) = InventoryState(
        open = true,
        items = listOf(
            InventoryItem(catalogId = "chair", name = "Poltrona", category = "collider", quantity = 3, thumbnail = "/c/chair.png"),
            InventoryItem(catalogId = "rug", name = "", category = "floor_decoration", quantity = 0),
            InventoryItem(catalogId = "cat", name = "Gato", category = "pet", quantity = 1),
        ),
        selectedCatalogId = selected,
    )

    @Test
    fun marksTheSelectedItemAndHidesAbsentQuantities() {
        val ui = state(selected = "rug").toUi()
        assertTrue(ui.open)
        assertEquals(3, ui.rows[0].quantity)
        assertEquals("/c/chair.png", ui.rows[0].thumbnail)
        assertFalse(ui.rows[0].selected)
        assertEquals("rug", ui.rows[1].name)
        assertNull(ui.rows[1].quantity)
        assertTrue(ui.rows[1].selected)
    }

    @Test
    fun emptyOnlyWhenNotLoadingAndNothingMatches() {
        assertTrue(InventoryState(open = true).toUi().isEmpty)
        assertFalse(InventoryState(open = true, loading = true).toUi().isEmpty)
        assertTrue(state().toUi(query = "zzz").isEmpty)
    }

    @Test
    fun categoriesAreFirstSeenAfterAllAndFilterTheList() {
        val ui = state().toUi(category = "collider")
        assertEquals(listOf(ALL_CATEGORIES, "collider", "floor_decoration", "pet"), ui.categories)
        assertEquals(listOf("chair"), ui.visible.map { it.catalogId })
        // A category that vanished falls back to everything.
        assertEquals(ALL_CATEGORIES, state().toUi(category = "gone").category)
    }

    @Test
    fun searchIgnoresCaseAndAccents() {
        assertEquals("divisoria", normalizeSearch("Divisória"))
        assertEquals(listOf("chair"), state().toUi(query = "POLTRONA").visible.map { it.catalogId })
        assertEquals(listOf("cat"), state().toUi(query = "gat").visible.map { it.catalogId })
    }

    @Test
    fun petsCannotBeDragged() {
        val rows = state().toUi().rows
        assertTrue(rows[0].draggable)
        assertFalse(rows[2].draggable)
    }

    @Test
    fun theWindowStaysInsideTheScreen() {
        assertEquals(Offset(0f, 0f), clampWindow(Offset(-50f, -9f), 1000f, 600f, 400f, 300f))
        assertEquals(Offset(600f, 300f), clampWindow(Offset(900f, 500f), 1000f, 600f, 400f, 300f))
        assertEquals(Offset(10f, 20f), clampWindow(Offset(10f, 20f), 1000f, 600f, 400f, 300f))
        // A window bigger than the area pins to the origin instead of crashing.
        assertEquals(Offset(0f, 0f), clampWindow(Offset(5f, 5f), 100f, 100f, 400f, 300f))
    }

    @Test
    fun aDropCountsOnlyOutsideTheWindow() {
        val window = Rect(100f, 100f, 500f, 400f)
        assertFalse(dropOutside(Offset(200f, 200f), window))
        assertTrue(dropOutside(Offset(50f, 200f), window))
        assertTrue(dropOutside(Offset(200f, 450f), window))
    }

    @Test
    fun dragPhasesFollowTheFingerAcrossTheWindowEdge() {
        assertNull(dragPhase(wasOutside = false, nowOutside = false, released = false)) // still inside
        assertEquals("move", dragPhase(wasOutside = false, nowOutside = true, released = false)) // just left
        assertEquals("move", dragPhase(wasOutside = true, nowOutside = true, released = false))
        assertEquals("cancel", dragPhase(wasOutside = true, nowOutside = false, released = false)) // came back
        assertEquals("end", dragPhase(wasOutside = true, nowOutside = true, released = true)) // released outside
        assertEquals("cancel", dragPhase(wasOutside = true, nowOutside = false, released = true)) // released inside
        assertNull(dragPhase(wasOutside = false, nowOutside = false, released = true))
    }

    @OptIn(ExperimentalCoroutinesApi::class)
    @Test
    fun dragSendsTheSurfacePixelsAndPhase() = runTest {
        val gateway = FakeGateway()
        val repo = InventoryRepository(gateway, CoroutineScope(UnconfinedTestDispatcher(testScheduler)))
        repo.drag("chair", 640f, 320.5f, "move")
        assertEquals("inventory_drag", gateway.sent.single().first)
        assertEquals("""{"catalog_id":"chair","x":640.0,"y":320.5,"phase":"move"}""", gateway.sent.single().second)
    }
}
