# Spec 0011: Login com Google Nativo (Android e iOS) e Apelido Persistente

## Status
Implemented (Android e iOS; desktop usa o login por dispositivo). Implementa a Fase 3/4 da [Spec 0010](0010-social-login.md) apenas para o Google.

## Contexto e Objetivo
Em mobile o jogador entra com a conta Google, escolhe um apelido único uma única vez e, nas próximas execuções, vai direto ao jogo. O cliente obtém um **ID token** nativo e o entrega ao Nakama, que o valida (nenhum segredo fica no cliente).

## Fluxo
1. `InitAuth` → `Login` quando não há perfil (ou o perfil é de uma conta Google: `UserProfile.google_account`).
2. `Login` (`assets/ui/src/login.html`): botão **Entrar com Google** chama `request_google_id_token()` (`google_signin.rs`).
   - Sem bridge nativa (desktop/dev) a resposta é `Unavailable` e o jogo segue com o **login por dispositivo**, sem `id_token`.
   - Cancelar ou falhar mantém a tela de login com uma mensagem.
3. Com o token, `nakama_client::check_google_account` autentica (cria a conta se for nova) e chama o RPC `get_profile_status`.
   - `nickname_set = true` → o cliente adota o username do servidor, salva o perfil local e pula `ChooseNickname`.
   - `false` → `ChooseNickname` (`nickname.html`, campo `TextInput`, regras de `validate_nickname`).
4. Em `Connecting`, o login Google se repete e, se o apelido ainda não existe no servidor, o RPC `set_nickname` o grava (único; conflito devolve erro).

## Ponte nativa
```rust
pub trait GoogleSignInBridge: Send + Sync {
    fn request_id_token(&self, server_client_id: &str, reply: Sender<GoogleSignInResult>);
}
pub fn register_google_sign_in_bridge(b: Box<dyn GoogleSignInBridge>) -> Result<(), _>;
pub fn request_google_id_token() -> Receiver<GoogleSignInResult>;
```
- **Android** (`android_google.rs` + `GoogleSignIn.java`): Credential Manager (`androidx.credentials`, `credentials-play-services-auth`, `googleid`) com `serverClientId` = Web client ID. Rust chama o método estático Java via JNI (carregando a classe pelo *class loader* da Activity) e recebe o resultado em `nativeOnResult`.
- **iOS** (`ios_google.rs` + `GoogleSignInBridge.swift`): GoogleSignIn-iOS via Swift Package Manager (sem CocoaPods). `GIDClientID`/`GIDServerClientID` e o URL scheme (client id invertido) ficam no `Info.plist`, preenchidos por `ios/Config/Base.xcconfig`.

IDs: Web `785803086063-jun3ecqn84ogjm34sf6727nnblacg0sj.apps.googleusercontent.com` (serverClientId), iOS `785803086063-3dvdtr3fa0ll312v41eit8opamc3dumk.apps.googleusercontent.com`.

## Configuração (uma vez)
- Google Cloud: cliente OAuth **Android** com pacote `br.eng.joaovictor.resort` e o **SHA-1** de cada keystore (debug: `keytool -list -v -keystore ~/.android/debug.keystore -alias androiddebugkey -storepass android`; release: o do `dev-release.keystore`); cliente **iOS** com bundle id `br.eng.joaovictor.resort`.
- Android: `scripts/run-android.sh` (debug) e `scripts/build-apk.sh` (release; senhas só por `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`; keystore em `RESORT_KEYSTORE`, padrão `~/Documents/keystore/resort/dev/dev-release.keystore`).
- iOS: `brew install xcodegen`, `scripts/gen-ios-project.sh`, copiar `ios/Config/Local.xcconfig.example` para `Local.xcconfig` com o `DEVELOPMENT_TEAM`; `scripts/run-ios-sim.sh` (simulador) e `scripts/export-ipa.sh` (IPA de desenvolvimento).
- Em modo dev (`GAME_DEV_MODE=1`, `--dev`) o login é pulado.

## Critérios de Aceite
- [x] Sem bridge, o botão cai no login por dispositivo (teste `test_google_unavailable_falls_back_to_device_login`).
- [x] Cancelamento/erro permanecem em `Login` (testes de `apply_google_result`).
- [x] Conta nova → `ChooseNickname`; conta com `nickname_set` pula a tela (`apply_profile_status`).
- [x] Perfil com `google_account` exige novo login Google na próxima execução (`config.rs`).
- [x] Servidor: `get_profile_status` e a marca `nickname_set` de `set_nickname` (`go test ./internal/rpc`).
- [ ] Teste manual em aparelho Android e iPhone reais (cadastro dos SHA-1/bundle no Google Cloud).
