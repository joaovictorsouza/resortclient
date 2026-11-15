package br.eng.joaovictor.resort.views.preview

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color

/**
 * Backdrop for previews: a dark, scene-like gradient under the app theme, so translucent overlays
 * (HUD, floating windows) read the way they do over the 3D room. Preview-only.
 */
@Composable
internal fun PreviewScene(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = darkColorScheme()) {
        Box(
            Modifier.fillMaxSize().background(
                Brush.verticalGradient(listOf(Color(0xFF1B3A4B), Color(0xFF2F5D50), Color(0xFF3B3226))),
            ),
        ) { content() }
    }
}
