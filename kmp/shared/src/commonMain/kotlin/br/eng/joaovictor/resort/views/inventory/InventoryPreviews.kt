package br.eng.joaovictor.resort.views.inventory

import androidx.compose.runtime.Composable
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene
import org.jetbrains.compose.ui.tooling.preview.Preview

private val Loaded = PreviewData.inventoryLoaded

// Thumbnails are absent in previews (the default loader returns null), so the placeholder shows.
@Preview(name = "Items, one selected (landscape)", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryItemsPreview() = PreviewScene { InventoryContent(Loaded, InventoryActions()) }

@Preview(name = "Items (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun InventoryPortraitPreview() = PreviewScene { InventoryContent(Loaded, InventoryActions()) }

@Preview(name = "Loading", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryLoadingPreview() =
    PreviewScene { InventoryContent(Loaded.copy(loading = true, rows = emptyList()), InventoryActions()) }

@Preview(name = "Empty inventory", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryEmptyPreview() =
    PreviewScene { InventoryContent(Loaded.copy(rows = emptyList()), InventoryActions()) }

@Preview(name = "Search with no result", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryNoResultPreview() =
    PreviewScene { InventoryContent(Loaded.copy(query = "zzz"), InventoryActions()) }

@Preview(name = "Not the owner (warning, no drag)", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryNotOwnerPreview() =
    PreviewScene { InventoryContent(Loaded.copy(canPlace = false), InventoryActions()) }

@Preview(name = "Category filter", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryCategoryPreview() =
    PreviewScene { InventoryContent(Loaded.copy(category = "collider"), InventoryActions()) }

@Preview(name = "Long names only", widthDp = 360, heightDp = 780)
@Composable
private fun InventoryLongNamesPreview() = PreviewScene {
    InventoryContent(Loaded.copy(rows = Loaded.rows.filter { it.name.length > 20 }), InventoryActions())
}
