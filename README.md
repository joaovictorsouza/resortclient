# resortclient

Cliente do Resort Game (Rust + Bevy), para desktop, Android e iOS.

## Quick start (desktop)

Pré-requisitos: Rust (toolchain de `rust-toolchain.toml`, com `rustfmt` e `clippy`).

```bash
make client                         # abre o jogo apontando para 127.0.0.1:7350
GAME_DEVICE_ID=alice make client    # segundo jogador em outro terminal: GAME_DEVICE_ID=bob
make check                          # fmt, clippy e testes
```

## Servidor

O servidor fica em [resort-game-server](https://github.com/joaovictorsouza/resort-game-server).
- Local: `make stack-up` sobe PostgreSQL + Nakama com a imagem `ghcr.io/joaovictorsouza/resort-game-server:develop`
  (precisa de `docker login ghcr.io` se a imagem for privada). Ou rode `make up` dentro do repositório do servidor.
- DEV: `environments/dev.env` aponta para `resort-svr-dev.joaovictor.eng.br`.

## Builds mobile

O mobile roda no host Kotlin Multiplatform (`kmp/`); o Android e o iOS nativos antigos foram removidos ([ADR 0014](ai-docs/decisions/0014-remocao-do-mobile-nativo-antigo.md)).
- Android: `cd kmp && ./gradlew :androidApp:assembleDebug` (instalação e emulador em [rodando-do-zero](ai-docs/architecture/rodando-do-zero.md)).
- iOS: `NAKAMA_SERVER_KEY=... ./scripts/kmp-ios-device.sh <udid> Debug` (aparelho) ou o simulador, como no guia acima.
- Ambiente: `ENVIRONMENT=dev|prod` escolhe `environments/<nome>.env`.

## Contrato com o servidor

`contract/` é uma cópia do contrato do servidor. Atualize com `make sync-contract` (espera o servidor em `../resort-game-server`).

## Mapa

`crates/`, `assets/`, `tools/`, `ai-docs/` (comece por [ai-docs/README.md](ai-docs/README.md)), `.agents/skills/`. Regras para agentes em [AGENTS.md](AGENTS.md).
