package br.eng.joaovictor.resort.ds.components

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme

/**
 * Elevation level 2: a hard 25% black rectangle offset by [offset], never `Modifier.shadow` or blur
 * (heavy on Skia/iOS and over the Bevy surface).
 */
@Composable
fun Modifier.drawSolidShadow(offset: Dp = 4.dp, corner: Dp = Radius.md): Modifier {
    val color = ResortTheme.colors.solidShadow
    return this.drawBehind {
        drawRoundRect(color, Offset(0f, offset.toPx()), size, CornerRadius(corner.toPx()))
    }
}
