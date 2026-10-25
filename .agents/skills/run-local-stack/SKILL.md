---
name: run-local-stack
description: >-
  Executa o cliente desktop contra o servidor DEV ou contra um servidor local (Nakama + PostgreSQL),
  inclusive dois jogadores simultâneos. Use para validar o jogo de ponta a ponta.
---

# Executar o cliente contra um servidor

## Opção 1: servidor local
```bash
make stack-up     # imagem ghcr.io/joaovictorsouza/resort-game-server:develop (docker login ghcr.io se for privada)
make stack-logs
```
Alternativa: no repositório do servidor, `make up` e `./scripts/smoke-test.sh`.

## Opção 2: servidor DEV
Build/execução com `ENVIRONMENT=dev` (lê `environments/dev.env`) e `NAKAMA_SERVER_KEY` do `/opt/resort/.env` da VPS.

## Dois jogadores no desktop
```bash
GAME_DEVICE_ID=alice make client   # terminal 1
GAME_DEVICE_ID=bob make client     # terminal 2
```
Os dois entram na sala pública "lobby" e se veem em tempo real.

## Mobile (host KMP)
O mobile roda pelo app Kotlin Multiplatform de `kmp/` (ver `ai-docs/architecture/kmp-host.md`).
```bash
# A chave do servidor vem do ambiente (~/.zshenv); nunca imprima nem grave.
zsh -c './scripts/kmp-ios-device.sh <udid> Debug'          # iPhone: compila, instala e abre (INSTALL=0 só compila)
cd kmp && ./gradlew :androidApp:assembleDebug && adb install -r androidApp/build/outputs/apk/debug/androidApp-debug.apk
```
Depois de abrir, `Documents/status.txt` e `Documents/game.log` do app mostram fps, memória e a conexão com o Nakama (`devicectl device copy from --domain-type appDataContainer --domain-identifier br.eng.joaovictor.resort`). Para limpar a sessão: botão "Limpar sessão (dev)" ou `-ResetSession`.
