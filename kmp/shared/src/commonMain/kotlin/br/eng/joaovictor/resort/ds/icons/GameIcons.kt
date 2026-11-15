package br.eng.joaovictor.resort.ds.icons

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.PathBuilder
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.DsPreviewScene

private const val STROKE = 2.25f

/**
 * Vector icons drawn on a 24dp grid with a 2.25 stroke and round caps and joins; no emoji and no
 * `material-icons-extended`. Each is built once (`by lazy`) and tinted by `Icon(tint = ...)`.
 */
object GameIcons {
    val Shop by lazy {
        icon("Shop") {
            stroke { moveTo(4f, 9f); lineTo(5.5f, 4f); lineTo(18.5f, 4f); lineTo(20f, 9f) }
            stroke {
                moveTo(4f, 9f); curveTo(4f, 11f, 7f, 11f, 8f, 9f); curveTo(9f, 11f, 11f, 11f, 12f, 9f)
                curveTo(13f, 11f, 15f, 11f, 16f, 9f); curveTo(17f, 11f, 20f, 11f, 20f, 9f)
            }
            stroke { moveTo(5f, 11.5f); lineTo(5f, 20f); lineTo(19f, 20f); lineTo(19f, 11.5f) }
            stroke { moveTo(10f, 20f); lineTo(10f, 15f); lineTo(14f, 15f); lineTo(14f, 20f) }
        }
    }
    val Backpack by lazy {
        icon("Backpack") {
            stroke { moveTo(9f, 6f); curveTo(9f, 2.5f, 15f, 2.5f, 15f, 6f) }
            stroke {
                moveTo(7f, 21f); lineTo(17f, 21f); curveTo(18.5f, 21f, 19f, 20f, 19f, 19f)
                lineTo(19f, 10f); curveTo(19f, 7.5f, 17.5f, 6f, 15f, 6f); lineTo(9f, 6f)
                curveTo(6.5f, 6f, 5f, 7.5f, 5f, 10f); lineTo(5f, 19f); curveTo(5f, 20f, 5.5f, 21f, 7f, 21f); close()
            }
            stroke { moveTo(8.5f, 14f); lineTo(15.5f, 14f); lineTo(15.5f, 18f); lineTo(8.5f, 18f); close() }
        }
    }
    val Compass by lazy {
        icon("Compass") {
            circle(12f, 12f, 9f)
            stroke { moveTo(15.5f, 8.5f); lineTo(13.5f, 13.5f); lineTo(8.5f, 15.5f); lineTo(10.5f, 10.5f); close() }
        }
    }
    val Hammer by lazy {
        icon("Hammer") {
            stroke { moveTo(13.5f, 10.5f); lineTo(4.5f, 19.5f) }
            stroke { moveTo(4f, 20f); lineTo(6f, 20f) }
            stroke { moveTo(11f, 4f); lineTo(17f, 4f); lineTo(20f, 7f); lineTo(17f, 10f); lineTo(14f, 7f); close() }
        }
    }
    val Gear by lazy {
        icon("Gear") {
            circle(12f, 12f, 3f)
            stroke {
                moveTo(12f, 3f); lineTo(12f, 5.2f); moveTo(12f, 18.8f); lineTo(12f, 21f)
                moveTo(3f, 12f); lineTo(5.2f, 12f); moveTo(18.8f, 12f); lineTo(21f, 12f)
                moveTo(5.6f, 5.6f); lineTo(7.2f, 7.2f); moveTo(16.8f, 16.8f); lineTo(18.4f, 18.4f)
                moveTo(5.6f, 18.4f); lineTo(7.2f, 16.8f); moveTo(16.8f, 7.2f); lineTo(18.4f, 5.6f)
            }
            circle(12f, 12f, 6.6f)
        }
    }
    val User by lazy {
        icon("User") {
            circle(12f, 8f, 3.6f)
            stroke { moveTo(4.5f, 20f); curveTo(4.5f, 15f, 19.5f, 15f, 19.5f, 20f) }
        }
    }
    val Close by lazy {
        icon("Close") { stroke { moveTo(6f, 6f); lineTo(18f, 18f); moveTo(18f, 6f); lineTo(6f, 18f) } }
    }
    val Search by lazy {
        icon("Search") { circle(10.5f, 10.5f, 6f); stroke { moveTo(15f, 15f); lineTo(20f, 20f) } }
    }
    val Back by lazy {
        icon("Back") { stroke { moveTo(19f, 12f); lineTo(5f, 12f); moveTo(11f, 6f); lineTo(5f, 12f); lineTo(11f, 18f) } }
    }
    val Check by lazy { icon("Check") { stroke { moveTo(5f, 12.5f); lineTo(10f, 17.5f); lineTo(19f, 7f) } } }
    val CoinSilver by lazy { coin("CoinSilver") }
    val CoinGold by lazy { coin("CoinGold", star = true) }
    val ChevronDown by lazy { icon("ChevronDown") { stroke { moveTo(6f, 9f); lineTo(12f, 15f); lineTo(18f, 9f) } } }
    val Lock by lazy {
        icon("Lock") {
            stroke { moveTo(6f, 11f); lineTo(18f, 11f); lineTo(18f, 20f); lineTo(6f, 20f); close() }
            stroke { moveTo(8.5f, 11f); lineTo(8.5f, 8f); curveTo(8.5f, 3.5f, 15.5f, 3.5f, 15.5f, 8f); lineTo(15.5f, 11f) }
        }
    }
    val Globe by lazy {
        icon("Globe") {
            circle(12f, 12f, 9f)
            stroke { moveTo(3f, 12f); lineTo(21f, 12f) }
            stroke { moveTo(12f, 3f); curveTo(7f, 8f, 7f, 16f, 12f, 21f); curveTo(17f, 16f, 17f, 8f, 12f, 3f) }
        }
    }
    val EyeOff by lazy {
        icon("EyeOff") {
            stroke { moveTo(3f, 12f); curveTo(6f, 6.5f, 18f, 6.5f, 21f, 12f); curveTo(18f, 17.5f, 6f, 17.5f, 3f, 12f) }
            circle(12f, 12f, 2.6f)
            stroke { moveTo(4f, 4f); lineTo(20f, 20f) }
        }
    }
    val Star by lazy {
        icon("Star") {
            stroke { moveTo(12f, 3.5f); lineTo(14.6f, 9f); lineTo(20.5f, 9.7f); lineTo(16.1f, 13.8f); lineTo(17.3f, 19.7f); lineTo(12f, 16.8f); lineTo(6.7f, 19.7f); lineTo(7.9f, 13.8f); lineTo(3.5f, 9.7f); lineTo(9.4f, 9f); close() }
        }
    }
    val Bell by lazy {
        icon("Bell") {
            stroke {
                moveTo(6f, 17f); lineTo(6f, 11f); curveTo(6f, 4f, 18f, 4f, 18f, 11f); lineTo(18f, 17f)
                lineTo(20f, 17f); lineTo(20f, 18.5f); lineTo(4f, 18.5f); lineTo(4f, 17f); close()
            }
            stroke { moveTo(10f, 21f); lineTo(14f, 21f) }
        }
    }

