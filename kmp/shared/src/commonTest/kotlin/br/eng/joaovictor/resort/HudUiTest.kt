package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.models.AppPhase
import br.eng.joaovictor.resort.models.Connection
import br.eng.joaovictor.resort.models.GameState
import br.eng.joaovictor.resort.models.RoomInfo
import br.eng.joaovictor.resort.models.Wallet
import br.eng.joaovictor.resort.views.hud.formatAmount
import br.eng.joaovictor.resort.views.hud.hudUi
import br.eng.joaovictor.resort.views.hud.roomTitle
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class HudUiTest {
    private fun inRoom(owner: Boolean = false, build: Boolean = false, connection: Connection = Connection.Connected) = GameState(
        app = AppPhase.InRoom,
        connection = connection,
        room = RoomInfo(id = "r", name = "Casa", visibility = "hidden", isOwner = owner),
        buildMode = build,
    )

    @Test
    fun amountsUseDotsAsThousandsSeparators() {
        assertEquals("0", formatAmount(0))
        assertEquals("999", formatAmount(999))
        assertEquals("1.234", formatAmount(1234))
        assertEquals("1.234.567", formatAmount(1234567))
        assertEquals("-1.000", formatAmount(-1000))
    }

    @Test
    fun theTitleShowsTheVisibilityAndTheLobbyForUnnamedRooms() {
        assertEquals("Casa  •  [Oculta]", roomTitle("Casa", "hidden"))
        assertEquals("Casa  •  [Pública]", roomTitle("Casa", "public"))
        assertEquals("The Grand Lobby  •  [Privada]", roomTitle("", "private"))
    }

    @Test
    fun theHudOnlyShowsInsideARoom() {
        assertFalse(hudUi(GameState.INITIAL, Wallet()).visible)
        assertTrue(hudUi(inRoom(), Wallet()).visible)
    }

    @Test
    fun theWalletShowsADashUntilItIsLoaded() {
        val ui = hudUi(inRoom(), Wallet(silver = 5, gold = 1200, loaded = false))
        assertEquals("-", ui.silver)
        assertEquals("-", ui.gold)
        val loaded = hudUi(inRoom(), Wallet(silver = 5, gold = 1200, loaded = true))
        assertEquals("5", loaded.silver)
        assertEquals("1.200", loaded.gold)
    }

    @Test
    fun ownerBuildAndReconnectingFlagsComeFromTheGameState() {
        val owner = hudUi(inRoom(owner = true, build = true), Wallet())
        assertTrue(owner.isOwner && owner.buildMode)
        assertFalse(hudUi(inRoom(owner = false), Wallet()).isOwner)
        assertTrue(hudUi(inRoom(connection = Connection.Reconnecting), Wallet()).reconnecting)
    }
}
