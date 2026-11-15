package br.eng.joaovictor.resort.views.navigator

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.components.ChipItem
import br.eng.joaovictor.resort.ds.components.CategoryChips
import br.eng.joaovictor.resort.ds.components.EmptyState
import br.eng.joaovictor.resort.ds.components.GameButton
import br.eng.joaovictor.resort.ds.components.ButtonSize
import br.eng.joaovictor.resort.ds.components.GameIconButton
import br.eng.joaovictor.resort.ds.components.GameTextField
import br.eng.joaovictor.resort.ds.components.IconButtonStyle
import br.eng.joaovictor.resort.ds.components.LocalToastHost
import br.eng.joaovictor.resort.ds.components.RoomListCard
import br.eng.joaovictor.resort.ds.components.SearchField
import br.eng.joaovictor.resort.ds.components.SelectableRow
import br.eng.joaovictor.resort.ds.components.SkeletonList
import br.eng.joaovictor.resort.ds.components.ToastKind
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.layout.AdaptivePanel
import br.eng.joaovictor.resort.ds.layout.PanelSpec
import br.eng.joaovictor.resort.ds.layout.panelSpecFor
import br.eng.joaovictor.resort.ds.layout.rememberPanelSpec
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.views.hud.CompactHudWhileVisible
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene

/** The HUD owner name the navigator holds while open (see `HudVisibility`); the HUD shrinks to Compact. */
internal const val NAVIGATOR_HUD_OWNER = "navigator"

/** What the navigator can ask for; previews pass no-ops. */
class NavigatorActions(
    val onClose: () -> Unit = {},
    val onTab: (NavTab) -> Unit = {},
    val onQuery: (String) -> Unit = {},
    val onSearch: () -> Unit = {},
    val onJoin: (String) -> Unit = {},
    val onCreateName: (String) -> Unit = {},
    val onLayout: (String) -> Unit = {},
    val onCreate: () -> Unit = {},
)

/**
 * The tabs shown as chips. Search is not one of them: it is the magnifier in the header, but it stays a
 * tab in the state (the game's `navigator_tab` key), so the state and the Bevy contract do not change.
 */
val NAV_CHIP_TABS: List<NavTab> = NavTab.entries.filter { it != NavTab.Search }

/** The chip to highlight: none while searching, the tab otherwise. */
fun navChipSelection(tab: NavTab): String? = if (tab == NavTab.Search) null else tab.key

/** Tapping the magnifier opens the search tab; tapping it again returns to the spaces. */
fun toggleNavSearch(tab: NavTab): NavTab = if (tab == NavTab.Search) NavTab.Public else NavTab.Search

/** A problem message becomes an error toast, once per distinct message. */
fun navigatorToastText(status: String?): String? = status?.takeIf { it.isNotBlank() }

/** Collects the state, shrinks the HUD to Compact and turns status messages into toasts. */
@Composable
fun NavigatorView(viewModel: NavigatorViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.open) return
    CompactHudWhileVisible(NAVIGATOR_HUD_OWNER)
    val toasts = LocalToastHost.current
    LaunchedEffect(ui.status) { navigatorToastText(ui.status)?.let { toasts?.post(ToastKind.Error, it) } }
    val actions = remember(viewModel) {
        NavigatorActions(
            viewModel::close, viewModel::selectTab, viewModel::setQuery, viewModel::search,
            viewModel::join, viewModel::setCreateName, viewModel::selectLayout, viewModel::create,
        )
    }
    NavigatorContent(ui, actions)
}

/**
 * Stateless room navigator in the adaptive panel: chips (spaces, popular, mine, create) and a search toggle
 * in the header, over a list of room cards or the create form (button in the sticky footer).
 */
