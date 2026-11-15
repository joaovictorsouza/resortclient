package br.eng.joaovictor.resort

import br.eng.joaovictor.resort.models.CreatorItem
import br.eng.joaovictor.resort.models.CreatorSection
import br.eng.joaovictor.resort.models.CreatorState
import br.eng.joaovictor.resort.repositories.CreatorRepository
import br.eng.joaovictor.resort.repositories.FakeGateway
import br.eng.joaovictor.resort.views.creator.ROTATE_STEP
import br.eng.joaovictor.resort.views.creator.confirmLabel
import br.eng.joaovictor.resort.views.creator.sameColor
import br.eng.joaovictor.resort.views.creator.toUi
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class CreatorUiTest {
    private val state = CreatorState(
        active = true,
        category = "hair",
        categories = listOf(
            CreatorSection("body", "Corpo", listOf(CreatorItem("female", "Feminino", selected = true))),
            CreatorSection(
                "hair", "Cabelo",
                items = listOf(CreatorItem("h1", "Liso", thumbnailFile = "/c/h1.png", selected = true), CreatorItem("h2", "Cacheado")),
                palette = listOf("#1C1C1C", "#E91E63"),
                color = "#e91e63",
            ),
        ),
    )

    @Test
    fun theTabsMarkTheActiveCategory() {
        val ui = state.toUi()
        assertEquals(listOf("Corpo", "Cabelo"), ui.tabs.map { it.label })
        assertEquals(listOf(false, true), ui.tabs.map { it.selected })
    }

    @Test
    fun theActiveSectionGivesOptionsAndSwatches() {
        val ui = state.toUi()
        assertEquals(listOf("h1", "h2"), ui.options.map { it.id })
        assertEquals("/c/h1.png", ui.options[0].thumbnailFile)
        assertEquals(listOf(false, true), ui.swatches.map { it.selected }) // colour compare ignores case
    }

    @Test
    fun aSectionWithoutOptionsOrAPaletteShowsNothing() {
        val ui = state.copy(category = "gone").toUi()
        assertTrue(ui.options.isEmpty() && ui.swatches.isEmpty())
    }

    @Test
    fun theConfirmLabelFollowsTheMode() {
        assertEquals("Confirmar e entrar no jogo", confirmLabel(false))
        assertEquals("Confirmar e voltar ao jogo", confirmLabel(true))
        assertEquals("Confirmar e voltar ao jogo", state.copy(editing = true).toUi().confirmLabel)
        assertEquals("do jogo", state.copy(confirmLabel = "do jogo").toUi().confirmLabel)
        assertTrue(state.copy(editing = true).toUi().editing)
        assertFalse(state.toUi().editing)
    }

    @Test
    fun colorsCompareIgnoringCase() {
        assertTrue(sameColor("#FFE0BD", "#ffe0bd"))
        assertFalse(sameColor(null, "#ffe0bd"))
    }

    @OptIn(ExperimentalCoroutinesApi::class)
    @Test
    fun theNewCommandsHaveTheDocumentedShape() = runTest {
        val gateway = FakeGateway()
        val repo = CreatorRepository(gateway, CoroutineScope(UnconfinedTestDispatcher(testScheduler)))
        repo.color("#FFE0BD")
        repo.randomize()
        repo.rotate(-ROTATE_STEP)
        assertEquals(
            listOf(
                "creator_color" to """{"hex":"#FFE0BD"}""",
                "creator_random" to "{}",
                "creator_rotate" to """{"delta":-0.45}""",
            ),
            gateway.sent.toList(),
        )
    }
}
