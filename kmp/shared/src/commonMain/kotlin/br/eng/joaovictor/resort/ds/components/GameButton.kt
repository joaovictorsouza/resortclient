package br.eng.joaovictor.resort.ds.components

import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Haptic
import br.eng.joaovictor.resort.ds.theme.Motion
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortColors
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

enum class ButtonVariant { Primary, Secondary, Success, Danger, Ghost }

/** Drawn height; the touch area is always at least 48dp. */
enum class ButtonSize(val height: Dp) {
    Large(DsSize.buttonHeight),
    Medium(DsSize.buttonHeightMedium),
    Small(DsSize.buttonHeightSmall),
}

/** Face, pressed face, lip and content colors of one button variant. */
internal data class ButtonColors(val face: Color, val pressed: Color, val lip: Color, val content: Color)

internal fun buttonColors(variant: ButtonVariant, c: ResortColors, enabled: Boolean): ButtonColors {
    if (!enabled) return ButtonColors(c.surfaceRaised, c.surfaceRaised, Color.Transparent, c.textDisabled)
    return when (variant) {
        ButtonVariant.Primary -> ButtonColors(c.primary, c.primaryPressed, c.primaryLip, c.onAccent)
        ButtonVariant.Secondary -> ButtonColors(c.secondary, c.secondaryPressed, c.secondaryLip, c.onAccent)
        ButtonVariant.Success -> ButtonColors(c.success, c.success, c.successLip, c.onAccent)
        ButtonVariant.Danger -> ButtonColors(c.danger, c.dangerPressed, c.dangerLip, c.onAccent)
        ButtonVariant.Ghost -> ButtonColors(Color.Transparent, Color.Transparent, Color.Transparent, c.secondary)
    }
}

/**
 * The game button: a colored face on a 4dp lip that sinks when pressed. Dark text on every colored face
 * (white on coral fails contrast). [Ghost][ButtonVariant.Ghost] has no face and no lip.
 */
@Composable
fun GameButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    variant: ButtonVariant = ButtonVariant.Primary,
    size: ButtonSize = ButtonSize.Medium,
    icon: ImageVector? = null,
    enabled: Boolean = true,
    loading: Boolean = false,
    haptic: Haptic = Haptic.Tap,
) {
    val colors = ResortTheme.colors
    val feedback = ResortTheme.feedback
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val focused by interaction.collectIsFocusedAsState()
    val palette = buttonColors(variant, colors, enabled)
    val ghost = variant == ButtonVariant.Ghost
    val clickable = enabled && !loading
    val sinkPx by animateFloatAsState(if (pressed && clickable && !ghost) 1f else 0f, tween(Motion.press))
    val shape = if (size == ButtonSize.Small) RoundedCornerShape(Radius.sm) else RoundedCornerShape(Radius.md)
    val lipDp = if (ghost || !enabled) 0.dp else DsSize.lip

    Box(
        modifier
            .defaultMinSize(minHeight = DsSize.touchMin)
            .semantics { if (!clickable) disabled() }
            .clickable(interaction, indication = null, enabled = clickable, role = Role.Button) {
                feedback.haptic(haptic)
                onClick()
            },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            Modifier
                .height(size.height + lipDp)
                .drawBehind {
                    if (lipDp > 0.dp) {
                        val r = CornerRadius(Radius.md.toPx())
                        drawRoundRect(palette.lip, Offset(0f, lipDp.toPx()), Size(this.size.width, this.size.height - lipDp.toPx()), r)
                    }
                },
        ) {
            val sink = (DsSize.lip - DsSize.lipPressed).value
            Box(
                Modifier
                    .height(size.height)
                    .graphicsLayer { translationY = sinkPx * sink * density }
                    .background(if (pressed && clickable) palette.pressed else palette.face, shape)
                    .then(if (focused) Modifier.border(BorderStroke(2.dp, colors.focusRing), shape) else Modifier)
                    .drawBehind {
                        if (!ghost && enabled) {
                            val r = Radius.md.toPx()
                            drawLine(colors.highlight, Offset(r, 1f), Offset(this.size.width - r, 1f), 1.dp.toPx())
                        }
                    }
                    .padding(horizontal = if (size == ButtonSize.Small) Space.md else Space.xl),
                contentAlignment = Alignment.Center,
            ) {
                Row(
                    Modifier.alpha(if (loading) 0f else 1f),
                    horizontalArrangement = Arrangement.spacedBy(Space.sm),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    if (icon != null) Icon(icon, null, tint = palette.content, modifier = Modifier.size(DsSize.iconSmall))
                    Text(text, style = ResortTheme.type.label, color = palette.content, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                if (loading) LoadingDots(palette.content)
            }
        }
    }
}

@Composable
private fun LoadingDots(color: Color) {
    val transition = androidx.compose.animation.core.rememberInfiniteTransition()
    Row(horizontalArrangement = Arrangement.spacedBy(Space.xs)) {
        repeat(3) { i ->
            val a by transition.animateFloat(
                0.3f, 1f,
                androidx.compose.animation.core.infiniteRepeatable(
                    tween(480, delayMillis = i * 120), androidx.compose.animation.core.RepeatMode.Reverse,
                ),
            )
            Box(Modifier.size(Space.sm).alpha(a).background(color, CircleShape))
        }
    }
}

// ---- Previews ----

@Preview(name = "GameButton states", widthDp = 360, heightDp = 640)
@Composable
private fun GameButtonPreview() = DsPreviewScene {
    Column(Modifier.padding(Space.lg), verticalArrangement = Arrangement.spacedBy(Space.md)) {
        ButtonVariant.entries.forEach { v ->
            Row(horizontalArrangement = Arrangement.spacedBy(Space.md), verticalAlignment = Alignment.CenterVertically) {
                GameButton(v.name, {}, variant = v)
                GameButton("Pequeno", {}, variant = v, size = ButtonSize.Small, icon = GameIcons.Check)
            }
        }
        GameButton("Desativado", {}, enabled = false)
        GameButton("Carregando", {}, loading = true)
        GameButton("Comprar", {}, size = ButtonSize.Large, icon = GameIcons.Shop)
    }
}
