package br.eng.joaovictor.resort.views.preview

import androidx.compose.runtime.Composable
import br.eng.joaovictor.resort.ds.DsPreviewScene

/**
 * Backdrop for previews: a dark, scene-like gradient under the app theme ([br.eng.joaovictor.resort.ds.theme.ResortTheme]),
 * so translucent overlays (HUD, panels) read the way they do over the 3D room. Preview-only.
 */
@Composable
internal fun PreviewScene(content: @Composable () -> Unit) = DsPreviewScene(content)
