package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.models.ShopCategory
import br.eng.joaovictor.resort.models.ShopOffer
import br.eng.joaovictor.resort.models.ShopState
import br.eng.joaovictor.resort.models.StatusLine
import br.eng.joaovictor.resort.models.Wallet
import br.eng.joaovictor.resort.views.shop.canAfford
import br.eng.joaovictor.resort.views.shop.defaultCategory
import br.eng.joaovictor.resort.views.shop.filterOffers
import br.eng.joaovictor.resort.views.shop.resolveCategory
import br.eng.joaovictor.resort.views.shop.toUi
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ShopUiTest {
    private val categories = listOf(
        ShopCategory("furniture", "Móveis"),
        ShopCategory("seats", "Assentos", parent = "furniture"),
        ShopCategory("floors", "Pisos"),
    )
    private val offers = listOf(
        ShopOffer(catalogId = "chair", categoryId = "seats", name = "Poltrona", priceSilver = 100),
        ShopOffer(catalogId = "table", categoryId = "furniture", name = "Mesa", priceSilver = 50, priceGold = 1),
        ShopOffer(catalogId = "rug", categoryId = "floors", name = "Tapete"),
    )

    @Test
    fun theWalletMustCoverSilverAndGold() {
        val chair = offers[0]
        val table = offers[1]
        assertTrue(canAfford(Wallet(silver = 100, gold = 0, loaded = true), chair))
        assertFalse(canAfford(Wallet(silver = 99, gold = 0, loaded = true), chair))
        assertFalse(canAfford(Wallet(silver = 50, gold = 0, loaded = true), table)) // gold too
        assertTrue(canAfford(Wallet(silver = 50, gold = 1, loaded = true), table))
    }

    @Test
    fun theDefaultCategoryIsTheFirstTopLevelOne() {
        assertEquals("furniture", defaultCategory(categories))
        assertNull(defaultCategory(emptyList()))
        assertEquals("floors", resolveCategory(categories, "floors"))
        assertEquals("furniture", resolveCategory(categories, "gone")) // vanished -> default
    }

    @Test
    fun aTopLevelCategoryIncludesItsSubCategories() {
        assertEquals(listOf("chair", "table"), filterOffers(categories, offers, "furniture", "").map { it.catalogId })
        assertEquals(listOf("chair"), filterOffers(categories, offers, "seats", "").map { it.catalogId })
    }

    @Test
    fun aSearchLooksAtEveryCategoryIgnoringCaseAndAccents() {
        assertEquals(listOf("rug"), filterOffers(categories, offers, "furniture", "TAPETE").map { it.catalogId })
        assertEquals(listOf("chair"), filterOffers(categories, offers, "floors", "poltrona").map { it.catalogId })
    }

    @Test
    fun theUiMarksTheSelectedOfferAndTheBuyRules() {
        val state = ShopState(open = true, categories = categories, offers = offers, selectedCategory = "furniture")
        val rich = state.toUi(Wallet(silver = 1000, gold = 5, loaded = true), selectedOfferId = "chair")
        assertEquals(listOf("furniture", "floors"), rich.topCategories.map { it.id })
        assertEquals(listOf("seats"), rich.subCategories.map { it.id }) // children of the open section
        assertEquals("chair", rich.selected?.catalogId)
        assertTrue(rich.canBuy)

        val poor = state.toUi(Wallet(silver = 10, loaded = true), selectedOfferId = "chair")
        assertFalse(poor.canBuy)

        val buying = state.copy(pending = "chair").toUi(Wallet(silver = 1000, loaded = true), selectedOfferId = "chair")
        assertTrue(buying.buying)
        assertFalse(buying.canBuy) // waiting for the server blocks a double buy

        assertFalse(state.toUi(Wallet(silver = 1000, loaded = true), selectedOfferId = null).canBuy) // nothing selected
    }

    @Test
    fun theStatusLineIsPassedThrough() {
        val state = ShopState(open = true, status = StatusLine("error", "Saldo insuficiente."))
        assertEquals("Saldo insuficiente.", state.toUi(Wallet(), null).status?.text)
    }
}
