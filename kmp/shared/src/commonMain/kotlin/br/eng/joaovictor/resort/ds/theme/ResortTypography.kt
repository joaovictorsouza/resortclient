package br.eng.joaovictor.resort.ds.theme

import androidx.compose.runtime.Immutable
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

@Immutable
data class ResortTypography(
    val display: TextStyle = style(30, 36, FontWeight.Black),
    val title: TextStyle = style(20, 26, FontWeight.ExtraBold),
    val titleSmall: TextStyle = style(16, 22, FontWeight.Bold),
    val body: TextStyle = style(15, 20, FontWeight.Medium),
    val label: TextStyle = style(15, 18, FontWeight.ExtraBold),
    val caption: TextStyle = style(12, 16, FontWeight.SemiBold),
    /** Tabular figures, so a balance does not jitter while it changes. */
    val number: TextStyle = style(16, 20, FontWeight.ExtraBold).copy(fontFeatureSettings = "tnum"),
    val badge: TextStyle = style(11, 14, FontWeight.Black).copy(letterSpacing = 0.6.sp),
)

private fun style(size: Int, line: Int, weight: FontWeight) =
    TextStyle(fontFamily = FontFamily.Default, fontSize = size.sp, lineHeight = line.sp, fontWeight = weight)
