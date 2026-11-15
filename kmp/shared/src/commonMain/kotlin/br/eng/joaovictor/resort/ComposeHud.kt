package br.eng.joaovictor.resort

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.game.BevyJson
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/** What the game mirrors for the Compose HUD test (`hud_state`). */
@Serializable
data class HudState(
    val name: String = "",
    val silver: Long = 0,
    val gold: Long = 0,
    @SerialName("in_room") val inRoom: Boolean = false,
    val reconnecting: Boolean = false,
)

fun parseHud(json: String): HudState? = BevyJson.decodeOrNull<HudState>(json)

/**
 * Feasibility test: a translucent bar, a [AlertDialog] and a [ModalBottomSheet] over the 3D scene,
 * next to (not instead of) the game's own bevy_hui HUD. Only shown with `COMPOSE_HUD=1`.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ComposeHudTest(hud: HudState, fps: Float, onToggleNavigator: () -> Unit) {
    if (!hud.inRoom) return
    var dialog by remember { mutableStateOf(false) }
    var sheet by remember { mutableStateOf(false) }
    Box(Modifier.fillMaxSize().safeDrawingPadding().padding(12.dp)) {
        Row(
            Modifier.align(Alignment.TopCenter)
                .background(Color.Black.copy(alpha = 0.35f), RoundedCornerShape(20.dp))
                .padding(horizontal = 14.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("Compose HUD · ${hud.name}", color = Color.White)
            Text("prata ${hud.silver} · ouro ${hud.gold}", color = Color.White)
            Text("${fps.toInt()} fps", color = Color.White)
            if (hud.reconnecting) Text("reconectando…", color = Color(0xFFFFC107))
        }
        Row(Modifier.align(Alignment.BottomStart), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = onToggleNavigator) { Text("Navegador") }
            OutlinedButton(onClick = { dialog = true }) { Text("Dialog") }
            OutlinedButton(onClick = { sheet = true }) { Text("Sheet") }
        }
    }
    if (dialog) {
        AlertDialog(
            onDismissRequest = { dialog = false },
            title = { Text("Dialog do Compose") },
            text = { Text("Sobre a cena 3D, com o jogo rodando atrás.") },
            confirmButton = { TextButton(onClick = { dialog = false }) { Text("Fechar") } },
        )
    }
    if (sheet) {
        ModalBottomSheet(onDismissRequest = { sheet = false }) {
            Column(Modifier.fillMaxWidth().padding(24.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Bottom sheet do Compose")
                Text("Prata ${hud.silver} · ouro ${hud.gold}")
                TextButton(onClick = { sheet = false }) { Text("Fechar") }
            }
        }
    }
}
