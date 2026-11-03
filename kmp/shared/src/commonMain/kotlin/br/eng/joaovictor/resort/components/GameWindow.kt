package br.eng.joaovictor.resort.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.roundToInt

private val WindowShape = RoundedCornerShape(16.dp)

/**
 * A floating window over the scene: a rounded card with a title bar that drags it (kept inside the safe
 * area), a close button and an optional search toggle. There is no scrim, so touches outside the window
 * reach the game. Stateless apart from its own position.
 *
 * @param searchOpen `null` hides the search icon; otherwise it is the open state of the search field the
 * caller shows (see [SearchSection]). A tap on the title area or the icon calls [onToggleSearch]; a drag on
 * the title bar moves the window instead.
 * @param windowAlpha lets the caller fade the window (e.g. while an item is dragged out of it).
 * @param onBoundsChange reports the window's bounds in the root, for hit tests outside it.
 * @param overlay drawn over the window in the same area (e.g. a drag ghost).
 */
@Composable
fun GameWindow(
    title: String,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
    maxWidth: Dp = 420.dp,
    widthFraction: Float = 0.6f,
    heightFraction: Float = 0.9f,
    searchOpen: Boolean? = null,
    onToggleSearch: () -> Unit = {},
    windowAlpha: Float = 1f,
    onBoundsChange: (Rect) -> Unit = {},
    overlay: @Composable BoxScope.() -> Unit = {},
    content: @Composable ColumnScope.() -> Unit,
) {
    BoxWithConstraints(modifier.fillMaxSize().safeDrawingPadding()) {
        val density = LocalDensity.current
        val maxW = constraints.maxWidth.toFloat()
        val maxH = constraints.maxHeight.toFloat()
        val winW = minOf(maxWidth, this.maxWidth * widthFraction)
        val winH = this.maxHeight * heightFraction
        val winWpx = with(density) { winW.toPx() }
        val winHpx = with(density) { winH.toPx() }
        var origin by remember { mutableStateOf<Offset?>(null) }
        val pos = origin ?: Offset((maxW - winWpx) / 2f, (maxH - winHpx) / 2f)

        Surface(
            modifier = Modifier
                .offset { IntOffset(pos.x.roundToInt(), pos.y.roundToInt()) }
                .width(winW)
                .height(winH)
                .alpha(windowAlpha)
                .onGloballyPositioned { onBoundsChange(it.boundsInRoot()) },
            shape = WindowShape,
            color = MaterialTheme.colorScheme.surface,
            shadowElevation = 12.dp,
            border = BorderStroke(1.dp, Color.White.copy(alpha = 0.15f)),
        ) {
            Column(Modifier.padding(bottom = 8.dp)) {
                WindowTitleBar(
                    title = title,
                    searchOpen = searchOpen,
                    onSearch = onToggleSearch,
                    onMove = { delta -> origin = clampWindow(pos + delta, maxW, maxH, winWpx, winHpx) },
                    onClose = onClose,
                )
                content()
            }
        }
        overlay()
    }
}

/** The window keeps fully inside the available area. */
fun clampWindow(target: Offset, maxW: Float, maxH: Float, winW: Float, winH: Float): Offset =
    Offset(
        target.x.coerceIn(0f, (maxW - winW).coerceAtLeast(0f)),
        target.y.coerceIn(0f, (maxH - winH).coerceAtLeast(0f)),
    )

@Composable
private fun WindowTitleBar(
    title: String,
    searchOpen: Boolean?,
    onSearch: () -> Unit,
    onMove: (Offset) -> Unit,
    onClose: () -> Unit,
) {
    // The gesture blocks below are created once; they must call the latest lambdas, which hold the
    // window's current position (a stale one made every drag restart from the first position).
    val move by rememberUpdatedState(onMove)
    val toggle by rememberUpdatedState(onSearch)
    val hasSearch = searchOpen != null
    Row(
        Modifier.fillMaxWidth()
            .background(Color.White.copy(alpha = 0.06f))
            // A tap on the title area toggles the search; a drag (below) moves the window instead. The
            // buttons consume their own taps, so they never reach this detector.
            .then(if (hasSearch) Modifier.pointerInput(Unit) { detectTapGestures { toggle() } } else Modifier)
            .pointerInput(Unit) { detectDragGestures { change, amount -> change.consume(); move(amount) } }
            .padding(start = 16.dp, end = 4.dp, top = 4.dp, bottom = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
        if (hasSearch) {
            TextButton(onClick = onSearch) {
                Text("🔍", fontSize = 18.sp, modifier = Modifier.alpha(if (searchOpen == true) 1f else 0.7f))
            }
        }
        TextButton(onClick = onClose) { Text("✕", fontSize = 18.sp) }
    }
}

/**
 * The window's search field: expands when [open], takes focus and shows the keyboard, and hides the
 * keyboard when it closes.
 */
@Composable
fun SearchSection(
    open: Boolean,
    query: String,
    onQuery: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "Buscar",
) {
    val keyboard = LocalSoftwareKeyboardController.current
    LaunchedEffect(open) { if (!open) keyboard?.hide() }
    AnimatedVisibility(visible = open, enter = expandVertically(), exit = shrinkVertically(), modifier = modifier) {
        val focus = remember { FocusRequester() }
        LaunchedEffect(Unit) {
            focus.requestFocus()
            keyboard?.show()
        }
        GameSearchBar(
            value = query,
            onValueChange = onQuery,
            placeholderText = placeholder,
            modifier = Modifier.fillMaxWidth().focusRequester(focus),
        )
    }
}
