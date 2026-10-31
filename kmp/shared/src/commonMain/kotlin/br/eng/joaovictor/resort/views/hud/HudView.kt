package br.eng.joaovictor.resort.views.hud

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/** Example HUD screen: Repository -> ViewModel -> View. Not wired into the app yet. */
@Composable
fun HudView(viewModel: HudViewModel) {
    val ui by viewModel.ui.collectAsState()
    Box(Modifier.fillMaxSize().safeDrawingPadding().padding(16.dp)) {
        if (ui.buildMode) {
            Text("Modo construção", Modifier.align(Alignment.TopCenter))
        }
        if (ui.canBuild) {
            FilledTonalButton(onClick = viewModel::toggleBuild, modifier = Modifier.align(Alignment.BottomEnd)) {
                Text(if (ui.buildMode) "Sair da construção" else "Construir")
            }
        }
    }
}
