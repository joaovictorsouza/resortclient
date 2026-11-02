package br.eng.joaovictor.resort.views.inventory

import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.ImageBitmap

/**
 * Loads a thumbnail from the local file the game cached; `null` when it is missing. The shell provides
 * the real loader (platform code); the default is a no-op so previews show the placeholder.
 */
val LocalThumbnailLoader = staticCompositionLocalOf<(String) -> ImageBitmap?> { { null } }
