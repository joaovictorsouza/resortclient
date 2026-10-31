package br.eng.joaovictor.resort.models

import br.eng.joaovictor.resort.game.BevyJson
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/** Which screen the game is on (`game_state.app`). A value this shell does not know is [Unknown]. */
@Serializable
enum class AppPhase {
    @SerialName("loading") Loading,
    @SerialName("login") Login,
    @SerialName("nickname") Nickname,
    @SerialName("creator") Creator,
    @SerialName("connecting") Connecting,
    @SerialName("in_room") InRoom,
    @SerialName("error") Error,
    Unknown,
}

/** State of the connection to the server (`game_state.connection`). */
@Serializable
enum class Connection {
    @SerialName("idle") Idle,
    @SerialName("connecting") Connecting,
    @SerialName("connected") Connected,
    @SerialName("reconnecting") Reconnecting,
    @SerialName("disconnected") Disconnected,
    @SerialName("error") Error,
    Unknown,
}

@Serializable
data class RoomInfo(
    val id: String = "",
    val name: String = "",
    val visibility: String = "",
    @SerialName("is_owner") val isOwner: Boolean = false,
)

/**
 * What the game is doing right now (`game_state`), for any screen or feature, not only the HUD. The
 * property defaults are the "unknown" values, which is what an unrecognised value from a newer game
 * is coerced to; [INITIAL] is the state before the first message arrives.
 */
@Serializable
data class GameState(
    val app: AppPhase = AppPhase.Unknown,
    val connection: Connection = Connection.Unknown,
    val error: String? = null,
    val room: RoomInfo? = null,
) {
    /** In a room (also while the connection is being restored: the room is kept). */
    val inRoom: Boolean get() = app == AppPhase.InRoom && room != null

    /** The room the player is in is theirs. */
    val isOwnRoom: Boolean get() = inRoom && room?.isOwner == true

    companion object {
        val INITIAL = GameState(app = AppPhase.Loading, connection = Connection.Idle)
    }
}

/** Parses `game_state`; a payload that is not a JSON object gives `null` (the caller keeps its last good state). */
fun parseGameState(json: String): GameState? = BevyJson.decodeOrNull<GameState>(json)
