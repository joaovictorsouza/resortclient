package br.eng.joaovictor.resort.ds.theme

import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf

val LocalResortColors = staticCompositionLocalOf { ResortColors() }
val LocalResortTypography = staticCompositionLocalOf { ResortTypography() }
val LocalGameFeedback = staticCompositionLocalOf<GameFeedback> { GameFeedback.None }

/** True when the system asks for reduced motion: no stagger, no counting, instant swaps. Platform signal comes later. */
val LocalReduceMotion = staticCompositionLocalOf { false }

object ResortTheme {
    val colors: ResortColors @Composable @ReadOnlyComposable get() = LocalResortColors.current
    val type: ResortTypography @Composable @ReadOnlyComposable get() = LocalResortTypography.current
    val feedback: GameFeedback @Composable @ReadOnlyComposable get() = LocalGameFeedback.current
}

/** Maps the tokens onto a Material3 color scheme, so screens not migrated yet already pick up the palette. */
internal fun materialColorScheme(c: ResortColors) = darkColorScheme(
    primary = c.primary, onPrimary = c.onAccent,
    secondary = c.secondary, onSecondary = c.onAccent,
    tertiary = c.accent, onTertiary = c.onAccent,
    background = c.surfaceSunken, onBackground = c.text,
    surface = c.surface, onSurface = c.text,
    surfaceVariant = c.surfaceRaised, onSurfaceVariant = c.textMuted,
    outline = c.outline, error = c.danger, onError = c.onAccent,
)

/** The app theme: tokens, typography and feedback for the design system, plus the Material3 mapping. */
@Composable
fun ResortTheme(feedback: GameFeedback = GameFeedback.None, content: @Composable () -> Unit) {
    val colors = ResortColors()
    MaterialTheme(colorScheme = materialColorScheme(colors)) {
        CompositionLocalProvider(
            LocalResortColors provides colors,
            LocalResortTypography provides ResortTypography(),
            LocalGameFeedback provides feedback,
            LocalContentColor provides colors.text,
            content = content,
        )
    }
}
