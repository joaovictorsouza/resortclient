package br.eng.joaovictor.resort.ds.layout

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.ui.platform.LocalInspectionMode
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.components.drawSolidShadow
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Motion
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

/**
 * Landscape panel docked to the right, full height, rounded only on the left. It leaves the scene visible
 * and **tappable** (nothing here has a scrim or a pointer handler outside its own bounds), so tapping the floor
 * does not close it. Slides with `translationX`, read at draw time, so the animation never recomposes.
 *
 * [tucked] slides it out of the way leaving a [DsSize.tuckedHandle] strip, without closing it.
 */
@Composable
fun SidePanel(
    visible: Boolean,
    title: String,
    onClose: () -> Unit,
    width: androidx.compose.ui.unit.Dp,
    modifier: Modifier = Modifier,
    icon: ImageVector? = null,
    headerActions: @Composable RowScope.() -> Unit = {},
    footer: (@Composable () -> Unit)? = null,
    tucked: Boolean = false,
    onBoundsChange: (Rect) -> Unit = {},
    /** Another panel was open a moment ago: fade in over it instead of sliding in. */
    swapIn: Boolean = false,
    content: @Composable ColumnScope.() -> Unit,
) {
    // 0 = fully shown, 1 = fully out; a tucked panel leaves its handle in view. A new panel slides in from
    // the edge (previews and swaps start in place).
    val inPlace = LocalInspectionMode.current || swapIn
    val hidden = remember { Animatable(if (visible && inPlace) 0f else 1f) }
    val fade = remember { Animatable(if (swapIn) 0.5f else 1f) }
    LaunchedEffect(Unit) { if (swapIn) fade.animateTo(1f, tween(Motion.base)) }
    val tuckedFraction = 1f - DsSize.tuckedHandle.value / width.value
    LaunchedEffect(visible, tucked, tuckedFraction) {
        hidden.animateTo(if (!visible) 1f else if (tucked) tuckedFraction else 0f, Motion.panelSpring)
    }
    if (!visible && hidden.value >= 0.999f) return

    val colors = ResortTheme.colors
    val shape = RoundedCornerShape(topStart = Radius.xl, bottomStart = Radius.xl)
    Box(modifier.fillMaxSize()) {
        Column(
            Modifier
                .align(Alignment.CenterEnd)
                .width(width)
                .fillMaxHeight()
                .graphicsLayer { translationX = hidden.value * size.width; alpha = fade.value }
                .onGloballyPositioned { onBoundsChange(it.boundsInRoot()) }
                .drawSolidShadow(corner = Radius.xl)
                .background(colors.surface, shape)
                .border(1.dp, colors.outline, shape)
                .windowInsetsPadding(WindowInsets.safeDrawing),
        ) {
            PanelHeader(title, onClose, icon = icon, actions = headerActions)
            Column(Modifier.weight(1f), content = content)
            if (footer != null) PanelFooter { footer() }
        }
    }
}

// ---- Previews ----

@Preview(name = "SidePanel (landscape)", widthDp = 800, heightDp = 360)
@Composable
private fun SidePanelPreview() = DsPreviewScene {
    SidePanel(visible = true, title = "Inventário", onClose = {}, width = 340.dp, icon = GameIcons.Backpack) {
        Text("Conteúdo do painel", color = ResortTheme.colors.textMuted, modifier = Modifier.padding(16.dp))
    }
}
