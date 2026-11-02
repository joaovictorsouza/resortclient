package br.eng.joaovictor.resort.views.hud

import androidx.compose.runtime.Composable
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene
import org.jetbrains.compose.ui.tooling.preview.Preview

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
