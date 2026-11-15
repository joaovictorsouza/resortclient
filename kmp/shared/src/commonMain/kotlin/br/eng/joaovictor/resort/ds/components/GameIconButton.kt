package br.eng.joaovictor.resort.ds.components

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Popup
import androidx.compose.ui.window.PopupProperties
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Haptic
import br.eng.joaovictor.resort.ds.theme.Motion
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

/** Plain has no face; Raised sits on the raised surface with a lip; the rest are action colors with a lip. */
enum class IconButtonStyle { Plain, Raised, Primary, Success, Danger }

/**
 * Square icon button. 48dp touch area by default (the HUD uses 56dp). [active] turns the face to the
 * accent color (build mode on). Long press shows [contentDescription] as a tooltip.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun GameIconButton(
    icon: ImageVector,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    style: IconButtonStyle = IconButtonStyle.Plain,
    size: Dp = DsSize.iconButton,
    active: Boolean = false,
    badge: BadgeSpec? = null,
    label: String? = null,
    enabled: Boolean = true,
) {
    val colors = ResortTheme.colors
    val feedback = ResortTheme.feedback
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val sink by animateFloatAsState(if (pressed && enabled) 1f else 0f, tween(Motion.press))
    var tip by remember { mutableStateOf(false) }
    LaunchedEffect(tip) { if (tip) { kotlinx.coroutines.delay(1_500); tip = false } }

    val face = when {
        !enabled -> if (style == IconButtonStyle.Plain) Color.Transparent else colors.surfaceRaised
        active -> colors.accent
        style == IconButtonStyle.Plain -> Color.Transparent
        style == IconButtonStyle.Raised -> if (pressed) colors.surfaceHigh else colors.surfaceRaised
        style == IconButtonStyle.Primary -> if (pressed) colors.primaryPressed else colors.primary
        style == IconButtonStyle.Success -> colors.success
        else -> if (pressed) colors.dangerPressed else colors.danger
    }
    val lip = when {
        !enabled || style == IconButtonStyle.Plain && !active -> Color.Transparent
        active -> colors.accentLip
        style == IconButtonStyle.Raised -> colors.outline
        style == IconButtonStyle.Primary -> colors.primaryLip
        style == IconButtonStyle.Success -> colors.successLip
        else -> colors.dangerLip
    }
    val tint = when {
        !enabled -> colors.textDisabled
        active || style == IconButtonStyle.Primary || style == IconButtonStyle.Success || style == IconButtonStyle.Danger -> colors.onAccent
        else -> colors.text
    }
    val shape = RoundedCornerShape(Radius.md)
    val lipDp = if (lip == Color.Transparent) 0.dp else DsSize.lip

    Column(
        modifier
            .defaultMinSize(minWidth = DsSize.touchMin, minHeight = DsSize.touchMin)
            .semantics(mergeDescendants = true) {
                this.contentDescription = contentDescription
                if (style != IconButtonStyle.Plain || active) stateDescription = if (active) "ligado" else "desligado"
            }
            .combinedClickable(
                interaction, indication = null, enabled = enabled, role = Role.Button,
                onLongClick = { tip = true },
            ) {
                feedback.haptic(Haptic.Tap)
                onClick()
            },
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Box(
            Modifier
                .size(size, size + lipDp)
                .drawBehind {
                    if (lipDp > 0.dp) {
                        drawRoundRect(lip, Offset(0f, lipDp.toPx()), androidx.compose.ui.geometry.Size(this.size.width, this.size.height - lipDp.toPx()), CornerRadius(Radius.md.toPx()))
                    }
                },
        ) {
            Box(
                Modifier
                    .size(size)
                    .graphicsLayer { translationY = sink * (DsSize.lip - DsSize.lipPressed).value * density * (if (lipDp > 0.dp) 1f else 0f) }
                    .background(face, shape),
                contentAlignment = Alignment.Center,
            ) {
                Icon(icon, null, tint = tint, modifier = Modifier.size(if (size >= DsSize.hudButton) DsSize.iconHud else DsSize.icon))
            }
            if (badge != null) Badge(badge, Modifier.align(Alignment.TopEnd))
            if (tip) {
                Popup(alignment = Alignment.TopCenter, offset = IntOffset(0, -120), properties = PopupProperties(focusable = false)) {
                    Text(
                        contentDescription, style = ResortTheme.type.caption, color = colors.text,
                        modifier = Modifier.background(colors.surfaceHigh, RoundedCornerShape(Radius.sm)).padding(horizontal = Space.sm, vertical = Space.xs),
                    )
                }
            }
        }
        if (label != null) Text(label, style = ResortTheme.type.caption, color = colors.textMuted, maxLines = 1)
    }
}

// ---- Previews ----

@Preview(name = "GameIconButton", widthDp = 360, heightDp = 200)
@Composable
private fun GameIconButtonPreview() = DsPreviewScene {
    Column(Modifier.padding(Space.lg), verticalArrangement = Arrangement.spacedBy(Space.md)) {
        Row(horizontalArrangement = Arrangement.spacedBy(Space.md)) {
            GameIconButton(GameIcons.Close, "Fechar", {})
            GameIconButton(GameIcons.Backpack, "Itens", {}, style = IconButtonStyle.Raised, badge = BadgeSpec.Count(3))
            GameIconButton(GameIcons.Shop, "Loja", {}, style = IconButtonStyle.Primary, size = DsSize.hudButton)
            GameIconButton(GameIcons.Hammer, "Construir", {}, style = IconButtonStyle.Raised, active = true)
            GameIconButton(GameIcons.Gear, "Ajustes", {}, style = IconButtonStyle.Raised, enabled = false)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(Space.md)) {
            GameIconButton(GameIcons.Compass, "Salas", {}, style = IconButtonStyle.Raised, label = "Salas")
            GameIconButton(GameIcons.Check, "Concluir", {}, style = IconButtonStyle.Success, label = "Concluir")
        }
    }
}
