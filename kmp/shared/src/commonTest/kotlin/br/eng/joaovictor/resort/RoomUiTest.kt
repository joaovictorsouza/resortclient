package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.models.RoomUiState
import br.eng.joaovictor.resort.models.parseRoomUi
import br.eng.joaovictor.resort.repositories.FakeGateway
import br.eng.joaovictor.resort.repositories.RoomUiRepository
import br.eng.joaovictor.resort.views.roomui.MAX_SETTINGS_NAME_CHARS
import br.eng.joaovictor.resort.views.roomui.RoomVisibility
import br.eng.joaovictor.resort.views.roomui.clampSettingsName
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class RoomUiTest {
    @Test
    fun theSnapshotParsesSnakeCaseKeys() {
        val ui = parseRoomUi("""{"avatar_menu":true,"settings":{"name":"Casa","visibility":"hidden","confirming_delete":true},"doorbell":{"request_id":"r1","requester_name":"Ana"},"furniture_menu":{"item_id":"i1","catalog_id":"chair"}}""")
        assertNotNull(ui)
        assertTrue(ui.avatarMenu)
        assertEquals("hidden", ui.settings?.visibility)
        assertTrue(ui.settings?.confirmingDelete == true)
        assertEquals("r1", ui.doorbell?.requestId)
        assertEquals("Ana", ui.doorbell?.requesterName)
        assertEquals("i1", ui.furnitureMenu?.itemId)
    }

    @Test
    fun nothingOpenByDefault() {
        val ui = RoomUiState()
        assertTrue(!ui.avatarMenu)
        assertNull(ui.settings)
        assertNull(ui.doorbell)
    }

    @Test
    fun visibilityKeysRoundTripAndUnknownIsPrivate() {
        RoomVisibility.entries.forEach { assertEquals(it, RoomVisibility.fromKey(it.key)) }
        assertEquals(RoomVisibility.Private, RoomVisibility.fromKey("nope"))
    }

    @Test
    fun theNameIsCutAtTheGameLimit() {
        assertEquals(MAX_SETTINGS_NAME_CHARS, clampSettingsName("x".repeat(90)).length)
    }

    @OptIn(ExperimentalCoroutinesApi::class)
    @Test
    fun commandsHaveTheDocumentedShape() = runTest {
        val gateway = FakeGateway()
        val repo = RoomUiRepository(gateway, CoroutineScope(UnconfinedTestDispatcher(testScheduler)))
        repo.settingsName("Casa")
        repo.settingsVisibility("hidden")
        repo.answerDoorbell("r1", true)
        repo.signOut()
        repo.furnitureRotate("i1")
        assertEquals(
            listOf(
                "settings_name" to """{"text":"Casa"}""",
                "settings_visibility" to """{"visibility":"hidden"}""",
                "doorbell_answer" to """{"request_id":"r1","allowed":true}""",
                "avatar_sign_out" to "{}",
                "furniture_rotate" to """{"item_id":"i1"}""",
            ),
            gateway.sent.toList(),
        )
    }
}
