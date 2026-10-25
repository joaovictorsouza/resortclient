# Relatório final: cliente no host KMP (branch `feat/bevy-kmp`)

Estado em 25/10/2026. O Resort Game roda dentro de um app Kotlin Multiplatform + Compose Multiplatform (Android e iOS) usando a biblioteca `bevy-kmp` (0.1.3), sem winit. Só o onboarding virou Compose; todas as outras telas continuam em `bevy_hui`.

## O que foi feito
- **Crate `game`**: features `winit-host` (padrão; desktop e o caminho antigo) e `kmp-host` (mobile; sem winit, `bevy_ios_app_delegate`, `android_main` nem `main_rs`), `cdylib + staticlib` com `bevy_kmp_ffi::export!`.
- **Adaptações**: assets embutidos lidos de um diretório (Android copia do APK; iOS lê do bundle) com o `cdn_assets` injetável; `gpu_compat` virou `gpu_hook`; `game_log` virou `log_layer`; `secure_store` e a ponte Google pedem ao shell por serviços nativos; `SafeArea` vem do resource `Insets`; reconexão ao voltar do background pelo `AppLifecycle`.
- **Shell** em `kmp/` (`shared`, `androidApp`, `iosApp` com xcodegen): host criado fora da UI, `dev.env` gerado no build com as variáveis do ambiente (a chave do servidor só em `NAKAMA_SERVER_KEY`, nunca no repositório).
- **Onboarding em Compose**: login Google (Credential Manager no Android; GoogleSignIn-iOS por Swift/SPM no iOS), refresh token no Keychain/Keystore, tela de nome (regra espelhada; a validação continua em Rust), mensagens tipadas, telas `bevy_hui` antigas atrás do resource `NativeOnboarding` (ADR 0012).
- **Toque fantasma**: um dedo que estava na tela quando o app foi para segundo plano nunca mandava o `Up`; ao reabrir, o jogo via dois ponteiros e tratava um toque como pinça da câmera. O `bevy-kmp` 0.1.5 cancela todos os ponteiros ativos ao pausar e ao desanexar a superfície (teste do runner: dois dedos presos geram Cancel nos dois e `Touches` volta a 0) e passa a escrever o `AppLifecycle` do Bevy, que o `net.rs` usa para reconectar o Nakama ao voltar do background.
- **Correções do caminho**: crash na segunda abertura (resposta de serviço antes de `hostRef` existir), toque no `bevy_picking` (bevy-kmp 0.1.3), Keychain que sobrevive à reinstalação no iOS (marcador de instalação e `-ResetSession`), botão dev "Limpar sessão", pulo do criador de personagem para conta com aparência no servidor (`get_avatar_appearance`), mensagem clara para "Server key invalid" em dev, APK debug de 2,7 GB para 292 MB.

## Como testar
```bash
# iPhone (a chave vem do ambiente; nunca imprima):
zsh -c './scripts/kmp-ios-device.sh <udid-do-iphone> Debug'
# Android (APK debug, emulador ou aparelho): cd kmp && ./gradlew :androidApp:assembleDebug
cargo test -p game --no-default-features --features kmp-host   # lógica (24 testes de identidade/onboarding)
```
Fluxo: login Google, nome de usuário, criador (ou direto à sala para quem já tem aparência no servidor), fechar e reabrir sem pedir login. Em dev há "Limpar sessão (dev)" no canto superior esquerdo.

## Números
Medido no iPhone 16 Pro (A18 Pro, Metal), build **Debug**, jogo real na sala `lobby` conectado ao servidor dev (leitura do `status.txt` e do `game.log` do container):
- FPS: 110 (tela de 120 Hz).
- Memória residente: 290 MB.
- Tamanho do app instalado: ~370 MB (Debug, segundo o João).
- Sessão restaurada do Keychain sem pedir login; Nakama conectado em ~1,1 s depois do perfil.
Exemplo da biblioteca (`bevy_kmp`): ver `docs/medicoes.md` lá. Paridade e endurecimento (Release/R8, medições completas) foram pulados por decisão do João.

## Ficou FORA / não validado
- **Android real**: login Google e Keystore não foram testados em aparelho, por decisão do João. Risco conhecido: Activity/Credential Manager e SHA-1 do keystore cadastrado no Google Cloud. No emulador só foi verificado o onboarding com o login de dev.
- **Teclado nos `TextInput` do `bevy_hui`** (inventário, loja, navegador): ponte de teclado não feita; o João migrará essas telas.
- **Áudio**: o `ndk-context` é inicializado no Android, mas áudio não foi exercitado nem testado com pausa de ciclo de vida.
- **Release**: R8 está configurado (`proguard-rules.pro`) mas não foi validado em aparelho; builds medidos foram Debug.
- **WebSocket em segundo plano / reconexão** em longa duração; **multitoque** no jogo (câmera) e **teclado flutuante do iPad**; **aviso de memória do iOS**.
- **Otimização sugerida ao servidor** (não implementada): incluir `appearance_set` em `get_profile_status`, para o cliente não depender de "aparência diferente do padrão" nem fazer uma ida extra à rede.
- **Publicação Maven** da biblioteca depende do João (`scripts/publish-kotlin.sh`); o cliente consome por `includeBuild` com as mesmas coordenadas.
- **Compose sobre a cena 3D (a validar depois)**: elementos Compose semitransparentes sobre o jogo (no Android, `SurfaceView`; plano B `TextureView`), `Dialog`/`Popup`/`ModalBottomSheet` sobre a cena, animações desses elementos e a passagem de toque do overlay para o mundo (hoje o overlay do onboarding cobre a tela inteira e só aparece fora do jogo).
- O commit da branch não foi enviado ao remoto do cliente.

## Próximo passo aprovado pelo João (depois do P4)
Teste de viabilidade de um **HUD em Compose** mínimo sobre o jogo, sem remover o HUD atual em `bevy_hui` (coexistência por flag/feature; o HUD atual segue como padrão): status da conexão, nome do jogador, saldo e um botão, alimentados por snapshots tipados vindos do Bevy, mais um comando Compose para Bevy (por exemplo "abrir menu"), translúcido sobre a cena 3D. Serve também para validar no iPhone o item acima e medir o impacto em FPS. Escopo, resultado e um ADR curto serão registrados quando for feito.