    /** Every icon with its name, for the catalog preview and tests. */
    val all: List<Pair<String, ImageVector>>
        get() = listOf(
            "Shop" to Shop, "Backpack" to Backpack, "Compass" to Compass, "Hammer" to Hammer, "Gear" to Gear,
            "User" to User, "Close" to Close, "Search" to Search, "Back" to Back, "Check" to Check,
            "CoinSilver" to CoinSilver, "CoinGold" to CoinGold, "ChevronDown" to ChevronDown, "Lock" to Lock,
            "Globe" to Globe, "EyeOff" to EyeOff, "Star" to Star, "Bell" to Bell,
        )
}

private class IconBuilder(val builder: ImageVector.Builder) {
    fun stroke(block: PathBuilder.() -> Unit) {
        builder.path(
            stroke = SolidColor(Color.Black),
            strokeLineWidth = STROKE,
            strokeLineCap = StrokeCap.Round,
            strokeLineJoin = StrokeJoin.Round,
            pathBuilder = block,
        )
    }

    fun circle(cx: Float, cy: Float, r: Float) = stroke {
        moveTo(cx - r, cy)
        arcToRelative(r, r, 0f, isMoreThanHalf = true, isPositiveArc = false, dx1 = 2 * r, dy1 = 0f)
        arcToRelative(r, r, 0f, isMoreThanHalf = true, isPositiveArc = false, dx1 = -2 * r, dy1 = 0f)
        close()
    }
}

private fun icon(name: String, block: IconBuilder.() -> Unit): ImageVector {
    val builder = ImageVector.Builder(name, 24.dp, 24.dp, 24f, 24f)
    IconBuilder(builder).block()
    return builder.build()
}

private fun coin(name: String, star: Boolean = false) = icon(name) {
    circle(12f, 12f, 9f)
    if (star) {
        stroke { moveTo(12f, 7f); lineTo(13.5f, 10.5f); lineTo(17f, 11f); lineTo(14.5f, 13.5f); lineTo(15f, 17f); lineTo(12f, 15.3f); lineTo(9f, 17f); lineTo(9.5f, 13.5f); lineTo(7f, 11f); lineTo(10.5f, 10.5f); close() }
    } else {
        stroke { moveTo(12f, 7f); lineTo(12f, 17f); moveTo(9f, 10f); lineTo(15f, 10f); moveTo(9f, 14f); lineTo(15f, 14f) }
    }
}

// ---- Previews ----

@Preview(name = "Icons", widthDp = 360, heightDp = 240)
@Composable
private fun GameIconsPreview() = DsPreviewScene {
    FlowRow(Modifier.padding(Space.lg), horizontalArrangement = Arrangement.spacedBy(Space.md), verticalArrangement = Arrangement.spacedBy(Space.md)) {
        GameIcons.all.forEach { (name, icon) ->
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Icon(icon, contentDescription = name, tint = ResortTheme.colors.text, modifier = Modifier.size(32.dp))
                Text(name, style = ResortTheme.type.caption, color = ResortTheme.colors.textMuted)
            }
        }
    }
}
