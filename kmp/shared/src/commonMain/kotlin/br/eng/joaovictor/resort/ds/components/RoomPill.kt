package br.eng.joaovictor.resort.ds.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
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

/** Room name with a visibility icon and a chevron; the name ellipsizes so a long one never pushes the HUD. */
@Composable
fun RoomPill(
    name: String,
    visibilityIcon: ImageVector,
    visibilityLabel: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = ResortTheme.colors
    val feedback = ResortTheme.feedback
    Box(
        modifier
            .defaultMinSize(minHeight = DsSize.touchMin)
            .semantics(mergeDescendants = true) { contentDescription = "$name, sala $visibilityLabel" }
            .clickable(role = Role.Button) { feedback.haptic(Haptic.Tap); onClick() },
        contentAlignment = Alignment.CenterStart,
    ) {
        Row(
            Modifier
                .background(colors.surface, Radius.pill)
                .border(1.dp, colors.outline, Radius.pill)
                .padding(horizontal = Space.md, vertical = Space.sm),
            horizontalArrangement = Arrangement.spacedBy(Space.sm),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(visibilityIcon, null, tint = colors.secondary, modifier = Modifier.size(DsSize.iconSmall))
            Text(name, style = ResortTheme.type.titleSmall, color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.widthIn(max = 220.dp))
            Icon(GameIcons.ChevronDown, null, tint = colors.textMuted, modifier = Modifier.size(DsSize.iconSmall))
        }
    }
}

// ---- Previews ----

@Preview(name = "RoomPill", widthDp = 360, heightDp = 160)
@Composable
private fun RoomPillPreview() = DsPreviewScene {
    androidx.compose.foundation.layout.Column(Modifier.padding(Space.lg), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
        RoomPill("Casa da Praia", GameIcons.Globe, "pública", {})
        RoomPill("A sala mais comprida de todo o resort, com um nome absurdamente longo", GameIcons.Lock, "privada", {})
    }
}
