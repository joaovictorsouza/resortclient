package br.eng.joaovictor.resort.game

import br.eng.joaovictor.resort.ffi.BevyEvent
import kotlinx.coroutines.flow.SharedFlow

interface GameGateway {
    val events: SharedFlow<BevyEvent> // nome + payload JSON
    fun send(name: String, json: String)
}
class GameHostGateway(private val host: GameHost) : GameGateway {
    override val events = host.events
    override fun send(name: String, json: String) = host.send(name, json)
}