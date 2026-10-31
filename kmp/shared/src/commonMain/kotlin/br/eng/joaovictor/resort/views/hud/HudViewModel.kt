package br.eng.joaovictor.resort.views.hud

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import br.eng.joaovictor.resort.repositories.BuildModeRepository
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn

data class HudUiState(
    val buildMode: Boolean = false,
    val canBuild: Boolean = false,
)

class HudViewModel(private val build: BuildModeRepository) : ViewModel() {
    val ui: StateFlow<HudUiState> = combine(build.active, build.canBuild) { active, can ->
        HudUiState(buildMode = active, canBuild = can)
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), HudUiState())

    fun toggleBuild() = build.set(!ui.value.buildMode)
}
