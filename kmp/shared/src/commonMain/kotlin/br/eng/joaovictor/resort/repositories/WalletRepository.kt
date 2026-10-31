package br.eng.joaovictor.resort.repositories

import br.eng.joaovictor.resort.ffi.BevyEvent
import br.eng.joaovictor.resort.game.GameGateway
import br.eng.joaovictor.resort.models.Wallet
import br.eng.joaovictor.resort.models.parseWallet
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.flow.filterIsInstance
import kotlinx.coroutines.flow.mapNotNull
import kotlinx.coroutines.flow.stateIn

/** Latest `wallet_state` published by the game. Create it once, right after the `GameHost`, so no snapshot is missed. */
class WalletRepository(game: GameGateway, scope: CoroutineScope) {
    val state: StateFlow<Wallet> = game.events
        .filterIsInstance<BevyEvent.App>()
        .filter { it.name == EVENT }
        .mapNotNull { parseWallet(it.payload.decodeToString()) }
        .stateIn(scope, SharingStarted.Eagerly, Wallet())

    private companion object {
        const val EVENT = "wallet_state"
    }
}
