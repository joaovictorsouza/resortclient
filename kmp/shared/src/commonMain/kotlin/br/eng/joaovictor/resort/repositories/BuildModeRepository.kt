package br.eng.joaovictor.resort.repositories

import br.eng.joaovictor.resort.game.GameGateway
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn

/**
 * Room build mode as the shell sees it: derived from `game_state.build_mode` and driven by the
 * `build_mode_set` command. The game stays the source of truth, so [active] only changes when the
 * game confirms it.
 */
class BuildModeRepository(
    private val game: GameGateway,
    gameState: GameStateRepository,
    scope: CoroutineScope,
) {
    /** The player is in a room that is in build mode. */
    val active: StateFlow<Boolean> = gameState.state
        .map { it.inRoom && it.buildMode }
        .distinctUntilChanged()
        .stateIn(scope, SharingStarted.Eagerly, false)

    /** Only the owner of the room can build in it. */
    val canBuild: StateFlow<Boolean> = gameState.state
        .map { it.isOwnRoom }
        .distinctUntilChanged()
        .stateIn(scope, SharingStarted.Eagerly, false)

    fun set(active: Boolean) = game.send("build_mode_set", """{"active":$active}""")
}
