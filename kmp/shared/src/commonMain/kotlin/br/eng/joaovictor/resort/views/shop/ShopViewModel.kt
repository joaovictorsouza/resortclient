package br.eng.joaovictor.resort.views.shop

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import br.eng.joaovictor.resort.models.ShopCategory
import br.eng.joaovictor.resort.models.ShopOffer
import br.eng.joaovictor.resort.models.ShopState
import br.eng.joaovictor.resort.models.StatusLine
import br.eng.joaovictor.resort.models.Wallet
import br.eng.joaovictor.resort.repositories.ShopRepository
import br.eng.joaovictor.resort.repositories.WalletRepository
import br.eng.joaovictor.resort.views.inventory.normalizeSearch
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn

/** One offer tile. */
data class ShopTile(
    val catalogId: String,
    val name: String,
    val thumbnail: String,
    val priceSilver: Long,
    val priceGold: Long,
    val affordable: Boolean,
    val selected: Boolean,
)

data class ShopUiState(
    val open: Boolean = false,
    val loading: Boolean = false,
    /** Top-level categories, then the sub-categories of the one holding the selection. */
    val topCategories: List<ShopCategory> = emptyList(),
    val subCategories: List<ShopCategory> = emptyList(),
    val selectedCategory: String? = null,
    val query: String = "",
    val tiles: List<ShopTile> = emptyList(),
    val selected: ShopTile? = null,
    /** An offer is being bought (waiting for the server): the buy button is disabled. */
    val buying: Boolean = false,
    val status: StatusLine? = null,
    val silver: Long = 0,
    val gold: Long = 0,
    val walletLoaded: Boolean = false,
) {
    val canBuy: Boolean get() = selected?.affordable == true && !buying
    val isEmpty: Boolean get() = !loading && tiles.isEmpty()
}

/** The wallet covers one unit of the offer (silver and gold are paid together), like the game's `can_afford`. */
fun canAfford(wallet: Wallet, offer: ShopOffer): Boolean =
    wallet.silver >= offer.priceSilver && wallet.gold >= offer.priceGold

/** The category shown when the shop opens: the first top-level one. */
fun defaultCategory(categories: List<ShopCategory>): String? = categories.firstOrNull { it.parent.isEmpty() }?.id

/** Keeps [selected] if it still exists in the catalog, otherwise the default. */
fun resolveCategory(categories: List<ShopCategory>, selected: String?): String? =
    selected?.takeIf { id -> categories.any { it.id == id } } ?: defaultCategory(categories)

/**
 * Offers to show. A non-blank search looks at every category (name, case and accent insensitive);
 * otherwise the selected category, including all of its sub-categories.
 */
fun filterOffers(
    categories: List<ShopCategory>,
    offers: List<ShopOffer>,
    selected: String?,
    query: String,
): List<ShopOffer> {
    val needle = normalizeSearch(query.trim())
    if (needle.isNotEmpty()) return offers.filter { normalizeSearch(it.name).contains(needle) }
    if (selected == null) return offers
    return offers.filter { o ->
        o.categoryId == selected || categories.any { it.id == o.categoryId && it.parent == selected }
    }
}

/** The top-level section that holds [selected] (itself or one of its children), if any. */
private fun sectionOf(categories: List<ShopCategory>, selected: String?): String? =
    categories.firstOrNull { it.id == selected }?.let { it.parent.ifEmpty { it.id } }

fun ShopState.toUi(wallet: Wallet, selectedOfferId: String?): ShopUiState {
    val category = resolveCategory(categories, selectedCategory)
    val section = sectionOf(categories, category)
    val shown = filterOffers(categories, offers, category, query)
    val tiles = shown.map {
        ShopTile(
            catalogId = it.catalogId,
            name = it.name,
            thumbnail = it.thumbnail,
            priceSilver = it.priceSilver,
            priceGold = it.priceGold,
            affordable = canAfford(wallet, it),
            selected = it.catalogId == selectedOfferId,
        )
    }
    return ShopUiState(
        open = open,
        loading = loading,
        topCategories = categories.filter { it.parent.isEmpty() },
        subCategories = categories.filter { it.parent.isNotEmpty() && it.parent == section },
        selectedCategory = category,
        query = query,
        tiles = tiles,
        selected = tiles.firstOrNull { it.selected },
        buying = pending != null,
        status = status,
        silver = wallet.silver,
        gold = wallet.gold,
        walletLoaded = wallet.loaded,
    )
}

class ShopViewModel(private val shop: ShopRepository, wallet: WalletRepository) : ViewModel() {
    private val selectedOffer = MutableStateFlow<String?>(null)

    val ui: StateFlow<ShopUiState> = combine(shop.state, wallet.state, selectedOffer) { state, w, offer ->
        state.toUi(w, offer)
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), ShopUiState())

    fun close() = shop.setOpen(false)
    fun selectCategory(id: String) {
        selectedOffer.value = null
        shop.selectCategory(id)
    }
    fun setQuery(query: String) = shop.search(query)
    fun selectOffer(catalogId: String) { selectedOffer.value = catalogId }
    fun buy() = selectedOffer.value?.let(shop::purchase)
}
