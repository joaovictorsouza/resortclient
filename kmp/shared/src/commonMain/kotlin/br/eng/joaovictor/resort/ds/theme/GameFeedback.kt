package br.eng.joaovictor.resort.ds.theme

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.hapticfeedback.HapticFeedback
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback

enum class Haptic { Tap, Select, Confirm, Error, DragStart }

enum class UiSound { Tap, Open, Close, Select, Purchase, Coin, Error, Doorbell }

/** Haptics and sound as one seam, so the implementation changes in one place. Previews use [None]. */
interface GameFeedback {
    fun haptic(kind: Haptic)
    fun sound(id: UiSound)

    object None : GameFeedback {
        override fun haptic(kind: Haptic) {}
        override fun sound(id: UiSound) {}
    }
}

/** Real haptics through the platform; sound stays a no-op until the Bevy `ui_sound` message exists. */
class PlatformGameFeedback(private val haptics: HapticFeedback) : GameFeedback {
    override fun haptic(kind: Haptic) {
        haptics.performHapticFeedback(hapticTypeFor(kind))
    }

    override fun sound(id: UiSound) {}
}

internal fun hapticTypeFor(kind: Haptic): HapticFeedbackType = when (kind) {
    Haptic.Tap -> HapticFeedbackType.LongPress
    Haptic.Select -> HapticFeedbackType.TextHandleMove
    Haptic.Confirm -> HapticFeedbackType.Confirm
    Haptic.Error -> HapticFeedbackType.Reject
    Haptic.DragStart -> HapticFeedbackType.LongPress
}

@Composable
fun rememberGameFeedback(): GameFeedback {
    val haptics = LocalHapticFeedback.current
    return remember(haptics) { PlatformGameFeedback(haptics) }
}
