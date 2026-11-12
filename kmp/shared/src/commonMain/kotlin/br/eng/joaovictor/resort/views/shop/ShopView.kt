package br.eng.joaovictor.resort.views.shop

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.components.GameThumbnail
import br.eng.joaovictor.resort.ds.components.ButtonSize
import br.eng.joaovictor.resort.ds.components.CategoryChips
import br.eng.joaovictor.resort.ds.components.CategoryRail
import br.eng.joaovictor.resort.ds.components.ChipItem
import br.eng.joaovictor.resort.ds.components.Currency
import br.eng.joaovictor.resort.ds.components.CurrencyPill
import br.eng.joaovictor.resort.ds.components.EmptyState
import br.eng.joaovictor.resort.ds.components.GameButton
import br.eng.joaovictor.resort.ds.components.GameIconButton
import br.eng.joaovictor.resort.ds.components.IconButtonStyle
import br.eng.joaovictor.resort.ds.components.LocalToastHost
import br.eng.joaovictor.resort.ds.components.LockReason
import br.eng.joaovictor.resort.ds.components.ToastKind
import br.eng.joaovictor.resort.models.StatusLine
import br.eng.joaovictor.resort.ds.components.OfferTile
import br.eng.joaovictor.resort.ds.components.Price
import br.eng.joaovictor.resort.ds.components.PriceTag
import br.eng.joaovictor.resort.ds.components.RailItem
import br.eng.joaovictor.resort.ds.components.SearchField
import br.eng.joaovictor.resort.ds.components.SkeletonGrid
import br.eng.joaovictor.resort.ds.components.TileState
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.layout.AdaptivePanel
import br.eng.joaovictor.resort.ds.layout.PanelForm
import br.eng.joaovictor.resort.ds.layout.PanelSpec
import br.eng.joaovictor.resort.ds.layout.panelSpecFor
import br.eng.joaovictor.resort.ds.layout.rememberPanelSpec
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize
import br.eng.joaovictor.resort.views.hud.CompactHudWhileVisible
import br.eng.joaovictor.resort.views.inventory.toggleSearch
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene

/** The HUD owner name the shop holds while open (see `HudVisibility`); the HUD shrinks to Compact. */
internal const val SHOP_HUD_OWNER = "shop"

/** What the shop can ask for; previews pass no-ops. */
class ShopActions(
    val onClose: () -> Unit = {},
    val onCategory: (String) -> Unit = {},
    val onQuery: (String) -> Unit = {},
    val onSelect: (String) -> Unit = {},
    val onBuy: () -> Unit = {},
)

/** The top-level category that holds the selection (itself or the parent of the selected sub-category). */
fun shopSection(ui: ShopUiState): String? =
    ui.subCategories.firstOrNull { it.id == ui.selectedCategory }?.parent ?: ui.selectedCategory

/** Rail entries: the top-level categories; the one holding the selection is expanded with its sub-categories. */
fun shopRail(ui: ShopUiState): List<RailItem> {
    val section = shopSection(ui)
    return ui.topCategories.map { top ->
        val open = top.id == section && ui.subCategories.isNotEmpty()
        RailItem(top.id, top.name, if (open) ui.subCategories.map { RailItem(it.id, it.name) } else emptyList(), expanded = open)
    }
}

fun offerPrice(tile: ShopTile) = Price(tile.priceSilver, tile.priceGold)

/** A tile the wallet cannot cover is locked (price in red, padlock) but stays tappable to show the reason. */
fun offerTileState(tile: ShopTile): TileState =
    TileState(selected = tile.selected, locked = if (tile.affordable) null else LockReason.CannotAfford)

/** A purchase result is a toast: success in green, anything else as an error. */
fun purchaseToastKind(status: StatusLine): ToastKind = if (status.kind == "success") ToastKind.Success else ToastKind.Error

/** Label of the buy button. */
fun buyLabel(ui: ShopUiState): String =
    if (ui.selected != null && !ui.selected.affordable) "Saldo insuficiente" else "Comprar"

/** Collects the state and shrinks the HUD to Compact while the shop is open. */
@Composable
fun ShopView(viewModel: ShopViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.open) return
    CompactHudWhileVisible(SHOP_HUD_OWNER)
    val toasts = LocalToastHost.current
    LaunchedEffect(ui.status) { ui.status?.let { toasts?.post(purchaseToastKind(it), it.text) } }
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

/**
 * Stateless shop in the adaptive panel: the balance and search in the header, categories (rail in landscape,
 * chips in portrait), the offers grid and a sticky buy bar.
 */
