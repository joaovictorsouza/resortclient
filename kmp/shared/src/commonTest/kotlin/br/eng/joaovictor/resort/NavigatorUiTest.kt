package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.models.NavigatorState
import br.eng.joaovictor.resort.models.PublicSpace
import br.eng.joaovictor.resort.models.RoomLayout
import br.eng.joaovictor.resort.models.RoomSummary
import br.eng.joaovictor.resort.views.navigator.LayoutChoice
import br.eng.joaovictor.resort.views.navigator.MAX_ROOM_NAME_CHARS
import br.eng.joaovictor.resort.views.navigator.NavTab
import br.eng.joaovictor.resort.views.navigator.canCreateRoom
import br.eng.joaovictor.resort.views.navigator.clampRoomName
import br.eng.joaovictor.resort.views.navigator.roomSubtitle
import br.eng.joaovictor.resort.views.navigator.toUi
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class NavigatorUiTest {
    private val room = RoomSummary(roomId = "r1", name = "Casa", ownerName = "Ana", visibility = "hidden", userCount = 3, maxUsers = 20)

    @Test
    fun tabKeysRoundTripAndUnknownFallsBackToPublic() {
        NavTab.entries.forEach { assertEquals(it, NavTab.fromKey(it.key)) }
        assertEquals(NavTab.Public, NavTab.fromKey("nope"))
    }

    @Test
    fun theSubtitleNamesTheOwnerOccupancyAndVisibility() {
        assertEquals("Criado por Ana  •  Ocupação 3/20  •  Oculta", roomSubtitle(room))
        assertEquals("Criado por Player  •  Ocupação 3/20  •  Pública", roomSubtitle(room.copy(ownerName = null, visibility = "public")))
    }

    @Test
    fun theListFollowsTheTab() {
        val state = NavigatorState(
            open = true,
            public = listOf(PublicSpace("lobby", "The Grand Lobby", "Central", 100)),
            popular = listOf(room),
            mine = emptyList(),
        )
        assertEquals(listOf("lobby"), state.copy(tab = "public").toUi("", null).cards.map { it.roomId })
        assertEquals(listOf("r1"), state.copy(tab = "popular").toUi("", null).cards.map { it.roomId })
        val mine = state.copy(tab = "mine").toUi("", null)
        assertTrue(mine.cards.isEmpty())
        assertEquals("Você ainda não criou nenhuma sala", mine.emptyMessage)
    }

    @Test
    fun theSearchTabExplainsBeforeAndAfterASearch() {
        val before = NavigatorState(open = true, tab = "search").toUi("", null)
        assertEquals("Digite um nome e busque", before.emptyMessage)
        val after = NavigatorState(open = true, tab = "search", hasSearched = true).toUi("", null)
        assertEquals("Nenhuma sala encontrada", after.emptyMessage)
        val busy = NavigatorState(open = true, tab = "search", searching = true).toUi("", null)
        assertTrue(busy.loading)
        assertNull(busy.emptyMessage)
    }

    @Test
    fun theFirstLayoutIsPreselectedUntilAnotherIsPicked() {
        val layouts = listOf(RoomLayout("a", "A", 8, 8, 10), RoomLayout("b", "B", 16, 16, 40))
        val state = NavigatorState(open = true, tab = "create", layouts = layouts)
        assertEquals(listOf(true, false), state.toUi("x", null).layouts.map { it.selected })
        assertEquals(listOf(false, true), state.toUi("x", "b").layouts.map { it.selected })
        assertEquals(listOf(true, false), state.toUi("x", "gone").layouts.map { it.selected })
    }

    @Test
    fun aRoomNeedsANameAndALayoutWhenThereAreSome() {
        val picked = listOf(LayoutChoice("a", "A", "8x8", selected = true))
        assertTrue(canCreateRoom("Casa", picked))
        assertFalse(canCreateRoom("   ", picked))
        assertTrue(canCreateRoom("Casa", emptyList())) // the game offers no layouts
        assertFalse(canCreateRoom("Casa", listOf(LayoutChoice("a", "A", "8x8", selected = false))))
    }

    @Test
    fun theNameIsCutAtTheGameLimit() {
        assertEquals(MAX_ROOM_NAME_CHARS, clampRoomName("x".repeat(80)).length)
        assertEquals("curto", clampRoomName("curto"))
    }
}
