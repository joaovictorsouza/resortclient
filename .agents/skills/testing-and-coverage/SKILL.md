---
name: testing-and-coverage
description: >-
  Executa a suíte de testes do cliente Rust e verifica as metas mínimas e a Definition of Done.
  Use antes de concluir qualquer tarefa ou submissão.
---

# Testes e Cobertura de Código

## Metas Mínimas
- `crates/protocol`: ≥ 80% (roundtrip das fixtures de `contract/fixtures/`)
- `crates/nakama_client`: testes de parsing e despacho de eventos
- `crates/game`: testes de funções puras de coordenadas e interpolação

## Como Executar
```bash
./scripts/check.sh      # contrato em sincronia, cargo fmt, clippy -D warnings e cargo test --workspace
cargo test -p protocol  # só o protocolo
```
