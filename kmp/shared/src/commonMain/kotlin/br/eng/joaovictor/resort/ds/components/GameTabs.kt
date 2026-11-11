package br.eng.joaovictor.resort.ds.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Haptic
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

@Immutable
data class TabItem(val key: String, val label: String, val icon: ImageVector? = null, val badge: BadgeSpec? = null)

@Immutable
data class ChipItem(val key: String, val label: String)

/** Segmented control in a pill; the active tab is a `secondary` face. For 2 to 5 tabs. */
@Composable
fun GameTabs(tabs: List<TabItem>, selected: String, onSelect: (String) -> Unit, modifier: Modifier = Modifier) {
    val colors = ResortTheme.colors
    val feedback = ResortTheme.feedback
    Row(modifier.background(colors.surfaceSunken, Radius.pill).padding(Space.xxs + Space.xxs)) {
        tabs.forEach { tab ->
            val active = tab.key == selected
            Box(
                Modifier
                    .weight(1f)
                    .defaultMinSize(minHeight = DsSize.touchMin - Space.xs)
                    .background(if (active) colors.secondary else androidx.compose.ui.graphics.Color.Transparent, Radius.pill)
                    .semantics { role = Role.Tab; this.selected = active }
                    .clickable(role = Role.Tab) { feedback.haptic(Haptic.Select); onSelect(tab.key) }
                    .padding(horizontal = Space.md),
                contentAlignment = Alignment.Center,
            ) {
                Row(horizontalArrangement = Arrangement.spacedBy(Space.xs + Space.xxs), verticalAlignment = Alignment.CenterVertically) {
                    if (tab.icon != null) Icon(tab.icon, null, tint = if (active) colors.onAccent else colors.textMuted, modifier = Modifier.size(DsSize.iconSmall))
                    Text(tab.label, style = ResortTheme.type.label, color = if (active) colors.onAccent else colors.textMuted, maxLines = 1)
                    if (tab.badge != null) Badge(tab.badge)
                }
            }
        }
    }
}

/** Scrollable row of category chips: 40dp drawn, 48dp touch area; the active one is `secondary`. */
@Composable
fun CategoryChips(items: List<ChipItem>, selected: String?, onSelect: (String) -> Unit, modifier: Modifier = Modifier) {
    val colors = ResortTheme.colors
    val feedback = ResortTheme.feedback
    Row(modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
        items.forEach { item ->
            val active = item.key == selected
            Box(
                Modifier
                    .defaultMinSize(minHeight = DsSize.touchMin)
                    .semantics { role = Role.Tab; this.selected = active }
                    .clickable(role = Role.Tab) { feedback.haptic(Haptic.Select); onSelect(item.key) },
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    Modifier.height(DsSize.chipHeight).background(if (active) colors.secondary else colors.surfaceRaised, Radius.pill).padding(horizontal = Space.lg),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(item.label, style = ResortTheme.type.label, color = if (active) colors.onAccent else colors.text, maxLines = 1)
                }
            }
        }
    }
}

// ---- Previews ----

@Preview(name = "Tabs and chips", widthDp = 360, heightDp = 220)
@Composable
private fun GameTabsPreview() = DsPreviewScene {
    Column(Modifier.padding(Space.lg), verticalArrangement = Arrangement.spacedBy(Space.md)) {
        GameTabs(
            listOf(TabItem("a", "Salas", GameIcons.Compass), TabItem("b", "Meus", GameIcons.User, BadgeSpec.Count(2)), TabItem("c", "Buscar", GameIcons.Search)),
            selected = "b", onSelect = {}, modifier = Modifier.fillMaxWidth(),
        )
        CategoryChips(listOf(ChipItem("all", "Tudo"), ChipItem("c", "Cadeiras"), ChipItem("t", "Tapetes"), ChipItem("p", "Plantas"), ChipItem("l", "Luminárias")), "c", {})
    }
}
