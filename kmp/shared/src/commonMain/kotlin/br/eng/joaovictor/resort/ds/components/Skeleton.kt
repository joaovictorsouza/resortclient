package br.eng.joaovictor.resort.ds.components

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.theme.LocalReduceMotion
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

/** A placeholder block; [alpha] comes from the screen's single shimmer ([SkeletonGrid] owns one). */
@Composable
fun Skeleton(modifier: Modifier = Modifier, shape: Shape = RoundedCornerShape(Radius.md), alpha: Float = 0.45f) {
    Box0(modifier.alpha(alpha).background(ResortTheme.colors.surfaceHigh, shape))
}

@Composable
private fun Box0(modifier: Modifier) = androidx.compose.foundation.layout.Box(modifier)

/** Grid of skeleton tiles with one alpha animation for all (35% to 60%); still when reduced motion is on. */
@Composable
fun SkeletonGrid(modifier: Modifier = Modifier, count: Int = 8, minSize: Dp = DsSize.tileMin, tileAspect: Float = 0.85f) {
    val alpha = if (LocalReduceMotion.current) 0.45f else {
        val transition = rememberInfiniteTransition()
        val a by transition.animateFloat(0.35f, 0.6f, infiniteRepeatable(tween(700), RepeatMode.Reverse))
        a
    }
    LazyVerticalGrid(
        GridCells.Adaptive(minSize), modifier.fillMaxSize().padding(horizontal = Space.md),
        horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalArrangement = Arrangement.spacedBy(Space.sm),
        userScrollEnabled = false,
    ) {
        items(count, key = { it }) { Skeleton(Modifier.fillMaxWidth().aspectRatio(tileAspect), alpha = alpha) }
    }
}

// ---- Previews ----

@Preview(name = "SkeletonGrid", widthDp = 360, heightDp = 300)
@Composable
private fun SkeletonGridPreview() = DsPreviewScene { SkeletonGrid() }
