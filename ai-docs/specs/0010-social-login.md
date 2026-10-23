# Spec 0010: Login Social (Apple e Google) por Plataforma

## Status
Draft (o Google nativo em Android/iOS já está implementado na [Spec 0011](0011-login-google-nativo.md))

## Contexto e Objetivo
Hoje o cliente autentica apenas por `device_id` (Spec 0002, *guest-first*) via `authenticate_device` em `crates/nakama_client/src/rest.rs`. A Spec 0002 deixou o login social fora do escopo e preparou o *account linking*. Esta spec define o login com conta Apple e Google, com a matriz de plataformas abaixo, e torna o **login social obrigatório**: não há modo convidado em produção. O `device_id` deixa de ser credencial e fica restrito a builds de desenvolvimento.

| Plataforma | Google | Apple | Mecanismo primário |
|---|---|---|---|
| iOS | Sim | Sim (obrigatório, App Store 4.8) | Apple: `AuthenticationServices` nativo. Google: OAuth PKCE via `ASWebAuthenticationSession` |
| Android | Sim | Não | Credential Manager + Sign in with Google (via ponte JNI em `MainActivity.java`) |
| Desktop (macOS/Win/Linux) | Sim | Sim | OAuth 2.0 Authorization Code + PKCE no navegador do sistema (redirect loopback) |

Princípio: o cliente obtém um **token do provedor** (ID token) e o entrega ao Nakama, que o valida e emite a sessão. Nenhum segredo de provedor fica no cliente.

## Escopo
- **Dentro do escopo**:
  - Autenticar e criar conta via Google e Apple no Nakama.
  - Exigir login social para entrar no jogo; sem botão de convidado.
  - Migrar contas guest existentes (device) para contas sociais via *link*, preservando inventário, moedas e salas.
  - Persistir e renovar a sessão (`token` + `refresh_token`) com armazenamento seguro.
  - Tela de login com os botões disponíveis na plataforma.
  - Logout. Troca de provedor fica fora (ver abaixo).
  - Bloquear device auth no servidor em produção; mantê-lo só em desenvolvimento (`--dev`, `--profile`).
- **FORA do escopo**:
  - Outros provedores (Facebook, Discord, e-mail/senha).
  - Vincular um segundo provedor à mesma conta e desvincular (cada conta tem um único provedor por ora).
  - Modo offline/convidado em produção.
  - Moderação de nickname e exclusão de conta (a exclusão de conta é exigida pela Apple quando há criação de conta, tratar em spec própria antes do envio à App Store).
  - Publicação nas lojas e configuração de CI de assinatura.

## Decisões a registrar (ADR)
1. **Login social obrigatório (decidido)**. Consequências: maior atrito na primeira execução e dependência dos provedores; em troca, identidade recuperável, menos contas descartáveis e fim do `device_id` como credencial. Registrar como ADR, superando a premissa *guest-first* da Spec 0002.
2. **Abstração por trait** `SocialAuthProvider` com implementações por `cfg(target_os)`, em vez de um SDK por plataforma espalhado em `lib.rs`.
3. **Apple no desktop**: fluxo web com Services ID exige redirect HTTPS (loopback não é aceito). Opções: (a) relay mínimo hospedado (página/endpoint que recebe o `form_post` e redireciona para `127.0.0.1:<porta>`); (b) no macOS, usar `ASAuthorizationAppleIDProvider` nativo e web flow só em Windows/Linux. Recomendado: (b) + (a).

## Requisitos Funcionais e Técnicos

### 1. Servidor (Nakama, repositório resort-game-server)
- `local.yml` (e config de produção): habilitar `social.apple.bundle_id` e credenciais Google conforme a documentação do Nakama 3.41. **Verificar** na versão 3.41 qual formato de token Google é aceito (ID token JWT vs access token) e como o Nakama valida `aud` para Apple.
- Risco Apple: o Nakama valida o `aud` contra um único `bundle_id`. O fluxo web (desktop Win/Linux) usa o Services ID como `aud`. Confirmar se há suporte a múltiplos valores; senão, usar o Services ID também no app iOS/macOS ou tratar via hook.
- Segredos (client secrets, chaves .p8) apenas via variáveis de ambiente/secret manager, nunca no repositório.
- Produção: rejeitar `AuthenticateDevice` com um *before hook* (`RegisterBeforeAuthenticateDevice`) controlado por variável de ambiente; liberado apenas em dev/local. Rejeitar também `create=true` de device.
- Revisar hooks existentes (criação de carteira/inventário inicial) para disparar também em contas criadas por login social (`AfterAuthenticate*`).
- Docs: atualizar `resort-game-server/ai-docs/architecture` e criar `resort-game-server/ai-docs/specs/0006-social-auth-config.md` (curto, só configuração e variáveis).

