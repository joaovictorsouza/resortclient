package br.eng.joaovictor.resort.views.hud

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.staticCompositionLocalOf
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/**
 * Lets Compose screens hide the HUD while they are on screen. The HUD stays hidden while any owner
 * holds it, so two screens never fight over it; the owner is any stable name (e.g. `"shop"`).
 * Hiding twice or releasing an owner that does not hold it is a no-op.
 */
class HudVisibility {
    private val owners = MutableStateFlow<Set<String>>(emptySet())
    private val _visible = MutableStateFlow(true)

    /** False while at least one owner holds the HUD hidden. */
    val visible: StateFlow<Boolean> = _visible.asStateFlow()

    fun hide(owner: String) = set { it + owner }

    fun show(owner: String) = set { it - owner }

    private fun set(change: (Set<String>) -> Set<String>) {
        owners.update(change)
        _visible.value = owners.value.isEmpty()
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
        onDispose { controller.show(owner) }
    }
}
