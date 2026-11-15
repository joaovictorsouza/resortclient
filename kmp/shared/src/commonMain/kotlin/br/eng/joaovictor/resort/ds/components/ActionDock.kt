package br.eng.joaovictor.resort.ds.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

@Immutable
data class DockAction(
    val key: String,
    val label: String,
    val icon: ImageVector,
    val style: IconButtonStyle = IconButtonStyle.Plain,
    val active: Boolean = false,
    val badge: BadgeSpec? = null,
    /** Starts a new group after a separator (owner actions). */
    val separatorBefore: Boolean = false,
    /** Bigger button for the main action (the shop). */
    val large: Boolean = false,
)

/**
 * The HUD dock: a `surface` pill with 56dp buttons, 8dp apart. In portrait the label shows under the icon;
 * in landscape and at large font scales only the icon (the label becomes the tooltip).
 */
@Composable
fun ActionDock(actions: List<DockAction>, onAction: (String) -> Unit, modifier: Modifier = Modifier, showLabels: Boolean = false) {
    val colors = ResortTheme.colors
    Row(
        modifier
            .drawSolidShadow(corner = 40.dp)
            .background(colors.surface, Radius.pill)
            .border(1.dp, colors.outline, Radius.pill)
            .padding(horizontal = Space.sm, vertical = Space.xs),
        horizontalArrangement = Arrangement.spacedBy(Space.sm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        actions.forEach { a ->
            if (a.separatorBefore) {
                Box(Modifier.padding(vertical = Space.sm).size(1.dp, 32.dp).background(colors.outline))
            }
            GameIconButton(
                a.icon, a.label, { onAction(a.key) },
                style = if (a.active) IconButtonStyle.Raised else a.style,
                size = if (a.large) DsSize.dockShopButton else DsSize.hudButton,
                active = a.active, badge = a.badge,
                label = if (showLabels) a.label else null,
            )
        }
    }
}

// ---- Previews ----

private val SampleDock = listOf(
    DockAction("rooms", "Salas", GameIcons.Compass),
    DockAction("items", "Itens", GameIcons.Backpack),
    DockAction("shop", "Loja", GameIcons.Shop, IconButtonStyle.Primary, large = true),
    DockAction("build", "Construir", GameIcons.Hammer, separatorBefore = true),
)

@Preview(name = "ActionDock", widthDp = 360, heightDp = 200)
@Composable
private fun ActionDockPreview() = DsPreviewScene {
    Box(Modifier.fillMaxSize().padding(Space.lg), contentAlignment = Alignment.BottomCenter) {
        androidx.compose.foundation.layout.Column(verticalArrangement = Arrangement.spacedBy(Space.md)) {
            ActionDock(SampleDock, {}, showLabels = true)
            ActionDock(SampleDock.map { if (it.key == "build") it.copy(active = true) else it }, {})
        }
    }
}
