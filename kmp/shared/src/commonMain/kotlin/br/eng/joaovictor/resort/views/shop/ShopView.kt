package br.eng.joaovictor.resort.views.shop

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import br.eng.joaovictor.resort.components.GameThumbnail
import br.eng.joaovictor.resort.components.GameWindow
import br.eng.joaovictor.resort.components.SearchSection
import br.eng.joaovictor.resort.views.hud.HideHudWhileVisible
import br.eng.joaovictor.resort.views.hud.formatAmount
import br.eng.joaovictor.resort.views.inventory.toggleSearch
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene
import org.jetbrains.compose.ui.tooling.preview.Preview

/** The HUD owner name the shop holds while open (see `HudVisibility`). */
internal const val SHOP_HUD_OWNER = "shop"

private val Selected = Color(0xFF4F46E5)
private val Buy = Color(0xFF059669)
private val SilverDot = Color(0xFFC0C6D0)
private val GoldDot = Color(0xFFFFC107)

/** What the shop can ask for; previews pass no-ops. */
class ShopActions(
    val onClose: () -> Unit = {},
    val onCategory: (String) -> Unit = {},
    val onQuery: (String) -> Unit = {},
    val onSelect: (String) -> Unit = {},
    val onBuy: () -> Unit = {},
)

/** Collects the state and holds the HUD hidden while the shop is open. */
@Composable
fun ShopView(viewModel: ShopViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.open) return
    HideHudWhileVisible(SHOP_HUD_OWNER)
    val actions = remember(viewModel) {
        ShopActions(viewModel::close, viewModel::selectCategory, viewModel::setQuery, viewModel::selectOffer, viewModel::buy)
    }
    var searchOpen by rememberSaveable { mutableStateOf(ui.query.isNotEmpty()) }
    ShopContent(
        ui,
        actions,
        searchOpen = searchOpen,
        onToggleSearch = {
            val next = toggleSearch(searchOpen, ui.query)
            searchOpen = next.open
            if (next.query != ui.query) actions.onQuery(next.query)
        },
    )
}

/** Stateless shop window: categories, an offers grid and a buy bar for the selected offer. */
@Composable
fun ShopContent(
    ui: ShopUiState,
    actions: ShopActions,
    searchOpen: Boolean = ui.query.isNotEmpty(),
    onToggleSearch: () -> Unit = {},
) {
    GameWindow(
        title = "Loja",
        onClose = actions.onClose,
        maxWidth = 760.dp,
        widthFraction = 0.85f,
        searchOpen = searchOpen,
        onToggleSearch = onToggleSearch,
    ) {
        Column(Modifier.padding(horizontal = 12.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            SearchSection(open = searchOpen, query = ui.query, onQuery = actions.onQuery)
            // A search looks at every category, so the category chips only apply while it is blank.
            CategoryRow(ui.topCategories.map { it.id to it.name }, ui.selectedCategory, actions.onCategory)
            if (ui.subCategories.isNotEmpty()) {
                CategoryRow(ui.subCategories.map { it.id to it.name }, ui.selectedCategory, actions.onCategory)
            }
        }
        Box(Modifier.weight(1f).fillMaxWidth()) {
            when {
                ui.loading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
                ui.isEmpty -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(if (ui.query.isNotBlank()) "Nenhum item encontrado" else "Nada à venda aqui")
                }
                else -> LazyVerticalGrid(
                    columns = GridCells.Adaptive(minSize = 96.dp),
                    modifier = Modifier.padding(horizontal = 12.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    items(ui.tiles, key = { it.catalogId }) { tile -> OfferTile(tile) { actions.onSelect(tile.catalogId) } }
                }
            }
        }
        BuyBar(ui, actions.onBuy)
    }
}

@Composable
private fun CategoryRow(entries: List<Pair<String, String>>, selected: String?, onPick: (String) -> Unit) {
    LazyRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        items(entries, key = { it.first }) { (id, name) ->
            val on = id == selected
            Text(
                name,
                color = Color.White,
                fontSize = 13.sp,
                modifier = Modifier
                    .background(if (on) Selected else Color.White.copy(alpha = 0.12f), RoundedCornerShape(14.dp))
                    .clickable { onPick(id) }
                    .padding(horizontal = 10.dp, vertical = 5.dp),
            )
        }
    }
}

