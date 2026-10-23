# Arquitetura Multi-Plataforma: Desktop, Android e iOS

O cliente foi projetado para compilar e executar em **Desktop** (macOS, Windows, Linux) e **Mobile** (Android e iOS).

## Estratégia de Compilação
- `crates/game/src/lib.rs`: Configurado no `Cargo.toml` como `crate-type = ["lib", "cdylib"]`.
  - No Android, a biblioteca `libgame.so` é carregada pela `NativeActivity` ou `cargo-apk`.
  - No iOS, a biblioteca é consumida pelo projeto Xcode do Bevy.
- `crates/game/src/main.rs`: Ponto de entrada padrão para builds Desktop.

## Abstração de Entradas (Mouse e Toque)
O sistema `handle_pointer_click_or_tap` abstrai o input:
- No Desktop: lê `ButtonInput<MouseButton>::just_pressed(MouseButton::Left)` e a posição do cursor.
- No Mobile: lê `Touches::iter_just_pressed()` e a posição de toque na tela.
Ambos projetam o mesmo raio 3D através da câmera para identificar o tile tocado.

## iOS (projeto Xcode)

O projeto vive em `ios/` (bundle id `br.eng.joaovictor.resort`), descrito em `ios/project.yml` (XcodeGen) e gerado por `scripts/gen-ios-project.sh` (`brew install xcodegen`). Uma fase de build executa `cargo build -p game --lib` para o alvo certo e linka `libgame.a`; o login Google usa GoogleSignIn-iOS por Swift Package Manager (sem CocoaPods). Veja a [Spec 0011](../specs/0011-login-google-nativo.md).

```bash
scripts/run-ios-sim.sh --dev      # simulador
DEVELOPMENT_TEAM=ABCDE12345 scripts/export-ipa.sh   # IPA de desenvolvimento
```

- `run-ios-sim.sh` cria/boota o simulador "new_game iPhone", compila com `xcodebuild` (sem assinatura), instala e abre. Argumentos após o script vão para o jogo (ex.: `--dev`). `NAKAMA_HOST`/`NAKAMA_PORT` apontam para o servidor (padrão `127.0.0.1:7350`).
- O `DEVELOPMENT_TEAM` (assinatura automática com o time pessoal) fica em `ios/Config/Local.xcconfig` (ignorado pelo git; veja o `.example`) ou na variável de ambiente.
- A primeira build leva ~15 min (Bevy completo para um target novo).
- **Delegates**: a ponte do Google não define `UIApplicationDelegate` nem `SceneDelegate` (quem os fornece é a crate `bevy_ios_app_delegate`, que adota o ciclo de vida UIScene exigido pelo iOS 27 quando o `Info.plist` declara `UIApplicationSceneManifest`). O redirect chega como `AppDelegateCall::OpenURL`, e `ios_google::forward_open_url` o entrega a `resort_google_handle_url(url)`. A tela do Google é apresentada pelo view controller do topo da `UIWindowScene` em primeiro plano. As chaves do Google ficam no fim de `ios/Info.plist`, longe do `UIApplicationSceneManifest`.

## Android (aparelho físico)

```bash
scripts/run-android.sh     # APK debug via Gradle, instala e abre
scripts/build-apk.sh       # APK release assinado: target/android/resort-release.apk
```

- Projeto Gradle em `android` (`applicationId br.eng.joaovictor.resort`, minSdk 26, targetSdk 35). `scripts/build-android-lib.sh` gera `libgame.so` com `cargo ndk` em `target/android/jniLibs`; o Gradle empacota a lib, `assets` e um `dev.env` gerado.
- Requer `cargo install cargo-ndk --locked`, NDK e platform-tools do SDK (`ANDROID_HOME`). API mínima 26 (AAudio).
- Release: keystore em `RESORT_KEYSTORE` (padrão `~/Documents/keystore/resort/dev/dev-release.keystore`) e senhas apenas por `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`. O script imprime o SHA-1 para cadastrar no Google Cloud.
- O servidor local é alcançado por `adb reverse tcp:7350 tcp:7350` (USB). Para Wi-Fi: `NAKAMA_HOST=<ip-do-mac>`. As variáveis vão no `assets/dev.env` do APK (`GAME_DEV_MODE=1` no `run-android.sh`, `0` no `build-apk.sh`); `EXTRA_DEV_ENV='CHAVE=valor\n'` injeta outras, como `RUST_LOG`.
- Nas Adrenos 650 e 730 há um workaround de skinning (`gpu_compat.rs`); ver [investigacao-android-skinning.md](../process/investigacao-android-skinning.md).
