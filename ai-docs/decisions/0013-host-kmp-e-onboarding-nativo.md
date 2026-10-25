# ADR 0013: Host Kotlin Multiplatform (bevy-kmp), onboarding nativo e login Google

Status: aceita (branch `feat/bevy-kmp`).

## Contexto
O cliente passa a rodar dentro de um app Kotlin Multiplatform com Compose Multiplatform (Android e iOS) usando a biblioteca `bevy-kmp`, em vez do winit. Todas as telas continuam em `bevy_hui`, exceto o onboarding (login Google, refresh token no Keychain/Keystore, nome de usuário), que passa para o Compose.

## Decisão: login Google
Avaliamos a biblioteca **KMPAuth** (`io.github.mirzemehdi:kmpauth-google`, Apache-2.0, mantida; última versão 3.0.6 em 2026-09). Descartada por ora:
- A versão atual exige Kotlin 2.4, Compose Multiplatform 1.11 e AGP 9; o projeto está preso a Kotlin 2.2.21, Compose 1.9.3 e AGP 8.13 por causa do Gobley 0.3.7 (UniFFI).
- No iOS ela depende de CocoaPods e do SDK GoogleSignIn gerenciado pelo plugin; o cliente já tem `GoogleSignInBridge.swift` e o fluxo UIScene/redirect próprio.
- Parte do valor dela é Firebase/Supabase, que o cliente não usa: o Nakama `authenticate/google` recebe o ID token direto.

Escolha: uma interface própria **`GoogleAuth`** em `commonMain` (`suspend fun idToken(serverClientId): Result`) com implementações pequenas:
- Android: Credential Manager + `GetGoogleIdOption(serverClientId)` (mesma abordagem do `GoogleSignIn.java` antigo).
- iOS: GoogleSignIn-iOS (SPM) em Swift, injetado no Kotlin por um protocolo exportado (`GoogleSignInProvider`); o redirect é tratado pelo SwiftUI (`onOpenURL`).

Prós: sem dependência nova incompatível, sem Firebase, mantém client IDs e plist atuais, ~100 linhas por plataforma. Contras: manutenção própria; revisar a decisão quando o projeto puder subir para Kotlin 2.4.

## Fluxo
Rust (estado e regras) continua dono do `AppState`, de `apply_google_result`, `persist_refresh_token` e `submit_nickname`. O Compose só mostra as telas e envia mensagens tipadas (`onboarding_google`, `onboarding_nickname`, `onboarding_device_login`); o Rust devolve `onboarding_stage {stage, busy, error, dev}`. O token Google atravessa o canal de serviços (`google_sign_in`) e o refresh token fica no Keychain/Keystore (`secure_get/put/delete`); nenhum token vai para log.
