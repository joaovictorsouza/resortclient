package br.eng.joaovictor.resort.ds.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.theme.Haptic
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

@Immutable
data class RailItem(val key: String, val label: String, val children: List<RailItem> = emptyList(), val expanded: Boolean = false)

/**
 * Vertical category list for landscape (112dp): saves height, the scarcest resource there. An item with
 * [RailItem.children] shows them indented while [RailItem.expanded].
 */
@Composable
fun CategoryRail(items: List<RailItem>, selected: String?, onSelect: (String) -> Unit, modifier: Modifier = Modifier) {
    Column(modifier.width(DsSize.sideRailWidth).verticalScroll(rememberScrollState()).padding(horizontal = Space.xs)) {
        items.forEach { item ->
            RailRow(item, item.key == selected, 0, onSelect)
            if (item.expanded) item.children.forEach { RailRow(it, it.key == selected, 1, onSelect) }
        }
    }
}

@Composable
private fun RailRow(item: RailItem, active: Boolean, depth: Int, onSelect: (String) -> Unit) {
    val colors = ResortTheme.colors
    val feedback = ResortTheme.feedback
    Box(
        Modifier.fillMaxWidth().defaultMinSize(minHeight = DsSize.touchMin)
            .semantics { role = Role.Tab; this.selected = active }
            .clickable(role = Role.Tab) { feedback.haptic(Haptic.Select); onSelect(item.key) },
        contentAlignment = Alignment.CenterStart,
    ) {
        Box(
            Modifier.fillMaxWidth().padding(start = (depth * Space.md.value).dp).defaultMinSize(minHeight = 44.dp)
                .background(if (active) colors.secondary else androidx.compose.ui.graphics.Color.Transparent, Radius.pill)
                .padding(horizontal = Space.md),
            contentAlignment = Alignment.CenterStart,
        ) {
            Text(
                item.label, style = ResortTheme.type.label, maxLines = 1, overflow = TextOverflow.Ellipsis,
                color = if (active) colors.onAccent else if (depth == 0) colors.text else colors.textMuted,
            )
        }
    }
}

// ---- Previews ----

@Preview(name = "CategoryRail", widthDp = 160, heightDp = 360)
@Composable
private fun CategoryRailPreview() = DsPreviewScene {
    CategoryRail(
        listOf(
            RailItem("all", "Tudo"),
            RailItem("f", "Móveis", listOf(RailItem("f1", "Cadeiras"), RailItem("f2", "Mesas")), expanded = true),
            RailItem("p", "Pisos"),
            RailItem("a", "Animais"),
        ),
        selected = "f1", onSelect = {},
    )
}
