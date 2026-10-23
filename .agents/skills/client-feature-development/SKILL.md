---
name: client-feature-development
description: >-
  Guia o desenvolvimento de features no cliente Bevy (Rust), cobrindo criação de plugins,
  lógica pura desacoplada, testes unitários, suporte cross-platform (desktop e mobile)
  e validação com scripts de checagem. Use sempre que for adicionar ou alterar código no cliente.
---

# Desenvolvimento de Features no Cliente (Bevy 0.19)

Esta skill guia o desenvolvimento de novas funcionalidades no cliente do **new_game**.

## Procedimento Passo a Passo

1. **Consultar Especificação e Decisões**:
   - Leia a especificação da feature em [ai-docs/specs/](../../../ai-docs/specs). Se não existir, utilize a skill `write-spec`.
   - Consulte as decisões de arquitetura em [ai-docs/decisions/](../../../ai-docs/decisions).
   - Verifique o guia de fluxo em [ai-docs/process/development-workflow.md](../../../ai-docs/process/development-workflow.md).
2. **Desenvolver Lógica Pura Desacoplada (TDD)**:
   - Escreva funções de suporte (cálculos matemáticos, mapeamento de coordenadas, manipulação de coleções) desacopladas de componentes ou mundos gráficos.
   - Escreva testes unitários no módulo `#[cfg(test)]`.
3. **Encapsular em um Plugin Bevy**:
   - Crie o plugin sob `crates/game/src/<nome_do_modulo>.rs`.
   - Utilize a API de mensagens do Bevy 0.19 (`#[derive(Message)]`, `MessageReader`, `MessageWriter`).
   - Mantenha o suporte unificado para mouse e touch, conforme documentado em [ai-docs/architecture/platforms.md](../../../ai-docs/architecture/platforms.md).
   - Registre o novo plugin em `crates/game/src/lib.rs`.
4. **Executar a Suíte de Validação**:
   - Execute o script de validação:
     ```bash
     ./scripts/check.sh
     ```
   - Garanta zero erros no `cargo fmt`, `cargo clippy -D warnings` e `cargo test`.
5. **Atualizar a Documentação**:
   - Se tomou uma nova decisão arquitetural, acione a skill `record-decision`.
   - Se o servidor publicou uma nova versão do contrato de rede, acione a skill `protocol-change-client`.
