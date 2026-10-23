---
name: write-spec
description: >-
  Cria ou atualiza especificações de requisitos técnicos e funcionais antes de iniciar
  o desenvolvimento de novas features no cliente ou servidor. Use sempre que planejar
  um novo recurso ou módulo.
---

# Redação de Especificações Técnicas (Specs)

Esta skill guia a criação de especificações técnicas formais antes de iniciar a implementação de qualquer funcionalidade relevante.

## Procedimento

1. **Determinar o Escopo da Spec**:
   - Se for uma feature de cliente: salvar em `ai-docs/specs/<numero>-<nome>.md`.
   - Se for uma feature de servidor: salvar em `resort-game-server/ai-docs/specs/<numero>-<nome>.md`.
   - Se for uma feature de ponta a ponta: criar a spec em ambos os lados, com o lado do cliente detalhando a interface/UX e o servidor detalhando a autoridade/persistência.
2. **Definir Numeração Sequencial**:
   - Verifique o último número no índice `ai-docs/specs/README.md` e incremente (ex: `0002-custom-rooms.md`).
3. **Preencher a Partir do Template**:
   - Utilize o modelo em [resources/spec-template.md](./resources/spec-template.md).
   - Seja rigoroso nos **Critérios de Aceite**: eles devem ser descritos como caixas de seleção `[ ]` diretamente testáveis.
4. **Atualizar o Índice**:
   - Adicione o link para a nova spec no arquivo `README.md` da pasta `specs/`.
