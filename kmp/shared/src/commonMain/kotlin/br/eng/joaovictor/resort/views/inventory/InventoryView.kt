package br.eng.joaovictor.resort.views.inventory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
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
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import br.eng.joaovictor.resort.loadImageFile
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlin.math.roundToInt

private val WindowShape = RoundedCornerShape(16.dp)
private val Selected = Color(0xFF4F46E5)

/** An item being dragged out of the window; [position] is the finger in root (surface) pixels. */
private data class Drag(val row: InventoryRow, val position: Offset)

/**
 * Floating inventory window: a movable card over the scene (no scrim, touches outside it reach the game).
 * Long-press an item and drag it out of the window to drop it on the floor; tap it to place it.
 */
@Composable
fun InventoryView(viewModel: InventoryViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.open) return
    BoxWithConstraints(Modifier.fillMaxSize().safeDrawingPadding()) {
        val density = androidx.compose.ui.platform.LocalDensity.current
        val maxW = constraints.maxWidth.toFloat()
        val maxH = constraints.maxHeight.toFloat()
        val winW = minOf(420.dp, maxWidth * 0.6f)
        val winH = maxHeight * 0.7f
        val winWpx = with(density) { winW.toPx() }
        val winHpx = with(density) { winH.toPx() }
        var origin by remember { mutableStateOf<Offset?>(null) }
        val pos = origin ?: Offset((maxW - winWpx) / 2f, (maxH - winHpx) / 2f)
        var windowBounds by remember { mutableStateOf(Rect.Zero) }
        var drag by remember { mutableStateOf<Drag?>(null) }
        var outside by remember { mutableStateOf(false) }

        Surface(
            modifier = Modifier
                .offset { IntOffset(pos.x.roundToInt(), pos.y.roundToInt()) }
                .width(winW)
                .height(winH)
                .alpha(if (drag == null) 1f else if (outside) 0f else 0.35f)
                .onGloballyPositioned { windowBounds = it.boundsInRoot() },
            shape = WindowShape,
            color = MaterialTheme.colorScheme.surface,
            shadowElevation = 12.dp,
            border = androidx.compose.foundation.BorderStroke(1.dp, Color.White.copy(alpha = 0.15f)),
        ) {
            Column(Modifier.padding(bottom = 8.dp)) {
                TitleBar(
                    onMove = { delta ->
                        origin = clampWindow(pos + delta, maxW, maxH, winWpx, winHpx)
                    },
                    onClose = viewModel::close,
                )
                Filters(ui, viewModel)
                Content(
                    ui,
                    viewModel,
                    onDragStart = { drag = it; outside = false },
                    onDragMove = { pos ->
                        val d = drag ?: return@Content
                        val now = dropOutside(pos, windowBounds)
                        dragPhase(outside, now, released = false)?.let { viewModel.drag(d.row.catalogId, pos.x, pos.y, it) }
                        outside = now
                        drag = d.copy(position = pos)
                    },
                    onDragEnd = { end ->
                        val d = drag
                        if (d != null) {
                            val now = dropOutside(end, windowBounds)
                            dragPhase(outside, now, released = true)?.let { viewModel.drag(d.row.catalogId, end.x, end.y, it) }
                        }
                        drag = null
                        outside = false
                    },
                    onDragCancel = {
                        val d = drag
                        if (d != null) dragPhase(outside, false, released = false)?.let { viewModel.drag(d.row.catalogId, 0f, 0f, it) }
                        drag = null
                        outside = false
                    },
                )
            }
        }
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
    }
}

/** The window keeps fully inside the available area. */
internal fun clampWindow(target: Offset, maxW: Float, maxH: Float, winW: Float, winH: Float): Offset =
    Offset(
        target.x.coerceIn(0f, (maxW - winW).coerceAtLeast(0f)),
        target.y.coerceIn(0f, (maxH - winH).coerceAtLeast(0f)),
    )

/** A drop counts only when released outside the window. */
internal fun dropOutside(point: Offset, window: Rect): Boolean = !window.contains(point)

@Composable
private fun TitleBar(onMove: (Offset) -> Unit, onClose: () -> Unit) {
    // The gesture block below is created once; it must call the latest lambda, which holds the window's
    // current position (a stale one made every drag restart from the first position, so it never moved).
    val move by rememberUpdatedState(onMove)
    Row(
        Modifier.fillMaxWidth()
            .background(Color.White.copy(alpha = 0.06f))
            .pointerInput(Unit) { detectDragGestures { change, amount -> change.consume(); move(amount) } }
            .padding(start = 16.dp, end = 4.dp, top = 4.dp, bottom = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("Inventário", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
        TextButton(onClick = onClose) { Text("✕", fontSize = 18.sp) }
    }
}

@Composable
private fun Filters(ui: InventoryUiState, viewModel: InventoryViewModel) {
    Column(Modifier.padding(horizontal = 12.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        OutlinedTextField(
            value = ui.query,
            onValueChange = viewModel::setQuery,
            singleLine = true,
            placeholder = { Text("Buscar") },
            modifier = Modifier.fillMaxWidth(),
        )
        LazyRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            items(ui.categories) { key ->
                val on = key == ui.category
                Text(
                    categoryLabel(key),
                    color = Color.White,
                    fontSize = 13.sp,
                    modifier = Modifier
                        .background(if (on) Selected else Color.White.copy(alpha = 0.12f), RoundedCornerShape(14.dp))
                        .clickable { viewModel.setCategory(key) }
                        .padding(horizontal = 10.dp, vertical = 5.dp),
                )
            }
        }
    }
}

@Composable
private fun Content(
    ui: InventoryUiState,
    viewModel: InventoryViewModel,
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
            LazyColumn(
                Modifier.padding(horizontal = 12.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                items(ui.visible, key = { it.catalogId }) { row ->
                    ItemRow(
                        row,
                        ui.canPlace && row.draggable,
                        { viewModel.select(row.catalogId) },
                        onDragStart, onDragMove, onDragEnd, onDragCancel,
                    )
                }
            }
        }
    }
}

@Composable
private fun ItemRow(
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
    Row(
        Modifier.fillMaxWidth()
            .background(if (row.selected) Selected.copy(alpha = 0.35f) else Color.White.copy(alpha = 0.05f), shape)
            .then(if (row.selected) Modifier.border(1.dp, Selected, shape) else Modifier)
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
            )
            .padding(8.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Thumbnail(row, Modifier.size(48.dp))
        Text(row.name, Modifier.weight(1f))
        row.quantity?.let { Text("x$it", color = Color.White.copy(alpha = 0.7f)) }
    }
}

/** The cached thumbnail, decoded off the main thread; a neutral square while it is not available. */
@Composable
private fun Thumbnail(row: InventoryRow, modifier: Modifier) {
    val image: ImageBitmap? by produceState<ImageBitmap?>(null, row.thumbnail) {
        value = if (row.thumbnail.isEmpty()) null else withContext(Dispatchers.Default) { loadImageFile(row.thumbnail) }
    }
    Box(modifier.background(Color.White.copy(alpha = 0.08f), RoundedCornerShape(8.dp)), contentAlignment = Alignment.Center) {
        image?.let { Image(it, row.name, Modifier.fillMaxSize().padding(2.dp)) }
    }
}
