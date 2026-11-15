package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.views.hud.HudMode
import br.eng.joaovictor.resort.views.hud.HudVisibility
import br.eng.joaovictor.resort.views.hud.resolveHudMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class HudModeTest {
    @Test
    fun hiddenBeatsCompactBeatsFull() {
        assertEquals(HudMode.Full, resolveHudMode(emptySet(), emptySet()))
        assertEquals(HudMode.Compact, resolveHudMode(emptySet(), setOf("shop")))
        assertEquals(HudMode.Hidden, resolveHudMode(setOf("creator"), setOf("shop")))
    }

    @Test
    fun theControllerKeepsTheMostRestrictiveModeAcrossOwners() {
        val hud = HudVisibility()
        assertEquals(HudMode.Full, hud.mode.value)
        hud.compact("shop")
        assertEquals(HudMode.Compact, hud.mode.value)
        assertTrue(hud.visible.value) // compact still shows the HUD
        hud.hide("dialog")
        assertEquals(HudMode.Hidden, hud.mode.value)
        assertFalse(hud.visible.value)
        hud.release("dialog")
        assertEquals(HudMode.Compact, hud.mode.value) // shop still holds it
        hud.release("shop")
        assertEquals(HudMode.Full, hud.mode.value)
    }

    @Test
    fun releasingAnOwnerThatHoldsNothingIsANoOp() {
        val hud = HudVisibility()
        hud.release("nobody")
        assertEquals(HudMode.Full, hud.mode.value)
        hud.compact("a")
        hud.compact("a") // twice is still one hold
        hud.release("a")
        assertEquals(HudMode.Full, hud.mode.value)
        hud.release("a")
        assertEquals(HudMode.Full, hud.mode.value)
    }

    @Test
    fun showStillReleasesAHideAsBefore() {
        val hud = HudVisibility()
        hud.hide("old-screen")
        assertFalse(hud.visible.value)
        hud.show("old-screen")
        assertTrue(hud.visible.value)
    }

    @Test
    fun releaseDropsBothKindsOfHoldOfTheSameOwner() {
        val hud = HudVisibility()
        hud.compact("x")
        hud.hide("x")
        hud.release("x")
        assertEquals(HudMode.Full, hud.mode.value)
    }
}
