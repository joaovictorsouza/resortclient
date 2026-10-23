#!/usr/bin/env bash
set -euo pipefail

echo "=== [Client] Verificando contrato com o servidor ==="
./scripts/check-contract.sh

echo "=== [Client] Verificando formatação (cargo fmt) ==="
cargo fmt --all -- --check
echo "Cargo fmt OK!"

echo "=== [Client] Executando linter (cargo clippy) ==="
cargo clippy --workspace --all-targets -- -D warnings
echo "Cargo clippy OK!"

echo "=== [Client] Executando testes unitários e de integração ==="
cargo test --workspace --verbose
echo "Todos os testes do cliente passaram com sucesso!"
