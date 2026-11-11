package br.eng.joaovictor.resort.views.hud

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.components.ActionDock
import br.eng.joaovictor.resort.ds.components.Currency
import br.eng.joaovictor.resort.ds.components.CurrencyPill
import br.eng.joaovictor.resort.ds.components.DockAction
import br.eng.joaovictor.resort.ds.components.GameIconButton
import br.eng.joaovictor.resort.ds.components.IconButtonStyle
import br.eng.joaovictor.resort.ds.components.RoomPill
import br.eng.joaovictor.resort.ds.components.ToastCard
import br.eng.joaovictor.resort.ds.components.ToastKind
import br.eng.joaovictor.resort.ds.components.ToastMessage
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.layout.PanelForm
import br.eng.joaovictor.resort.ds.layout.panelSpecFor
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize
import br.eng.joaovictor.resort.views.preview.PreviewData
import br.eng.joaovictor.resort.views.preview.PreviewScene

/** What the HUD buttons can ask for; previews pass no-ops. */
class HudActions(
    val onShop: () -> Unit = {},
    val onAvatar: () -> Unit = {},
    val onBuild: () -> Unit = {},
    val onSettings: () -> Unit = {},
    val onInventory: () -> Unit = {},
    val onNavigator: () -> Unit = {},
)

/** Dock buttons, in order. Pure data so the rule "who sees what" is tested without a window. */
enum class DockId { Navigator, Inventory, Shop, Build, BuildDone }

/**
 * Visitor: rooms, items, shop. Owner: the same plus Build. Owner in build mode: items (to place) and Done.
 * Room settings left the dock for the room pill.
 */
fun dockActions(ui: HudUiState): List<DockId> = when {
    ui.isOwner && ui.buildMode -> listOf(DockId.Inventory, DockId.BuildDone)
    ui.isOwner -> listOf(DockId.Navigator, DockId.Inventory, DockId.Shop, DockId.Build)
    else -> listOf(DockId.Navigator, DockId.Inventory, DockId.Shop)
}

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
        // The HUD must not swallow the scene: its numbers stop growing at 1.3x.
        val density = LocalDensity.current
        val capped = remember(density) { Density(density.density, density.fontScale.coerceAtMost(MAX_HUD_FONT_SCALE)) }
        CompositionLocalProvider(LocalDensity provides capped) { HudBody(ui, actions) }
    }
}

private const val MAX_HUD_FONT_SCALE = 1.3f

@Composable
private fun HudBody(ui: HudUiState, actions: HudActions) {
    BoxWithConstraints(Modifier.fillMaxSize().safeDrawingPadding().padding(Space.md)) {
        val spec = panelSpecFor(maxWidth, maxHeight)
        val landscape = spec.form == PanelForm.Side
        val compact = ui.mode == HudMode.Compact
        val ownerOpensSettings = ui.isOwner

        // Top left: the room pill (and the coins below it in portrait).
        Column(Modifier.align(Alignment.TopStart), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
            RoomPill(
                name = ui.roomName.ifEmpty { ui.roomTitle },
                visibilityIcon = visibilityIcon(ui.visibility),
                visibilityLabel = visibilityLabel(ui.visibility).lowercase(),
                onClick = if (ownerOpensSettings) actions.onSettings else actions.onNavigator,
            )
            if (!landscape) Wallet(ui, actions.onShop)
        }

        // Top right: coins (landscape; in compact they move left of the side panel) and the profile.
        Row(
            Modifier.align(Alignment.TopEnd).then(if (compact && landscape) Modifier.padding(end = spec.sideWidth + Space.md) else Modifier),
            horizontalArrangement = Arrangement.spacedBy(Space.sm),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (landscape) Wallet(ui, actions.onShop)
            if (!compact) {
                GameIconButton(GameIcons.User, "Perfil", actions.onAvatar, style = IconButtonStyle.Raised)
            }
        }

        // Top center: persistent state banners.
        Column(Modifier.align(Alignment.TopCenter), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(Space.xs)) {
            if (ui.reconnecting) ToastCard(RECONNECTING)
            if (ui.buildMode && !compact) BuildBadge()
        }

        // Bottom: the dock (Full only; panels have their own controls).
        if (ui.mode == HudMode.Full) {
            val dock = remember(ui.isOwner, ui.buildMode) { dockActions(ui).map(::dockAction) }
            ActionDock(
                dock,
                onAction = { key -> dispatch(DockId.valueOf(key), actions) },
                modifier = Modifier.align(if (landscape) Alignment.BottomEnd else Alignment.BottomCenter),
                showLabels = !landscape,
            )
        }
    }
}

