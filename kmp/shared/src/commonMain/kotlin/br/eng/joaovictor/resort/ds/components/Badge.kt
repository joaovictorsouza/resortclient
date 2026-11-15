package br.eng.joaovictor.resort.ds.components

import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.theme.Motion
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space

@Immutable
sealed interface BadgeSpec {
    data object Dot : BadgeSpec
    data class Count(val n: Int) : BadgeSpec
    data class Label(val text: String) : BadgeSpec
}

/** `9+` above nine; pure so it is testable. */
fun badgeCountText(n: Int): String = if (n > 9) "9+" else n.toString()

@Composable
fun Badge(spec: BadgeSpec, modifier: Modifier = Modifier, color: Color = ResortTheme.colors.danger) {
    val scale = remember { Animatable(0.6f) }
    LaunchedEffect(Unit) { scale.animateTo(1f, Motion.popSpring) }
    val pop = Modifier.graphicsLayer { scaleX = scale.value; scaleY = scale.value }
    when (spec) {
        BadgeSpec.Dot -> androidx.compose.foundation.layout.Box(modifier.then(pop).size(10.dp).background(color, CircleShape))
        is BadgeSpec.Count -> Pill(badgeCountText(spec.n), color, modifier.then(pop))
        is BadgeSpec.Label -> Pill(spec.text, color, modifier.then(pop))
    }
}

@Composable
private fun Pill(text: String, color: Color, modifier: Modifier) {
    androidx.compose.foundation.layout.Box(
        modifier.defaultMinSize(minWidth = 18.dp, minHeight = 18.dp).background(color, Radius.pill).padding(horizontal = Space.xs + Space.xxs),
        contentAlignment = Alignment.Center,
    ) {
        Text(text.uppercase(), style = ResortTheme.type.badge, color = ResortTheme.colors.onAccent, maxLines = 1)
    }
}

// ---- Previews ----

@Preview(name = "Badge", widthDp = 240, heightDp = 80)
@Composable
private fun BadgePreview() = DsPreviewScene {
    Row(Modifier.padding(Space.lg), horizontalArrangement = Arrangement.spacedBy(Space.md), verticalAlignment = Alignment.CenterVertically) {
        Badge(BadgeSpec.Dot)
        Badge(BadgeSpec.Count(3))
        Badge(BadgeSpec.Count(42))
        Badge(BadgeSpec.Label("Novo"), color = ResortTheme.colors.accent)
    }
}
