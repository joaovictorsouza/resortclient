package br.eng.joaovictor.resort.ds.components

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.format.formatGroupedAmount
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Haptic
import br.eng.joaovictor.resort.ds.theme.Motion
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortColors
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

enum class Rarity { Common, Rare, Epic, Legendary }

enum class LockReason { CannotAfford, NotOwner, Owned }

@Immutable
data class TileState(
    val selected: Boolean = false,
    val locked: LockReason? = null,
    val isNew: Boolean = false,
    /** `null` = no rarity frame. The data does not exist yet; the visual is ready. */
    val rarity: Rarity? = null,
    /** "x3" is shown when above 1. */
    val quantity: Int? = null,
)

@Immutable
data class Price(val silver: Long, val gold: Long) {
    val isFree: Boolean get() = silver == 0L && gold == 0L
}

internal fun rarityColor(rarity: Rarity, c: ResortColors): Color = when (rarity) {
    Rarity.Common -> c.rarityCommon
    Rarity.Rare -> c.rarityRare
    Rarity.Epic -> c.rarityEpic
    Rarity.Legendary -> c.rarityLegendary
}

/** Spoken description of a tile ("Poltrona, 450 pratas, bloqueado: saldo insuficiente"); pure, so it is tested. */
fun tileDescription(name: String, price: Price?, state: TileState): String = buildString {
    append(name)
    if (price != null) {
        append(", ")
        append(
            when {
                price.isFree -> "grátis"
                else -> listOfNotNull(
                    price.silver.takeIf { it > 0 }?.let { "${formatGroupedAmount(it)} pratas" },
                    price.gold.takeIf { it > 0 }?.let { "${formatGroupedAmount(it)} ouros" },
                ).joinToString(" e ")
            },
        )
    }
    state.quantity?.takeIf { it > 1 }?.let { append(", quantidade $it") }
    if (state.isNew) append(", novo")
    when (state.locked) {
        LockReason.CannotAfford -> append(", bloqueado: saldo insuficiente")
        LockReason.NotOwner -> append(", bloqueado: só o dono da sala")
        LockReason.Owned -> append(", você já tem")
        null -> {}
    }
}

/** An inventory tile. [dragModifier] carries the long-press drag the screen owns. */
@Composable
fun ItemTile(
    name: String,
    state: TileState,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    dragModifier: Modifier = Modifier,
    thumbnail: @Composable (Modifier) -> Unit,
) = TileBase(name, null, state, onClick, modifier, dragModifier, thumbnail, 0.9f)

/** A shop tile: like [ItemTile] with the price under the name. Stays tappable when locked, to show the reason. */
@Composable
fun OfferTile(
    name: String,
    price: Price,
    state: TileState,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    thumbnail: @Composable (Modifier) -> Unit,
) = TileBase(name, price, state, onClick, modifier, Modifier, thumbnail, 0.78f)

