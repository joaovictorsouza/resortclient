package br.eng.joaovictor.resort

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.game.BevyJson
import kotlinx.serialization.Serializable
import androidx.compose.material3.darkColorScheme
import br.eng.joaovictor.resort.views.preview.PreviewData
import org.jetbrains.compose.ui.tooling.preview.Preview

private const val NICKNAME_MAX_LEN = 16

/** What the game reports about the onboarding (`onboarding_stage`). */
@Serializable
data class OnboardingStage(
    val stage: String = "loading",
    val busy: Boolean = false,
    val error: String? = null,
    /** The device login (no Google) is available: development environments only. */
    val dev: Boolean = false,
) {
    val visible: Boolean get() = stage == "login" || stage == "nickname"
}

/** Keeps only what a nickname may contain; the game validates again with the same rule. */
fun sanitizeNickname(input: String): String =
    input.filter { it.isLetterOrDigit() && it.code < 128 || it == '_' }.take(NICKNAME_MAX_LEN)

/** Parses `onboarding_stage`; a payload that is not a JSON object gives `null` (the caller keeps the last stage). */
fun parseStage(json: String): OnboardingStage? = BevyJson.decodeOrNull<OnboardingStage>(json)

/** Command payload of `onboarding_nickname`. */
@Serializable
data class NicknameMessage(val nickname: String)

/**
 * Native login and nickname screens, drawn above the game while it is in those states. The game
 * still owns the rules and the state; this only collects input and shows what it reports.
 */
@Composable
fun OnboardingOverlay(
    stage: OnboardingStage,
    onGoogle: () -> Unit,
    onDeviceLogin: () -> Unit,
    onNickname: (String) -> Unit,
    onReset: () -> Unit = {},
) {
    if (stage.dev && stage.stage != "loading") {
        // Development-only escape hatch that does not depend on the bevy_hui buttons.
        Box(Modifier.safeDrawingPadding()) {
            androidx.compose.material3.TextButton(onClick = onReset) { Text("Limpar sessão (dev)", color = Color(0xFFB0B8C8)) }
        }
    }
    if (!stage.visible) return
    Box(
        Modifier.fillMaxSize().background(Color(0xFF10141C)).safeDrawingPadding().imePadding().padding(24.dp),
        contentAlignment = Alignment.Center,
    ) {
        Column(
            Modifier.widthIn(max = 420.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            when (stage.stage) {
                "login" -> {
                    Text("Resort", style = MaterialTheme.typography.headlineLarge, color = Color.White)
                    if (stage.busy) {
                        CircularProgressIndicator()
                    } else {
                        Button(onClick = onGoogle) { Text("Entrar com o Google") }
                        if (stage.dev) {
                            OutlinedButton(onClick = onDeviceLogin) { Text("Entrar sem Google (dev)") }
                        }
                    }
                }
                else -> {
                    var nickname by remember { mutableStateOf("") }
                    Text("Escolha seu nome de usuário", style = MaterialTheme.typography.titleLarge, color = Color.White)
                    OutlinedTextField(
                        value = nickname,
                        onValueChange = { nickname = sanitizeNickname(it) },
                        singleLine = true,
                        label = { Text("Nome de usuário") },
                        supportingText = { Text("3 a 16 letras, números ou _") },
                    )
                    Button(onClick = { onNickname(nickname) }, enabled = nickname.isNotEmpty()) { Text("Continuar") }
                }
            }
            stage.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        }
    }
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

@Composable
private fun Shown(stage: OnboardingStage) = MaterialTheme(colorScheme = darkColorScheme()) {
    OnboardingOverlay(stage, onGoogle = {}, onDeviceLogin = {}, onNickname = {})
}

@Preview(name = "Login", widthDp = 800, heightDp = 360)
@Composable
private fun LoginPreview() = Shown(PreviewData.onboardingLogin)

@Preview(name = "Login (dev, with device login)", widthDp = 800, heightDp = 360)
@Composable
private fun LoginDevPreview() = Shown(PreviewData.onboardingLogin.copy(dev = true))

@Preview(name = "Login busy", widthDp = 800, heightDp = 360)
@Composable
private fun LoginBusyPreview() = Shown(PreviewData.onboardingLogin.copy(busy = true))

@Preview(name = "Login error", widthDp = 360, heightDp = 780)
@Composable
private fun LoginErrorPreview() = Shown(PreviewData.onboardingLogin.copy(error = "Não foi possível entrar. Tente de novo."))

@Preview(name = "Nickname", widthDp = 800, heightDp = 360)
@Composable
private fun NicknamePreview() = Shown(PreviewData.onboardingNickname)

@Preview(name = "Nickname with error (portrait)", widthDp = 360, heightDp = 780)
@Composable
private fun NicknameErrorPreview() = Shown(PreviewData.onboardingNickname.copy(error = "Esse nome já está em uso."))
