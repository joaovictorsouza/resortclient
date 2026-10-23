#!/usr/bin/env bash
# Builds the game for the iOS Simulator with xcodebuild, installs it and launches it.
# Usage: scripts/run-ios-sim.sh [-- game args]
# Env:   NAKAMA_HOST (default 127.0.0.1), NAKAMA_PORT, SIM_DEVICE (default "new_game iPhone"),
#        CONFIGURATION (Debug|Release, default Debug), ENVIRONMENT/GAME_ENV (dev|prod, default dev: prod hides the status panel)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONFIGURATION="${CONFIGURATION:-Debug}"
SIM_DEVICE="${SIM_DEVICE:-new_game iPhone}"
BUNDLE_ID=br.eng.joaovictor.resort
DERIVED="$ROOT/target/ios-build"
APP="$DERIVED/Build/Products/$CONFIGURATION-iphonesimulator/new_game.app"

cd "$ROOT"

# Create/boot the simulator device.
UDID="$(xcrun simctl list devices | grep -F "$SIM_DEVICE (" | grep -oE '[0-9A-F-]{36}' | head -1 || true)"
if [ -z "$UDID" ]; then
  RUNTIME="$(xcrun simctl list runtimes | grep -oE 'com\.apple\.CoreSimulator\.SimRuntime\.iOS-[0-9-]+' | tail -1)"
  UDID="$(xcrun simctl create "$SIM_DEVICE" com.apple.CoreSimulator.SimDeviceType.iPhone-16 "$RUNTIME")"
fi
xcrun simctl boot "$UDID" 2>/dev/null || true
open -a Simulator 2>/dev/null || true  # the window is optional, simctl works headless

[ -d ios/Resort.xcodeproj ] || scripts/gen-ios-project.sh
xcodebuild -project ios/Resort.xcodeproj -scheme Resort -configuration "$CONFIGURATION" \
  -sdk iphonesimulator -destination "id=$UDID" -derivedDataPath "$DERIVED" \
  CODE_SIGNING_ALLOWED=NO -quiet build

# Install and launch (SIMCTL_CHILD_ prefix forwards env vars to the app).
xcrun simctl terminate "$UDID" "$BUNDLE_ID" 2>/dev/null || true
xcrun simctl install "$UDID" "$APP"
export SIMCTL_CHILD_NAKAMA_HOST="${NAKAMA_HOST:-127.0.0.1}"
export SIMCTL_CHILD_NAKAMA_PORT="${NAKAMA_PORT:-7350}"
export SIMCTL_CHILD_GAME_ENV="${ENVIRONMENT:-${GAME_ENV:-dev}}"
xcrun simctl launch --terminate-running-process "$UDID" "$BUNDLE_ID" "$@"
echo "Launched on $UDID. Logs: xcrun simctl spawn $UDID log stream --level debug --predicate 'process == \"new_game\"'"
