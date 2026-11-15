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

echo "=== [Client] Clippy do host KMP (feature kmp-host, sem winit) ==="
cargo clippy -p game --no-default-features --features kmp-host --all-targets -- -D warnings
echo "Clippy kmp-host OK!"

echo "=== [Client] Executando testes unitários e de integração ==="
cargo test --workspace --verbose
echo "=== [Client] Testes do host KMP (identidade e onboarding) ==="
cargo test -p game --no-default-features --features kmp-host -- identity onboarding
echo "Todos os testes do cliente passaram com sucesso!"
