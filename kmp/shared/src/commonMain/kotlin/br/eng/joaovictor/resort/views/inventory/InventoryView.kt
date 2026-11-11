package br.eng.joaovictor.resort.views.inventory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import br.eng.joaovictor.resort.components.GameThumbnail
import br.eng.joaovictor.resort.components.GameWindow
import br.eng.joaovictor.resort.components.SearchSection
import br.eng.joaovictor.resort.views.hud.HideHudWhileVisible
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlin.math.roundToInt
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene
import androidx.compose.ui.tooling.preview.Preview

/** The HUD owner name the window holds while open (see `HudVisibility`). */
internal const val INVENTORY_HUD_OWNER = "inventory"

/** Share of the available (safe area) height the window takes. */
internal const val WINDOW_HEIGHT_FRACTION = 0.9f

private val WindowShape = RoundedCornerShape(16.dp)
private val Selected = Color(0xFF4F46E5)

/** An item being dragged out of the window; [position] is the finger in root (surface) pixels. */
private data class Drag(val row: InventoryRow, val position: Offset)

/** What the inventory window can ask for; previews pass no-ops. */
class InventoryActions(
    val onClose: () -> Unit = {},
    val onSelect: (String) -> Unit = {},
    val onQuery: (String) -> Unit = {},
    val onCategory: (String) -> Unit = {},
    /** A drag that left the window: (catalogId, x, y, phase) in physical pixels of the game surface. */
    val onDrag: (String, Float, Float, String) -> Unit = { _, _, _, _ -> },
)

/** Collects the state and holds the HUD hidden while the window is open. */
@Composable
fun InventoryView(viewModel: InventoryViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.open) return
    // The window covers most of the scene: the HUD stays hidden while it is open (also during a drag),
    // and comes back when the window leaves the composition (closed by the player or by the game).
    HideHudWhileVisible(INVENTORY_HUD_OWNER)
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
 * Floating inventory window: a movable card over the scene (no scrim, touches outside it reach the game).
 * Long-press an item and drag it out of the window to drop it on the floor; tap it to place it.
 * Stateless: it takes plain UI state, so it can be previewed without a ViewModel or the game.
 */
@Composable
fun InventoryContent(
    ui: InventoryUiState,
    actions: InventoryActions,
    /** The search field is hidden until the player taps the search icon or the title bar. */
    searchOpen: Boolean = ui.query.isNotEmpty(),
    onToggleSearch: () -> Unit = {},
) {
    var windowBounds by remember { mutableStateOf(Rect.Zero) }
    var drag by remember { mutableStateOf<Drag?>(null) }
    var outside by remember { mutableStateOf(false) }

    GameWindow(
        title = "Inventário",
        onClose = actions.onClose,
        searchOpen = searchOpen,
        onToggleSearch = onToggleSearch,
        windowAlpha = if (outside) 0f else if (drag != null) 0.35f else 1f,
        onBoundsChange = { windowBounds = it },
        overlay = {
            drag?.takeIf { !outside }?.let { d ->
                Box(
                    Modifier.offset { IntOffset((d.position.x - 40).roundToInt(), (d.position.y - 40).roundToInt()) }
                        .size(80.dp)
                        .background(Color.Black.copy(alpha = 0.4f), RoundedCornerShape(12.dp)),
                    contentAlignment = Alignment.Center,
                ) {
                    Thumbnail(d.row, Modifier.size(72.dp))
                }
            }
        },
    ) {
        Filters(ui, actions, searchOpen)
        Content(
            ui,
            actions,
            onDragStart = { drag = it; outside = false },
            onDragMove = { pos ->
                val d = drag ?: return@Content
                val now = leftWindow(outside, pos, windowBounds)
                dragPhase(outside, now, released = false)?.let { actions.onDrag(d.row.catalogId, pos.x, pos.y, it) }
                outside = now
                drag = d.copy(position = pos)
            },
            onDragEnd = { end ->
                val d = drag
                if (d != null) {
                    val now = leftWindow(outside, end, windowBounds)
                    dragPhase(outside, now, released = true)?.let { actions.onDrag(d.row.catalogId, end.x, end.y, it) }
                }
                drag = null
                // A committed drag leaves the window hidden until the game closes it (no flash back).
            },
            onDragCancel = {
                val d = drag
                if (d != null) dragPhase(outside, false, released = false)?.let { actions.onDrag(d.row.catalogId, 0f, 0f, it) }
                drag = null
                outside = false
            },
        )
    }
}

/** A point outside the window's bounds. */
internal fun dropOutside(point: Offset, window: Rect): Boolean = !window.contains(point)

/**
 * Once a drag has left the window it stays "outside" until it ends: the window is hidden then, so its
 * old area (often the middle of the screen) must be usable to position the item in the room.
 */
internal fun leftWindow(wasOutside: Boolean, point: Offset, window: Rect): Boolean =
    wasOutside || dropOutside(point, window)

