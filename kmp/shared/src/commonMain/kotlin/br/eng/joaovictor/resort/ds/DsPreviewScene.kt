package br.eng.joaovictor.resort.ds

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import br.eng.joaovictor.resort.ds.theme.ResortTheme

/**
 * Backdrop for previews: a scene-like gradient under [ResortTheme], so translucent overlays read the way
 * they do over the 3D room. Preview-only; `views/preview/PreviewScene` delegates here.
 */
@Composable
internal fun DsPreviewScene(content: @Composable () -> Unit) {
    ResortTheme {
        Box(
            Modifier.fillMaxSize().background(
                Brush.verticalGradient(listOf(Color(0xFF1B3A4B), Color(0xFF2F5D50), Color(0xFF3B3226))),
            ),
        ) { content() }
    }
}
