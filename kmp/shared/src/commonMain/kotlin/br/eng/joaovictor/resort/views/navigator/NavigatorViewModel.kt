package br.eng.joaovictor.resort.views.navigator

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import br.eng.joaovictor.resort.models.NavigatorState
import br.eng.joaovictor.resort.models.RoomLayout
import br.eng.joaovictor.resort.models.RoomSummary
import br.eng.joaovictor.resort.repositories.NavigatorRepository
import br.eng.joaovictor.resort.views.hud.visibilityLabel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn

const val MAX_ROOM_NAME_CHARS = 50

/** The navigator tabs, with the key the game uses (`navigator_tab`). */
enum class NavTab(val key: String, val label: String) {
    Public("public", "Espaços"),
    Popular("popular", "Populares"),
    Mine("mine", "Minhas salas"),
    Search("search", "Buscar"),
    Create("create", "Criar"),
    ;

    companion object {
        fun fromKey(key: String): NavTab = entries.firstOrNull { it.key == key } ?: Public
    }
}

/** A room (or public space) the player can join. */
data class RoomCard(val roomId: String, val name: String, val subtitle: String)

data class LayoutChoice(val id: String, val name: String, val meta: String, val selected: Boolean)

data class NavigatorUiState(
    val open: Boolean = false,
    val tab: NavTab = NavTab.Public,
    val cards: List<RoomCard> = emptyList(),
    val loading: Boolean = false,
    /** Shown instead of the list when there is nothing to list; `null` when there are cards. */
    val emptyMessage: String? = null,
    val query: String = "",
    val searching: Boolean = false,
    val layouts: List<LayoutChoice> = emptyList(),
    val createName: String = "",
    val status: String? = null,
) {
    val canCreate: Boolean get() = canCreateRoom(createName, layouts)
}

/** `"Criado por X  •  Ocupação 3/40  •  Pública"`, like the game's `room_meta`. */
fun roomSubtitle(room: RoomSummary): String =
    "Criado por ${room.ownerName ?: "Player"}  •  Ocupação ${room.userCount}/${room.maxUsers}  •  ${visibilityLabel(room.visibility)}"

fun layoutMeta(layout: RoomLayout): String = "${layout.width}x${layout.height} (máx. ${layout.maxUsers})"

fun clampRoomName(text: String): String = text.take(MAX_ROOM_NAME_CHARS)

/** A room needs a name and, when the game offers layouts, one of them. */
fun canCreateRoom(name: String, layouts: List<LayoutChoice>): Boolean =
    name.isNotBlank() && (layouts.isEmpty() || layouts.any { it.selected })

fun emptyMessage(tab: NavTab, hasSearched: Boolean): String = when (tab) {
    NavTab.Public -> "Nenhum espaço público"
    NavTab.Popular -> "Nenhuma sala popular agora"
    NavTab.Mine -> "Você ainda não criou nenhuma sala"
    NavTab.Search -> if (hasSearched) "Nenhuma sala encontrada" else "Digite um nome e busque"
    NavTab.Create -> ""
}

fun NavigatorState.toUi(createName: String, chosenLayout: String?): NavigatorUiState {
    val tab = NavTab.fromKey(tab)
    val cards = when (tab) {
        NavTab.Public -> public.map { RoomCard(it.roomId, it.name, "${it.description}  •  Capacidade ${it.maxUsers}") }
        NavTab.Popular -> popular.map { RoomCard(it.roomId, it.name, roomSubtitle(it)) }
        NavTab.Mine -> mine.map { RoomCard(it.roomId, it.name, roomSubtitle(it)) }
        NavTab.Search -> search.map { RoomCard(it.roomId, it.name, roomSubtitle(it)) }
        NavTab.Create -> emptyList()
    }
    // The first layout is preselected until the player picks another one.
    val picked = chosenLayout?.takeIf { id -> layouts.any { it.id == id } } ?: layouts.firstOrNull()?.id
    val loading = (tab == NavTab.Popular && loadingPopular) || (tab == NavTab.Search && searching)
    return NavigatorUiState(
        open = open,
        tab = tab,
        cards = cards,
        loading = loading,
        emptyMessage = if (cards.isEmpty() && !loading && tab != NavTab.Create) emptyMessage(tab, hasSearched) else null,
        query = query,
        searching = searching,
        layouts = layouts.map { LayoutChoice(it.id, it.name, layoutMeta(it), selected = it.id == picked) },
        createName = createName,
        status = status,
    )
}

class NavigatorViewModel(private val navigator: NavigatorRepository) : ViewModel() {
    private val createName = MutableStateFlow("")
    private val layout = MutableStateFlow<String?>(null)

    val ui: StateFlow<NavigatorUiState> = combine(navigator.state, createName, layout) { state, name, chosen ->
        state.toUi(name, chosen)
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), NavigatorUiState())

    fun close() = navigator.setOpen(false)
    fun selectTab(tab: NavTab) = navigator.selectTab(tab.key)
    fun setQuery(query: String) = navigator.setQuery(query)
    fun search() = navigator.search()
    fun join(roomId: String) = navigator.join(roomId)
    fun setCreateName(text: String) { createName.value = clampRoomName(text) }
    fun selectLayout(id: String) { layout.value = id }

    fun create() {
        val state = ui.value
        if (!state.canCreate) return
        navigator.create(state.createName.trim(), state.layouts.firstOrNull { it.selected }?.id)
    }
}
