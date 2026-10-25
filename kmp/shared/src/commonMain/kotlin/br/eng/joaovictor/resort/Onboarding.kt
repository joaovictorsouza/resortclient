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

private const val NICKNAME_MAX_LEN = 16

/** What the game reports about the onboarding (`onboarding_stage`). */
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

/** Parses the flat JSON object the game sends (strings, booleans, null). */
fun parseStage(json: String): OnboardingStage {
    val map = parseFlatJson(json)
    return OnboardingStage(
        stage = map["stage"] as? String ?: "loading",
        busy = map["busy"] as? Boolean ?: false,
        error = map["error"] as? String,
        dev = map["dev"] as? Boolean ?: false,
    )
}

internal fun parseFlatJson(json: String): Map<String, Any?> {
    val out = mutableMapOf<String, Any?>()
    var i = 0
    fun skipWs() { while (i < json.length && json[i].isWhitespace()) i++ }
    fun string(): String {
        val sb = StringBuilder()
        i++ // opening quote
        while (i < json.length && json[i] != '"') {
            if (json[i] == '\\' && i + 1 < json.length) {
                i++
                when (json[i]) {
                    'n' -> sb.append('\n')
                    't' -> sb.append('\t')
                    'u' -> {
                        sb.append(json.substring(i + 1, i + 5).toInt(16).toChar())
                        i += 4
                    }
                    else -> sb.append(json[i])
                }
            } else {
                sb.append(json[i])
            }
            i++
        }
        i++ // closing quote
        return sb.toString()
    }
    skipWs()
    if (i < json.length && json[i] == '{') i++
    while (true) {
        skipWs()
        if (i >= json.length || json[i] == '}') break
        if (json[i] == ',') { i++; continue }
        val key = string()
        skipWs()
        i++ // colon
        skipWs()
        out[key] = when {
            json[i] == '"' -> string()
            json.startsWith("true", i) -> { i += 4; true }
            json.startsWith("false", i) -> { i += 5; false }
            json.startsWith("null", i) -> { i += 4; null }
            else -> {
                val start = i
                while (i < json.length && json[i] != ',' && json[i] != '}') i++
                json.substring(start, i).trim().toDoubleOrNull()
            }
        }
    }
    return out
}

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
