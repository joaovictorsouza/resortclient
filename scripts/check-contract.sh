#!/usr/bin/env bash
# Skipped when no server contract is reachable (set CONTRACT_SRC or keep ../resort-game-server next to this repo).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${CONTRACT_SRC:-$ROOT/../resort-game-server/contract}"

if [ ! -f "$SRC/CONTRACT_VERSION" ]; then
  echo "Server contract not found at $SRC; skipping drift check."
  exit 0
fi
diff -r "$SRC/fixtures" "$ROOT/contract/fixtures"
diff "$SRC/CONTRACT_VERSION" "$ROOT/contract/CONTRACT_VERSION"
diff "$SRC/furniture-catalog.json" "$ROOT/contract/furniture-catalog.json"
echo "Contract in sync."
