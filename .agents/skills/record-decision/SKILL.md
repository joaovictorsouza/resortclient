---
name: record-decision
description: >-
  Documenta Architectural Decision Records (ADRs) para escolhas de design, bibliotecas,
  estratégias de concorrência ou padrões estruturais no cliente ou servidor. Use sempre que tomar
  uma decisão que impacte a arquitetura do projeto.
---

# Registro de Decisões de Arquitetura (ADRs)

Decisões arquiteturais fundamentam o projeto e garantem que o contexto histórico e as justificativas técnicas sejam preservadas.

## Procedimento

1. **Determinar o Escopo**:
   - Se for uma decisão de cliente: salvar em `ai-docs/decisions/<numero>-<slug>.md`.
   - Se for uma decisão de servidor: salvar em `resort-game-server/ai-docs/decisions/<numero>-<slug>.md`.
2. **Definir Numeração**:
   - Verifique o último número no índice `ai-docs/decisions/README.md` e incremente (ex: `0006-novo-sistema.md`).
3. **Preencher Utilizando o Template**:
   - Utilize o modelo em [resources/adr-template.md](./resources/adr-template.md).
   - Seja claro no **Contexto**, na **Decisão**, nas **Alternativas Consideradas** e nas **Consequências**.
4. **Atualizar o Índice**:
   - Registre a nova ADR no `README.md` da respectiva pasta `decisions/`.
