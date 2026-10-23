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
