# Architectural Decision Records (ADRs) - Client

Este diretório registra as decisões técnicas do cliente Rust/Bevy.

## Índice de Decisões
- [ADR 0001: Bevy 0.19 e Avian 0.7](./0001-bevy-0.19-avian-0.7.md)
- [ADR 0002: Cliente Nakama Customizado em Rust](./0002-custom-nakama-client.md)
- [ADR 0003: Separação em Cargo Workspace (protocol, nakama_client, game)](./0003-cargo-workspace-split.md)
- [ADR 0004: Movimentação por Clique com Caminho Autoritativo](./0004-click-to-move-server-path.md)
- [ADR 0005: Arquitetura Cross-Platform (Desktop + Mobile)](./0005-cross-platform-architecture.md)
- [ADR 0006: Modo Construção, Preview Holográfico e Renderização Paramétrica de Mobílias](./0006-room-building-mode-and-furniture-rendering.md)
- [ADR 0007: Pipeline Modular de Personagens Teenz 3D e Customização em Camadas](./0007-modular-character-pipeline-and-customization.md) (parcialmente substituída pela 0010)
- [ADR 0008: Pipeline de Animação do Avatar (esqueleto único + retarget do Mixamo)](./0008-avatar-animation-pipeline.md) (substituída pela 0010)
- [ADR 0009: bevy_hui com Tailwind em build time e ícones Phosphor rasterizados](./0009-bevy-hui-tailwind-tokens.md)
- [ADR 0010: Pacote Creative Characters, esqueleto único de 44 joints e poses via Mixamo](./0010-creative-characters-pack-and-single-skeleton.md)
- [ADR 0011: Assets pesados e catálogos fora do app](./0011-assets-via-cdn.md)
- [ADR 0012: Cliente público, assets só pela CDN e GitHub Actions manuais](./0012-cliente-publico-e-github-actions-manuais.md)
- [ADR 0013: Host Kotlin Multiplatform (bevy-kmp), onboarding nativo e login Google](./0013-host-kmp-e-onboarding-nativo.md)
- [ADR 0014: Remoção do mobile nativo antigo (winit)](./0014-remocao-do-mobile-nativo-antigo.md)
- [ADR 0015: Personagens toon numa crate reutilizável (`toon_avatar`)](./0015-crate-de-personagens-toon.md)
- [ADR 0017: Telas do jogo migradas de bevy_hui para Compose no host KMP](./0017-telas-compose-no-host-kmp.md)
