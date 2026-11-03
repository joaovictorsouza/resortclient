package br.eng.joaovictor.resort.views.navigator

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import br.eng.joaovictor.resort.components.GameSearchBar
import br.eng.joaovictor.resort.components.GameWindow
import br.eng.joaovictor.resort.views.hud.HideHudWhileVisible
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene
import org.jetbrains.compose.ui.tooling.preview.Preview

/** The HUD owner name the navigator holds while open (see `HudVisibility`). */
internal const val NAVIGATOR_HUD_OWNER = "navigator"

private val Selected = Color(0xFF4F46E5)

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

/** Collects the state and holds the HUD hidden while the navigator is open. */
@Composable
fun NavigatorView(viewModel: NavigatorViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.open) return
    HideHudWhileVisible(NAVIGATOR_HUD_OWNER)
    val actions = remember(viewModel) {
        NavigatorActions(
            viewModel::close, viewModel::selectTab, viewModel::setQuery, viewModel::search,
            viewModel::join, viewModel::setCreateName, viewModel::selectLayout, viewModel::create,
        )
    }
    NavigatorContent(ui, actions)
}

/** Stateless room navigator: tabs (spaces, popular, mine, search, create) over a room list or the create form. */
@Composable
fun NavigatorContent(ui: NavigatorUiState, actions: NavigatorActions) {
    GameWindow(
        title = "Navegador de salas",
        onClose = actions.onClose,
        maxWidth = 640.dp,
        widthFraction = 0.85f,
    ) {
        LazyRow(
            Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            items(NavTab.entries) { tab ->
                Chip(tab.label, tab == ui.tab) { actions.onTab(tab) }
            }
        }
        Box(Modifier.weight(1f).fillMaxWidth()) {
            when (ui.tab) {
                NavTab.Create -> CreateForm(ui, actions)
                NavTab.Search -> Column {
                    Row(
                        Modifier.padding(horizontal = 12.dp, vertical = 4.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        GameSearchBar(ui.query, actions.onQuery, "Nome da sala", Modifier.weight(1f))
                        Button(onClick = actions.onSearch, enabled = ui.query.isNotBlank() && !ui.searching) { Text("Buscar") }
                    }
                    RoomList(ui, actions, Modifier.weight(1f))
                }
                else -> RoomList(ui, actions, Modifier.fillMaxSize())
            }
        }
        ui.status?.let {
            Text(it, color = Color(0xFFFB7185), fontSize = 12.sp, modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp))
        }
    }
}

@Composable
private fun RoomList(ui: NavigatorUiState, actions: NavigatorActions, modifier: Modifier) {
    when {
        ui.loading -> Box(modifier, contentAlignment = Alignment.Center) { CircularProgressIndicator() }
        ui.emptyMessage != null -> Box(modifier, contentAlignment = Alignment.Center) { Text(ui.emptyMessage) }
        else -> LazyColumn(
            modifier.padding(horizontal = 12.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            items(ui.cards, key = { it.roomId }) { card -> RoomRow(card) { actions.onJoin(card.roomId) } }
        }
    }
}

@Composable
private fun RoomRow(card: RoomCard, onJoin: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().background(Color.White.copy(alpha = 0.06f), RoundedCornerShape(10.dp)).padding(10.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(card.name, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(card.subtitle, color = Color.White.copy(alpha = 0.65f), fontSize = 12.sp, maxLines = 2, overflow = TextOverflow.Ellipsis)
        }
        Button(onClick = onJoin) { Text("Entrar") }
    }
}

@Composable
private fun CreateForm(ui: NavigatorUiState, actions: NavigatorActions) {
    Column(Modifier.fillMaxSize().padding(12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        GameSearchBar(ui.createName, actions.onCreateName, "Nome da sala (até $MAX_ROOM_NAME_CHARS letras)", Modifier.fillMaxWidth())
        Text("Modelo da sala", color = Color.White.copy(alpha = 0.7f), fontSize = 12.sp)
        LazyColumn(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            items(ui.layouts, key = { it.id }) { layout ->
                val shape = RoundedCornerShape(10.dp)
                Column(
                    Modifier.fillMaxWidth()
                        .background(if (layout.selected) Selected.copy(alpha = 0.35f) else Color.White.copy(alpha = 0.06f), shape)
                        .border(if (layout.selected) 2.dp else 1.dp, if (layout.selected) Selected else Color.White.copy(alpha = 0.12f), shape)
                        .clickable { actions.onLayout(layout.id) }
                        .padding(10.dp),
                ) {
                    Text(layout.name)
                    Text(layout.meta, color = Color.White.copy(alpha = 0.65f), fontSize = 12.sp)
                }
            }
        }
        Button(onClick = actions.onCreate, enabled = ui.canCreate, modifier = Modifier.fillMaxWidth()) { Text("Criar espaço") }
    }
}

@Composable
private fun Chip(text: String, on: Boolean, onClick: () -> Unit) {
    Text(
        text,
        color = Color.White,
        fontSize = 13.sp,
        modifier = Modifier
            .background(if (on) Selected else Color.White.copy(alpha = 0.12f), RoundedCornerShape(14.dp))
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 6.dp),
    )
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

@Preview(name = "Public spaces (landscape)", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorPublicPreview() = PreviewScene { NavigatorContent(PreviewData.navigatorPublic, NavigatorActions()) }

@Preview(name = "Popular rooms", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorPopularPreview() = PreviewScene { NavigatorContent(PreviewData.navigatorRooms(NavTab.Popular), NavigatorActions()) }

@Preview(name = "Popular, loading", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorLoadingPreview() =
    PreviewScene { NavigatorContent(PreviewData.navigatorPublic.copy(tab = NavTab.Popular, cards = emptyList(), loading = true), NavigatorActions()) }

@Preview(name = "My rooms, empty", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorMineEmptyPreview() = PreviewScene {
    NavigatorContent(
        PreviewData.navigatorPublic.copy(tab = NavTab.Mine, cards = emptyList(), emptyMessage = emptyMessage(NavTab.Mine, false)),
        NavigatorActions(),
    )
}

@Preview(name = "Search, no result", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorSearchPreview() = PreviewScene {
    NavigatorContent(
        PreviewData.navigatorPublic.copy(tab = NavTab.Search, cards = emptyList(), query = "zzz", emptyMessage = emptyMessage(NavTab.Search, true)),
        NavigatorActions(),
    )
}

@Preview(name = "Create room", widthDp = 800, heightDp = 360)
@Composable
private fun NavigatorCreatePreview() = PreviewScene { NavigatorContent(PreviewData.navigatorCreate, NavigatorActions()) }

@Preview(name = "Error status (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun NavigatorErrorPreview() =
    PreviewScene { NavigatorContent(PreviewData.navigatorRooms(NavTab.Mine).copy(status = "Não foi possível entrar na sala."), NavigatorActions()) }
