package br.eng.joaovictor.resort.ds.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Haptic
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

/** A list row that can be selected (room layouts, settings): 2.5dp `secondary` border and a check when selected. */
@Composable
fun SelectableRow(title: String, subtitle: String?, selected: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val colors = ResortTheme.colors
    val feedback = ResortTheme.feedback
    val shape = RoundedCornerShape(Radius.md)
    Row(
        modifier.fillMaxWidth().defaultMinSize(minHeight = DsSize.touchMin)
            .semantics { this.selected = selected }
            .background(if (selected) colors.surfaceHigh else colors.surfaceRaised, shape)
            .border(if (selected) 2.5.dp else 1.dp, if (selected) colors.secondary else colors.outline, shape)
            .clickable(role = Role.RadioButton) { feedback.haptic(Haptic.Select); onClick() }
            .padding(Space.md),
        horizontalArrangement = Arrangement.spacedBy(Space.md),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, style = ResortTheme.type.titleSmall, color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (subtitle != null) Text(subtitle, style = ResortTheme.type.caption, color = colors.textMuted, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
        if (selected) Icon(GameIcons.Check, null, tint = colors.secondary, modifier = Modifier.size(DsSize.icon))
    }
}

/** A room (or public space) in a list, with its action (join). */
@Composable
fun RoomListCard(name: String, subtitle: String, actionLabel: String, onAction: () -> Unit, modifier: Modifier = Modifier) {
    val colors = ResortTheme.colors
    Row(
        modifier.fillMaxWidth()
            .background(colors.surfaceRaised, RoundedCornerShape(Radius.md))
            .border(1.dp, colors.outline, RoundedCornerShape(Radius.md))
            .padding(start = Space.md, top = Space.sm, bottom = Space.sm, end = Space.sm),
        horizontalArrangement = Arrangement.spacedBy(Space.md),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(name, style = ResortTheme.type.titleSmall, color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(subtitle, style = ResortTheme.type.caption, color = colors.textMuted, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
        GameButton(actionLabel, onAction, size = ButtonSize.Small, variant = ButtonVariant.Secondary)
    }
}

// ---- Previews ----

@Preview(name = "SelectableRow and RoomListCard", widthDp = 360, heightDp = 340)
@Composable
private fun SelectableRowPreview() = DsPreviewScene {
    Column(Modifier.padding(Space.lg), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
        SelectableRow("Praia pequena", "12x12 (máx. 20)", selected = true, onClick = {})
        SelectableRow("Salão", "20x20 (máx. 40)", selected = false, onClick = {})
        RoomListCard("Casa da Praia", "Criado por João  •  Ocupação 3/40  •  Pública", "Entrar", {})
        RoomListCard("Uma sala com um nome muito comprido para testar a quebra de linha", "Criado por Player  •  Ocupação 0/40  •  Privada", "Entrar", {})
    }
}
