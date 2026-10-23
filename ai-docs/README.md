# Documentação do Cliente (Rust / Bevy)

Esta pasta contém a base de conhecimento viva do cliente do **new_game**.

## Estrutura da Documentação

- [**process/**](./process/development-workflow.md): Guias operacionais para desenvolvimento de sistemas Bevy e testes.
  - [development-workflow.md](./process/development-workflow.md): Ciclo de desenvolvimento no client.
  - [testing.md](./process/testing.md): Metodologia de testes unitários de lógica pura e sistemas Bevy.
  - [code-style.md](./process/code-style.md): Padrões de código, organização em plugins e tratamento de estado.
  - [definition-of-done.md](./process/definition-of-done.md): Critérios para validação de features do client.
  - [importar-pacote-de-personagens.md](./process/importar-pacote-de-personagens.md): Como trocar ou atualizar o pacote de personagens.
- [**architecture/**](./architecture/overview.md): Desenho técnico e decisões estruturais do cliente.
  - [overview.md](./architecture/overview.md): Visão geral dos crates (`protocol`, `nakama_client`, `game`).
  - [plugins-and-states.md](./architecture/plugins-and-states.md): Árvore de plugins Bevy e máquina de estados (`AppState`).
  - [networking.md](./architecture/networking.md): Integração entre Tokio assíncrono e o loop de renderização do Bevy.
  - [world-and-tiles.md](./architecture/world-and-tiles.md): Coordenadas discretas vs mundo 3D, colliders e raycast.
  - [platforms.md](./architecture/platforms.md): Suporte e estratégia para Desktop (macOS/Win/Linux) e Mobile (Android/iOS).
- [**decisions/**](./decisions/README.md): Architectural Decision Records (ADRs).
  - [0001: Bevy 0.19 e Avian 0.7](./decisions/0001-bevy-0.19-avian-0.7.md)
  - [0002: Cliente Nakama Customizado em Rust](./decisions/0002-custom-nakama-client.md)
  - [0003: Separação em Cargo Workspace](./decisions/0003-cargo-workspace-split.md)
  - [0004: Movimentação por Clique com Caminho Autoritativo](./decisions/0004-click-to-move-server-path.md)
  - [0005: Arquitetura Cross-Platform (Desktop + Mobile)](./decisions/0005-cross-platform-architecture.md)
  - [0006: Modo de construção e renderização de móveis](./decisions/0006-room-building-mode-and-furniture-rendering.md)
  - [0007: Pipeline modular de personagem e customização](./decisions/0007-modular-character-pipeline-and-customization.md) (parcialmente substituída pela 0010)
  - [0008: Pipeline de animação de avatar](./decisions/0008-avatar-animation-pipeline.md) (substituída pela 0010)
  - [0009: bevy_hui com tokens Tailwind](./decisions/0009-bevy-hui-tailwind-tokens.md)
  - [0010: Pacote Creative Characters e esqueleto único](./decisions/0010-creative-characters-pack-and-single-skeleton.md)
- [**specs/**](./specs/README.md): Especificações de requisitos e funcionalidades (0001 a 0010).