@Composable
fun ShopContent(
    ui: ShopUiState,
    actions: ShopActions,
    searchOpen: Boolean = ui.query.isNotEmpty(),
    onToggleSearch: () -> Unit = {},
    spec: PanelSpec = rememberPanelSpec(),
) {
    val keyboard = LocalSoftwareKeyboardController.current
    LaunchedEffect(searchOpen) { if (!searchOpen) keyboard?.hide() }
    AdaptivePanel(
        visible = true,
        title = "Loja",
        onClose = actions.onClose,
        icon = GameIcons.Shop,
        headerActions = {
            CurrencyPill(Currency.Silver, if (ui.walletLoaded) ui.silver else null, compact = true)
            CurrencyPill(Currency.Gold, if (ui.walletLoaded) ui.gold else null, compact = true)
            GameIconButton(GameIcons.Search, "Buscar", onToggleSearch, style = IconButtonStyle.Raised, active = searchOpen)
        },
        footer = { BuyBar(ui, actions.onBuy) },
        spec = spec,
        panelKey = "shop",
    ) {
        if (searchOpen) {
            SearchField(
                ui.query, actions.onQuery, "Buscar",
                Modifier.fillMaxWidth().padding(horizontal = Space.md, vertical = Space.xs),
                autoFocus = true,
            )
        }
        if (spec.form == PanelForm.Side) {
            Row(Modifier.weight(1f)) {
                CategoryRail(shopRail(ui), ui.selectedCategory, actions.onCategory)
                Offers(ui, actions, Modifier.weight(1f))
            }
        } else {
            CategoryChips(ui.topCategories.map { ChipItem(it.id, it.name) }, shopSection(ui), actions.onCategory, Modifier.padding(horizontal = Space.md))
            if (ui.subCategories.isNotEmpty()) {
                CategoryChips(ui.subCategories.map { ChipItem(it.id, it.name) }, ui.selectedCategory, actions.onCategory, Modifier.padding(horizontal = Space.md))
            }
            Offers(ui, actions, Modifier.weight(1f))
        }
    }
}

@Composable
private fun Offers(ui: ShopUiState, actions: ShopActions, modifier: Modifier) {
    Box(modifier.fillMaxWidth()) {
        when {
            ui.loading -> SkeletonGrid(minSize = 96.dp, tileAspect = 0.78f)
            ui.isEmpty -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                if (ui.query.isNotBlank()) {
                    EmptyState(GameIcons.Search, "Nenhum item encontrado", action = "Limpar busca" to { actions.onQuery("") })
                } else {
                    EmptyState(GameIcons.Shop, "Nada à venda aqui")
                }
            }
            else -> LazyVerticalGrid(
                columns = GridCells.Adaptive(minSize = 96.dp),
                modifier = Modifier.padding(horizontal = Space.md),
                horizontalArrangement = Arrangement.spacedBy(Space.sm),
                verticalArrangement = Arrangement.spacedBy(Space.sm),
            ) {
                items(ui.tiles, key = { it.catalogId }, contentType = { "offer" }) { tile ->
                    OfferTile(
                        tile.name, offerPrice(tile), offerTileState(tile), { actions.onSelect(tile.catalogId) },
                        thumbnail = { m -> GameThumbnail(tile.thumbnail, tile.name, m) },
                    )
                }
            }
        }
    }
}

/** The sticky bottom bar: purchase status, the selected offer and its price, and the buy button. */
@Composable
private fun BuyBar(ui: ShopUiState, onBuy: () -> Unit) {
    val colors = ResortTheme.colors
    Column(verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Space.md)) {
            Column(Modifier.weight(1f)) {
                val sel = ui.selected
                if (sel == null) {
                    Text("Escolha um item", style = ResortTheme.type.body, color = colors.textMuted)
                } else {
                    Text(sel.name, style = ResortTheme.type.titleSmall, color = colors.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    PriceTag(offerPrice(sel), dim = !sel.affordable)
                }
            }
            GameButton(buyLabel(ui), onBuy, size = ButtonSize.Large, enabled = ui.canBuy || ui.buying, loading = ui.buying)
        }
    }
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

private val LandscapeSpec = panelSpecFor(800.dp, 360.dp)
private val PortraitSpec = panelSpecFor(360.dp, 780.dp)

@Preview(name = "Shop, offer selected (landscape)", widthDp = 800, heightDp = 360)
@Composable
private fun ShopSelectedPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded, ShopActions(), spec = LandscapeSpec) }

@Preview(name = "Shop, cannot afford", widthDp = 800, heightDp = 360)
@Composable
private fun ShopCannotAffordPreview() = PreviewScene { ShopContent(PreviewData.shopPoor, ShopActions(), spec = LandscapeSpec) }

@Preview(name = "Shop, buying (pending)", widthDp = 800, heightDp = 360)
@Composable
private fun ShopBuyingPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded.copy(buying = true), ShopActions(), spec = LandscapeSpec) }

@Preview(name = "Shop, loading", widthDp = 800, heightDp = 360)
@Composable
private fun ShopLoadingPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded.copy(loading = true, tiles = emptyList(), selected = null), ShopActions(), spec = LandscapeSpec) }

@Preview(name = "Shop, empty", widthDp = 800, heightDp = 360)
@Composable
private fun ShopEmptyPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded.copy(tiles = emptyList(), selected = null), ShopActions(), spec = LandscapeSpec) }

@Preview(name = "Shop, search open, no result", widthDp = 800, heightDp = 360)
@Composable
private fun ShopSearchPreview() = PreviewScene {
    ShopContent(PreviewData.shopLoaded.copy(query = "zzz", tiles = emptyList(), selected = null), ShopActions(), spec = LandscapeSpec, searchOpen = true)
}

@Preview(name = "Shop (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun ShopPortraitPreview() = PreviewScene { ShopContent(PreviewData.shopLoaded, ShopActions(), spec = PortraitSpec) }
