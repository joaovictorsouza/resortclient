#!/usr/bin/env bash
# Copies the server-owned contract (protocol fixtures, version, furniture catalog) into contract/. Never edit contract by hand.
#
# CONTRACT_SRC defaults to ../resort-game-server/contract (sibling checkout). Use CONTRACT_SRC to point at another
# checkout of resort-game-server/contract (a release download replaces this later).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${CONTRACT_SRC:-$ROOT/../resort-game-server/contract}"

[ -f "$SRC/CONTRACT_VERSION" ] || { echo "No contract at $SRC (set CONTRACT_SRC)"; exit 1; }

rm -rf "$ROOT/contract/fixtures"
mkdir -p "$ROOT/contract"
cp -R "$SRC/fixtures" "$ROOT/contract/fixtures"
cp "$SRC/CONTRACT_VERSION" "$SRC/furniture-catalog.json" "$ROOT/contract/"
echo "Contract synced: $(tr '\n' ' ' < "$ROOT/contract/CONTRACT_VERSION")"
