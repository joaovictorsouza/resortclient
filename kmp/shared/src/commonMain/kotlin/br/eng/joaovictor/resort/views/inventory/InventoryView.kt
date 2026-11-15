package br.eng.joaovictor.resort.views.inventory

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.components.GameThumbnail
import br.eng.joaovictor.resort.ds.components.CategoryChips
import br.eng.joaovictor.resort.ds.components.CategoryRail
import br.eng.joaovictor.resort.ds.components.ChipItem
import br.eng.joaovictor.resort.ds.components.EmptyState
import br.eng.joaovictor.resort.ds.components.GameIconButton
import br.eng.joaovictor.resort.ds.components.IconButtonStyle
import br.eng.joaovictor.resort.ds.components.ItemTile
import br.eng.joaovictor.resort.ds.components.LockReason
import br.eng.joaovictor.resort.ds.components.RailItem
import br.eng.joaovictor.resort.ds.components.SearchField
import br.eng.joaovictor.resort.ds.components.SkeletonGrid
import br.eng.joaovictor.resort.ds.components.TileState
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.layout.AdaptivePanel
import br.eng.joaovictor.resort.ds.layout.PanelForm
import br.eng.joaovictor.resort.ds.layout.PanelSpec
import br.eng.joaovictor.resort.ds.layout.panelSpecFor
import br.eng.joaovictor.resort.ds.layout.rememberPanelSpec
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize
import br.eng.joaovictor.resort.views.hud.CompactHudWhileVisible
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene
import kotlin.math.roundToInt

/** The HUD owner name the panel holds while open (see `HudVisibility`); the HUD shrinks to Compact. */
internal const val INVENTORY_HUD_OWNER = "inventory"

/** An item being dragged out of the panel; [position] is the finger in root (surface) pixels. */
private data class Drag(val row: InventoryRow, val position: Offset)

/** What the inventory panel can ask for; previews pass no-ops. */
class InventoryActions(
    val onClose: () -> Unit = {},
    val onSelect: (String) -> Unit = {},
    val onQuery: (String) -> Unit = {},
    val onCategory: (String) -> Unit = {},
    /** A drag that left the panel: (catalogId, x, y, phase) in physical pixels of the game surface. */
    val onDrag: (String, Float, Float, String) -> Unit = { _, _, _, _ -> },
)

/** Visual state of an inventory tile: selected, quantity, and a lock when the player cannot place items here. */
fun inventoryTileState(row: InventoryRow, canPlace: Boolean): TileState =
    TileState(selected = row.selected, locked = if (canPlace) null else LockReason.NotOwner, quantity = row.quantity)

/** Category entries for the rail (landscape) and the chips (portrait). */
fun inventoryRail(ui: InventoryUiState): List<RailItem> = ui.categories.map { RailItem(it, categoryLabel(it)) }

/** Collects the state and shrinks the HUD to Compact while the panel is open. */
@Composable
fun InventoryView(viewModel: InventoryViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.open) return
    CompactHudWhileVisible(INVENTORY_HUD_OWNER)
    val actions = remember(viewModel) {
        InventoryActions(
            onClose = viewModel::close,
            onSelect = viewModel::select,
            onQuery = viewModel::setQuery,
            onCategory = viewModel::setCategory,
            onDrag = viewModel::drag,
        )
    }
    var searchOpen by rememberSaveable { mutableStateOf(ui.query.isNotEmpty()) }
    InventoryContent(
        ui,
        actions,
        searchOpen = searchOpen,
        onToggleSearch = {
            val next = toggleSearch(searchOpen, ui.query)
            searchOpen = next.open
            if (next.query != ui.query) actions.onQuery(next.query)
        },
    )
}

/**
 * Inventory in the adaptive panel: a side panel with a category rail in landscape, a bottom sheet with chips
 * in portrait. The scene stays visible and tappable. Long-press an item and drag it out of the panel to drop
 * it on the floor (the panel tucks away while the finger is outside); tap it to place it.
 * Stateless: it takes plain UI state, so it can be previewed without a ViewModel or the game.
 */