### 2. Camada de rede (`crates/nakama_client`)
Adicionar em `rest.rs`, com testes de serialização/erros (mock HTTP):
- `authenticate_google(token, create, username)` → `POST /v2/account/authenticate/google`.
- `authenticate_apple(token, create, username)` → `POST /v2/account/authenticate/apple`.
- `link_google` / `link_apple` (`/v2/account/link/*`), exigindo sessão válida, usados só na migração de contas guest legadas. Sem `unlink`.
- `refresh_session(refresh_token)` → `POST /v2/account/session/refresh`.
- Mapear `409` (conta social já vinculada a outro usuário) para um erro tipado `RestError::AccountConflict`.

### 3. Cliente (`crates/game`)
- Novo módulo `social_auth.rs`:
  ```rust
  pub enum Provider { Google, Apple }
  pub struct ProviderToken { pub provider: Provider, pub id_token: String }
  pub trait SocialAuthProvider: Send + Sync {
      fn available() -> Vec<Provider>;           // define quais botões aparecem
      fn sign_in(&self, p: Provider) -> Receiver<Result<ProviderToken, SocialAuthError>>;
  }
  ```
- Funções puras e testáveis em `oauth_pkce.rs`: geração de `code_verifier`/`code_challenge` (S256), `state` anti-CSRF, montagem da URL de autorização, parsing do callback e servidor loopback efêmero (porta aleatória, aceita uma requisição, timeout).
- Implementações por plataforma (`cfg(target_os)`):
  - **Desktop**: PKCE + loopback + abertura do navegador do sistema (`open`/`webbrowser`). Google via cliente OAuth tipo "Desktop". Apple via relay (ver ADR 3).
  - **Android**: método Java estático em `android/java/dev/newgame/client/MainActivity.java` que chama Credential Manager (`GetGoogleIdOption` com `serverClientId` = Web client ID) e devolve o ID token ao Rust por JNI/canal. Google apenas; Apple não aparece.
  - **iOS**: Apple via `ASAuthorizationController` (identity token + nonce); Google via `ASWebAuthenticationSession` reutilizando `oauth_pkce.rs`. Código ObjC em `ios/` (ou `objc2` no Rust). Adicionar capability *Sign in with Apple* e URL scheme do redirect do Google em `Info.plist`.
- **Nonce**: gerar nonce aleatório por tentativa; enviar o hash ao provedor e validar no retorno (Apple), contra *replay*.
- **Estados** (`state.rs`): inserir `AppState::Login` entre `InitAuth` e `ChooseNickname`:
  - `InitAuth`: se há sessão salva e válida/renovável → `Connecting`; se há perfil guest legado (`device_id` sem provedor) → `Login` em modo migração; senão → `Login`. Em build de dev com `--dev`/`--profile`, usa device auth e pula `Login`.
  - `Login`: apenas botões de provedor, sem saída alternativa. Sucesso em conta nova → `ChooseNickname` (se o provedor não der nome utilizável) → `CharacterCreator`/`Connecting`. Conta existente → `Connecting`.
  - Estado de erro/cancelamento volta a `Login` com mensagem; cancelamento do usuário não é erro.
- **Migração de guest legado**: no `Login` em modo migração, autenticar o device (`create=false`) para obter sessão, obter o token do provedor e chamar `link_*` na conta guest; os dados são preservados. Em `AccountConflict` (provedor já ligado a outra conta), mostrar confirmação explícita: entrar na conta social existente descarta o guest local, pois dados não são mesclados. Remover o modo migração após uma janela definida (decisão de produto) e então bloquear device no servidor.
- **UI**: HTML/Tailwind em `assets/ui/` seguindo a Spec 0009 (responsivo/mobile); alvos de toque ≥ 44 pt; respeitar *safe area*. Seguir as guidelines de marca dos botões Apple e Google.
- **Armazenamento seguro**: tokens em Keychain (iOS/macOS), Android Keystore/EncryptedSharedPreferences, e Credential Manager do SO/crate `keyring` no Windows/Linux. `profile.json` guarda só dados não sensíveis (nickname, provedor usado). O `device_id` só persiste em perfis de desenvolvimento e na migração legada; ele é equivalente a uma credencial até a migração acabar.
- `GameConfig` (`config.rs`): novo `auth_mode` (`Social | DevDevice`). `DevDevice` só é aceito em `debug_assertions` ou com feature `dev-auth`, mantendo `--dev`, `--profile` e `--device-id` para multi-cliente local. Builds de release ignoram essas flags.

