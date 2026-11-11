package br.eng.joaovictor.resort.views.hud

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.staticCompositionLocalOf
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/** How much of the HUD shows. The most restrictive request wins: Hidden over Compact over Full. */
enum class HudMode { Full, Compact, Hidden }

/** Pure resolution of the mode from who asks for what. */
fun resolveHudMode(hiders: Set<String>, compacters: Set<String>): HudMode = when {
    hiders.isNotEmpty() -> HudMode.Hidden
    compacters.isNotEmpty() -> HudMode.Compact
    else -> HudMode.Full
}

/**
 * Lets Compose screens shrink or hide the HUD while they are on screen. Each owner (any stable name,
 * e.g. `"shop"`) asks for [hide] or [compact]; the HUD takes the most restrictive request. Asking twice,
 * or releasing an owner that holds nothing, is a no-op.
 */
class HudVisibility {
    private data class Holds(val hiders: Set<String> = emptySet(), val compacters: Set<String> = emptySet())

    private val holds = MutableStateFlow(Holds())
    private val _mode = MutableStateFlow(HudMode.Full)
    private val _visible = MutableStateFlow(true)

    val mode: StateFlow<HudMode> = _mode.asStateFlow()

    /** False only while the mode is [HudMode.Hidden]. */
    val visible: StateFlow<Boolean> = _visible.asStateFlow()

    fun hide(owner: String) = set { it.copy(hiders = it.hiders + owner) }

    /** Panels: only the room pill and the coins stay. */
    fun compact(owner: String) = set { it.copy(compacters = it.compacters + owner) }

    fun release(owner: String) = set { Holds(it.hiders - owner, it.compacters - owner) }

    /** Same as [release]; kept for the screens that still use `hide`/`show`. */
    fun show(owner: String) = release(owner)

    private fun set(change: (Holds) -> Holds) {
        holds.update(change)
        val mode = resolveHudMode(holds.value.hiders, holds.value.compacters)
        _mode.value = mode
        _visible.value = mode != HudMode.Hidden
    }
}

/** Provided by the shell (`App`); `null` outside it, where hiding the HUD does nothing. */
val LocalHudVisibility = staticCompositionLocalOf<HudVisibility?> { null }

/**
 * Call this from a full-screen Compose screen: the HUD hides when it enters the composition and comes
 * back when it leaves.
 */
@Composable
fun HideHudWhileVisible(owner: String, controller: HudVisibility? = LocalHudVisibility.current) {
    if (controller == null) return
    DisposableEffect(controller, owner) {
        controller.hide(owner)
        onDispose { controller.release(owner) }
    }
}

/** Like [HideHudWhileVisible], but the HUD only shrinks to the room pill and the coins. */
@Composable
fun CompactHudWhileVisible(owner: String, controller: HudVisibility? = LocalHudVisibility.current) {
    if (controller == null) return
    DisposableEffect(controller, owner) {
        controller.compact(owner)
        onDispose { controller.release(owner) }
    }
}
