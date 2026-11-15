package br.eng.joaovictor.resort.views.roomui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import br.eng.joaovictor.resort.components.GameSearchBar
import br.eng.joaovictor.resort.components.GameWindow
import br.eng.joaovictor.resort.models.DoorbellInfo
import br.eng.joaovictor.resort.models.FurnitureMenuInfo
import br.eng.joaovictor.resort.models.SettingsInfo
import br.eng.joaovictor.resort.views.hud.HideHudWhileVisible
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene
import androidx.compose.ui.tooling.preview.Preview

private val Selected = Color(0xFF4F46E5)
private val Danger = Color(0xFFE11D48)
private val Card = Color(0xF20F172A)

/** What the in-room popups can ask for; previews pass no-ops. */
class RoomUiActions(
    val onCloseMenu: () -> Unit = {},
    val onVisuals: () -> Unit = {},
    val onSignOut: () -> Unit = {},
    val onName: (String) -> Unit = {},
    val onVisibility: (RoomVisibility) -> Unit = {},
    val onSave: () -> Unit = {},
    val onAskDelete: () -> Unit = {},
    val onCancelDelete: () -> Unit = {},
    val onConfirmDelete: () -> Unit = {},
    val onCloseSettings: () -> Unit = {},
    val onDoorbell: (String, Boolean) -> Unit = { _, _ -> },
    val onFurnitureRotate: (String) -> Unit = {},
    val onFurnitureMove: (String) -> Unit = {},
    val onFurniturePickup: (String) -> Unit = {},
    val onFurnitureClose: () -> Unit = {},
)

/** Collects the popups' state: avatar menu, room settings window and the doorbell. */
@Composable
fun RoomUiView(viewModel: RoomUiViewModel) {
    val ui by viewModel.ui.collectAsState()
    val actions = remember(viewModel) {
        RoomUiActions(
            viewModel::closeAvatarMenu, viewModel::visuals, viewModel::signOut, viewModel::setName, viewModel::setVisibility,
            viewModel::save, viewModel::askDelete, viewModel::cancelDelete, viewModel::confirmDelete,
            viewModel::closeSettings, viewModel::answerDoorbell, viewModel::furnitureRotate, viewModel::furnitureMove,
            viewModel::furniturePickup, viewModel::furnitureClose,
        )
    }
    if (ui.avatarMenu) AvatarMenu(actions)
    ui.settings?.let { RoomSettingsWindow(it, actions) }
    ui.doorbell?.let { Doorbell(it, actions) }
    ui.furnitureMenu?.let { FurnitureMenu(it, actions) }
}

/**
 * The menu under the HUD's avatar button: "Visuais" and "Sair". A transparent backdrop closes it
 * (it takes the touch, like the bevy_hui one).
 */
@Composable
fun AvatarMenu(actions: RoomUiActions) {
    Box(Modifier.fillMaxSize().clickable(onClick = actions.onCloseMenu)) {
        Column(
            Modifier.safeDrawingPadding().padding(top = 68.dp, end = 12.dp).align(Alignment.TopEnd).widthIn(min = 160.dp)
                .background(Card, RoundedCornerShape(12.dp)).padding(6.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            MenuRow("Visuais", Color.White.copy(alpha = 0.08f), actions.onVisuals)
            MenuRow("Sair", Danger.copy(alpha = 0.55f), actions.onSignOut)
        }
    }
}

@Composable
private fun MenuRow(text: String, bg: Color, onClick: () -> Unit) {
    Text(text, color = Color.White, fontSize = 15.sp, modifier = Modifier.fillMaxWidth().background(bg, RoundedCornerShape(8.dp)).clickable(onClick = onClick).padding(12.dp))
}

/** The room settings window (owner): name, visibility, save and a confirmed delete. */
@Composable
fun RoomSettingsWindow(settings: SettingsInfo, actions: RoomUiActions) {
    HideHudWhileVisible("room-settings")
    // The field keeps its own text so typing never fights the round trip to the game.
    var name by remember { mutableStateOf(settings.name) }
    GameWindow(title = "Configurações do espaço", onClose = actions.onCloseSettings, maxWidth = 460.dp, widthFraction = 0.8f, heightFraction = 0.8f) {
        Column(Modifier.weight(1f).padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text("Nome do espaço (até $MAX_SETTINGS_NAME_CHARS caracteres)", fontSize = 12.sp, color = Color.White.copy(alpha = 0.7f))
            GameSearchBar(name, { name = clampSettingsName(it); actions.onName(name) }, "Nome do espaço", Modifier.fillMaxWidth())
            Text("Visibilidade de acesso", fontSize = 12.sp, color = Color.White.copy(alpha = 0.7f))
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                RoomVisibility.entries.forEach { v ->
                    val on = v.key == settings.visibility
                    Text(
                        v.label,
                        color = Color.White,
                        modifier = Modifier
                            .background(if (on) Selected else Color.White.copy(alpha = 0.12f), RoundedCornerShape(14.dp))
                            .border(if (on) 2.dp else 0.dp, if (on) Color.White.copy(alpha = 0.6f) else Color.Transparent, RoundedCornerShape(14.dp))
                            .clickable { actions.onVisibility(v) }
                            .padding(horizontal = 14.dp, vertical = 8.dp),
                    )
                }
            }
            Button(onClick = actions.onSave, modifier = Modifier.fillMaxWidth()) { Text("Salvar alterações") }
            if (settings.confirmingDelete) {
                Text("Tem certeza que deseja excluir esta sala? Você e todos os avatares retornarão ao Lobby.", color = Color(0xFFFB7185), fontSize = 13.sp)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(onClick = actions.onConfirmDelete, colors = ButtonDefaults.buttonColors(containerColor = Danger)) { Text("Sim, excluir") }
                    OutlinedButton(onClick = actions.onCancelDelete) { Text("Cancelar") }
                }
            } else {
                OutlinedButton(onClick = actions.onAskDelete, modifier = Modifier.fillMaxWidth()) { Text("Excluir espaço", color = Color(0xFFFB7185)) }
            }
        }
    }
}

