package br.eng.joaovictor.resort.ds.theme

import androidx.compose.animation.core.spring
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.unit.dp

/** 4dp scale, with 2dp for fine adjustments. */
object Space {
    val xxs = 2.dp
    val xs = 4.dp
    val sm = 8.dp
    val md = 12.dp
    val lg = 16.dp
    val xl = 20.dp
    val xxl = 24.dp
    val xxxl = 32.dp
}

object Radius {
    val xs = 6.dp
    val sm = 10.dp
    val md = 14.dp
    val lg = 20.dp
    val xl = 28.dp
    val pill = RoundedCornerShape(percent = 50)
}

object Size {
    val touchMin = 48.dp
    val iconButton = 48.dp
    val hudButton = 56.dp
    val dockShopButton = 64.dp
    val buttonHeight = 52.dp
    val buttonHeightMedium = 44.dp
    val buttonHeightSmall = 40.dp
    val chipHeight = 40.dp
    val searchHeight = 48.dp
    val panelHeader = 56.dp
    val tileMin = 84.dp
    val icon = 24.dp
    val iconSmall = 18.dp
    val iconHud = 28.dp
    val lip = 4.dp
    val lipPressed = 1.dp
    val sideRailWidth = 112.dp
    val tuckedHandle = 24.dp
}

object Motion {
    const val press = 70
    const val fast = 120
    const val base = 200
    const val slow = 320
    const val staggerPerItem = 18
    const val staggerMaxItems = 8
    const val toastMillis = 2_800L
    val panelSpring = spring<Float>(dampingRatio = 0.86f, stiffness = 520f)
    val popSpring = spring<Float>(dampingRatio = 0.55f, stiffness = 700f)
}
