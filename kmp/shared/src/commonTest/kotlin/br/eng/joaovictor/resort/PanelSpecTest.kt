package br.eng.joaovictor.resort

import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.layout.PanelForm
import br.eng.joaovictor.resort.ds.layout.SheetDetent
import br.eng.joaovictor.resort.ds.layout.panelSpecFor
import kotlin.test.Test
import kotlin.test.assertEquals

class PanelSpecTest {
    @Test
    fun iPhoneLandscapeGetsA42PercentSidePanel() {
        val s = panelSpecFor(852.dp, 393.dp)
        assertEquals(PanelForm.Side, s.form)
        assertEquals(852.dp * 0.42f, s.sideWidth)
    }

    @Test
    fun smallPhonesLandscapeGetTheMinimumWidthWithinTheCap() {
        assertEquals(340.dp, panelSpecFor(800.dp, 360.dp).sideWidth)
        assertEquals(PanelForm.Side, panelSpecFor(800.dp, 360.dp).form)
        // 640 * 0.55 = 352 >= 340, so the minimum holds.
        assertEquals(340.dp, panelSpecFor(640.dp, 360.dp).sideWidth)
    }

    @Test
    fun portraitPhoneGetsASheetWithThreeDetents() {
        val s = panelSpecFor(360.dp, 780.dp)
        assertEquals(PanelForm.Sheet, s.form)
        assertEquals(136.dp, s.heightAt(SheetDetent.Peek))
        assertEquals(780.dp * 0.52f, s.heightAt(SheetDetent.Half))
        assertEquals(780.dp * 0.92f, s.heightAt(SheetDetent.Full))
    }

    @Test
    fun tabletPortraitIsASheetAndTabletLandscapeIsCappedAt480() {
        assertEquals(PanelForm.Sheet, panelSpecFor(820.dp, 1180.dp).form)
        val land = panelSpecFor(1180.dp, 820.dp)
        assertEquals(PanelForm.Side, land.form)
        assertEquals(480.dp, land.sideWidth)
    }

    @Test
    fun aWideWindowIsASidePanelEvenWhenTaller() {
        assertEquals(PanelForm.Side, panelSpecFor(900.dp, 1000.dp).form)
    }

    @Test
    fun theSidePanelNeverTakesMoreThan55PercentOfTheWidth() {
        val s = panelSpecFor(560.dp, 300.dp)
        assertEquals(PanelForm.Side, s.form)
        assertEquals(560.dp * 0.55f, s.sideWidth)
    }
}