private val RECONNECTING = ToastMessage(0, ToastKind.Warning, "Reconectando…", persistent = true)

@Composable
private fun Wallet(ui: HudUiState, onShop: () -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
        CurrencyPill(Currency.Silver, ui.silverAmount, onClick = onShop)
        CurrencyPill(Currency.Gold, ui.goldAmount, onClick = onShop)
    }
}

@Composable
private fun BuildBadge() {
    val colors = ResortTheme.colors
    Row(
        Modifier.background(colors.accent, Radius.pill).border(1.dp, colors.accentLip, Radius.pill).padding(horizontal = Space.md, vertical = Space.xs),
        horizontalArrangement = Arrangement.spacedBy(Space.xs),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(GameIcons.Hammer, null, tint = colors.onAccent, modifier = Modifier.size(DsSize.iconSmall))
        Text("MODO CONSTRUÇÃO", style = ResortTheme.type.badge, color = colors.onAccent)
    }
}

private fun visibilityIcon(visibility: String) = when (visibility) {
    "public" -> GameIcons.Globe
    "hidden" -> GameIcons.EyeOff
    else -> GameIcons.Lock
}

private fun dockAction(id: DockId): DockAction = when (id) {
    DockId.Navigator -> DockAction(id.name, "Salas", GameIcons.Compass)
    DockId.Inventory -> DockAction(id.name, "Itens", GameIcons.Backpack)
    DockId.Shop -> DockAction(id.name, "Loja", GameIcons.Shop, IconButtonStyle.Primary, large = true)
    DockId.Build -> DockAction(id.name, "Construir", GameIcons.Hammer, separatorBefore = true)
    DockId.BuildDone -> DockAction(id.name, "Concluir", GameIcons.Check, IconButtonStyle.Success, separatorBefore = true)
}

private fun dispatch(id: DockId, actions: HudActions) = when (id) {
    DockId.Navigator -> actions.onNavigator()
    DockId.Inventory -> actions.onInventory()
    DockId.Shop -> actions.onShop()
    DockId.Build, DockId.BuildDone -> actions.onBuild()
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

@Preview(name = "Visitor (no build)", widthDp = 852, heightDp = 393)
@Composable
private fun HudVisitorPreview() = PreviewScene { HudContent(InRoom, HudActions()) }

@Preview(name = "Compact (panel open)", widthDp = 800, heightDp = 360)
@Composable
private fun HudCompactPreview() = PreviewScene { HudContent(InRoom.copy(isOwner = true, mode = HudMode.Compact), HudActions()) }

@Preview(name = "Reconnecting", widthDp = 800, heightDp = 360)
@Composable
private fun HudReconnectingPreview() =
    PreviewScene { HudContent(InRoom.copy(reconnecting = true), HudActions()) }

@Preview(name = "Wallet loading (skeleton)", widthDp = 800, heightDp = 360)
@Composable
private fun HudWalletLoadingPreview() = PreviewScene {
    HudContent(InRoom.copy(silver = "-", gold = "-", silverAmount = null, goldAmount = null, isOwner = true), HudActions())
}

@Preview(name = "Long room name", widthDp = 800, heightDp = 360)
@Composable
private fun HudLongNamePreview() = PreviewScene {
    HudContent(InRoom.copy(roomName = "A sala mais comprida de todo o resort, com um nome absurdamente longo", visibility = "hidden"), HudActions())
}

@Preview(name = "Portrait", widthDp = 360, heightDp = 780)
@Composable
private fun HudPortraitPreview() = PreviewScene { HudContent(InRoom.copy(isOwner = true), HudActions()) }

@Preview(name = "Portrait, build mode", widthDp = 360, heightDp = 780)
@Composable
private fun HudPortraitBuildPreview() = PreviewScene { HudContent(InRoom.copy(isOwner = true, buildMode = true), HudActions()) }

@Preview(name = "Portrait, font scale 1.3", widthDp = 360, heightDp = 780, fontScale = 1.3f)
@Composable
private fun HudPortraitLargeFontPreview() = PreviewScene { HudContent(InRoom.copy(isOwner = true), HudActions()) }

@Preview(name = "Hidden (renders nothing)", widthDp = 800, heightDp = 360)
@Composable
private fun HudHiddenPreview() = PreviewScene { HudContent(InRoom.copy(visible = false, mode = HudMode.Hidden), HudActions()) }
