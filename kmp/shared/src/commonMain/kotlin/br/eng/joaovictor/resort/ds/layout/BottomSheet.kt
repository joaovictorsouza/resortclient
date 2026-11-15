package br.eng.joaovictor.resort.ds.layout

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalInspectionMode
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.AnchoredDraggableDefaults
import androidx.compose.foundation.gestures.AnchoredDraggableState
import androidx.compose.foundation.gestures.FlingBehavior
import androidx.compose.foundation.gestures.ScrollScope
import androidx.compose.foundation.gestures.DraggableAnchors
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.anchoredDraggable
import androidx.compose.foundation.gestures.animateTo
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.components.drawSolidShadow
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Motion
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import kotlin.math.roundToInt

/**
 * Portrait bottom sheet with three detents (Peek, Half, Full), no scrim, no blocking of the scene.
 *
 * Built on `anchoredDraggable` from foundation, not Material3's `ModalBottomSheet` (it has a scrim and no
 * peek). **This is the only file that imports the anchored-draggable API**, so a Compose upgrade touches one place.
 * The sheet is bottom-anchored and its *height* follows the drag (read in the layout phase, no recomposition),
 * so the footer always sits at the visible bottom edge.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun BottomSheet(
    visible: Boolean,
    title: String,
    onClose: () -> Unit,
    spec: PanelSpec,
    modifier: Modifier = Modifier,
    icon: ImageVector? = null,
    headerActions: @Composable RowScope.() -> Unit = {},
    footer: (@Composable () -> Unit)? = null,
    initialDetent: SheetDetent = SheetDetent.Half,
    /** Lowers the sheet to Peek without closing it. */
    tucked: Boolean = false,
    onBoundsChange: (Rect) -> Unit = {},
    /** Another panel was open a moment ago: fade in over it instead of sliding up. */
    swapIn: Boolean = false,
    content: @Composable ColumnScope.() -> Unit,
) {
    val density = LocalDensity.current
    val fullPx = with(density) { spec.full.toPx() }
    val anchors = remember(spec, density) {
        DraggableAnchors {
            // The offset is how far the sheet is below Full.
            SheetDetent.Full at 0f
            SheetDetent.Half at with(density) { (spec.full - spec.half).toPx() }
            SheetDetent.Peek at with(density) { (spec.full - spec.peek).toPx() }
        }
    }
    val state = remember { AnchoredDraggableState(initialValue = initialDetent) }
    SideEffect { state.updateAnchors(anchors) }
    val inPlace = LocalInspectionMode.current || swapIn
    val shown = remember { Animatable(if (visible && inPlace) 1f else 0f) }
    val fade = remember { Animatable(if (swapIn) 0.5f else 1f) }
    LaunchedEffect(Unit) { if (swapIn) fade.animateTo(1f, tween(Motion.base)) }
    LaunchedEffect(visible) { shown.animateTo(if (visible) 1f else 0f, Motion.panelSpring) }
    LaunchedEffect(tucked) {
        if (tucked) state.animateTo(SheetDetent.Peek, Motion.panelSpring) else if (state.targetValue == SheetDetent.Peek) state.animateTo(initialDetent, Motion.panelSpring)
    }
    if (!visible && shown.value <= 0.001f) return

    val colors = ResortTheme.colors
    val shape = RoundedCornerShape(topStart = Radius.xl, topEnd = Radius.xl)
    val fling = AnchoredDraggableDefaults.flingBehavior(state, positionalThreshold = { distance -> distance * 0.5f }, animationSpec = Motion.panelSpring)
    val nested = remember(state, fling) { sheetNestedScroll(state, fling) }
    Box(modifier.fillMaxSize(), contentAlignment = Alignment.BottomCenter) {
        Column(
            Modifier
                .fillMaxWidth()
                .layout { measurable, constraints ->
                    val offset = if (state.offset.isNaN()) 0f else state.offset
                    val h = ((fullPx - offset) * shown.value).roundToInt().coerceAtLeast(0)
                    val p = measurable.measure(constraints.copy(minHeight = h, maxHeight = h))
                    layout(p.width, h) { p.place(0, 0) }
                }
                .onGloballyPositioned { onBoundsChange(it.boundsInRoot()) }
                .graphicsLayer { alpha = fade.value }
                .drawSolidShadow(corner = Radius.xl)
                .background(colors.surface, shape)
                .border(1.dp, colors.outline, shape)
                .anchoredDraggable(state, Orientation.Vertical, flingBehavior = fling)
                .nestedScroll(nested)
                .windowInsetsPadding(WindowInsets.safeDrawing),
        ) {
            Box(Modifier.fillMaxWidth().padding(top = 8.dp), contentAlignment = Alignment.Center) {
                Box(Modifier.width(36.dp).height(4.dp).background(colors.outlineStrong, Radius.pill))
            }
            PanelHeader(title, onClose, icon = icon, actions = headerActions)
            Column(Modifier.weight(1f), content = content)
            if (footer != null && state.currentValue != SheetDetent.Peek) PanelFooter { footer() }
        }
    }
}

/** Dragging up moves the sheet first, the list scrolls only at Full; a downward overscroll of the list drags the sheet down. */
@OptIn(ExperimentalFoundationApi::class)
private fun sheetNestedScroll(state: AnchoredDraggableState<SheetDetent>, fling: FlingBehavior) = object : NestedScrollConnection {
    override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
        val delta = available.y
        return if (delta < 0 && source == NestedScrollSource.UserInput) Offset(0f, state.dispatchRawDelta(delta)) else Offset.Zero
    }

    override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset =
        if (source == NestedScrollSource.UserInput) Offset(0f, state.dispatchRawDelta(available.y)) else Offset.Zero

    override suspend fun onPreFling(available: Velocity): Velocity {
        val toFling = available.y
        val offset = if (state.offset.isNaN()) 0f else state.offset
        return if (toFling < 0 && offset > 0f) {
            settle(state, fling, toFling)
            available
        } else {
            Velocity.Zero
        }
    }

    override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
        settle(state, fling, available.y)
        return available
    }
}

private suspend fun settle(state: AnchoredDraggableState<SheetDetent>, fling: FlingBehavior, velocity: Float) {
    val scope = object : ScrollScope {
        override fun scrollBy(pixels: Float): Float = state.dispatchRawDelta(pixels)
    }
    with(fling) { scope.performFling(velocity) }
}

// ---- Previews ----

@Preview(name = "BottomSheet (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun BottomSheetPreview() = DsPreviewScene {
    BottomSheet(visible = true, title = "Inventário", onClose = {}, spec = panelSpecFor(360.dp, 780.dp), icon = GameIcons.Backpack) {
        Text("Conteúdo do sheet", color = ResortTheme.colors.textMuted, modifier = Modifier.padding(16.dp))
    }
}
