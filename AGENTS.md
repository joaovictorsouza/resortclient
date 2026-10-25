# Diretrizes para Agentes: resortclient (Bevy 0.19 + Avian 0.7)

Cliente do Resort Game, um simulador social 3D no estilo Habbo Hotel, em **Rust** com o motor **Bevy 0.19** e física **Avian 0.7**.

Outros repositórios do projeto:
- [resort-game-server](https://github.com/joaovictorsouza/resort-game-server): servidor Go + Nakama, **dono do contrato de rede**.
- `resort-game-docs`: documentação para humanos (ainda não criada; será a última etapa da migração).

Plataformas: desktop (macOS, Windows, Linux) e mobile (Android e iOS). A lógica vive em `crates/game/src/lib.rs` (`cdylib` + `lib`); `main.rs` é o launcher desktop. No mobile o jogo roda dentro de um host Kotlin Multiplatform (`kmp/`, feature `kmp-host`, biblioteca `bevy-kmp`); ver [ai-docs/architecture/kmp-host.md](ai-docs/architecture/kmp-host.md).

## Estrutura
- `crates/protocol`: serialização e contratos de mensagens (agnóstico de engine).
- `crates/nakama_client`: REST e WebSocket para o Nakama (tokio).
- `crates/game`: biblioteca central do jogo e executável desktop.
- `contract/`: **cópia** do contrato publicado pelo servidor (fixtures, versão, catálogo de móveis). Nunca edite à mão.
- `assets/`, `tools/`: apenas fonte Inter e UI (HTML gerado por Tailwind em `tools/ui`). Modelos, animações, áudio e miniaturas vivem no repositório privado `resort-game-assets` e chegam só pela CDN.
- `android/`, `ios/`, `scripts/`, `environments/`: builds mobile antigos (winit) e ambientes (`dev.env`, `prod.env`).
- `kmp/`: host Kotlin Multiplatform do mobile (`shared`, `androidApp`, `iosApp`).
- `ai-docs/`: documentação viva (`process`, `architecture`, `decisions`, `specs`).
- `.agents/skills/`: skills para agentes.

## Regras Obrigatórias

1. **Consulte a documentação antes de codificar**:
   - Fluxo e TDD: [ai-docs/process/development-workflow.md](ai-docs/process/development-workflow.md)
   - Pirâmide de testes no Bevy: [ai-docs/process/testing.md](ai-docs/process/testing.md)
   - Plugins e ECS: [ai-docs/process/code-style.md](ai-docs/process/code-style.md)
   - Rede com Tokio e Bevy: [ai-docs/architecture/networking.md](ai-docs/architecture/networking.md)
   - Multi-plataforma: [ai-docs/architecture/platforms.md](ai-docs/architecture/platforms.md)
   - Host KMP e onboarding nativo: [ai-docs/architecture/kmp-host.md](ai-docs/architecture/kmp-host.md)
   - Rodando do zero após clonar: [ai-docs/architecture/rodando-do-zero.md](ai-docs/architecture/rodando-do-zero.md)
   - Contrato de rede: [protocol.md no servidor](https://github.com/joaovictorsouza/resort-game-server/blob/develop/contract/protocol.md). A cópia local de `contract/` só muda com `scripts/sync-contract.sh` e a skill `protocol-change-client`.
2. **Skills de `.agents/skills/`**:
   - `client-feature-development`: features no Bevy com TDD.
   - `protocol-change-client`: acompanhar uma nova versão do contrato do servidor.
   - `write-spec`: especificações antes de features complexas.
   - `record-decision`: ADRs.
   - `testing-and-coverage`: testes e metas de cobertura.
   - `run-local-stack`: rodar o cliente contra o DEV ou contra um servidor local.
3. **Plugins**: todo subsistema é um `bevy::prelude::Plugin`; separe lógica pura (testável sem janela) dos sistemas de renderização.
4. **Mobile**: use eventos de ponteiro unificados (mouse e `Touches`).
5. **Idioma**: documentação, specs e ADRs em **português**; código, identificadores, commits e comentários técnicos em **inglês**.
6. **Definition of Done**: testes unitários para código novo; `./scripts/check.sh` verde (`cargo fmt`, `clippy -D warnings` no desktop e no host KMP, `cargo test`, drift do contrato).
7. **GitHub Actions só os de [ADR 0012](ai-docs/decisions/0012-cliente-publico-e-github-actions-manuais.md)**: `check.yml` (PR) e builds manuais `build-android`/`build-ios` (`workflow_dispatch`). Rode `./scripts/check.sh` antes de commitar. Publicação de assets na CDN é manual, no repositório `resort-game-assets`. Nunca use `pull_request_target` nem anexe binários de prod como artifact.
8. **Segredos**: `NAKAMA_SERVER_KEY` e senhas de keystore entram por variável de ambiente no build, nunca no repositório.
