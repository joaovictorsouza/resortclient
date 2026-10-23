# ADR 0005: Arquitetura Cross-Platform (Desktop, Android e iOS)

## Status
Aceita

## Contexto
O usuário definiu que o jogo será lançado para Desktop (macOS/Win/Linux), Android e iOS. Durante a fase de desenvolvimento, o foco de execução imediata é o Desktop, mas a arquitetura deve estar totalmente preparada para compilação mobile.

## Decisão
1. Definir o crate `game` como `crate-type = ["lib", "cdylib"]`, mantendo o `main.rs` como thin wrapper para Desktop.
2. Unificar os eventos de ponteiro no `InputPlugin` para tratar toques em telas touch (`Touches`) de forma transparente e idêntica ao clique do mouse.
3. Não utilizar APIs ou caminhos de arquivos exclusivos de desktop no loop principal.

## Consequências
- A base de código está 100% pronta para ser encapsulada em APKs Android ou pacotes IPA do iOS sem refatoração na lógica do jogo.
