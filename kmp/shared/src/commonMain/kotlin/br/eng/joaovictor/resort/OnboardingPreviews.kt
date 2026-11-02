package br.eng.joaovictor.resort

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import br.eng.joaovictor.resort.views.preview.PreviewData
import org.jetbrains.compose.ui.tooling.preview.Preview

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
