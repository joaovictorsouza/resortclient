package br.eng.joaovictor.resort.ds.layout

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

enum class PanelForm { Side, Sheet }

enum class SheetDetent { Peek, Half, Full }

@Immutable
data class PanelSpec(val form: PanelForm, val sideWidth: Dp, val peek: Dp, val half: Dp, val full: Dp) {
    /** Visible height of the sheet at [detent]. */
    fun heightAt(detent: SheetDetent): Dp = when (detent) {
        SheetDetent.Peek -> peek
        SheetDetent.Half -> half
        SheetDetent.Full -> full
    }
}

/**
 * Landscape (or a wide window) gets a side panel docked to the right; portrait gets a bottom sheet with
 * three detents. Pure, so it is tested without a window.
 */
fun panelSpecFor(width: Dp, height: Dp): PanelSpec {
    val side = (width > height && width >= 560.dp) || width >= 840.dp
    val sideWidth = (width * 0.42f).coerceIn(340.dp, 480.dp).coerceAtMost(width * 0.55f)
    return PanelSpec(
        form = if (side) PanelForm.Side else PanelForm.Sheet,
        sideWidth = sideWidth,
        peek = 136.dp,
        half = height * 0.52f,
        full = height * 0.92f,
    )
}

/** The spec for the current window. */
@Composable
fun rememberPanelSpec(): PanelSpec {
    val size = LocalWindowInfo.current.containerSize
    val density = LocalDensity.current
    return remember(size, density) {
        with(density) { panelSpecFor(size.width.toDp(), size.height.toDp()) }
    }
}
