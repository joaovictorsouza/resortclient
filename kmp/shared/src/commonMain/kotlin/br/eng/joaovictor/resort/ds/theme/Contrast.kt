package br.eng.joaovictor.resort.ds.theme

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.compositeOver
import kotlin.math.pow

/** WCAG 2.1 relative luminance of an opaque sRGB color. */
fun relativeLuminance(color: Color): Double {
    fun channel(v: Float): Double {
        val d = v.toDouble()
        return if (d <= 0.03928) d / 12.92 else ((d + 0.055) / 1.055).pow(2.4)
    }
    return 0.2126 * channel(color.red) + 0.7152 * channel(color.green) + 0.0722 * channel(color.blue)
}

/** WCAG contrast ratio (1..21) of [foreground] over [background]; translucent colors are first composited over [backdrop]. */
fun contrastRatio(foreground: Color, background: Color, backdrop: Color = Color.Black): Double {
    val bg = background.compositeOver(backdrop)
    val fg = foreground.compositeOver(bg)
    val a = relativeLuminance(fg)
    val b = relativeLuminance(bg)
    return (maxOf(a, b) + 0.05) / (minOf(a, b) + 0.05)
}