@Composable
private fun TileBase(
    name: String,
    price: Price?,
    state: TileState,
    onClick: () -> Unit,
    modifier: Modifier,
    dragModifier: Modifier,
    thumbnail: @Composable (Modifier) -> Unit,
    aspect: Float,
) {
    val colors = ResortTheme.colors
    val feedback = ResortTheme.feedback
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val scale by animateFloatAsState(if (pressed) 0.96f else 1f, tween(Motion.press))
    val shape = RoundedCornerShape(Radius.md)
    val rarity = state.rarity?.let { rarityColor(it, colors) }
    val border = when {
        state.selected -> colors.secondary to 2.5.dp
        rarity != null -> rarity to 2.dp
        else -> colors.outline to 1.dp
    }
    val locked = state.locked != null
    val description = tileDescription(name, price, state)

    Box(
        modifier.fillMaxWidth().aspectRatio(aspect)
            .graphicsLayer { scaleX = scale; scaleY = scale }
            .semantics(mergeDescendants = true) {
                contentDescription = description
                selected = state.selected
                if (locked) stateDescription = "bloqueado"
            }
            .background(if (pressed) colors.surfaceHigh else colors.surfaceRaised, shape)
            .border(border.second, border.first, shape)
            .clickable(interaction, indication = null, role = Role.Button) { feedback.haptic(Haptic.Select); onClick() }
            .then(dragModifier)
            .padding(Space.xs + Space.xxs),
    ) {
        Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(Space.xxs)) {
            Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                // Pedestal under the thumbnail.
                Box(Modifier.fillMaxWidth(0.7f).aspectRatio(3f).align(Alignment.BottomCenter).background(colors.surfaceSunken, RoundedCornerShape(50)))
                thumbnail(Modifier.fillMaxSize().alpha(if (locked) 0.5f else 1f))
            }
            Text(
                name, style = ResortTheme.type.caption, color = colors.text, maxLines = 2, overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth(),
            )
            if (price != null) PriceTag(price, dim = state.locked == LockReason.CannotAfford)
        }
        if (state.isNew) Badge(BadgeSpec.Label("Novo"), Modifier.align(Alignment.TopStart), color = colors.accent)
        state.quantity?.takeIf { it > 1 }?.let {
            Badge(BadgeSpec.Label("x$it"), Modifier.align(Alignment.TopEnd), color = colors.surfaceHigh)
        }
        Column(Modifier.align(Alignment.BottomEnd), horizontalAlignment = Alignment.End) {
            if (state.selected) Icon(GameIcons.Check, null, tint = colors.secondary, modifier = Modifier.size(DsSize.iconSmall))
            if (locked) Icon(GameIcons.Lock, null, tint = colors.textMuted, modifier = Modifier.size(DsSize.iconSmall))
        }
        if (state.rarity != null && state.quantity.let { it == null || it <= 1 }) {
            Icon(GameIcons.Star, null, tint = rarity ?: colors.text, modifier = Modifier.align(Alignment.TopEnd).size(DsSize.iconSmall))
        }
    }
}

/** Silver and gold prices as coin + amount; zero amounts are left out and no price reads "Grátis". */
@Composable
fun PriceTag(price: Price, modifier: Modifier = Modifier, dim: Boolean = false) {
    val colors = ResortTheme.colors
    val textColor = if (dim) colors.dangerText else colors.text
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
        if (price.silver > 0) Amount(colors.silver, colors.silverLip, price.silver, textColor)
        if (price.gold > 0) Amount(colors.gold, colors.goldLip, price.gold, textColor)
        if (price.isFree) Text("Grátis", style = ResortTheme.type.caption, color = textColor)
    }
}

@Composable
private fun Amount(face: Color, lip: Color, amount: Long, textColor: Color) {
    Row(horizontalArrangement = Arrangement.spacedBy(Space.xxs + Space.xxs), verticalAlignment = Alignment.CenterVertically) {
        Canvas(Modifier.size(12.dp)) {
            drawCircle(lip, radius = size.minDimension / 2)
            drawCircle(face, radius = size.minDimension / 2 - 1.dp.toPx())
        }
        Text(formatGroupedAmount(amount), style = ResortTheme.type.caption, color = textColor, maxLines = 1)
    }
}

// ---- Previews ----

@Preview(name = "Tiles", widthDp = 360, heightDp = 420)
@Composable
private fun TilesPreview() = DsPreviewScene {
    val thumb: @Composable (Modifier) -> Unit = { Box(it.background(ResortTheme.colors.surfaceHigh, RoundedCornerShape(Radius.sm))) }
    LazyVerticalGrid(
        GridCells.Fixed(3), Modifier.padding(Space.md),
        horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        val items: List<Pair<String, TileState>> = listOf(
            "Normal" to TileState(),
            "Selecionado" to TileState(selected = true),
            "Quantidade" to TileState(quantity = 3),
            "Bloqueado" to TileState(locked = LockReason.NotOwner),
            "Novo" to TileState(isNew = true),
            "Raro com nome comprido" to TileState(rarity = Rarity.Rare),
        )
        items.forEach { (n, s) -> item { ItemTile(n, s, {}, thumbnail = thumb) } }
        item { OfferTile("Sofá", Price(450, 0), TileState(), {}, thumbnail = thumb) }
        item { OfferTile("Troféu", Price(0, 12), TileState(locked = LockReason.CannotAfford, rarity = Rarity.Legendary), {}, thumbnail = thumb) }
        item { OfferTile("Brinde", Price(0, 0), TileState(isNew = true, selected = true), {}, thumbnail = thumb) }
    }
}
