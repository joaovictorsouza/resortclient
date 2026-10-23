#!/usr/bin/env bash
# Builds a signed release APK (target/android/new_game.apk) and prints the keystore SHA-1 to register
# in the Google Cloud Android OAuth client.
# Env (never commit passwords): ANDROID_KEYSTORE_PASSWORD, ANDROID_KEY_ALIAS, ANDROID_KEY_PASSWORD
#   RESORT_KEYSTORE (default ~/Documents/keystore/resort/dev/dev-release.keystore);
#   plus the variables of run-android.sh (ENVIRONMENT, NAKAMA_HOST, ...). GAME_DEV_MODE defaults to 0.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export KEYSTORE="${KEYSTORE:-${RESORT_KEYSTORE:-$HOME/Documents/keystore/resort/dev/dev-release.keystore}}"
export KEYSTORE_PASS="${KEYSTORE_PASS:-${ANDROID_KEYSTORE_PASSWORD:-}}"
export KEY_ALIAS="${KEY_ALIAS:-${ANDROID_KEY_ALIAS:-}}"
export KEY_PASS="${KEY_PASS:-${ANDROID_KEY_PASSWORD:-}}"
[ -f "$KEYSTORE" ] || { echo "Keystore not found: $KEYSTORE (set RESORT_KEYSTORE)"; exit 1; }
for v in KEYSTORE_PASS KEY_ALIAS KEY_PASS; do
  [ -n "${!v}" ] || { echo "Set ANDROID_KEYSTORE_PASSWORD, ANDROID_KEY_ALIAS and ANDROID_KEY_PASSWORD"; exit 1; }
done

GAME_DEV_MODE="${GAME_DEV_MODE:-0}" PROFILE=release BUILD_ONLY=1 "$ROOT/scripts/run-android.sh"
echo "Release keystore SHA-1:"
keytool -list -v -keystore "$KEYSTORE" -storepass "$KEYSTORE_PASS" -alias "$KEY_ALIAS" | grep -E "SHA1|SHA-1"
