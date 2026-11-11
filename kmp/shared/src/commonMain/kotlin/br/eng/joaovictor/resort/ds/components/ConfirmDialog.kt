package br.eng.joaovictor.resort.ds.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Haptic
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space

/** The card of a confirmation, without the window; previewable and reusable. */
@Composable
fun ConfirmCard(
    title: String,
    message: String,
    confirmLabel: String,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
    destructive: Boolean = false,
    icon: ImageVector? = null,
    dismissLabel: String = "Cancelar",
) {
    val colors = ResortTheme.colors
    Column(
        modifier
            .widthIn(max = 420.dp)
            .background(colors.surface, RoundedCornerShape(Radius.lg))
            .border(1.dp, colors.outline, RoundedCornerShape(Radius.lg))
            .padding(Space.xl),
        verticalArrangement = Arrangement.spacedBy(Space.md),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
            if (icon != null) Icon(icon, null, tint = if (destructive) colors.dangerText else colors.secondary, modifier = Modifier.size(28.dp))
            Text(title, style = ResortTheme.type.title, color = colors.text)
        }
        Text(message, style = ResortTheme.type.body, color = colors.textMuted)
        Row(Modifier.align(Alignment.End), horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
            GameButton(dismissLabel, onDismiss, variant = ButtonVariant.Ghost)
            GameButton(
                confirmLabel, onConfirm,
                variant = if (destructive) ButtonVariant.Danger else ButtonVariant.Primary,
                haptic = if (destructive) Haptic.Error else Haptic.Confirm,
            )
        }
    }
}

/**
 * The only component with a scrim: it blocks the scene because the action cannot be undone. Whoever shows
 * it should also hold the HUD hidden (`HideHudWhileVisible`); `ds` does not depend on the HUD.
 */
@Composable
fun ConfirmDialog(
    title: String,
    message: String,
    confirmLabel: String,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
    destructive: Boolean = false,
    icon: ImageVector? = null,
) {
    Dialog(onDismissRequest = onDismiss, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        val scrim = ResortTheme.colors.scrim
        Box(
            Modifier.fillMaxSize().background(scrim)
                .clickable(remember { MutableInteractionSource() }, indication = null, onClick = onDismiss)
                .safeDrawingPadding().padding(Space.lg),
            contentAlignment = Alignment.Center,
        ) {
            // The card swallows taps so only the scrim dismisses.
            ConfirmCard(
                title, message, confirmLabel, onConfirm, onDismiss, destructive = destructive, icon = icon,
                modifier = Modifier.clickable(remember { MutableInteractionSource() }, indication = null) {},
            )
        }
    }
}

// ---- Previews ----

@Preview(name = "ConfirmCard", widthDp = 360, heightDp = 300)
@Composable
private fun ConfirmCardPreview() = DsPreviewScene {
    Box(Modifier.fillMaxSize().padding(Space.lg), contentAlignment = Alignment.Center) {
        ConfirmCard("Recolher tudo?", "Todos os móveis voltam para o seu inventário.", "Recolher", {}, {}, destructive = true, icon = GameIcons.Lock)
    }
}
