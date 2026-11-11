package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.ds.components.LockReason
import br.eng.joaovictor.resort.ds.components.Price
import br.eng.joaovictor.resort.ds.components.TileState
import br.eng.joaovictor.resort.ds.components.tileDescription
import br.eng.joaovictor.resort.models.ShopCategory
import br.eng.joaovictor.resort.views.inventory.ALL_CATEGORIES
import br.eng.joaovictor.resort.views.inventory.InventoryRow
import br.eng.joaovictor.resort.views.inventory.InventoryUiState
import br.eng.joaovictor.resort.views.inventory.inventoryRail
import br.eng.joaovictor.resort.views.inventory.inventoryTileState
import br.eng.joaovictor.resort.views.shop.ShopTile
import br.eng.joaovictor.resort.views.shop.ShopUiState
import br.eng.joaovictor.resort.views.shop.buyLabel
import br.eng.joaovictor.resort.views.shop.offerPrice
import br.eng.joaovictor.resort.views.shop.offerTileState
import br.eng.joaovictor.resort.views.shop.shopRail
import br.eng.joaovictor.resort.views.shop.shopSection
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PanelScreensUiTest {
    private fun row(selected: Boolean = false, qty: Int? = 1) = InventoryRow("c", "Poltrona", "collider", qty, selected, "")
    private fun tile(affordable: Boolean = true, selected: Boolean = false) = ShopTile("o", "Sofá", "", 450, 0, affordable, selected)
    private val cats = listOf(
        ShopCategory(id = "f", name = "Móveis", parent = ""),
        ShopCategory(id = "f1", name = "Cadeiras", parent = "f"),
        ShopCategory(id = "p", name = "Pisos", parent = ""),
    )

    @Test
    fun anInventoryTileIsLockedWhenThePlayerCannotPlace() {
        assertNull(inventoryTileState(row(), canPlace = true).locked)
        assertEquals(LockReason.NotOwner, inventoryTileState(row(), canPlace = false).locked)
        assertTrue(inventoryTileState(row(selected = true), true).selected)
        assertEquals(3, inventoryTileState(row(qty = 3), true).quantity)
    }

    @Test
    fun theInventoryRailListsEveryCategoryWithItsLabel() {
        val ui = InventoryUiState(categories = listOf(ALL_CATEGORIES, "collider", "pet"))
        assertEquals(listOf("Todos", "Móveis", "Animais"), inventoryRail(ui).map { it.label })
        assertEquals(listOf(ALL_CATEGORIES, "collider", "pet"), inventoryRail(ui).map { it.key })
    }

    @Test
    fun anUnaffordableOfferIsLockedButStillATile() {
        assertNull(offerTileState(tile()).locked)
        assertEquals(LockReason.CannotAfford, offerTileState(tile(affordable = false)).locked)
        assertTrue(offerTileState(tile(selected = true)).selected)
        assertEquals(Price(450, 0), offerPrice(tile()))
    }

    @Test
    fun theBuyLabelExplainsAShortWallet() {
        val ui = ShopUiState(tiles = listOf(tile(affordable = false, selected = true)), selected = tile(affordable = false, selected = true))
        assertEquals("Saldo insuficiente", buyLabel(ui))
        assertEquals("Comprar", buyLabel(ShopUiState(selected = tile())))
        assertEquals("Comprar", buyLabel(ShopUiState()))
    }

    @Test
    fun theShopRailExpandsOnlyTheSectionHoldingTheSelection() {
        val top = cats.filter { it.parent.isEmpty() }
        val sub = cats.filter { it.parent == "f" }
        val inSub = shopRail(ShopUiState(topCategories = top, subCategories = sub, selectedCategory = "f1"))
        assertEquals(listOf("f", "p"), inSub.map { it.key })
        assertTrue(inSub[0].expanded)
        assertEquals(listOf("f1"), inSub[0].children.map { it.key })
        assertFalse(inSub[1].expanded)
        val onTop = shopRail(ShopUiState(topCategories = top, selectedCategory = "p"))
        assertTrue(onTop.none { it.expanded })
    }

    @Test
    fun theSectionOfASubCategoryIsItsParent() {
        val top = cats.filter { it.parent.isEmpty() }
        val sub = cats.filter { it.parent == "f" }
        assertEquals("f", shopSection(ShopUiState(topCategories = top, subCategories = sub, selectedCategory = "f1")))
        assertEquals("p", shopSection(ShopUiState(topCategories = top, selectedCategory = "p")))
    }

    @Test
    fun tilesAreDescribedForScreenReaders() {
        assertEquals("Sofá, 450 pratas", tileDescription("Sofá", Price(450, 0), TileState()))
        assertEquals("Troféu, 1.200 pratas e 3 ouros, bloqueado: saldo insuficiente", tileDescription("Troféu", Price(1200, 3), TileState(locked = LockReason.CannotAfford)))
        assertEquals("Brinde, grátis, novo", tileDescription("Brinde", Price(0, 0), TileState(isNew = true)))
        assertEquals("Cadeira, quantidade 3", tileDescription("Cadeira", null, TileState(quantity = 3)))
        assertEquals("Cadeira", tileDescription("Cadeira", null, TileState(quantity = 1)))
    }
}