@Composable
fun InventoryContent(
    ui: InventoryUiState,
    actions: InventoryActions,
    /** The search field is hidden until the player taps the search icon. */
    searchOpen: Boolean = ui.query.isNotEmpty(),
    onToggleSearch: () -> Unit = {},
    spec: PanelSpec = rememberPanelSpec(),
) {
    var panelBounds by remember { mutableStateOf(Rect.Zero) }
    var drag by remember { mutableStateOf<Drag?>(null) }
    var outside by remember { mutableStateOf(false) }
    val keyboard = LocalSoftwareKeyboardController.current
    LaunchedEffect(searchOpen) { if (!searchOpen) keyboard?.hide() }

    Box(Modifier.fillMaxSize()) {
        AdaptivePanel(
            visible = true,
            title = "Inventário",
            onClose = actions.onClose,
            icon = GameIcons.Backpack,
            headerActions = {
                GameIconButton(GameIcons.Search, "Buscar", onToggleSearch, style = IconButtonStyle.Raised, active = searchOpen)
            },
            tucked = outside,
            onBoundsChange = { panelBounds = it },
            spec = spec,
            panelKey = "inventory",
        ) {
            if (searchOpen) {
                SearchField(
                    ui.query, actions.onQuery, "Buscar",
                    Modifier.fillMaxWidth().padding(horizontal = Space.md, vertical = Space.xs),
                    autoFocus = true,
                )
            }
            val body: @Composable (Modifier) -> Unit = { m ->
                Body(
                    ui, actions, m,
                    onDragStart = { drag = it; outside = false },
                    onDragMove = { pos ->
                        val d = drag ?: return@Body
                        val now = leftWindow(outside, pos, panelBounds)
                        dragPhase(outside, now, released = false)?.let { actions.onDrag(d.row.catalogId, pos.x, pos.y, it) }
                        outside = now
                        drag = d.copy(position = pos)
                    },
                    onDragEnd = { end ->
                        val d = drag
                        if (d != null) {
                            val now = leftWindow(outside, end, panelBounds)
                            dragPhase(outside, now, released = true)?.let { actions.onDrag(d.row.catalogId, end.x, end.y, it) }
                        }
                        drag = null
                        // A committed drag leaves the panel tucked until the game closes it (no flash back).
                    },
                    onDragCancel = {
                        val d = drag
                        if (d != null) dragPhase(outside, false, released = false)?.let { actions.onDrag(d.row.catalogId, 0f, 0f, it) }
                        drag = null
                        outside = false
                    },
                )
            }
            if (spec.form == PanelForm.Side) {
                Row(Modifier.weight(1f)) {
                    CategoryRail(inventoryRail(ui), ui.category, actions.onCategory)
                    body(Modifier.weight(1f))
                }
            } else {
                CategoryChips(
                    ui.categories.map { ChipItem(it, categoryLabel(it)) }, ui.category, actions.onCategory,
                    Modifier.padding(horizontal = Space.md),
                )
                body(Modifier.weight(1f))
            }
        }
        drag?.takeIf { !outside }?.let { d ->
            Box(
                Modifier.offset { IntOffset((d.position.x - 40).roundToInt(), (d.position.y - 40).roundToInt()) }
                    .size(80.dp)
                    .background(ResortTheme.colors.scrim, RoundedCornerShape(12.dp)),
                contentAlignment = Alignment.Center,
            ) {
                GameThumbnail(d.row.thumbnail, d.row.name, Modifier.size(72.dp))
            }
        }
    }
}

/** A point outside the panel's bounds. */
internal fun dropOutside(point: Offset, window: Rect): Boolean = !window.contains(point)

/**
 * Once a drag has left the panel it stays "outside" until it ends: the panel is tucked then, so its
 * old area must be usable to position the item in the room.
 */
internal fun leftWindow(wasOutside: Boolean, point: Offset, window: Rect): Boolean =
    wasOutside || dropOutside(point, window)

@Composable
private fun Body(
    ui: InventoryUiState,
    actions: InventoryActions,
    modifier: Modifier,
    onDragStart: (Drag) -> Unit,
    onDragMove: (Offset) -> Unit,
    onDragEnd: (Offset) -> Unit,
    onDragCancel: () -> Unit,
) {
    Column(modifier) {
        when {
            ui.loading -> SkeletonGrid(Modifier.weight(1f))
            ui.isEmpty -> Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                if (ui.rows.isEmpty()) {
                    EmptyState(GameIcons.Backpack, "Seu inventário está vazio", message = "Compre móveis na loja para decorar sua sala.")
                } else {
                    EmptyState(
                        GameIcons.Search, "Nenhum item encontrado",
                        action = if (ui.query.isNotEmpty()) "Limpar busca" to { actions.onQuery("") } else null,
                    )
                }
            }
            else -> {
                if (!ui.canPlace) {
                    Text(
                        "Só o dono da sala pode colocar itens aqui.",
                        style = ResortTheme.type.caption, color = ResortTheme.colors.accent,
                        modifier = Modifier.padding(horizontal = Space.lg, vertical = Space.xs),
                    )
                }
                LazyVerticalGrid(
                    columns = GridCells.Adaptive(minSize = DsSize.tileMin),
                    modifier = Modifier.weight(1f).padding(horizontal = Space.md),
                    horizontalArrangement = Arrangement.spacedBy(Space.sm),
                    verticalArrangement = Arrangement.spacedBy(Space.sm),
                ) {
                    items(ui.visible, key = { it.catalogId }, contentType = { "item" }) { row ->
                        ItemTile(
                            row.name,
                            inventoryTileState(row, ui.canPlace),
                            { actions.onSelect(row.catalogId) },
                            dragModifier = rememberDragModifier(row, ui.canPlace && row.draggable, onDragStart, onDragMove, onDragEnd, onDragCancel),
                            thumbnail = { m -> GameThumbnail(row.thumbnail, row.name, m) },
                        )
                    }
                }
            }
        }
    }
}

