package br.eng.joaovictor.resort.ds.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space

/** Nothing here yet; always offers a way out through [action] (label to callback) when it makes sense. */
@Composable
fun EmptyState(
    icon: ImageVector,
    title: String,
    modifier: Modifier = Modifier,
    message: String? = null,
    action: Pair<String, () -> Unit>? = null,
) {
    val colors = ResortTheme.colors
    Column(
        modifier.padding(Space.xxl),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        Icon(icon, null, tint = colors.textMuted, modifier = Modifier.size(48.dp))
        Text(title, style = ResortTheme.type.titleSmall, color = colors.text, textAlign = TextAlign.Center)
        if (message != null) Text(message, style = ResortTheme.type.body, color = colors.textMuted, textAlign = TextAlign.Center)
        if (action != null) GameButton(action.first, action.second, variant = ButtonVariant.Secondary, size = ButtonSize.Small)
    }
}

// ---- Previews ----

@Preview(name = "EmptyState", widthDp = 360, heightDp = 280)
@Composable
private fun EmptyStatePreview() = DsPreviewScene {
    EmptyState(GameIcons.Backpack, "Seu inventário está vazio", message = "Compre móveis na loja para decorar sua sala.", action = "Ir à loja" to {})
}
