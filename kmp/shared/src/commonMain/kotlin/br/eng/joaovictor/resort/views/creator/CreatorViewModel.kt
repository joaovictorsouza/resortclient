package br.eng.joaovictor.resort.views.creator

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import br.eng.joaovictor.resort.models.CreatorState
import br.eng.joaovictor.resort.repositories.CreatorRepository
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn

/** Radians the preview turns per tap on a rotate button, like the bevy_hui creator. */
const val ROTATE_STEP = 0.45f

data class CreatorTab(val key: String, val label: String, val selected: Boolean)

data class CreatorOption(val id: String, val name: String, val thumbnailFile: String, val selected: Boolean)

data class CreatorSwatch(val hex: String, val selected: Boolean)

data class CreatorUiState(
    val active: Boolean = false,
    /** Editing an existing avatar: the player can close without saving. */
    val editing: Boolean = false,
    val confirmLabel: String = "",
    val tabs: List<CreatorTab> = emptyList(),
    val swatches: List<CreatorSwatch> = emptyList(),
    val options: List<CreatorOption> = emptyList(),
)

/** Text of the confirm button: the editor returns to the game, the first creation enters it. */
fun confirmLabel(editing: Boolean): String =
    if (editing) "Confirmar e voltar ao jogo" else "Confirmar e entrar no jogo"

/** Same colour ignoring case (`#ffe0bd` equals `#FFE0BD`). */
fun sameColor(a: String?, b: String?): Boolean = a != null && b != null && a.equals(b, ignoreCase = true)

fun CreatorState.toUi(): CreatorUiState {
    val section = section(category)
    return CreatorUiState(
        active = active,
        editing = editing,
        confirmLabel = confirmLabel.ifEmpty { confirmLabel(editing) },
        tabs = categories.map { CreatorTab(it.key, it.label.ifEmpty { it.key }, selected = it.key == category) },
        swatches = section?.palette.orEmpty().map { CreatorSwatch(it, selected = sameColor(it, section?.color)) },
        options = section?.items.orEmpty().map { CreatorOption(it.id, it.name, it.thumbnailFile, it.selected) },
    )
}

class CreatorViewModel(private val creator: CreatorRepository) : ViewModel() {
    val ui: StateFlow<CreatorUiState> = creator.state.map { it.toUi() }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), CreatorUiState())

    fun selectTab(key: String) = creator.selectCategory(key)
    fun pick(id: String) = creator.pick(id)
    fun color(hex: String) = creator.color(hex)
    fun randomize() = creator.randomize()
    fun rotateLeft() = creator.rotate(-ROTATE_STEP)
    fun rotateRight() = creator.rotate(ROTATE_STEP)
    fun save() = creator.save()
    fun close() = creator.cancel()
}
