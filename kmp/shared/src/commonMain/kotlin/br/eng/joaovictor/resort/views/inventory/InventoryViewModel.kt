package br.eng.joaovictor.resort.views.inventory

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import br.eng.joaovictor.resort.models.InventoryState
import br.eng.joaovictor.resort.repositories.InventoryRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn

const val ALL_CATEGORIES = "all"

data class InventoryRow(
    val catalogId: String,
    val name: String,
    val category: String,
    /** `null` when the game sent no quantity. */
    val quantity: Int?,
    val selected: Boolean,
    /** Local file of the cached thumbnail; empty while it is not downloaded. */
    val thumbnail: String,
) {
    /** Pets are placed by their own flow, not by dropping them on the floor. */
    val draggable: Boolean get() = category != "pet"
}

data class InventoryUiState(
    val open: Boolean = false,
    val loading: Boolean = false,
    /** All rows, before the search and category filter. */
    val rows: List<InventoryRow> = emptyList(),
    val categories: List<String> = listOf(ALL_CATEGORIES),
    val category: String = ALL_CATEGORIES,
    val query: String = "",
    /** Only the owner of the room can place items (tap or drag). */
    val canPlace: Boolean = false,
) {
    val visible: List<InventoryRow> get() = filterRows(rows, category, query)
    val isEmpty: Boolean get() = !loading && visible.isEmpty()
}

fun categoryLabel(category: String): String = when (category) {
    ALL_CATEGORIES -> "Todos"
    "collider" -> "Móveis"
    "floor_decoration" -> "Pisos"
    "pet" -> "Animais"
    else -> category
}

/** Lowercases and strips common Portuguese diacritics, like the game's own search. */
fun normalizeSearch(text: String): String = buildString {
    for (c in text.lowercase()) {
        append(
            when (c) {
                'á', 'à', 'â', 'ã', 'ä' -> 'a'
                'é', 'è', 'ê', 'ë' -> 'e'
                'í', 'ì', 'î', 'ï' -> 'i'
                'ó', 'ò', 'ô', 'õ', 'ö' -> 'o'
                'ú', 'ù', 'û', 'ü' -> 'u'
                'ç' -> 'c'
                else -> c
            },
        )
    }
}

/** Rows of [category] (`all` keeps everything) whose name contains [query] (blank keeps everything). */
fun filterRows(rows: List<InventoryRow>, category: String, query: String): List<InventoryRow> {
    val needle = normalizeSearch(query.trim())
    return rows.filter {
        (category == ALL_CATEGORIES || it.category == category) &&
            (needle.isEmpty() || normalizeSearch(it.name).contains(needle))
    }
}

/** Category keys present in the items (first-seen order), preceded by `all`. */
fun categoriesOf(rows: List<InventoryRow>): List<String> =
    listOf(ALL_CATEGORIES) + rows.map { it.category }.distinct()

fun InventoryState.toUi(category: String = ALL_CATEGORIES, query: String = ""): InventoryUiState {
    val rows = items.map {
        InventoryRow(
            catalogId = it.catalogId,
            name = it.name.ifBlank { it.catalogId },
            category = it.category,
            quantity = it.quantity.takeIf { q -> q > 0 },
            selected = it.catalogId == selectedCatalogId,
            thumbnail = it.thumbnail,
        )
    }
    val categories = categoriesOf(rows)
    return InventoryUiState(
        open = open,
        loading = loading,
        rows = rows,
        categories = categories,
        // A category that is no longer present falls back to everything.
        category = category.takeIf { it in categories } ?: ALL_CATEGORIES,
        query = query,
        canPlace = canPlace,
    )
}

class InventoryViewModel(private val inventory: InventoryRepository) : ViewModel() {
    private val category = MutableStateFlow(ALL_CATEGORIES)
    private val query = MutableStateFlow("")

    val ui: StateFlow<InventoryUiState> =
        combine(inventory.state, category, query) { state, cat, q -> state.toUi(cat, q) }
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), InventoryUiState())

    fun close() = inventory.setOpen(false)
    fun select(catalogId: String) = inventory.select(catalogId)
    fun setCategory(value: String) { category.value = value }
    fun setQuery(value: String) { query.value = value }

    /** The finger of a drag that left the window, at ([x], [y]) physical pixels of the game surface. */
    fun drag(catalogId: String, x: Float, y: Float, phase: String) = inventory.drag(catalogId, x, y, phase)
}

/**
 * What to tell the game about a drag step, or `null` for nothing. Inside the window nothing is sent;
 * outside, every move is; coming back (or cancelling) after having been outside cancels the placement.
 */
fun dragPhase(wasOutside: Boolean, nowOutside: Boolean, released: Boolean): String? = when {
    nowOutside && released -> "end"
    nowOutside -> "move"
    wasOutside -> "cancel"
    else -> null
}

/** Search field state of the window: whether it is shown and what it filters by. */
data class SearchToggle(val open: Boolean, val query: String)

/**
 * Tapping the search icon (or the title bar) toggles the field. Closing clears the query, so the list is
 * never filtered by a field the player cannot see.
 */
fun toggleSearch(open: Boolean, query: String): SearchToggle =
    if (open) SearchToggle(open = false, query = "") else SearchToggle(open = true, query = query)