@Composable
private fun OfferTile(tile: ShopTile, onClick: () -> Unit) {
    val shape = RoundedCornerShape(10.dp)
    Box(
        Modifier.fillMaxWidth().aspectRatio(1f).clip(shape)
            .background(if (tile.selected) Selected.copy(alpha = 0.35f) else Color.White.copy(alpha = 0.06f))
            .border(if (tile.selected) 2.dp else 1.dp, if (tile.selected) Selected else Color.White.copy(alpha = 0.12f), shape)
            .clickable(onClick = onClick),
    ) {
        GameThumbnail(tile.thumbnail, tile.name, Modifier.fillMaxSize().padding(bottom = 34.dp))
        Column(
            Modifier.align(Alignment.BottomCenter).fillMaxWidth().background(Color.Black.copy(alpha = 0.55f)).padding(horizontal = 4.dp, vertical = 2.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(tile.name, color = Color.White, fontSize = 11.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
            PriceChips(tile.priceSilver, tile.priceGold, dim = !tile.affordable)
        }
    }
}

/** Silver and gold prices as small dot + amount chips; a price of zero is left out. */
@Composable
private fun PriceChips(silver: Long, gold: Long, dim: Boolean = false) {
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
        if (silver > 0) PriceChip(SilverDot, silver, dim)
        if (gold > 0) PriceChip(GoldDot, gold, dim)
        if (silver == 0L && gold == 0L) Text("Grátis", color = Color.White, fontSize = 11.sp)
    }
}

@Composable
private fun PriceChip(dot: Color, amount: Long, dim: Boolean) {
    Row(horizontalArrangement = Arrangement.spacedBy(3.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(8.dp).background(dot, CircleShape))
        Text(formatAmount(amount), color = if (dim) Color(0xFFFB7185) else Color.White, fontSize = 11.sp)
    }
}

@Composable
private fun BuyBar(ui: ShopUiState, onBuy: () -> Unit) {
    Column(Modifier.fillMaxWidth().background(Color.White.copy(alpha = 0.06f)).padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        ui.status?.let {
            Text(it.text, color = if (it.kind == "success") Color(0xFF34D399) else Color(0xFFFB7185), fontSize = 12.sp)
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Column(Modifier.weight(1f)) {
                val sel = ui.selected
                if (sel == null) {
                    Text("Escolha um item", color = Color.White.copy(alpha = 0.7f), fontSize = 13.sp)
                } else {
                    Text(sel.name, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    PriceChips(sel.priceSilver, sel.priceGold, dim = !sel.affordable)
                }
            }
            Column(horizontalAlignment = Alignment.End) {
                Text("Seu saldo", color = Color.White.copy(alpha = 0.6f), fontSize = 11.sp)
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    PriceChip(SilverDot, if (ui.walletLoaded) ui.silver else 0, false)
                    PriceChip(GoldDot, if (ui.walletLoaded) ui.gold else 0, false)
                }
            }
            Button(onClick = onBuy, enabled = ui.canBuy) {
                Text(
                    when {
                        ui.buying -> "Comprando…"
                        ui.selected != null && !ui.selected.affordable -> "Saldo insuficiente"
                        else -> "Comprar"
                    },
                )
            }
        }
    }
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

@Preview(name = "Shop, offer selected (landscape)", widthDp = 800, heightDp = 360)
@Composable
private fun ShopSelectedPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded, ShopActions()) }

@Preview(name = "Shop, cannot afford", widthDp = 800, heightDp = 360)
@Composable
private fun ShopCannotAffordPreview() = PreviewScene { ShopContent(PreviewData.shopPoor, ShopActions()) }

@Preview(name = "Shop, buying (pending)", widthDp = 800, heightDp = 360)
@Composable
private fun ShopBuyingPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded.copy(buying = true), ShopActions()) }

@Preview(name = "Shop, success status", widthDp = 800, heightDp = 360)
@Composable
private fun ShopSuccessPreview() = PreviewScene {
    ShopContent(PreviewData.shopLoaded.copy(status = br.eng.joaovictor.resort.models.StatusLine("success", "Poltrona comprada!")), ShopActions())
}

@Preview(name = "Shop, error status", widthDp = 800, heightDp = 360)
@Composable
private fun ShopErrorPreview() = PreviewScene {
    ShopContent(PreviewData.shopLoaded.copy(status = br.eng.joaovictor.resort.models.StatusLine("error", "Saldo insuficiente.")), ShopActions())
}

@Preview(name = "Shop, loading", widthDp = 800, heightDp = 360)
@Composable
private fun ShopLoadingPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded.copy(loading = true, tiles = emptyList(), selected = null), ShopActions()) }

@Preview(name = "Shop, empty", widthDp = 800, heightDp = 360)
@Composable
private fun ShopEmptyPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded.copy(tiles = emptyList(), selected = null), ShopActions()) }

@Preview(name = "Shop, search open, no result", widthDp = 800, heightDp = 360)
@Composable
private fun ShopSearchPreview() = PreviewScene {
    ShopContent(PreviewData.shopLoaded.copy(query = "zzz", tiles = emptyList(), selected = null), ShopActions(), searchOpen = true)
}

@Preview(name = "Shop (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun ShopPortraitPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded, ShopActions()) }
