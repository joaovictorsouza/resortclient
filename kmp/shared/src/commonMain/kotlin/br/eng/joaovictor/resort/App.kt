package br.eng.joaovictor.resort

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import br.eng.joaovictor.bevykmp.BevyHostEffects
import br.eng.joaovictor.bevykmp.BevyView
import androidx.lifecycle.viewmodel.compose.viewModel
import br.eng.joaovictor.resort.game.GameHost
import br.eng.joaovictor.resort.views.inventory.InventoryView
import br.eng.joaovictor.resort.views.inventory.InventoryViewModel

/**
 * The whole screen is the Bevy view: the game's UI is still bevy_hui. Native screens (the
 * onboarding, the inventory) are drawn above it by the shell.
 */
@Composable
fun App(game: GameHost) {
    MaterialTheme(colorScheme = darkColorScheme()) {
        BevyHostEffects(game.backend)
        val error by game.error.collectAsState()
        val stage by game.stage.collectAsState()
        Box(Modifier.fillMaxSize()) {
            BevyView(game.backend, Modifier.fillMaxSize())
            OnboardingOverlay(
                stage = stage,
                onGoogle = game::onboardingGoogle,
                onDeviceLogin = game::onboardingDeviceLogin,
                onNickname = game::onboardingNickname,
                onReset = game::onboardingReset,
            )
            if (game.composeHud) {
                val hud by game.hud.collectAsState()
                val fps by game.fps.collectAsState()
                ComposeHudTest(hud, fps, game::hudToggleNavigator)
            }
            val inventoryViewModel = viewModel { InventoryViewModel(game.inventory) }
            InventoryView(inventoryViewModel)
            error?.let { Text("Bevy error: $it", color = MaterialTheme.colorScheme.error) }
        }
    }
}
