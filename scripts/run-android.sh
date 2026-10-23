#!/usr/bin/env bash
# Builds the game for Android (arm64) with Gradle, installs it on the connected device/emulator and
# launches it. Output: target/android/new_game.apk
# Env: NAKAMA_HOST  server address as seen by the phone. Default: 127.0.0.1 through `adb reverse`
#                   (USB only, no Wi-Fi needed). For Wi-Fi use the Mac's LAN IP, e.g. 192.168.1.50.
#      NAKAMA_SERVER_KEY  server socket key; the DEV server rejects the default one
#      EXTRA_DEV_ENV extra lines for assets/dev.env, e.g. 'GAME_NO_ANIM=1\n'
#      NAKAMA_PORT  (default 7350), PROFILE (debug|release), ANDROID_SERIAL (pick a device)
#      GAME_DEV_MODE (default 1: ephemeral identity, skips login)
#      ENVIRONMENT  dev|prod: take host/port/dev-mode from environments/<name>.env instead of the local defaults.
#      BUILD_ONLY=1 stops after building (no adb needed; used by CI). PROFILE=release builds the release
#      variant, signed with KEYSTORE, KEYSTORE_PASS, KEY_ALIAS, KEY_PASS (default: the debug key).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${PROFILE:-debug}"
SDK="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_HOME="$SDK"
if [ -n "${ENVIRONMENT:-}" ]; then
  source "$ROOT/scripts/load-environment.sh"
  load_environment "$ENVIRONMENT"
fi
export NAKAMA_HOST="${NAKAMA_HOST:-127.0.0.1}"
export NAKAMA_PORT="${NAKAMA_PORT:-7350}"
export GAME_DEV_MODE="${GAME_DEV_MODE:-1}"
export EXTRA_DEV_ENV="${EXTRA_DEV_ENV:-}"
# Written to assets/dev.env: "prod" hides the in-game status panel.
export GAME_ENV="${ENVIRONMENT:-${GAME_ENV:-dev}}"
PKG=br.eng.joaovictor.resort
ADB="$SDK/platform-tools/adb"
OUT="$ROOT/target/android"

[ -n "${BUILD_ONLY:-}" ] || "$ADB" get-state >/dev/null 2>&1 || { echo "No Android device found (adb devices). Enable USB debugging and accept the prompt."; exit 1; }

"$ROOT/scripts/build-android-lib.sh" "$PROFILE"

cd "$ROOT/android"
if [ "$PROFILE" = release ]; then
  ./gradlew --no-daemon -q :app:assembleRelease
  cp app/build/outputs/apk/release/app-release.apk "$OUT/new_game.apk"
else
  ./gradlew --no-daemon -q :app:assembleDebug
  cp app/build/outputs/apk/debug/app-debug.apk "$OUT/new_game.apk"
fi

if [ -n "${BUILD_ONLY:-}" ]; then echo "APK: $OUT/new_game.apk"; exit 0; fi

if [ "$NAKAMA_HOST" = 127.0.0.1 ]; then
  "$ADB" reverse "tcp:$NAKAMA_PORT" "tcp:$NAKAMA_PORT"   # phone's localhost -> this Mac, over USB
fi
"$ADB" install -r "$OUT/new_game.apk"
"$ADB" shell am force-stop "$PKG" || true
"$ADB" shell am start -n "$PKG/.MainActivity"
echo "Launched. Logs: $ADB logcat -s RustStdoutStderr:V bevy:V GoogleSignIn:V *:E"
