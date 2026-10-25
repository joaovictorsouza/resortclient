# Rodando do zero após clonar

Passo a passo curto para rodar o cliente (host KMP, Android e iOS) numa máquina nova. O código, as `environments/*.env` e o `.cargo/config.toml` já vêm no git; o que **não** vem (ignorado) está listado abaixo.

## 1. Repositórios lado a lado
O Gradle do `kmp/` usa o código-fonte da biblioteca `bevy-kmp` se existir a pasta `../bevy_kmp` ao lado do cliente (`kmp/settings.gradle.kts`: `includeBuild` de `../../bevy_kmp/kmp/bevy-kmp`):
```
~/Projetos/resortclient    # este repositório (GitLab resort-game, branch main)
~/Projetos/bevy_kmp        # https://gitlab.com/joaovictorsouza/bevy_kmp (main)
```
Sem a pasta ao lado, o Gradle procura `br.eng.joaovictor:bevy-kmp:0.1.5` no `mavenLocal` ou no registro Maven do GitLab (precisa ter sido publicado; veja o README do bevy_kmp). O lado Rust busca o bevy-kmp pelo git (`tag = "v0.1.5"`); o `.cargo/config.toml` já manda o Cargo usar o `git` da linha de comando, então o seu login/credencial do GitLab precisa funcionar no terminal (repositório privado).

## 2. Ferramentas
- Rust estável (`rustup`) com os alvos: `rustup target add aarch64-linux-android aarch64-apple-ios aarch64-apple-ios-sim`.
- Android: JDK 17, Android SDK e NDK (`ANDROID_HOME`), `cargo install cargo-ndk --locked`.
- iOS: Xcode, `brew install xcodegen`.
- Aparelhos: o simulador/AVD já baixado (não baixe outros), e `adb` no `PATH`.

## 3. Arquivos locais que o git ignora (crie quando precisar)
| Arquivo / variável | Para quê | Como obter |
|---|---|---|
| `NAKAMA_SERVER_KEY` (variável de ambiente) | chave do servidor Nakama no build | só por ambiente: `export NAKAMA_SERVER_KEY=...`; nunca em arquivo, log ou commit |
| `kmp/local.properties` | caminho do SDK Android | `sdk.dir=/Users/<voce>/Library/Android/sdk` (ou use `ANDROID_HOME`) |
| `ios/Config/Local.xcconfig` | `DEVELOPMENT_TEAM` do projeto iOS antigo (winit) | copie de `ios/Config/Local.xcconfig.example` |
| `.assets-publish.env` | publicar assets (só quem publica na CDN) | não precisa para rodar o jogo |
| `kmp/iosApp/iosApp.xcodeproj` | projeto Xcode do host KMP | gerado por `xcodegen generate` (o `scripts/kmp-ios-device.sh` já roda) |
| `target/`, `kmp/**/build/`, `kmp/.gradle/` | artefatos de build | regeneram sozinhos |

`environments/dev.env` e `environments/prod.env` (host, porta, `ASSETS_MANIFEST_URL`) **estão no git**; viram `assets/dev.env` no build.

## 4. Rodar
```bash
# Android (debug)
cd kmp && ./gradlew :androidApp:assembleDebug && adb install -r androidApp/build/outputs/apk/debug/androidApp-debug.apk

# iOS no aparelho (DEVELOPMENT_TEAM padrão no script; mude por variável se o seu for outro)
NAKAMA_SERVER_KEY=... ./scripts/kmp-ios-device.sh <udid> Debug
```
Primeira compilação de cada alvo: ~15 min e vários GB em `target/`. Detalhes de diagnóstico: [kmp-host.md](kmp-host.md).

## 5. Opcionais
- **HUD de teste em Compose** (desligado por padrão): [hud-compose-teste.md](hud-compose-teste.md).
- **Login Google no Android**: o APK precisa ser assinado com a keystore cujo SHA-1 está no cliente OAuth do Google Cloud. Use `scripts/android-signing.env.example` (senhas sem eco; nada em arquivo) e rode o Gradle no mesmo terminal.
- **Checagens antes de commitar**: `./scripts/check.sh`.