/** The long-press drag of a tile; reports the finger in root coordinates. */
@Composable
private fun rememberDragModifier(
    row: InventoryRow,
    enabled: Boolean,
    onDragStart: (Drag) -> Unit,
    onDragMove: (Offset) -> Unit,
    onDragEnd: (Offset) -> Unit,
    onDragCancel: () -> Unit,
): Modifier {
    var topLeft by remember { mutableStateOf(Offset.Zero) }
    var finger by remember { mutableStateOf(Offset.Zero) }
    return Modifier.onGloballyPositioned { topLeft = it.positionInRoot() }.then(
        if (enabled) {
            Modifier.pointerInput(row.catalogId) {
                detectDragGesturesAfterLongPress(
                    onDragStart = { local ->
                        finger = topLeft + local
                        onDragStart(Drag(row, finger))
                    },
                    onDrag = { change, amount ->
                        change.consume()
                        finger += amount
                        onDragMove(finger)
                    },
                    onDragEnd = { onDragEnd(finger) },
                    onDragCancel = onDragCancel,
                )
            }
        } else {
            Modifier
        },
    )
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

private val Loaded = PreviewData.inventoryLoaded
private val LandscapeSpec = panelSpecFor(800.dp, 360.dp)
private val PortraitSpec = panelSpecFor(360.dp, 780.dp)

// Thumbnails are absent in previews (the default loader returns null), so the placeholder shows.
@Preview(name = "Items, one selected (landscape)", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryItemsPreview() = PreviewScene { InventoryContent(Loaded, InventoryActions(), spec = LandscapeSpec) }

@Preview(name = "Grid with many items", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryManyPreview() =
    PreviewScene { InventoryContent(PreviewData.inventoryMany, InventoryActions(), spec = LandscapeSpec) }

@Preview(name = "Items (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun InventoryPortraitPreview() = PreviewScene { InventoryContent(Loaded, InventoryActions(), spec = PortraitSpec) }

@Preview(name = "Loading", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryLoadingPreview() =
    PreviewScene { InventoryContent(Loaded.copy(loading = true, rows = emptyList()), InventoryActions(), spec = LandscapeSpec) }

@Preview(name = "Empty inventory", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryEmptyPreview() =
    PreviewScene { InventoryContent(Loaded.copy(rows = emptyList()), InventoryActions(), spec = LandscapeSpec) }

@Preview(name = "Search closed (icon only)", widthDp = 800, heightDp = 360)
@Composable
private fun InventorySearchClosedPreview() =
    PreviewScene { InventoryContent(Loaded, InventoryActions(), spec = LandscapeSpec, searchOpen = false) }

@Preview(name = "Search open, empty", widthDp = 800, heightDp = 360)
@Composable
private fun InventorySearchOpenEmptyPreview() =
    PreviewScene { InventoryContent(Loaded, InventoryActions(), spec = LandscapeSpec, searchOpen = true) }

@Preview(name = "Search open, with text", widthDp = 800, heightDp = 360)
@Composable
private fun InventorySearchOpenPreview() =
    PreviewScene { InventoryContent(Loaded.copy(query = "pol"), InventoryActions(), spec = LandscapeSpec, searchOpen = true) }

@Preview(name = "Search with no result", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryNoResultPreview() =
    PreviewScene { InventoryContent(Loaded.copy(query = "zzz"), InventoryActions(), spec = LandscapeSpec, searchOpen = true) }

@Preview(name = "Not the owner (warning, no drag)", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryNotOwnerPreview() =
    PreviewScene { InventoryContent(Loaded.copy(canPlace = false), InventoryActions(), spec = LandscapeSpec) }

@Preview(name = "Category filter", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryCategoryPreview() =
    PreviewScene { InventoryContent(Loaded.copy(category = "collider"), InventoryActions(), spec = LandscapeSpec) }

@Preview(name = "Long names only", widthDp = 360, heightDp = 780)
@Composable
private fun InventoryLongNamesPreview() = PreviewScene {
    InventoryContent(Loaded.copy(rows = Loaded.rows.filter { it.name.length > 20 }), InventoryActions(), spec = PortraitSpec)
}