## Fases de Entrega
1. **Fundação** (sem SDK nativo): endpoints REST novos, `oauth_pkce.rs`, trait `SocialAuthProvider`, `AppState::Login` e UI, tudo testável com provedor *fake*.
2. **Servidor + Google desktop**: config Nakama, PKCE + loopback, e2e local com o stack Docker (`run-local-stack`).
3. **Android Google**: ponte JNI, Credential Manager, teste em dispositivo físico (já suportado pelo `run-android.sh`).
4. **iOS Apple + Google**: `AuthenticationServices` e `ASWebAuthenticationSession`, capability e Info.plist.
5. **Apple desktop**: macOS nativo e relay web para Windows/Linux.
6. **Migração de guests legados, armazenamento seguro e renovação de sessão**; bloqueio de device auth em produção; exclusão de conta.

## Pré-requisitos Externos (fora do código)
- Google Cloud: tela de consentimento, clientes OAuth Web (usado como `serverClientId`), Android (SHA-1 de debug e release), iOS e Desktop.
- Apple Developer: App ID com *Sign in with Apple*, Services ID, chave .p8 e domínio verificado para o relay.
- Domínio HTTPS para o relay e para as políticas de privacidade exigidas pelas lojas.

## Critérios de Aceite (Testáveis)
- [ ] `code_challenge` S256 reproduz os vetores de teste da RFC 7636.
- [ ] O callback com `state` divergente ou sem `code` é rejeitado; o servidor loopback encerra após uma requisição ou timeout.
- [ ] `authenticate_google/apple` enviam o payload correto, e `409` resulta em `AccountConflict`.
- [ ] `available()` retorna `[Apple, Google]` em iOS e desktop, e `[Google]` em Android.
- [ ] Sem sessão salva, o cliente entra em `Login`, que não oferece caminho de convidado; com sessão válida ou renovável, conecta direto.
- [ ] Cancelar o login volta a `Login` sem erro fatal.
- [ ] Usuário guest legado com inventário migra para conta social e mantém inventário, moedas e salas (teste e2e no Nakama local).
- [ ] Conflito de vínculo exibe a confirmação e não apaga dados sem consentimento.
- [ ] Tokens não aparecem em `profile.json` nem em logs.
- [ ] Em build de dev, `--dev` e `--profile` continuam funcionando sem login social; em build de release essas flags são ignoradas.
- [ ] Com o bloqueio ativo, o servidor rejeita `authenticate/device`; com a variável de dev, aceita.
- [ ] `cargo clippy`, `cargo test` e `go test -race` passam; lógica pura (PKCE, parsing, máquina de estados) com cobertura ≥ 85%.

## Plano de Testes e Validação
- **Unitários**: PKCE, URL de autorização, parsing de callback, transições de `AppState`, serialização REST, `AccountConflict`.
- **Integração**: cliente REST contra Nakama local com tokens de teste (provedor *fake* ou ambiente de sandbox do Google/Apple).
- **Manual por plataforma** (não automatizável): checklist em dispositivo físico Android, simulador/dispositivo iOS (Apple exige dispositivo real para alguns fluxos) e os três desktops.

## Riscos
- Validação de `aud` do Apple no Nakama com múltiplos clientes (ver Servidor).
- Apple no desktop depende de infraestrutura hospedada (relay).
- Divergência entre o formato de token aceito pelo Nakama 3.41 e o emitido pelo Credential Manager/PKCE (verificar cedo, na Fase 2).
- Login obrigatório aumenta o atrito e a dependência de Google/Apple (indisponibilidade ou conta banida bloqueia o jogador); definir plano de suporte.
- Revisão das lojas: exigência de exclusão de conta e do botão Apple quando há Google no iOS.
