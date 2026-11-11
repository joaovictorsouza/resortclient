package br.eng.joaovictor.resort

import androidx.compose.ui.graphics.Color
import br.eng.joaovictor.resort.ds.theme.ResortColors
import br.eng.joaovictor.resort.ds.theme.contrastRatio
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/** Protects the palette from future "tweaks": every text pair the UI uses must keep WCAG AA (4.5:1). */
class DesignTokensTest {
    private val c = ResortColors()
    private val aa = 4.5

    private fun assertAa(name: String, fg: Color, bg: Color) {
        val ratio = contrastRatio(fg, bg, backdrop = c.surfaceSunken)
        assertTrue(ratio >= aa, "$name has contrast $ratio, below $aa")
    }

    @Test
    fun contrastMatchesKnownWcagValues() {
        assertEquals(21.0, contrastRatio(Color.White, Color.Black), 0.01)
        assertEquals(1.0, contrastRatio(Color.White, Color.White), 0.01)
    }

    @Test
    fun bodyTextIsReadableOnEverySurface() {
        assertAa("text on surface", c.text, c.surface)
        assertAa("text on raised", c.text, c.surfaceRaised)
        assertAa("text on high", c.text, c.surfaceHigh)
        assertAa("muted on surface", c.textMuted, c.surface)
        assertAa("muted on raised", c.textMuted, c.surfaceRaised)
        assertAa("muted on high", c.textMuted, c.surfaceHigh)
        assertAa("danger text on surface", c.dangerText, c.surface)
        assertAa("danger text on raised", c.dangerText, c.surfaceRaised)
    }

    @Test
    fun darkInkIsReadableOnEveryActionColor() {
        mapOf(
            "primary" to c.primary, "primaryPressed" to c.primaryPressed,
            "secondary" to c.secondary, "secondaryPressed" to c.secondaryPressed,
            "accent" to c.accent, "success" to c.success,
            "danger" to c.danger, "dangerPressed" to c.dangerPressed,
            "gold" to c.gold, "silver" to c.silver,
        ).forEach { (name, face) -> assertAa("onAccent on $name", c.onAccent, face) }
    }

    @Test
    fun whiteOnCoralWouldFail() {
        // The reason button text is dark: guards against "just make it white".
        assertTrue(contrastRatio(Color.White, c.primary) < aa)
    }

    @Test
    fun secondaryAndFocusRingStandOutFromSurfaces() {
        // Non-text UI (selection border, focus) needs 3:1.
        assertTrue(contrastRatio(c.secondary, c.surface, c.surfaceSunken) >= 3.0)
        assertTrue(contrastRatio(c.focusRing, c.surface, c.surfaceSunken) >= 3.0)
    }
}
