package br.eng.joaovictor.resort.views.creator

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import br.eng.joaovictor.resort.components.GameThumbnail
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene
import androidx.compose.ui.tooling.preview.Preview

private val Selected = Color(0xFF4F46E5)
private val Confirm = Color(0xFF059669)

/** What the creator can ask for; previews pass no-ops. */
class CreatorActions(
    val onTab: (String) -> Unit = {},
    val onPick: (String) -> Unit = {},
    val onColor: (String) -> Unit = {},
    val onRandomize: () -> Unit = {},
    val onRotateLeft: () -> Unit = {},
    val onRotateRight: () -> Unit = {},
    val onSave: () -> Unit = {},
    val onClose: () -> Unit = {},
)

/** Collects the state; the creator's 3D preview is drawn by the game behind this panel. */
@Composable
fun CreatorView(viewModel: CreatorViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.active) return
    val actions = remember(viewModel) {
        CreatorActions(
            viewModel::selectTab, viewModel::pick, viewModel::color, viewModel::randomize,
            viewModel::rotateLeft, viewModel::rotateRight, viewModel::save, viewModel::close,
        )
    }
    CreatorContent(ui, actions)
}

/**
 * Stateless creator controls: rotate/randomize on the left, the customizer panel on the right. The rest
 * of the screen is left empty so the game's 3D avatar preview shows through (and nothing there takes touches).
 */
@Composable
fun CreatorContent(ui: CreatorUiState, actions: CreatorActions) {
    BoxWithConstraints(Modifier.fillMaxSize().safeDrawingPadding().padding(16.dp)) {
        Column(Modifier.align(Alignment.BottomStart), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            RoundButton("↺", actions.onRotateLeft)
            RoundButton("↻", actions.onRotateRight)
            Button(onClick = actions.onRandomize) { Text("🎲 Aleatório") }
        }
        Surface(
            modifier = Modifier.align(Alignment.CenterEnd).width(minOf(440.dp, maxWidth * 0.42f)).fillMaxHeight(),
            shape = RoundedCornerShape(16.dp),
            color = MaterialColor,
            border = androidx.compose.foundation.BorderStroke(1.dp, Color.White.copy(alpha = 0.15f)),
        ) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text("Seu avatar", fontSize = 20.sp)
                        Text("Personalize cada detalhe do seu personagem", fontSize = 12.sp, color = Color.White.copy(alpha = 0.6f))
                    }
                    // Only when editing: the first creation cannot be skipped.
                    if (ui.editing) RoundButton("✕", actions.onClose, size = 40.dp)
                }
                LazyRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    items(ui.tabs, key = { it.key }) { tab ->
                        Text(
                            tab.label,
                            color = Color.White,
                            fontSize = 13.sp,
                            modifier = Modifier
                                .background(if (tab.selected) Selected else Color.White.copy(alpha = 0.12f), RoundedCornerShape(14.dp))
                                .clickable { actions.onTab(tab.key) }
                                .padding(horizontal = 12.dp, vertical = 6.dp),
                        )
                    }
                }
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    if (ui.swatches.isNotEmpty()) Swatches(ui, actions)
                    if (ui.options.isNotEmpty()) OptionGrid(ui, actions, Modifier.weight(1f))
                }
                Button(
                    onClick = actions.onSave,
                    modifier = Modifier.fillMaxWidth(),
                ) { Text(ui.confirmLabel) }
            }
        }
    }
}

private val MaterialColor = Color(0xF20F172A)

@Composable
private fun Swatches(ui: CreatorUiState, actions: CreatorActions) {
    LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        items(ui.swatches, key = { it.hex }) { sw ->
            Box(
                Modifier.size(32.dp)
                    .background(parseHex(sw.hex), CircleShape)
                    .border(if (sw.selected) 3.dp else 1.dp, if (sw.selected) Color.White else Color.White.copy(alpha = 0.3f), CircleShape)
                    .clickable { actions.onColor(sw.hex) },
            )
        }
    }
}

@Composable
private fun OptionGrid(ui: CreatorUiState, actions: CreatorActions, modifier: Modifier) {
    LazyVerticalGrid(
        columns = GridCells.Adaptive(minSize = 72.dp),
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(ui.options, key = { it.id }) { option ->
            val shape = RoundedCornerShape(10.dp)
            Box(
                Modifier.fillMaxWidth().aspectRatio(1f).clip(shape)
                    .background(if (option.selected) Selected.copy(alpha = 0.35f) else Color.White.copy(alpha = 0.06f))
                    .border(if (option.selected) 2.dp else 1.dp, if (option.selected) Selected else Color.White.copy(alpha = 0.12f), shape)
                    .clickable { actions.onPick(option.id) },
            ) {
                GameThumbnail(option.thumbnailFile, option.name, Modifier.fillMaxSize().padding(bottom = 18.dp))
                Text(
                    option.name,
                    color = Color.White,
                    fontSize = 11.sp,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.align(Alignment.BottomCenter).fillMaxWidth()
                        .background(Color.Black.copy(alpha = 0.45f)).padding(horizontal = 4.dp, vertical = 2.dp),
                )
            }
        }
    }
}

@Composable
private fun RoundButton(glyph: String, onClick: () -> Unit, size: Dp = 48.dp) {
    Box(
        Modifier.size(size).background(Color(0xD90F172A), CircleShape).clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) { Text(glyph, fontSize = 20.sp, color = Color.White) }
}

/** `#RRGGBB` to a color; anything unparsable is a neutral grey. */
internal fun parseHex(hex: String): Color {
    val v = hex.removePrefix("#").toLongOrNull(16) ?: return Color.Gray
    return Color(0xFF000000 or v)
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

@Preview(name = "Creator, first creation (hair tab)", widthDp = 800, heightDp = 360)
@Composable
private fun CreatorFirstPreview() = PreviewScene { CreatorContent(PreviewData.creatorHair, CreatorActions()) }

@Preview(name = "Creator, editing (close button)", widthDp = 800, heightDp = 360)
@Composable
private fun CreatorEditingPreview() = PreviewScene { CreatorContent(PreviewData.creatorHair.copy(editing = true, confirmLabel = confirmLabel(true)), CreatorActions()) }

@Preview(name = "Creator, skin (swatches only)", widthDp = 800, heightDp = 360)
@Composable
private fun CreatorSkinPreview() = PreviewScene { CreatorContent(PreviewData.creatorSkin, CreatorActions()) }

@Preview(name = "Creator, body type", widthDp = 800, heightDp = 360)
@Composable
private fun CreatorBodyPreview() = PreviewScene { CreatorContent(PreviewData.creatorBody, CreatorActions()) }

@Preview(name = "Creator (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun CreatorPortraitPreview() = PreviewScene { CreatorContent(PreviewData.creatorHair, CreatorActions()) }
