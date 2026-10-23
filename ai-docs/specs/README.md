# Especificações de Requisitos (Specs) - Client

Este diretório contém as especificações de requisitos de funcionalidades do cliente.

## Índice de Especificações
- [Spec 0001: Walking Skeleton](./0001-walking-skeleton.md)
- [Spec 0002: Gerenciamento de Nickname e Identidade Local](./0002-nickname-and-identity.md)
- [Spec 0003: Room Layouts e Room Navigator](./0003-room-layouts-and-navigator.md)
- [Spec 0004: Interface de Inventário e Modo Construção de Salas](./0004-inventory-and-room-building.md)
- [Spec 0005: Criador de Personagem e Pipeline de Assets](./0005-character-creator-and-asset-pipeline.md)
- [Spec 0006: Poses e Animações do Avatar](./0006-avatar-poses-and-animations.md)
- [Spec 0007: Inventário com bevy_hui, Phosphor Icons e Tailwind](./0007-inventory-ui-hui.md)
- [Spec 0008: Moedas no HUD e Loja (bevy_hui)](./0008-currency-hud-and-shop.md)
- [Spec 0009: Tailwind Responsivo e HUD Mobile](./0009-tailwind-responsivo-e-hud-mobile.md)
- [Spec 0010: Assets na CDN (R2) e catálogos do servidor](./0010-assets-cdn.md)
- [Spec 0010: Login Social (Apple e Google) por Plataforma](./0010-social-login.md)
- [Spec 0012: Padronização de modais, safe areas e bloqueio de cliques](./0012-padronizacao-de-modais.md)
- [Spec 0013: Câmera que segue o avatar, zoom e paredes do fundo](./0013-camera-seguindo-e-paredes.md)
- [Spec 0014: Login persistente e menu do avatar](./0014-login-persistente-e-menu-do-avatar.md)

---

## Template para Novas Especificações

```markdown
# Spec [Número]: [Nome da Feature]

## Status
[Draft | Approved | Implemented | Deprecated]

## Contexto e Objetivo
Descrição da experiência do jogador e motivação.

## Escopo
- O que está dentro do escopo:
- O que está FORA do escopo:

## Componentes Bevy Envolvidos
Plugins, Componentes, Recursos e Eventos/Messages.

## Critérios de Aceite
- [ ] Cenário 1...
- [ ] Cenário 2...
```
