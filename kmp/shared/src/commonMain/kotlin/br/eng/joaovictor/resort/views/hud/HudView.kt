package br.eng.joaovictor.resort.views.hud

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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

private val Silver = Color(0xFFC0C6D0)
private val Gold = Color(0xFFFFC107)
private val Chip = Color(0xB3020617)

/** The in-room HUD (replaces the bevy_hui one under the KMP host). Touches outside the controls reach the game. */
@Composable
fun HudView(viewModel: HudViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.visible) return
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
            HudButton("🏪", Color(0xFFB88514), onClick = viewModel::toggleShop)
            HudButton("👤", Color(0xFF5940A6), onClick = viewModel::toggleAvatarMenu)
        }
        Row(Modifier.align(Alignment.BottomEnd), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            if (ui.isOwner) {
                HudButton("🔨", Color(0xFFB37333), active = ui.buildMode, onClick = viewModel::toggleBuild)
                HudButton("⚙", Color(0xFF4D5973), onClick = viewModel::toggleRoomSettings)
            }
            HudButton("🎒", Color(0xFF338C73), onClick = viewModel::toggleInventory)
            HudButton("🧭", Color(0xFF3359A6), onClick = viewModel::toggleNavigator)
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
