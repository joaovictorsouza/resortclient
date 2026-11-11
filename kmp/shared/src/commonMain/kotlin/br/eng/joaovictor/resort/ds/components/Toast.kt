package br.eng.joaovictor.resort.ds.components

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Haptic
import br.eng.joaovictor.resort.ds.theme.Motion
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

enum class ToastKind { Info, Success, Warning, Error }

@Immutable
data class ToastMessage(
    val id: Long,
    val kind: ToastKind,
    val text: String,
    val actionLabel: String? = null,
    val persistent: Boolean = false,
)

/** One toast at a time, with a queue. Plain state (no Compose), so it is testable. */
class ToastHostState {
    private val queue = ArrayDeque<ToastMessage>()
    private val _current = MutableStateFlow<ToastMessage?>(null)
    val current: StateFlow<ToastMessage?> = _current.asStateFlow()

    fun show(message: ToastMessage) {
        if (_current.value == null) _current.value = message else queue.addLast(message)
    }

    fun dismiss(id: Long) {
        queue.removeAll { it.id == id }
        if (_current.value?.id == id) _current.value = queue.removeFirstOrNull()
    }
}

/** The toast surface alone (also used for the persistent "Reconectando…" banner). */
@Composable
fun ToastCard(message: ToastMessage, modifier: Modifier = Modifier, onAction: () -> Unit = {}) {
    val colors = ResortTheme.colors
    val (icon, tint) = when (message.kind) {
        ToastKind.Info -> GameIcons.Bell to colors.secondary
        ToastKind.Success -> GameIcons.Check to colors.success
        ToastKind.Warning -> GameIcons.Bell to colors.accent
        ToastKind.Error -> GameIcons.Close to colors.dangerText
    }
    Row(
        modifier
            .widthIn(max = 420.dp)
            .drawSolidShadow()
            .background(colors.surface, RoundedCornerShape(Radius.md))
            .border(1.dp, colors.outline, RoundedCornerShape(Radius.md))
            .padding(horizontal = Space.md, vertical = Space.sm),
        horizontalArrangement = Arrangement.spacedBy(Space.sm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(icon, null, tint = tint, modifier = Modifier.size(20.dp))
        Text(message.text, style = ResortTheme.type.body, color = colors.text, modifier = Modifier.weight(1f, fill = false))
        if (message.actionLabel != null) GameButton(message.actionLabel, onAction, variant = ButtonVariant.Ghost, size = ButtonSize.Small)
    }
}

/** Top-centered host: enters from above with the pop spring and leaves after [Motion.toastMillis] unless persistent. */
@Composable
fun ToastHost(state: ToastHostState, onAction: (Long) -> Unit, modifier: Modifier = Modifier) {
    val current by state.current.collectAsState()
    val feedback = ResortTheme.feedback
    LaunchedEffect(current?.id) {
        val m = current ?: return@LaunchedEffect
        if (m.kind == ToastKind.Error) feedback.haptic(Haptic.Error)
        if (!m.persistent) {
            delay(Motion.toastMillis)
            state.dismiss(m.id)
        }
    }
    Box(modifier.fillMaxWidth(), contentAlignment = Alignment.TopCenter) {
        AnimatedContent(
            current,
            transitionSpec = {
                (slideInVertically(spring(0.55f, 700f)) { -it } + fadeIn()) togetherWith (slideOutVertically { -it } + fadeOut())
            },
        ) { m ->
            if (m != null) ToastCard(m, onAction = { onAction(m.id) })
        }
    }
}

// ---- Previews ----

@Preview(name = "Toast kinds", widthDp = 360, heightDp = 300)
@Composable
private fun ToastPreview() = DsPreviewScene {
    Column(Modifier.padding(Space.lg), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
        ToastCard(ToastMessage(1, ToastKind.Info, "Alguém tocou a campainha", "Ver"))
        ToastCard(ToastMessage(2, ToastKind.Success, "Comprado! Está no inventário"))
        ToastCard(ToastMessage(3, ToastKind.Warning, "Reconectando…", persistent = true))
        ToastCard(ToastMessage(4, ToastKind.Error, "Saldo insuficiente"))
    }
}
