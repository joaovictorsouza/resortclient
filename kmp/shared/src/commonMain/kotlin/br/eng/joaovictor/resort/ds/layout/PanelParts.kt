package br.eng.joaovictor.resort.ds.layout

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextOverflow
import br.eng.joaovictor.resort.ds.components.GameIconButton
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

/** 56dp header: icon and title, the extra actions, then the close button. */
@Composable
fun PanelHeader(
    title: String,
    onClose: () -> Unit,
    modifier: Modifier = Modifier,
    icon: ImageVector? = null,
    actions: @Composable RowScope.() -> Unit = {},
) {
    val colors = ResortTheme.colors
    Row(
        modifier.fillMaxWidth().height(DsSize.panelHeader).padding(start = Space.lg, end = Space.xs),
        horizontalArrangement = Arrangement.spacedBy(Space.sm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) Icon(icon, null, tint = colors.secondary, modifier = Modifier.size(DsSize.icon))
        Text(title, style = ResortTheme.type.title, color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
        actions()
        GameIconButton(GameIcons.Close, "Fechar", onClose)
    }
}

/** Sticky bottom bar (buy bar, place button). */
@Composable
fun PanelFooter(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Column(modifier.fillMaxWidth().padding(horizontal = Space.lg, vertical = Space.md), content = content)
}

/** Section title inside a panel. */
@Composable
fun PanelSection(title: String, modifier: Modifier = Modifier) {
    Text(title, style = ResortTheme.type.titleSmall, color = ResortTheme.colors.textMuted, modifier = modifier.padding(horizontal = Space.lg, vertical = Space.sm))
}
