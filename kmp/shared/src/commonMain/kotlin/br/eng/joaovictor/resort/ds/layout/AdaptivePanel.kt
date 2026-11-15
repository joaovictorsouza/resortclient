package br.eng.joaovictor.resort.ds.layout

import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.ResortTheme

/**
 * The one panel of the game UI: a side panel docked right in landscape, a three-detent bottom sheet in
 * portrait (see [panelSpecFor]). Replaces the floating `GameWindow`. It never darkens the scene or
 * blocks touches outside its own bounds.
 */
@Composable
fun AdaptivePanel(
    visible: Boolean,
    title: String,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
    icon: ImageVector? = null,
    /** Header extras between the title and the close button (search toggle, balance). */
    headerActions: @Composable RowScope.() -> Unit = {},
    /** Sticky bottom bar (buy bar, place button). */
    footer: (@Composable () -> Unit)? = null,
    initialDetent: SheetDetent = SheetDetent.Half,
    /** Slides the panel out of the way without closing it (e.g. while an item is dragged to the scene). */
    tucked: Boolean = false,
    onBoundsChange: (Rect) -> Unit = {},
    spec: PanelSpec = rememberPanelSpec(),
    /** Identifies this panel to the [PanelHostState]; panels with a key swap without a double animation. */
    panelKey: String? = null,
    content: @Composable ColumnScope.() -> Unit,
) {
    val host = LocalPanelHost.current
    val swapIn = remember(panelKey, host) { panelKey != null && host != null && host.claim(panelKey) }
    DisposableEffect(panelKey, host) { onDispose { if (panelKey != null) host?.release(panelKey) } }
    when (spec.form) {
        PanelForm.Side -> SidePanel(
            visible, title, onClose, spec.sideWidth, modifier, icon, headerActions, footer, tucked, onBoundsChange, swapIn, content,
        )
        PanelForm.Sheet -> BottomSheet(
            visible, title, onClose, spec, modifier, icon, headerActions, footer, initialDetent, tucked, onBoundsChange, swapIn, content,
        )
    }
}

// ---- Previews ----

@Preview(name = "AdaptivePanel landscape", widthDp = 800, heightDp = 360)
@Composable
private fun AdaptivePanelLandscapePreview() = DsPreviewScene {
    AdaptivePanel(true, "Loja", {}, icon = GameIcons.Shop, spec = panelSpecFor(800.dp, 360.dp)) {
        Text("Landscape", color = ResortTheme.colors.textMuted, modifier = Modifier.padding(16.dp))
    }
}

@Preview(name = "AdaptivePanel portrait", widthDp = 360, heightDp = 780)
@Composable
private fun AdaptivePanelPortraitPreview() = DsPreviewScene {
    AdaptivePanel(true, "Loja", {}, icon = GameIcons.Shop, spec = panelSpecFor(360.dp, 780.dp)) {
        Text("Portrait", color = ResortTheme.colors.textMuted, modifier = Modifier.padding(16.dp))
    }
}
