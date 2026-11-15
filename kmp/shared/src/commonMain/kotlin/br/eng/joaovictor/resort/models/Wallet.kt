package br.eng.joaovictor.resort.models

import br.eng.joaovictor.resort.game.BevyJson
import kotlinx.serialization.Serializable

/**
 * The player's balances, as the game mirrors them (`wallet_state`). Server-authoritative: the shell
 * only displays them. [loaded] is false until the first server reply, so the UI can show "-"
 * instead of a fake "0".
 */
@Serializable
data class Wallet(
    val silver: Long = 0,
    val gold: Long = 0,
    val loaded: Boolean = false,
)

/** Parses `{"silver":12,"gold":3,"loaded":true}`; a payload that is not a JSON object gives `null` (the caller keeps its last good value). */
fun parseWallet(json: String): Wallet? = BevyJson.decodeOrNull<Wallet>(json)