@Composable
fun NavigatorContent(ui: NavigatorUiState, actions: NavigatorActions, spec: PanelSpec = rememberPanelSpec()) {
    val searching = ui.tab == NavTab.Search
    AdaptivePanel(
        visible = true,
        title = "Salas",
        onClose = actions.onClose,
        icon = GameIcons.Compass,
        headerActions = {
            GameIconButton(
                GameIcons.Search, "Buscar sala", { actions.onTab(toggleNavSearch(ui.tab)) },
                style = IconButtonStyle.Raised, active = searching,
            )
        },
        footer = if (ui.tab == NavTab.Create) {
            { GameButton("Criar espaço", actions.onCreate, Modifier.fillMaxWidth(), size = ButtonSize.Large, enabled = ui.canCreate) }
        } else {
            null
        },
        spec = spec,
        panelKey = "navigator",
    ) {
        CategoryChips(
            NAV_CHIP_TABS.map { ChipItem(it.key, it.label) }, navChipSelection(ui.tab),
            { key -> actions.onTab(NavTab.fromKey(key)) },
            Modifier.padding(horizontal = Space.md),
        )
        if (searching) {
            SearchField(
                ui.query, actions.onQuery, "Nome da sala",
                Modifier.fillMaxWidth().padding(horizontal = Space.md, vertical = Space.xs),
                autoFocus = true, onSubmit = { if (ui.query.isNotBlank() && !ui.searching) actions.onSearch() },
            )
        }
        Box(Modifier.weight(1f).fillMaxWidth()) {
            if (ui.tab == NavTab.Create) CreateForm(ui, actions) else RoomList(ui, actions)
        }
    }
}

@Composable
private fun RoomList(ui: NavigatorUiState, actions: NavigatorActions) {
    when {
        ui.loading -> SkeletonList()
        ui.emptyMessage != null -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            EmptyState(
                GameIcons.Compass, ui.emptyMessage,
                action = when (ui.tab) {
                    NavTab.Mine -> "Criar espaço" to { actions.onTab(NavTab.Create) }
                    NavTab.Search -> if (ui.query.isNotBlank()) "Limpar busca" to { actions.onQuery("") } else null
                    else -> null
                },
            )
        }
        else -> LazyColumn(
            Modifier.fillMaxSize().padding(horizontal = Space.md),
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            items(ui.cards, key = { it.roomId }, contentType = { "room" }) { card ->
                RoomListCard(card.name, card.subtitle, "Entrar", { actions.onJoin(card.roomId) })
            }
        }
    }
}

@Composable
private fun CreateForm(ui: NavigatorUiState, actions: NavigatorActions) {
    Column(Modifier.fillMaxSize().padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.md)) {
        GameTextField(
            ui.createName, actions.onCreateName, "Nome da sala",
            placeholder = "Até $MAX_ROOM_NAME_CHARS letras", maxLength = MAX_ROOM_NAME_CHARS,
        )
        androidx.compose.material3.Text("Modelo da sala", style = ResortTheme.type.caption, color = ResortTheme.colors.textMuted)
        LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
            items(ui.layouts, key = { it.id }) { layout ->
                SelectableRow(layout.name, layout.meta, layout.selected, { actions.onLayout(layout.id) })
            }
        }
    }
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

private val LandscapeSpec = panelSpecFor(800.dp, 360.dp)
private val PortraitSpec = panelSpecFor(360.dp, 780.dp)

@Preview(name = "Public spaces (landscape)", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorPublicPreview() = PreviewScene { NavigatorContent(PreviewData.navigatorPublic, NavigatorActions(), spec = LandscapeSpec) }

@Preview(name = "Popular rooms", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorPopularPreview() = PreviewScene { NavigatorContent(PreviewData.navigatorRooms(NavTab.Popular), NavigatorActions(), spec = LandscapeSpec) }

@Preview(name = "Popular, loading", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorLoadingPreview() =
    PreviewScene { NavigatorContent(PreviewData.navigatorPublic.copy(tab = NavTab.Popular, cards = emptyList(), loading = true), NavigatorActions(), spec = LandscapeSpec) }

@Preview(name = "My rooms, empty", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorMineEmptyPreview() = PreviewScene {
    NavigatorContent(
        PreviewData.navigatorPublic.copy(tab = NavTab.Mine, cards = emptyList(), emptyMessage = emptyMessage(NavTab.Mine, false)),
        NavigatorActions(), spec = LandscapeSpec,
    )
}

@Preview(name = "Search, no result", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorSearchPreview() = PreviewScene {
    NavigatorContent(
        PreviewData.navigatorPublic.copy(tab = NavTab.Search, cards = emptyList(), query = "zzz", emptyMessage = emptyMessage(NavTab.Search, true)),
        NavigatorActions(), spec = LandscapeSpec,
    )
}

@Preview(name = "Create room", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorCreatePreview() = PreviewScene { NavigatorContent(PreviewData.navigatorCreate, NavigatorActions(), spec = LandscapeSpec) }

@Preview(name = "Error status (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun NavigatorErrorPreview() =
    PreviewScene { NavigatorContent(PreviewData.navigatorRooms(NavTab.Mine).copy(status = "Não foi possível entrar na sala."), NavigatorActions(), spec = PortraitSpec) }
