package br.eng.joaovictor.resort.views.hud

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import br.eng.joaovictor.resort.models.Connection
import br.eng.joaovictor.resort.models.GameState
import br.eng.joaovictor.resort.models.Wallet
import br.eng.joaovictor.resort.repositories.BuildModeRepository
import br.eng.joaovictor.resort.repositories.GameStateRepository
import br.eng.joaovictor.resort.repositories.HudRepository
import br.eng.joaovictor.resort.repositories.InventoryRepository
import br.eng.joaovictor.resort.repositories.NavigatorRepository
import br.eng.joaovictor.resort.repositories.ShopRepository
import br.eng.joaovictor.resort.repositories.WalletRepository
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn

data class HudUiState(
    /** The HUD only exists inside a room. */
    val visible: Boolean = false,
    val roomTitle: String = "",
    val isOwner: Boolean = false,
    val silver: String = "-",
    val gold: String = "-",
    val reconnecting: Boolean = false,
    val buildMode: Boolean = false,
)

/** `1234567` -> `1.234.567` (pt-BR), like the game's `format_amount`. */
fun formatAmount(value: Long): String {
    val digits = kotlin.math.abs(value).toString()
    val grouped = digits.reversed().chunked(3).joinToString(".").reversed()
    return if (value < 0) "-$grouped" else grouped
}

fun visibilityLabel(visibility: String): String = when (visibility) {
    "public" -> "Pública"
    "hidden" -> "Oculta"
    else -> "Privada"
}

/** `"Nome  •  [Pública]"`; an unnamed room is the lobby. */
fun roomTitle(name: String, visibility: String): String =
    "${name.ifEmpty { "The Grand Lobby" }}  •  [${visibilityLabel(visibility)}]"

/**
 * The HUD shows in a room, unless a bevy_hui full-screen screen is open ([GameState.fullScreenOpen])
 * or a Compose screen holds it hidden ([hiddenByCompose], see [HudVisibility]).
 */
fun hudUi(game: GameState, wallet: Wallet, hiddenByCompose: Boolean = false): HudUiState {
    val room = game.room
    return HudUiState(
        visible = game.inRoom && !game.fullScreenOpen && !hiddenByCompose,
        roomTitle = room?.let { roomTitle(it.name, it.visibility) }.orEmpty(),
        isOwner = game.isOwnRoom,
        silver = if (wallet.loaded) formatAmount(wallet.silver) else "-",
        gold = if (wallet.loaded) formatAmount(wallet.gold) else "-",
        reconnecting = game.connection == Connection.Reconnecting,
        buildMode = game.inRoom && game.buildMode,
    )
}

class HudViewModel(
    gameState: GameStateRepository,
    wallet: WalletRepository,
    private val build: BuildModeRepository,
    private val shop: ShopRepository,
    private val navigator: NavigatorRepository,
    private val inventory: InventoryRepository,
    private val hud: HudRepository,
    visibility: HudVisibility,
) : ViewModel() {
    val ui: StateFlow<HudUiState> =
        combine(gameState.state, wallet.state, visibility.visible) { game, w, visible -> hudUi(game, w, !visible) }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), HudUiState())

    fun toggleBuild() = build.set(!ui.value.buildMode)
    fun toggleShop() = shop.setOpen(!shop.state.value.open)
    fun toggleNavigator() = navigator.setOpen(!navigator.state.value.open)
    fun toggleInventory() = inventory.setOpen(!inventory.state.value.open)
    fun toggleAvatarMenu() = hud.toggleAvatarMenu()
    fun toggleRoomSettings() = hud.toggleRoomSettings()
}
