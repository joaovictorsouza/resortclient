package br.eng.joaovictor.resort.repositories

import br.eng.joaovictor.resort.models.AppPhase
import br.eng.joaovictor.resort.models.Connection
import br.eng.joaovictor.resort.models.GameState
import br.eng.joaovictor.resort.models.parseGameState
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

private const val IN_ROOM_MINE =
    """{"app":"in_room","connection":"connected","error":null,"room":{"id":"r1","name":"Minha sala","visibility":"private","is_owner":true}}"""
private const val IN_ROOM_OTHER =
    """{"app":"in_room","connection":"connected","error":null,"room":{"id":"lobby","name":"The Grand Lobby","visibility":"public","is_owner":false}}"""

@OptIn(ExperimentalCoroutinesApi::class)
class GameStateRepositoryTest {
    private fun TestScope.repo(gateway: FakeGateway) =
        GameStateRepository(gateway, CoroutineScope(UnconfinedTestDispatcher(testScheduler)))

    @Test
    fun startsLoadingAndIdle() = runTest {
        val state = repo(FakeGateway()).state.value
        assertEquals(GameState.INITIAL, state)
        assertFalse(state.inRoom)
    }

    @Test
    fun inRoomReportsTheRoomAndOwnership() = runTest {
        val gateway = FakeGateway()
        val repo = repo(gateway)
        gateway.emitApp("game_state", IN_ROOM_MINE)
        val state = repo.state.value
        assertTrue(state.inRoom)
        assertTrue(state.isOwnRoom)
        assertEquals("Minha sala", state.room?.name)
        assertEquals(Connection.Connected, state.connection)
    }

    @Test
    fun changingRoomClearsOwnership() = runTest {
        val gateway = FakeGateway()
        val repo = repo(gateway)
        gateway.emitApp("game_state", IN_ROOM_MINE)
        assertTrue(repo.state.value.isOwnRoom)
        gateway.emitApp("game_state", IN_ROOM_OTHER)
        assertTrue(repo.state.value.inRoom)
        assertFalse(repo.state.value.isOwnRoom)
        assertEquals("lobby", repo.state.value.room?.id)
    }

    @Test
    fun reconnectingKeepsTheRoom() = runTest {
        val gateway = FakeGateway()
        val repo = repo(gateway)
        gateway.emitApp("game_state", IN_ROOM_MINE.replace("connected", "reconnecting"))
        val state = repo.state.value
        assertEquals(Connection.Reconnecting, state.connection)
        assertTrue(state.inRoom)
        assertEquals("r1", state.room?.id)
    }

    @Test
    fun leavingTheRoomClearsIt() = runTest {
        val gateway = FakeGateway()
        val repo = repo(gateway)
        gateway.emitApp("game_state", IN_ROOM_MINE)
        gateway.emitApp("game_state", """{"app":"login","connection":"idle","error":null,"room":null}""")
        assertFalse(repo.state.value.inRoom)
        assertFalse(repo.state.value.isOwnRoom)
        assertNull(repo.state.value.room)
    }

    @Test
    fun aMalformedPayloadKeepsTheLastState() = runTest {
        val gateway = FakeGateway()
        val repo = repo(gateway)
        gateway.emitApp("game_state", IN_ROOM_MINE)
        gateway.emitApp("game_state", "garbage")
        gateway.emitApp("game_state", "")
        assertEquals("r1", repo.state.value.room?.id)
        gateway.emitApp("game_state", IN_ROOM_OTHER) // the flow is still alive
        assertEquals("lobby", repo.state.value.room?.id)
    }

    @Test
    fun otherEventsAreIgnored() = runTest {
        val gateway = FakeGateway()
        val repo = repo(gateway)
        gateway.emitApp("wallet_state", """{"silver":1,"gold":1,"loaded":true}""")
        assertEquals(GameState.INITIAL, repo.state.value)
    }

    @Test
    fun unknownValuesFallBackToUnknown() {
        val state = parseGameState(
            """{"app":"brand_new_screen","connection":"quantum","error":null,"room":null,"extra":1}""",
        )!!
        assertEquals(AppPhase.Unknown, state.app)
        assertEquals(Connection.Unknown, state.connection)
        assertFalse(state.inRoom)
    }

    @Test
    fun anInRoomStateWithoutARoomIsNotInRoom() {
        val state = parseGameState("""{"app":"in_room","connection":"connected","room":null}""")!!
        assertFalse(state.inRoom)
    }
}
