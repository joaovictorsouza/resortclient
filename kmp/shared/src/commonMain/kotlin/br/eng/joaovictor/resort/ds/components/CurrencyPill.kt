package br.eng.joaovictor.resort.ds.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.format.formatGroupedAmount
import br.eng.joaovictor.resort.ds.theme.LocalReduceMotion
import br.eng.joaovictor.resort.ds.theme.Motion
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

enum class Currency(val label: String) { Silver("pratas"), Gold("ouros") }

/**
 * The amount to celebrate with a "+n" and a pop: only a rise, and nothing on the first load (previous
 * unknown) or while the balance is unknown. A fall (a purchase) just counts down.
 */
fun currencyDelta(previous: Long?, current: Long?): Long? {
    if (previous == null || current == null) return null
    val delta = current - previous
    return if (delta > 0) delta else null
}

/** A coin and its balance; `amount == null` (not loaded yet) shows a skeleton instead of a dash. */
@Composable
fun CurrencyPill(
    currency: Currency,
    amount: Long?,
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
    compact: Boolean = false,
) {
    val colors = ResortTheme.colors
    val reduce = LocalReduceMotion.current
    var shown by remember { mutableStateOf(amount) }
    var previous by remember { mutableStateOf(amount) }
    var gain by remember { mutableStateOf<Long?>(null) }
    val pop = remember { Animatable(1f) }
    val gainAnim = remember { Animatable(0f) }

    LaunchedEffect(amount) {
        val before = previous
        previous = amount
        if (amount == null || before == null || before == amount || reduce) {
            shown = amount
            return@LaunchedEffect
        }
        val delta = currencyDelta(before, amount)
        if (delta != null) {
            gain = delta
            gainAnim.snapTo(0f)
        }
        // The two animations below run one after the other only for rises; a fall just counts down.
        animate(before.toFloat(), amount.toFloat(), animationSpec = tween(Motion.slow)) { v, _ -> shown = v.toLong() }
        shown = amount
        if (delta != null) {
            pop.snapTo(1f)
            pop.animateTo(1.25f, tween(60))
            pop.animateTo(1f, Motion.popSpring)
            gainAnim.animateTo(1f, tween(Motion.slow * 2))
            gain = null
        }
    }

    val clickMod = if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier
    val coin = if (currency == Currency.Silver) colors.silver to colors.silverLip else colors.gold to colors.goldLip
    val spoken = if (amount == null) "${currency.label}: carregando" else "${formatGroupedAmount(amount)} ${currency.label}"

    Box(modifier.then(clickMod).defaultMinSize(minHeight = DsSize.touchMin).semantics(mergeDescendants = true) { contentDescription = spoken }, contentAlignment = Alignment.Center) {
        Row(
            Modifier
                .height(36.dp)
                .background(colors.surface, Radius.pill)
                .padding(start = Space.xs + Space.xxs, end = if (compact) Space.sm else Space.md),
            horizontalArrangement = Arrangement.spacedBy(Space.xs + Space.xxs),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Canvas(Modifier.size(24.dp).graphicsLayer { scaleX = pop.value; scaleY = pop.value }) {
                drawCircle(coin.second, radius = size.minDimension / 2)
                drawCircle(coin.first, radius = size.minDimension / 2 - 2.dp.toPx(), center = center - Offset(0f, 1.dp.toPx()))
                drawCircle(Color.White.copy(alpha = 0.35f), radius = size.minDimension / 6, center = center - Offset(size.minDimension / 5, size.minDimension / 4 + 1.dp.toPx()))
            }
            val value = shown
            if (value == null) {
                Box(Modifier.width(40.dp).height(14.dp).background(colors.surfaceRaised, Radius.pill).alpha(0.6f))
            } else {
                Text(formatGroupedAmount(value), style = ResortTheme.type.number, color = colors.text, maxLines = 1)
            }
        }
        val g = gain
        if (g != null) {
            Text(
                "+${formatGroupedAmount(g)}", style = ResortTheme.type.caption, color = colors.accent,
                modifier = Modifier.align(Alignment.TopEnd).graphicsLayer { translationY = -16.dp.toPx() * gainAnim.value; alpha = 1f - gainAnim.value },
            )
        }
    }
}

// ---- Previews ----

@Preview(name = "CurrencyPill", widthDp = 360, heightDp = 160)
@Composable
private fun CurrencyPillPreview() = DsPreviewScene {
    Column(Modifier.padding(Space.lg), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
        Row(horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
            CurrencyPill(Currency.Silver, 1_234_567)
            CurrencyPill(Currency.Gold, 56)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
            CurrencyPill(Currency.Silver, null)
            CurrencyPill(Currency.Gold, 0, compact = true)
        }
    }
}
