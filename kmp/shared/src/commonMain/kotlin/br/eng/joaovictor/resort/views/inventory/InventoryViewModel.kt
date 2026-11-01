package br.eng.joaovictor.resort.views.inventory

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import br.eng.joaovictor.resort.models.InventoryState
import br.eng.joaovictor.resort.repositories.InventoryRepository
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn

data class InventoryRow(
    val catalogId: String,
    val name: String,
    /** `null` when the game sent no quantity. */
    val quantity: Int?,
    val selected: Boolean,
)

data class InventoryUiState(
    val open: Boolean = false,
    val loading: Boolean = false,
    val rows: List<InventoryRow> = emptyList(),
) {
    val isEmpty: Boolean get() = !loading && rows.isEmpty()
}

fun InventoryState.toUi(): InventoryUiState = InventoryUiState(
    open = open,
    loading = loading,
    rows = items.map {
        InventoryRow(
            catalogId = it.catalogId,
            name = it.name.ifBlank { it.catalogId },
            quantity = it.quantity.takeIf { q -> q > 0 },
            selected = it.catalogId == selectedCatalogId,
        )
    },
)

class InventoryViewModel(private val inventory: InventoryRepository) : ViewModel() {
    val ui: StateFlow<InventoryUiState> = inventory.state.map { it.toUi() }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), InventoryUiState())

    fun close() = inventory.setOpen(false)
    fun select(catalogId: String) = inventory.select(catalogId)
}
