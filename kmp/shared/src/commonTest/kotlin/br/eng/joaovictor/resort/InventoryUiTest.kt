package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.models.InventoryItem
import br.eng.joaovictor.resort.models.InventoryState
import br.eng.joaovictor.resort.views.inventory.toUi
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class InventoryUiTest {
    @Test
    fun marksTheSelectedItemAndHidesAbsentQuantities() {
        val ui = InventoryState(
            open = true,
            items = listOf(
                InventoryItem(catalogId = "chair", name = "Chair", quantity = 3),
                InventoryItem(catalogId = "rug", name = "", quantity = 0),
            ),
            selectedCatalogId = "rug",
        ).toUi()
        assertTrue(ui.open)
        assertEquals(3, ui.rows[0].quantity)
        assertFalse(ui.rows[0].selected)
        assertEquals("rug", ui.rows[1].name)
        assertNull(ui.rows[1].quantity)
        assertTrue(ui.rows[1].selected)
    }

    @Test
    fun emptyOnlyWhenNotLoading() {
        assertTrue(InventoryState(open = true).toUi().isEmpty)
        assertFalse(InventoryState(open = true, loading = true).toUi().isEmpty)
    }
}
