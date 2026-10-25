## O quê
O mobile (Android e iOS) passa a rodar dentro de um app **Kotlin Multiplatform + Compose Multiplatform** que hospeda o Bevy numa view nativa, com a biblioteca [`bevy-kmp`](https://gitlab.com/joaovictorsouza/bevy_kmp) (sem winit). O desktop não muda. Só o **onboarding** (login Google, nome de usuário) virou Compose; todas as outras telas continuam em `bevy_hui`.

## Como ficou
- `crates/game`: duas features. `winit-host` (padrão: desktop e o caminho antigo de mobile) e `kmp-host` (mobile: `cdylib` + `staticlib`, `bevy_kmp_ffi::export!`, sem winit, sem `bevy_ios_app_delegate`).
- `kmp/`: shell Gradle (`shared`, `androidApp`) e app iOS (xcodegen). Detalhes em `ai-docs/architecture/kmp-host.md` e ADR 0013.
- Onboarding nativo: o Rust continua dono do estado e das regras (`identity.rs`); o Compose desenha e troca mensagens tipadas (`onboarding_stage`, `onboarding_google`, `onboarding_nickname`...). Google: Credential Manager (Android) e GoogleSignIn-iOS (iOS); o refresh token fica no Keychain/Keystore pelo Kotlin (o `secure_store` e a ponte Google pedem ao shell por serviços nativos).
- Conta que já tem nome e aparência no servidor pula o criador de personagem (`get_avatar_appearance`, sem mudar o servidor).
- Ferramentas de dev: botão "Limpar sessão (dev)", `-ResetSession` no iOS, `status.txt`/`game.log` no container.
- `scripts/check.sh` agora também faz clippy e testes da feature `kmp-host`.
- Dependência: `bevy-kmp-ffi` por **git + tag v0.1.5** e `[patch.crates-io] raw-window-metal` (cópia patchada dentro do repositório do `bevy-kmp`, necessária no iOS). `.cargo/config.toml` ativa `git-fetch-with-cli` por causa do repositório privado.

## Como buildar
```bash
./scripts/check.sh
cd kmp && ./gradlew :androidApp:assembleDebug            # Android
NAKAMA_SERVER_KEY=... ./scripts/kmp-ios-device.sh <udid> Debug   # iPhone (a chave vem do ambiente)
```
O Gradle resolve a biblioteca Kotlin `br.eng.joaovictor:bevy-kmp` por `includeBuild` se o checkout do `bevy_kmp` estiver ao lado (`../../bevy_kmp`), senão por `mavenLocal` ou pelo registro Maven do GitLab (`GITLAB_TOKEN`).

## Validado
- iPhone 16 Pro (físico): login Google real, sessão persistida no Keychain, criador pulado para conta existente, sala, toque no chão, HUD/loja/inventário (picking), fechar e reabrir, segundo plano e volta, ~110 FPS e 290 MB residentes (Debug).
- Android Emulator: fluxo do onboarding até o criador de personagem.
- `./scripts/check.sh` verde (fmt, clippy desktop e `kmp-host`, testes).

## NÃO validado
- Android em aparelho real (login Google/Keystore, jogo completo) e Release com R8.
- Teclado nos `TextInput` do `bevy_hui` (ponte de teclado não feita), áudio, multitoque da câmera no Android, aviso de memória do iOS.
- Compose semitransparente/`Dialog`/`ModalBottomSheet` sobre a cena (próximo passo: teste de HUD em Compose).
- Publicação da biblioteca no registro Maven do GitLab (depende do João).
Lista completa em `docs/relatorio-final.md`.

## Observações
- O histórico desta branch é a continuação do GitLab (`main`); uma branch anterior do trabalho partiu do clone legado do GitHub e foi reaplicada aqui.
- Nenhum segredo no diff (`NAKAMA_SERVER_KEY` só pelo ambiente em tempo de build).