@Composable
private fun Filters(ui: InventoryUiState, actions: InventoryActions, searchOpen: Boolean) {
    Column(Modifier.padding(horizontal = 12.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        SearchSection(open = searchOpen, query = ui.query, onQuery = actions.onQuery)
        LazyRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            items(ui.categories) { key ->
                val on = key == ui.category
                Text(
                    categoryLabel(key),
                    color = Color.White,
                    fontSize = 13.sp,
                    modifier = Modifier
                        .background(if (on) Selected else Color.White.copy(alpha = 0.12f), RoundedCornerShape(14.dp))
                        .clickable { actions.onCategory(key) }
                        .padding(horizontal = 10.dp, vertical = 5.dp),
                )
            }
        }
    }
}

@Composable
private fun Content(
    ui: InventoryUiState,
    actions: InventoryActions,
    onDragStart: (Drag) -> Unit,
    onDragMove: (Offset) -> Unit,
    onDragEnd: (Offset) -> Unit,
    onDragCancel: () -> Unit,
) {
    when {
        ui.loading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
        ui.isEmpty -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Text(if (ui.rows.isEmpty()) "Seu inventário está vazio" else "Nenhum item encontrado")
        }
        else -> Column {
            if (!ui.canPlace) {
                Text(
                    "Só o dono da sala pode colocar itens aqui.",
                    color = Color(0xFFFFC107),
                    fontSize = 12.sp,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 2.dp),
                )
            }
            // Square tiles, as many per row as fit.
            LazyVerticalGrid(
                columns = GridCells.Adaptive(minSize = 72.dp),
                modifier = Modifier.padding(horizontal = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(ui.visible, key = { it.catalogId }) { row ->
                    ItemTile(
                        row,
                        ui.canPlace && row.draggable,
                        { actions.onSelect(row.catalogId) },
                        onDragStart, onDragMove, onDragEnd, onDragCancel,
                    )
                }
            }
        }
    }
}

/** A square inventory tile: thumbnail, quantity badge and name caption; highlighted when selected. */
@Composable
private fun ItemTile(
    row: InventoryRow,
    draggable: Boolean,
    onTap: () -> Unit,
    onDragStart: (Drag) -> Unit,
    onDragMove: (Offset) -> Unit,
    onDragEnd: (Offset) -> Unit,
    onDragCancel: () -> Unit,
) {
    var topLeft by remember { mutableStateOf(Offset.Zero) }
    var finger by remember { mutableStateOf(Offset.Zero) }
    val shape = RoundedCornerShape(10.dp)
    Box(
        Modifier.fillMaxWidth()
            .aspectRatio(1f)
            .clip(shape)
            .background(if (row.selected) Selected.copy(alpha = 0.35f) else Color.White.copy(alpha = 0.06f))
            .border(if (row.selected) 2.dp else 1.dp, if (row.selected) Selected else Color.White.copy(alpha = 0.12f), shape)
            .onGloballyPositioned { topLeft = it.positionInRoot() }
            .clickable(onClick = onTap)
            .then(
                if (draggable) {
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
            ),
    ) {
        Thumbnail(row, Modifier.fillMaxSize().padding(bottom = 18.dp))
        Text(
            row.name,
            color = Color.White,
            fontSize = 11.sp,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.align(Alignment.BottomCenter).fillMaxWidth()
                .background(Color.Black.copy(alpha = 0.45f))
                .padding(horizontal = 4.dp, vertical = 2.dp),
        )
        row.quantity?.takeIf { it > 1 }?.let { // a quantity of 1 needs no badge
            Text(
                "x$it",
                color = Color.White,
                fontSize = 11.sp,
                modifier = Modifier.align(Alignment.TopEnd).padding(4.dp)
                    .background(Color.Black.copy(alpha = 0.6f), RoundedCornerShape(8.dp))
                    .padding(horizontal = 6.dp, vertical = 1.dp),
            )
        }
    }
}

@Composable
private fun Thumbnail(row: InventoryRow, modifier: Modifier) = GameThumbnail(row.thumbnail, row.name, modifier)

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

private val Loaded = PreviewData.inventoryLoaded

// Thumbnails are absent in previews (the default loader returns null), so the placeholder shows.
@Preview(name = "Items, one selected (landscape)", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryItemsPreview() = PreviewScene { InventoryContent(Loaded, InventoryActions()) }

@Preview(name = "Grid with many items", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryManyPreview() =
    PreviewScene { InventoryContent(PreviewData.inventoryMany, InventoryActions()) }

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

@Preview(name = "Search closed (icon only)", widthDp = 800, heightDp = 360)
@Composable
private fun InventorySearchClosedPreview() =
    PreviewScene { InventoryContent(Loaded, InventoryActions(), searchOpen = false) }

@Preview(name = "Search open, empty", widthDp = 800, heightDp = 360)
@Composable
private fun InventorySearchOpenEmptyPreview() =
    PreviewScene { InventoryContent(Loaded, InventoryActions(), searchOpen = true) }

@Preview(name = "Search open, with text", widthDp = 800, heightDp = 360)
@Composable
private fun InventorySearchOpenPreview() =
    PreviewScene { InventoryContent(Loaded.copy(query = "pol"), InventoryActions(), searchOpen = true) }

@Preview(name = "Search with no result", widthDp = 800, heightDp = 360)
@Composable
private fun InventoryNoResultPreview() =
    PreviewScene { InventoryContent(Loaded.copy(query = "zzz"), InventoryActions(), searchOpen = true) }

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
