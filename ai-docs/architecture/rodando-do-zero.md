# Rodando do zero após clonar

Passo a passo para rodar o cliente (host KMP, Android e iOS) numa máquina nova, e para editar **Rust e Kotlin juntos** no IntelliJ. O código, as `environments/*.env` e o `.cargo/config.toml` já vêm no git; o que **não** vem (ignorado) está na seção 3. O mobile nativo antigo foi removido ([ADR 0014](../decisions/0014-remocao-do-mobile-nativo-antigo.md)): **só `kmp/` e `scripts/kmp-ios-device.sh` buildam Android e iOS**.

Legenda: **[confirmado]** = executado nesta máquina; **[suposição]** = não executado, vem de conhecimento do Gradle/IntelliJ e deve ser conferido.

## 1. Repositórios lado a lado
O Gradle do `kmp/` usa o código-fonte da biblioteca `bevy-kmp` se existir a pasta `bevy_kmp` ao lado do cliente (`kmp/settings.gradle.kts`: `includeBuild("../../bevy_kmp/kmp/bevy-kmp")`):
```
~/Projetos/resortclient    # este repositório (GitLab resort-game, branch main)
~/Projetos/bevy_kmp        # https://gitlab.com/joaovictorsouza/bevy_kmp (main)
```
Sem a pasta ao lado, o Gradle procura `br.eng.joaovictor:bevy-kmp:0.1.5` no `mavenLocal` ou no registro Maven do GitLab (a biblioteca precisa ter sido publicada). O lado Rust busca o bevy-kmp pelo git (`tag = "v0.1.5"`): o `.cargo/config.toml` manda o Cargo usar o `git` da linha de comando, então o seu login/credencial do GitLab precisa funcionar no terminal (repositório privado).

## 2. Ferramentas
- **Rust** estável (`rustup`) com os alvos: `rustup target add aarch64-linux-android aarch64-apple-ios aarch64-apple-ios-sim`. O Gradle também roda `rustup target add` sozinho para o Android.
- **Android**: JDK 17 ou superior (o Gradle desta máquina roda com JDK 21 do SDKMAN **[confirmado]**), Android SDK e NDK em `ANDROID_HOME` (aqui: `~/Library/Android/sdk`, NDK `30.0.15729638`) e `adb` no `PATH`. **`cargo-ndk` não é necessário**: o Gobley (plugin `dev.gobley.cargo` 0.3.7) usa o NDK direto; nenhuma classe dele cita o `cargo-ndk` **[confirmado: busca no JAR]**.
- **iOS**: Xcode e `brew install xcodegen`.
- Aparelhos: use o simulador/AVD já instalado (não baixe outros). Aqui há o AVD `bevykmp_api35` e simuladores iPhone 16 / 16 Pro / iPad Pro.

