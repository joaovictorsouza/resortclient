package br.eng.joaovictor.resort.views.roomui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import br.eng.joaovictor.resort.models.RoomUiState
import br.eng.joaovictor.resort.repositories.RoomUiRepository
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn

/** The room visibilities the settings offer, with the key the game uses. */
enum class RoomVisibility(val key: String, val label: String) {
    Public("public", "Pública"),
    Hidden("hidden", "Oculta"),
    Private("private", "Privada"),
    ;

    companion object {
        fun fromKey(key: String): RoomVisibility = entries.firstOrNull { it.key == key } ?: Private
    }
}

/** Longest room name the game accepts; the field cuts typing there. */
const val MAX_SETTINGS_NAME_CHARS = 50

fun clampSettingsName(text: String): String = text.take(MAX_SETTINGS_NAME_CHARS)

class RoomUiViewModel(private val roomUi: RoomUiRepository) : ViewModel() {
    val ui: StateFlow<RoomUiState> = roomUi.state
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), RoomUiState())

    fun closeAvatarMenu() = roomUi.closeAvatarMenu()
    fun visuals() = roomUi.visuals()
    fun signOut() = roomUi.signOut()

    fun setName(text: String) = roomUi.settingsName(clampSettingsName(text))
    fun setVisibility(visibility: RoomVisibility) = roomUi.settingsVisibility(visibility.key)
    fun save() = roomUi.settingsSave()
    fun askDelete() = roomUi.settingsAskDelete()
    fun cancelDelete() = roomUi.settingsCancelDelete()
    fun confirmDelete() = roomUi.settingsConfirmDelete()
    fun closeSettings() = roomUi.settingsClose()
    fun furnitureRotate(itemId: String) = roomUi.furnitureRotate(itemId)
    fun furnitureMove(itemId: String) = roomUi.furnitureMove(itemId)
    fun furniturePickup(itemId: String) = roomUi.furniturePickup(itemId)
    fun furnitureClose() = roomUi.furnitureMenuClose()
    fun answerDoorbell(requestId: String, allowed: Boolean) = roomUi.answerDoorbell(requestId, allowed)
}
