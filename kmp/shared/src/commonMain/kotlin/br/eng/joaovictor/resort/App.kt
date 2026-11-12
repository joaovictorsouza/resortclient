package br.eng.joaovictor.resort

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.remember
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import br.eng.joaovictor.bevykmp.BevyHostEffects
import br.eng.joaovictor.bevykmp.BevyView
import androidx.lifecycle.viewmodel.compose.viewModel
import br.eng.joaovictor.resort.components.LocalThumbnailLoader
import br.eng.joaovictor.resort.ds.components.LocalToastHost
import br.eng.joaovictor.resort.ds.components.ToastHost
import br.eng.joaovictor.resort.ds.components.ToastHostState
import br.eng.joaovictor.resort.ds.layout.LocalPanelHost
import br.eng.joaovictor.resort.ds.layout.PanelHostState
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.rememberGameFeedback
import br.eng.joaovictor.resort.game.GameHost
import br.eng.joaovictor.resort.views.creator.CreatorView
import br.eng.joaovictor.resort.views.creator.CreatorViewModel
import br.eng.joaovictor.resort.views.hud.HudView
import br.eng.joaovictor.resort.views.hud.LocalHudVisibility
import br.eng.joaovictor.resort.views.hud.HudViewModel
import br.eng.joaovictor.resort.views.inventory.InventoryView
import br.eng.joaovictor.resort.views.inventory.InventoryViewModel
import br.eng.joaovictor.resort.views.navigator.NavigatorView
import br.eng.joaovictor.resort.views.navigator.NavigatorViewModel
import br.eng.joaovictor.resort.views.roomui.RoomUiView
import br.eng.joaovictor.resort.views.roomui.RoomUiViewModel
import br.eng.joaovictor.resort.views.shop.ShopView
import br.eng.joaovictor.resort.views.shop.ShopViewModel

/**
 * The whole screen is the Bevy view: the game's UI is still bevy_hui. Native screens (the
 * onboarding, the inventory) are drawn above it by the shell.
 */
@Composable
fun App(game: GameHost) {
    ResortTheme(feedback = rememberGameFeedback()) {
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
            val hudViewModel = viewModel {
                HudViewModel(
                    game.gameState, game.wallet, game.buildMode, game.shop, game.navigator,
                    game.inventory, game.hudCommands, game.hudVisibility,
                )
            }
            val toastHost = remember { ToastHostState() }
            val panelHost = remember { PanelHostState() }
            CompositionLocalProvider(
                LocalToastHost provides toastHost,
                LocalPanelHost provides panelHost,
                LocalHudVisibility provides game.hudVisibility,
                LocalThumbnailLoader provides ::loadImageFile,
            ) {
                HudView(hudViewModel)
                val inventoryViewModel = viewModel { InventoryViewModel(game.inventory) }
                InventoryView(inventoryViewModel)
                val shopViewModel = viewModel { ShopViewModel(game.shop, game.wallet) }
                ShopView(shopViewModel)
                val navigatorViewModel = viewModel { NavigatorViewModel(game.navigator) }
                NavigatorView(navigatorViewModel)
                val creatorViewModel = viewModel { CreatorViewModel(game.creator) }
                CreatorView(creatorViewModel)
                val roomUiViewModel = viewModel { RoomUiViewModel(game.roomUi) }
                RoomUiView(roomUiViewModel)
                ToastHost(toastHost, onAction = {}, modifier = Modifier.safeDrawingPadding().padding(top = 8.dp))
            }
            error?.let { Text("Bevy error: $it", color = MaterialTheme.colorScheme.error) }
        }
    }
}
