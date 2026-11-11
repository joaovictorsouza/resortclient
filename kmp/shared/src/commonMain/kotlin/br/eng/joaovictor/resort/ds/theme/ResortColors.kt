package br.eng.joaovictor.resort.ds.theme

import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color

/** Semantic color tokens of the game UI (single dark theme). See ai-docs/architecture/design-system.md. */
@Immutable
data class ResortColors(
    // Surfaces, from the back to the front
    val surfaceSunken: Color = Color(0xFF0B2129),
    val surface: Color = Color(0xF512313B),
    val surfaceRaised: Color = Color(0xFF1A4250),
    val surfaceHigh: Color = Color(0xFF22566A),
    val outline: Color = Color(0xFF2C5F6E),
    val outlineStrong: Color = Color(0xFF4A8796),
    val highlight: Color = Color(0x24FFFFFF),
    val solidShadow: Color = Color(0x40000000),
    val scrim: Color = Color(0x99061418),
    // Text
    val text: Color = Color(0xFFFFF6EA),
    val textMuted: Color = Color(0xFFB8C7C9),
    val textDisabled: Color = Color(0xFF6F8A90),
    val onAccent: Color = Color(0xFF2B1A12),
    // Actions: face, pressed face, lip
    val primary: Color = Color(0xFFFF7A59),
    val primaryPressed: Color = Color(0xFFF0663F),
    val primaryLip: Color = Color(0xFFC4472B),
    val secondary: Color = Color(0xFF2EC4B6),
    val secondaryPressed: Color = Color(0xFF22AFA2),
    val secondaryLip: Color = Color(0xFF1A8C82),
    val accent: Color = Color(0xFFFFC845),
    val accentLip: Color = Color(0xFFC8921A),
    val success: Color = Color(0xFF3DBE7A),
    val successLip: Color = Color(0xFF23864F),
    val danger: Color = Color(0xFFFF6B6E),
    val dangerPressed: Color = Color(0xFFF2575B),
    val dangerLip: Color = Color(0xFFC23A44),
    /** Error text over [surface]. */
    val dangerText: Color = Color(0xFFFF8A8C),
    // Coins, rarity, focus
    val silver: Color = Color(0xFFC9D3DD),
    val silverLip: Color = Color(0xFF8C99A6),
    val gold: Color = Color(0xFFFFB627),
    val goldLip: Color = Color(0xFFB87A0A),
    val rarityCommon: Color = Color(0xFF9FB3B8),
    val rarityRare: Color = Color(0xFF4FA3FF),
    val rarityEpic: Color = Color(0xFFB57BFF),
    val rarityLegendary: Color = Color(0xFFFFB627),
    val focusRing: Color = Color(0xFFFFE08A),
)
