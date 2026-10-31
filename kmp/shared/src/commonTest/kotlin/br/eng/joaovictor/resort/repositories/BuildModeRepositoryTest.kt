package br.eng.joaovictor.resort.repositories

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

private fun state(app: String, owner: Boolean, build: Boolean) =
    """{"app":"$app","connection":"connected","error":null,"room":{"id":"r1","name":"Sala","visibility":"private","is_owner":$owner},"build_mode":$build}"""

@OptIn(ExperimentalCoroutinesApi::class)
class BuildModeRepositoryTest {
    private fun TestScope.repo(gateway: FakeGateway): BuildModeRepository {
        val scope = CoroutineScope(UnconfinedTestDispatcher(testScheduler))
        return BuildModeRepository(gateway, GameStateRepository(gateway, scope), scope)
    }

    @Test
    fun followsTheGameBuildFlag() = runTest {
        val gateway = FakeGateway()
        val repo = repo(gateway)
        assertFalse(repo.active.value)
        gateway.emitApp("game_state", state("in_room", owner = true, build = true))
        assertTrue(repo.active.value)
        gateway.emitApp("game_state", state("in_room", owner = true, build = false))
        assertFalse(repo.active.value)
    }

    @Test
    fun onlyTheOwnerCanBuild() = runTest {
        val gateway = FakeGateway()
        val repo = repo(gateway)
        gateway.emitApp("game_state", state("in_room", owner = false, build = false))
        assertFalse(repo.canBuild.value)
        gateway.emitApp("game_state", state("in_room", owner = true, build = false))
        assertTrue(repo.canBuild.value)
    }

    @Test
    fun missingFlagMeansOff() = runTest {
        val gateway = FakeGateway()
        val repo = repo(gateway)
        gateway.emitApp("game_state", """{"app":"in_room","room":{"id":"r1","is_owner":true}}""")
        assertFalse(repo.active.value)
    }

    @Test
    fun setSendsTheCommand() = runTest {
        val gateway = FakeGateway()
        repo(gateway).set(true)
        assertEquals(listOf("build_mode_set" to """{"active":true}"""), gateway.sent)
    }
}
