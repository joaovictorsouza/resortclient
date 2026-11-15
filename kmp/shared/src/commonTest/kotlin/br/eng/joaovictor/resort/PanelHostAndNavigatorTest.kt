package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.ds.components.ToastHostState
import br.eng.joaovictor.resort.ds.components.ToastKind
import br.eng.joaovictor.resort.ds.layout.PanelHostState
import br.eng.joaovictor.resort.models.StatusLine
import br.eng.joaovictor.resort.views.navigator.NAV_CHIP_TABS
import br.eng.joaovictor.resort.views.navigator.NavTab
import br.eng.joaovictor.resort.views.navigator.navChipSelection
import br.eng.joaovictor.resort.views.navigator.navigatorToastText
import br.eng.joaovictor.resort.views.navigator.toggleNavSearch
import br.eng.joaovictor.resort.views.shop.purchaseToastKind
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PanelHostAndNavigatorTest {
    @Test
    fun theFirstPanelSlidesInAndASecondOneSwaps() {
        val host = PanelHostState()
        assertFalse(host.claim("inventory")) // nothing was open: normal slide
        assertTrue(host.claim("shop")) // inventory still holds the slot: swap, no double animation
        assertEquals("shop", host.active)
    }

    @Test
    fun theOldPanelLeavingDoesNotFreeTheNewOnesSlot() {
        val host = PanelHostState()
        host.claim("inventory")
        host.claim("shop")
        host.release("inventory") // disposed after the shop took the slot
        assertEquals("shop", host.active)
        host.release("shop")
        assertNull(host.active)
        assertFalse(host.claim("navigator")) // closed in between: slides in again
    }

    @Test
    fun reopeningTheSamePanelIsNotASwap() {
        val host = PanelHostState()
        host.claim("shop")
        assertFalse(host.claim("shop"))
    }

    @Test
    fun searchIsTheHeaderIconNotAChip() {
        assertFalse(NavTab.Search in NAV_CHIP_TABS)
        assertEquals(listOf(NavTab.Public, NavTab.Popular, NavTab.Mine, NavTab.Create), NAV_CHIP_TABS)
        assertNull(navChipSelection(NavTab.Search))
        assertEquals("mine", navChipSelection(NavTab.Mine))
    }

    @Test
    fun theMagnifierTogglesBetweenSearchAndSpaces() {
        assertEquals(NavTab.Search, toggleNavSearch(NavTab.Mine))
        assertEquals(NavTab.Public, toggleNavSearch(NavTab.Search))
    }

    @Test
    fun onlyRealMessagesBecomeToasts() {
        assertNull(navigatorToastText(null))
        assertNull(navigatorToastText("  "))
        assertEquals("Não foi possível entrar na sala.", navigatorToastText("Não foi possível entrar na sala."))
    }

    @Test
    fun aPurchaseResultPicksTheToastKind() {
        assertEquals(ToastKind.Success, purchaseToastKind(StatusLine("success", "Comprado!")))
        assertEquals(ToastKind.Error, purchaseToastKind(StatusLine("error", "Saldo insuficiente.")))
    }

    @Test
    fun postingToastsAssignsIncreasingIdsAndQueues() {
        val host = ToastHostState()
        val a = host.post(ToastKind.Info, "a")
        val b = host.post(ToastKind.Error, "b")
        assertTrue(b > a)
        assertEquals("a", host.current.value?.text)
        host.dismiss(a)
        assertEquals("b", host.current.value?.text)
    }
}
