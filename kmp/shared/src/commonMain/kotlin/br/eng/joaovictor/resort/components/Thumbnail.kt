package br.eng.joaovictor.resort.components

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Loads a thumbnail from the local file the game cached; `null` when it is missing. The shell provides
 * the real loader (platform code); the default is a no-op so previews show the placeholder.
 */
val LocalThumbnailLoader = staticCompositionLocalOf<(String) -> ImageBitmap?> { { null } }

/** The cached thumbnail at [path], decoded off the main thread; a neutral square while it is not available. */
@Composable
fun GameThumbnail(path: String, description: String, modifier: Modifier = Modifier) {
    val load = LocalThumbnailLoader.current
    val image: ImageBitmap? by produceState<ImageBitmap?>(null, path, load) {
        value = if (path.isEmpty()) null else withContext(Dispatchers.Default) { load(path) }
    }
    Box(modifier.background(Color.White.copy(alpha = 0.08f), RoundedCornerShape(8.dp)), contentAlignment = Alignment.Center) {
        image?.let { Image(it, description, Modifier.fillMaxSize().padding(2.dp)) }
    }
}