/** A visitor rang the doorbell of the owner's room: let them in or not. */
@Composable
fun Doorbell(info: DoorbellInfo, actions: RoomUiActions) {
    Box(Modifier.fillMaxSize().safeDrawingPadding().padding(top = 76.dp), contentAlignment = Alignment.TopCenter) {
        Column(
            Modifier.widthIn(max = 420.dp).fillMaxWidth(0.96f).background(Card, RoundedCornerShape(16.dp)).padding(20.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("🔔 ${info.requesterName} quer entrar", fontSize = 16.sp)
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Button(onClick = { actions.onDoorbell(info.requestId, true) }) { Text("Permitir") }
                OutlinedButton(onClick = { actions.onDoorbell(info.requestId, false) }) { Text("Negar") }
            }
        }
    }
}

/**
 * Options of a placed item in build mode: rotate, move, pick up. A card at the bottom of the screen so
 * the room stays visible; touches elsewhere reach the game (tapping the floor dismisses it there).
 */
@Composable
fun FurnitureMenu(info: FurnitureMenuInfo, actions: RoomUiActions) {
    Box(Modifier.fillMaxSize().safeDrawingPadding().padding(bottom = 72.dp), contentAlignment = Alignment.BottomCenter) {
        Column(
            Modifier.widthIn(max = 360.dp).background(Card, RoundedCornerShape(16.dp)).padding(14.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Text("Opções do móvel", fontSize = 15.sp)
                Text("✕", fontSize = 18.sp, modifier = Modifier.clickable(onClick = actions.onFurnitureClose).padding(6.dp))
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { actions.onFurnitureRotate(info.itemId) }) { Text("Girar 90°") }
                Button(onClick = { actions.onFurnitureMove(info.itemId) }) { Text("Mover") }
                Button(onClick = { actions.onFurniturePickup(info.itemId) }, colors = ButtonDefaults.buttonColors(containerColor = Danger)) { Text("Recolher") }
            }
        }
    }
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

@Preview(name = "Avatar menu", widthDp = 800, heightDp = 360)
@Composable
private fun AvatarMenuPreview() = PreviewScene { AvatarMenu(RoomUiActions()) }

@Preview(name = "Room settings", widthDp = 800, heightDp = 360)
@Composable
private fun RoomSettingsPreview() = PreviewScene { RoomSettingsWindow(PreviewData.roomSettings, RoomUiActions()) }

@Preview(name = "Room settings, confirming delete", widthDp = 800, heightDp = 360)
@Composable
private fun RoomSettingsDeletePreview() =
    PreviewScene { RoomSettingsWindow(PreviewData.roomSettings.copy(confirmingDelete = true), RoomUiActions()) }

@Preview(name = "Doorbell", widthDp = 800, heightDp = 360)
@Composable
private fun DoorbellPreview() = PreviewScene { Doorbell(PreviewData.doorbell, RoomUiActions()) }

@Preview(name = "Doorbell, long name (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun DoorbellPortraitPreview() =
    PreviewScene { Doorbell(PreviewData.doorbell.copy(requesterName = "Um visitante com um nome bem comprido mesmo"), RoomUiActions()) }

@Preview(name = "Furniture menu", widthDp = 800, heightDp = 360)
@Composable
private fun FurnitureMenuPreview() = PreviewScene { FurnitureMenu(FurnitureMenuInfo("i1", "chair"), RoomUiActions()) }
