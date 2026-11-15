package br.eng.joaovictor.resort.ds.layout

import androidx.compose.runtime.staticCompositionLocalOf

/**
 * Knows which panel holds the slot, so the UI keeps "one panel at a time" without a double animation: a
 * panel that opens while another still holds the slot ([claim] returns true) does not slide in, it fades
 * in over where the other was. Plain state (no Compose), so it is testable.
 */
class PanelHostState {
    var active: String? = null
        private set

    /** Takes the slot; true when a different panel held it (a swap). */
    fun claim(key: String): Boolean {
        val swap = active != null && active != key
        active = key
        return swap
    }

    /** Frees the slot, but only if [key] still holds it (a newer panel may have taken it). */
    fun release(key: String) {
        if (active == key) active = null
    }
}

/** Provided by the shell (`App`); `null` outside it, where every panel just slides in. */
val LocalPanelHost = staticCompositionLocalOf<PanelHostState?> { null }
