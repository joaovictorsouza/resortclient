# Previews dos composables (IntelliJ / Android Studio)

Toda tela Compose do shell (`kmp/shared/src/commonMain`) tem `@Preview`, para ver e ajustar o visual na IDE sem rodar o jogo.

**Os previews ficam no fim do próprio arquivo da tela**, como funções `private`: o painel Preview da IDE mostra só os previews do arquivo aberto, então assim você edita a tela e vê o resultado ao mesmo tempo. Os dados falsos ficam compartilhados em `views/preview/PreviewData.kt` e o fundo em `PreviewScene.kt`.

## Regra

**O ViewModel continua.** A tela com estado só coleta o estado e liga os callbacks; quem desenha é um composable **sem estado**, que recebe o estado de UI pronto:

```kotlin
// Com estado: coleta e delega. Não desenha nada.
@Composable
fun InventoryView(viewModel: InventoryViewModel) {
    val ui by viewModel.ui.collectAsState()
    InventoryContent(ui, InventoryActions(onClose = viewModel::close, /* ... */))
}

// Sem estado: estado de UI + callbacks. Sem ViewModel, repositório, GameHost, ffi ou código de plataforma.
@Composable
fun InventoryContent(ui: InventoryUiState, actions: InventoryActions) { /* desenho */ }
```

Telas cobertas: `HudView`/`HudContent`, `InventoryView`/`InventoryContent` e `OnboardingOverlay` (já era sem estado). A barra de teste `ComposeHudTest` (atrás de `COMPOSE_HUD=1`) não tem preview: é só um teste de viabilidade.

## Como abrir os previews

1. Abra o repositório no IntelliJ IDEA ou no Android Studio com o plugin **Kotlin Multiplatform** habilitado (o projeto Gradle é `kmp/`).
2. Faça o sync do Gradle.
3. Abra o arquivo da tela (`views/hud/HudView.kt`, `views/inventory/InventoryView.kt`, `Onboarding.kt`) e abra o painel **Preview** (ícone na lateral do editor ou no gutter ao lado de cada `@Preview`); os previews estão no fim do arquivo. O Compose Multiplatform renderiza os `@Preview` do `commonMain`.
4. Cada função tem `name`, `widthDp` e `heightDp`: paisagem (800x360) e retrato (360x780).

Dependências (já no projeto): `org.jetbrains.compose.ui.tooling.preview.Preview` vem de `components-ui-tooling-preview` no `commonMain`; o Android usa `ui-tooling` em `debugImplementation`. Ambas no catálogo `kmp/gradle/libs.versions.toml`, na versão do Compose Multiplatform.

## Dados falsos: `PreviewData`

`views/preview/PreviewData.kt` guarda os estados de UI falsos (`hudInRoom`, `inventoryLoaded`, `onboardingLogin`...). Cada preview parte de um e usa `.copy(...)` para o estado que quer mostrar. `PreviewScene` (mesma pasta) põe um fundo escuro parecido com a cena sob o tema, para sobreposições translúcidas lerem como no jogo.

## Imagens (miniaturas)

O carregamento de imagem é plataforma (Android `BitmapFactory`, iOS Skia). A tela lê o carregador pelo `LocalThumbnailLoader`: o `App` fornece o real e o **padrão é um carregador que devolve `null`**, então nos previews aparece o espaço reservado. Faça o mesmo para qualquer recurso de plataforma.

## Como adicionar um preview a uma tela nova

1. Separe a tela: `MinhaView(viewModel)` (coleta) e `MinhaContent(ui, actions)` (sem estado).
2. Crie um estado de UI falso em `PreviewData`.
3. No **fim do arquivo da tela**, crie funções `@Preview(name = ..., widthDp = ..., heightDp = ...) @Composable private fun ...Preview()` com `PreviewScene { MinhaContent(PreviewData.x, MinhaActions()) }`. Se a tela passar de ~8 previews, deixe 3 ou 4 principais no arquivo da tela e mova o resto para um `MinhaPreviewStates.kt` (`internal`, para ser reaproveitado). Cubra os estados (carregando, vazio, erro, cheio, texto longo) em paisagem e retrato.
4. Nada de `GameHost`, repositório, `ViewModel` ou ffi dentro de um preview. Callbacks são `{}`.

## Verificação

`./gradlew :shared:allTests` compila o `commonMain` (previews incluídos) para Android e iOS. Os previews não foram renderizados por linha de comando; veja-os na IDE.
