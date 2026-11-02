package br.eng.joaovictor.resort.views.hud

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene
import org.jetbrains.compose.ui.tooling.preview.Preview

private val Silver = Color(0xFFC0C6D0)
private val Gold = Color(0xFFFFC107)
private val Chip = Color(0xB3020617)

/** What the HUD buttons can ask for; previews pass no-ops. */
class HudActions(
    val onShop: () -> Unit = {},
    val onAvatar: () -> Unit = {},
    val onBuild: () -> Unit = {},
    val onSettings: () -> Unit = {},
    val onInventory: () -> Unit = {},
    val onNavigator: () -> Unit = {},
)

/** The in-room HUD (replaces the bevy_hui one under the KMP host). Collects the state and delegates to [HudContent]. */
@Composable
fun HudView(viewModel: HudViewModel) {
    val ui by viewModel.ui.collectAsState()
    val actions = remember(viewModel) {
        HudActions(
            onShop = viewModel::toggleShop,
            onAvatar = viewModel::toggleAvatarMenu,
            onBuild = viewModel::toggleBuild,
            onSettings = viewModel::toggleRoomSettings,
            onInventory = viewModel::toggleInventory,
            onNavigator = viewModel::toggleNavigator,
        )
    }
    HudContent(ui, actions)
}

/**
 * Stateless HUD: plain UI state in, callbacks out, so it can be previewed without a ViewModel or the
 * game. Touches outside the controls reach the game; hidden, it leaves the composition.
 */
@Composable
fun HudContent(ui: HudUiState, actions: HudActions) {
    AnimatedVisibility(
        visible = ui.visible,
        enter = fadeIn() + slideInVertically { -it / 4 },
        exit = fadeOut() + slideOutVertically { -it / 4 },
    ) {
        HudBody(ui, actions)
    }
}

@Composable
private fun HudBody(ui: HudUiState, actions: HudActions) {
    Box(Modifier.fillMaxSize().safeDrawingPadding().padding(12.dp)) {
        Text(
            ui.roomTitle,
            color = Color(0xFFE6EFFF),
            fontSize = 16.sp,
            modifier = Modifier.align(Alignment.TopStart),
        )
        if (ui.reconnecting) {
            Banner("Reconectando…", Color(0xFFFFC107), Modifier.align(Alignment.TopCenter))
        } else if (ui.buildMode) {
            Banner("Modo construção", Color(0xFFB3722F), Modifier.align(Alignment.TopCenter))
        }
        Row(
            Modifier.align(Alignment.TopEnd),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            WalletChip(Silver, ui.silver)
            WalletChip(Gold, ui.gold)
            HudButton("🏪", Color(0xFFB88514), onClick = actions.onShop)
            HudButton("👤", Color(0xFF5940A6), onClick = actions.onAvatar)
        }
        Row(Modifier.align(Alignment.BottomEnd), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            if (ui.isOwner) {
                HudButton("🔨", Color(0xFFB37333), active = ui.buildMode, onClick = actions.onBuild)
                HudButton("⚙", Color(0xFF4D5973), onClick = actions.onSettings)
            }
            HudButton("🎒", Color(0xFF338C73), onClick = actions.onInventory)
            HudButton("🧭", Color(0xFF3359A6), onClick = actions.onNavigator)
        }
    }
}

@Composable
private fun Banner(text: String, color: Color, modifier: Modifier) {
    Text(
        text,
        color = Color.White,
        fontSize = 13.sp,
        fontWeight = FontWeight.Medium,
        modifier = modifier.background(color.copy(alpha = 0.85f), RoundedCornerShape(16.dp))
            .padding(horizontal = 12.dp, vertical = 4.dp),
    )
}

@Composable
private fun WalletChip(dot: Color, amount: String) {
    Row(
        Modifier.background(Chip, RoundedCornerShape(16.dp)).padding(horizontal = 10.dp, vertical = 6.dp),
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(10.dp).background(dot, CircleShape))
        Text(amount, color = Color.White, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
private fun HudButton(glyph: String, color: Color, active: Boolean = false, onClick: () -> Unit) {
    val shape = RoundedCornerShape(12.dp)
    Box(
        Modifier.size(44.dp)
            .background(color, shape)
            .then(if (active) Modifier.border(BorderStroke(3.dp, Color.White), shape) else Modifier)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Text(glyph, fontSize = 20.sp)
    }
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

private val InRoom = PreviewData.hudInRoom

@Preview(name = "Owner, build off (landscape)", widthDp = 800, heightDp = 360)
@Composable
private fun HudOwnerPreview() = PreviewScene { HudContent(InRoom.copy(isOwner = true), HudActions()) }

@Preview(name = "Owner, build mode on", widthDp = 800, heightDp = 360)
@Composable
private fun HudBuildModePreview() =
    PreviewScene { HudContent(InRoom.copy(isOwner = true, buildMode = true), HudActions()) }

@Preview(name = "Visitor (no build/settings)", widthDp = 800, heightDp = 360)
@Composable
private fun HudVisitorPreview() = PreviewScene { HudContent(InRoom, HudActions()) }

@Preview(name = "Reconnecting", widthDp = 800, heightDp = 360)
@Composable
private fun HudReconnectingPreview() =
    PreviewScene { HudContent(InRoom.copy(reconnecting = true), HudActions()) }

@Preview(name = "Wallet loading ('-')", widthDp = 800, heightDp = 360)
@Composable
private fun HudWalletLoadingPreview() =
    PreviewScene { HudContent(InRoom.copy(silver = "-", gold = "-", isOwner = true), HudActions()) }

@Preview(name = "Long room name", widthDp = 800, heightDp = 360)
@Composable
private fun HudLongNamePreview() = PreviewScene {
    HudContent(InRoom.copy(roomTitle = "A sala mais comprida de todo o resort, com um nome absurdamente longo  •  [Oculta]"), HudActions())
}

@Preview(name = "Portrait", widthDp = 360, heightDp = 780)
@Composable
private fun HudPortraitPreview() = PreviewScene { HudContent(InRoom.copy(isOwner = true), HudActions()) }

@Preview(name = "Hidden (renders nothing)", widthDp = 800, heightDp = 360)
@Composable
private fun HudHiddenPreview() = PreviewScene { HudContent(InRoom.copy(visible = false), HudActions()) }
