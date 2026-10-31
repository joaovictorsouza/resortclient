package br.eng.joaovictor.resort.repositories

import br.eng.joaovictor.resort.ffi.BevyEvent
import br.eng.joaovictor.resort.game.GameGateway
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow

/** Stands in for the game: tests emit [BevyEvent]s and read what the code under test sent. */
class FakeGateway : GameGateway {
    val flow = MutableSharedFlow<BevyEvent>(extraBufferCapacity = 16)
    val sent = mutableListOf<Pair<String, String>>()
    override val events: SharedFlow<BevyEvent> = flow
    override fun send(name: String, json: String) { sent += name to json }

    suspend fun emitApp(name: String, json: String) = flow.emit(BevyEvent.App(name, json.encodeToByteArray()))
}