## 3. Arquivos locais que o git ignora (crie quando precisar)
| Arquivo / variável | Para quê | Como obter |
|---|---|---|
| `NAKAMA_SERVER_KEY` (variável de ambiente) | chave do servidor Nakama no build | só por ambiente: `export NAKAMA_SERVER_KEY=...`; nunca em arquivo, log ou commit |
| `ANDROID_KEYSTORE`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD` | assinar o APK com a keystore do projeto (necessário para o login Google no Android) | `scripts/android-signing.env.example` (as senhas são pedidas sem eco); sem elas o APK usa a chave de debug |
| `kmp/local.properties` | caminho do SDK Android | `sdk.dir=/Users/<voce>/Library/Android/sdk` (ou `ANDROID_HOME`; aqui o build rodou só com `ANDROID_HOME`) |
| `.assets-publish.env` | publicar assets na CDN | só quem publica; não precisa para rodar |
| `kmp/iosApp/iosApp.xcodeproj` | projeto Xcode do host KMP | gerado por `xcodegen generate` (o `kmp-ios-device.sh` já roda) |
| `target/`, `kmp/**/build/`, `kmp/.gradle/` | artefatos de build | regeneram sozinhos |

`environments/dev.env` e `environments/prod.env` (host, porta, `ASSETS_MANIFEST_URL`) **estão no git**; viram `assets/dev.env` no build. `ENVIRONMENT=dev|prod` escolhe o arquivo (padrão `dev`).

## 4. Rodar pela linha de comando

### Android (aparelho)
```bash
cd kmp
./gradlew :androidApp:assembleDebug
adb devices                                   # confira o serial
adb install -r androidApp/build/outputs/apk/debug/androidApp-debug.apk
adb shell am start -n br.eng.joaovictor.resort/.MainActivity
```
O Gradle compila o Rust (`cargoBuildAndroidArm64Debug`, feature `kmp-host`, `--no-default-features`) e já desliga o debuginfo para o `.so` não passar de GB. Primeira build: ~15 min; as seguintes: poucos minutos.

### Android (emulador)
O build acima só gera `arm64-v8a`, então use um AVD **arm64** (num Mac Apple Silicon: imagem `arm64-v8a`, como o `bevykmp_api35`). Pelo terminal:
```bash
~/Library/Android/sdk/emulator/emulator -list-avds
~/Library/Android/sdk/emulator/emulator -avd bevykmp_api35 &
adb wait-for-device && adb install -r androidApp/build/outputs/apk/debug/androidApp-debug.apk
```
Servidor local (`make stack-up`): `adb reverse tcp:7350 tcp:7350` e `NAKAMA_HOST=127.0.0.1` no build (o padrão do DEV usa o host de `environments/dev.env`). Feche o emulador ao terminar.

### Android com login Google
```bash
source ~/android-signing.env        # copiado de scripts/android-signing.env.example; pede as senhas sem eco
cd kmp && ./gradlew :androidApp:assembleDebug
```
O SHA-1 da keystore (`keytool -list -v -keystore "$ANDROID_KEYSTORE" -alias "$ANDROID_KEY_ALIAS" | grep SHA1`) precisa estar cadastrado no cliente OAuth Android do Google Cloud.

### iOS (aparelho)
```bash
NAKAMA_SERVER_KEY=... ./scripts/kmp-ios-device.sh <udid> Debug     # INSTALL=0 só compila; LAUNCH=0 não abre
```
O script gera o projeto (`xcodegen`), compila com `xcodebuild` e instala/abre. O time de assinatura vem de `DEVELOPMENT_TEAM` (padrão no script; mude por variável). Avise antes de usar o iPhone físico.

### iOS (simulador)
O `kmp-ios-device.sh` mira o aparelho. Para o simulador use o Xcode **[suposição, não executado]**: `cd kmp/iosApp && xcodegen generate && open iosApp.xcodeproj`, escolha o simulador **já instalado** (ex.: iPhone 16 Pro), e rode (⌘R). O Gobley/cargo compila o alvo `aarch64-apple-ios-sim` na fase de build do Xcode. Feche o simulador ao terminar (`xcrun simctl shutdown <udid>`).

## 5. IntelliJ IDEA: Rust e Kotlin no mesmo projeto

### O que há instalado nesta máquina **[confirmado]**
- IntelliJ IDEA Ultimate 2026.1 (`~/Applications/IntelliJ IDEA.app`, build `IU-261`) e Android Studio (`~/Applications/Android Studio.app`).
- No IntelliJ vêm instalados: Kotlin, Gradle, Compose, TOML. **Não vêm**: o plugin **Android** (necessário para o módulo `androidApp` e para rodar no aparelho) e o plugin **Rust**. Instale os dois em *Settings → Plugins → Marketplace* ("Android" e "Rust" da JetBrains). Alternativas: rodar o Android no **Android Studio** e/ou editar o Rust no **RustRover** **[suposição: preço/licença do plugin Rust não conferidos]**.
- Não há `.idea/`, `*.iml` nem `.run/` versionados **[confirmado]**; `.idea/` está no `.gitignore`. Nada versionado atrapalha o import.

### Abrir o projeto (recomendado)
1. **File → Open** na **raiz** `~/Projetos/resortclient` (não em `kmp/`). Assim o IntelliJ enxerga `crates/` (Rust) e `kmp/` (Kotlin) no mesmo projeto. **[suposição]**
2. A raiz tem um `settings.gradle.kts` mínimo que inclui o build de `kmp/` (*composite build*) e um `gradlew` na raiz (links para `kmp/gradlew` e `kmp/gradle/wrapper`, uma versão só do Gradle). Por isso o IDE **não** reclama mais de "settings.gradle[.kts] is not found". **[confirmado pela linha de comando]**: `./gradlew -p . projects` na raiz lista os builds incluídos `:kmp` e `:bevy-kmp`; `./gradlew :kmp:androidApp:assembleDebug -m` reconhece a tarefa; e `cd kmp && ./gradlew :androidApp:assembleDebug` continua funcionando como antes. Se o IDE ainda pedir **Link Gradle Project**, escolha o `settings.gradle.kts` da **raiz** (não o de `kmp/`). **[A importação pela interface do IntelliJ não foi testada visualmente.]**
   Dentro do IDE as tarefas aparecem como `:kmp:androidApp:assembleDebug` (build `kmp`); no terminal use o `cd kmp && ./gradlew ...` das seções acima.
3. Em *Settings → Build Tools → Gradle*: **Gradle JDK** = JDK 17 ou 21 (o de `JAVA_HOME`/SDKMAN, ou o JBR do IntelliJ; ambos servem, o projeto compila para Java 17) e "Build and run using: Gradle". **[confirmado que o Gradle roda com JDK 21; o JBR do IDE não foi testado]**
4. Plugin Rust: abra `Cargo.toml` da raiz (*Cargo → Attach Cargo Project*, ou o aviso do plugin). O workspace tem `crates/protocol`, `crates/nakama_client` e `crates/game`; a navegação funciona para eles. **[suposição]**
5. **Rust no IDE usa as features padrão do `game`** (`winit-host`, desktop). Para navegar no código do host KMP, em *Settings → Languages & Frameworks → Rust → Cargo*, marque as features do pacote `game` como `kmp-host` e desmarque `winit-host` (ou o equivalente "Use these features"); sem isso, `kmp_host.rs` e `hud_bridge.rs` aparecem como código inativo (cinza). **[suposição; o `cargo clippy -p game --no-default-features --features kmp-host` é a checagem confirmada no `check.sh`]**

Alternativa se a raiz causar conflito de módulos: abra **`kmp/`** no IntelliJ para o Kotlin/Android e a raiz no RustRover (ou noutra janela) para o Rust. **[suposição]**

O IntelliJ cria `local.properties` (com o `sdk.dir`) e `.gradle/` na raiz ao importar; ambos estão no `.gitignore`. Se o IDE oferecer adicionar arquivos novos ao Git, recuse (já aconteceu com o `local.properties`).

### PATH quando o IDE abre pelo Dock **[suposição, causa comum]**
Apps abertos pelo Dock/Spotlight **não herdam** o `PATH` e as variáveis do seu shell: o Gradle não acha `cargo`/`rustup` (erro tipo "cargo not found" no sync ou no `cargoBuild...`). Soluções, na ordem de preferência:
1. Abrir o IDE pelo terminal: `open -a "IntelliJ IDEA"` (herda o ambiente do terminal) ou o comando de linha do IDE.
2. Garantir `~/.cargo/bin` e as variáveis em `~/.zprofile` (e não só `~/.zshrc`) e reiniciar a sessão.
3. Configurar por Run Configuration (próximo item). `ANDROID_HOME` também deve estar disponível ao IDE; a ausência dele faz o Gradle falhar procurando o SDK, e `kmp/local.properties` com `sdk.dir=` resolve.

### Variáveis (NAKAMA_SERVER_KEY e assinatura) sem commitar **[suposição]**
- *Run → Edit Configurations → (Android App `androidApp`) → Environment variables* aceita `NAKAMA_SERVER_KEY=...` mas isso vai para `.idea/workspace.xml` (ignorado pelo git), em texto puro. **Evite senhas ali**; prefira abrir o IDE pelo terminal depois de `source ~/android-signing.env`.
- O Gradle lê `NAKAMA_SERVER_KEY`, `ENVIRONMENT`, `ANDROID_KEYSTORE*` do ambiente **do daemon**; se mudar uma variável, rode `./gradlew --stop` (o daemon antigo guarda as antigas).

### Rodar o Android pelo IntelliJ **[suposição]**
- Com o plugin Android: configuração **Android App**, módulo `androidApp.main`, aparelho USB ou emulador (arm64). Antes, o IDE roda o `assembleDebug` do Gradle.
- Sem o plugin: terminal do IDE com os comandos da seção 4. É o caminho que **foi executado aqui** (veja a seção 7).

### iOS pelo IntelliJ
Não dá para rodar o iOS direto do IntelliJ sem o plugin *Kotlin Multiplatform* e Xcode configurado **[suposição]**. Use o `scripts/kmp-ios-device.sh` (aparelho) ou o Xcode (simulador), como na seção 4. O IntelliJ serve para editar o Kotlin/Rust.

### Editar o `bevy_kmp` junto
- **Kotlin**: já é automático. Com `~/Projetos/bevy_kmp` ao lado, o `includeBuild` do `kmp/settings.gradle.kts` coloca o módulo `bevy-kmp` no mesmo build; as classes dele aparecem para navegação e edição. Depois do sync, mudanças no Kotlin dele entram no próximo `assembleDebug`. **[confirmado que o includeBuild existe e é usado no build; a navegação no IDE é suposição]**
- **Rust**: o cliente usa o `bevy-kmp-ffi` por **git+tag**, então o código vem do cache do Cargo (somente leitura). Para editar o `bevy_kmp` ao lado sem commitar, aponte o Cargo para a pasta local com um `[patch]` em um arquivo **fora do git**, por exemplo `~/.cargo/config.toml` (vale para todos os projetos) ou um `.cargo/config.local.toml` passado por `--config`:
  ```toml
  [patch."https://gitlab.com/joaovictorsouza/bevy_kmp"]
  bevy-kmp-ffi = { path = "../bevy_kmp/crates/bevy-kmp-ffi" }
  bevy-kmp     = { path = "../bevy_kmp/crates/bevy-kmp" }
  ```
  Não coloque isso no `.cargo/config.toml` versionado do cliente. O `[patch.crates-io] raw-window-metal` do `Cargo.toml` raiz continua apontando para a tag. O resultado do teste desta configuração está na seção 7.

### Quais edições disparam rebuild pelo Gradle **[confirmado]**
- Alterar Rust em `crates/game/src/**` ou no `Cargo.toml` do `game`: o Gradle refaz `cargoBuild*` (o `packageDirectory` do plugin é `crates/game`) e, no Android, recopia o `.so`.
- Rust em `crates/protocol` ou `crates/nakama_client` (dependências por path): **também** dispara o Gradle **[confirmado]**: editei `crates/protocol/src/lib.rs` e o `cargoBuildAndroidArm64Debug` rodou de novo (45 s, incremental), sem `UP-TO-DATE`.
- Alterar só Kotlin: rebuild incremental do Kotlin/Compose, sem recompilar o Rust.
- Alterar `assets/` ou `environments/*.env`: regera `assets/dev.env` e reempacota o APK.

### Depurar rápido (sem esperar o APK)
No terminal do IDE, na raiz:
```bash
cargo check -p game --no-default-features --features kmp-host     # erros de compilação em segundos
cargo clippy -p game --no-default-features --features kmp-host --all-targets -- -D warnings
cargo test  -p game --no-default-features --features kmp-host -- identity onboarding hud_bridge
./scripts/check.sh                                                # tudo que o CI local roda
```
Logs no aparelho: `adb logcat | grep -i -E "bevy|game|panic"`; no app, `files/status.txt` (`adb shell run-as br.eng.joaovictor.resort cat files/status.txt`).

## 6. Problemas comuns
- **Sync ou build falha com "cargo/rustup not found"**: o IDE foi aberto pelo Dock sem seu `PATH`. Abra pelo terminal ou ajuste o `~/.zprofile` (seção 5).
- **"SDK location not found"**: crie `kmp/local.properties` (`sdk.dir=...`) ou exporte `ANDROID_HOME`.
- **JDK errado** (Gradle reclama de versão do Java/AGP): use JDK 17 ou 21 em *Gradle JDK*; JDK 8/11 não serve.
- **Erro de `ndk-context` / `android-activity` no Android**: vem do caminho antigo (winit); o host KMP **não** usa `android-activity` (o `android_main` é o vazio do `bevy-kmp-ffi`). Se aparecer, o build está sem `--no-default-features --features kmp-host`, ou há cache de Gradle velho: `./gradlew clean` e rebuild.
- **`libgame.so` com GB de tamanho**: debuginfo ligado. O Gradle já define `CARGO_PROFILE_DEV_DEBUG=0`; se rodou `cargo build` manual, apague `target/` antes de empacotar.
- **Falha ao baixar o `bevy_kmp` (git)**: login do GitLab no terminal (`git ls-remote https://gitlab.com/joaovictorsouza/bevy_kmp.git` deve funcionar sem pedir senha).
- **Login Google falha no Android com `GetCredentialException`**: APK assinado com a chave de debug; assine com a keystore do projeto (seção 4).
- **Disco cheio**: cada alvo de Bevy usa vários GB em `target/`; `cargo clean` e `./gradlew clean` liberam, ao custo de recompilar.

## 7. O que foi verificado nesta máquina
Executado em 25/10/2026, Mac Apple Silicon, JDK 21 (SDKMAN), NDK 30.0.15729638, `ANDROID_HOME` definido, sem `kmp/local.properties`:
- `./gradlew :androidApp:assembleDebug` a partir de `kmp/`, build do zero (com o Rust): **sucesso em 12 min**, APK de ~292 MB (debug, sem debuginfo no `.so`). O IntelliJ chama o Gradle de forma equivalente **[suposição]**.
- `./scripts/check.sh`: **verde** (fmt, clippy desktop e `kmp-host`, testes, contrato).
- `[patch]` local do `bevy_kmp` por `--config` (sem arquivo no repositório): `cargo check -p game --no-default-features --features kmp-host` com o patch compilou o `bevy-kmp` e o `bevy-kmp-ffi` de `../bevy_kmp` (`cargo tree` mostra o caminho local).
- Edição em `crates/protocol` refez o `cargoBuildAndroidArm64Debug` (45 s).
- Pesquisa por referências aos arquivos removidos: o único código dependente era um teste que lia `ios/Info.plist` (`info_plist_tests` em `lib.rs`); foi removido junto, porque guardava o plist do app iOS antigo.

**Não executado**: abrir o IntelliJ e o sync pela interface (apenas o equivalente por linha de comando); instalar e rodar o APK no S22 (o aparelho estava desconectado) e no emulador; o `kmp-ios-device.sh` no iPhone e o app no simulador iOS (o iPhone não foi ligado; simulador não aberto para economizar memória/disco); build de release/R8; os passos de menu do IntelliJ marcados como suposição acima.

## 8. Opcionais
- **HUD de teste em Compose** (desligado por padrão): [hud-compose-teste.md](hud-compose-teste.md).
- **Portar a UI para Compose**: [portar-ui-para-compose.md](portar-ui-para-compose.md).
- **Checagens antes de commitar**: `./scripts/check.sh`.
