#!/usr/bin/env bash
# Builds the KMP-hosted iOS app (kmp/iosApp) for a physical device, installs and launches it.
# Env: DEVELOPMENT_TEAM (default: the paid team), NAKAMA_SERVER_KEY etc. for the runtime config.
# INSTALL=0 only builds; LAUNCH=0 installs without opening the app.
# Usage: scripts/kmp-ios-device.sh [device-udid] [Debug|Release]
set -euo pipefail
cd "$(dirname "$0")/../kmp/iosApp"
DEVELOPMENT_TEAM="${DEVELOPMENT_TEAM:-Q7X85BP8VA}"
DEV="${1:-$(xcrun devicectl list devices | awk '/physical/ && /iPhone/ {for(i=1;i<=NF;i++) if($i ~ /^[0-9A-F]{8}-[0-9A-F]{16}$/) {print $i; exit}}')}"
CONF="${2:-Debug}"
# Build-only runs do not need the device to be connected.
if [ "${INSTALL:-1}" = 0 ]; then DEST="generic/platform=iOS"; else DEST="id=$DEV"; fi
export ANDROID_HOME="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
xcodegen generate >/dev/null
xcodebuild -project iosApp.xcodeproj -scheme iosApp -configuration "$CONF" \
  -destination "$DEST" -derivedDataPath build-device -allowProvisioningUpdates \
  DEVELOPMENT_TEAM="$DEVELOPMENT_TEAM" CODE_SIGN_STYLE=Automatic build | tail -5
APP=$(find build-device/Build/Products -name 'iosApp.app' -maxdepth 3 | head -1)
[ "${INSTALL:-1}" = 0 ] && { echo "Built $APP (INSTALL=0: not installed)"; exit 0; }
xcrun devicectl device install app --device "$DEV" "$APP"
[ "${LAUNCH:-1}" = 0 ] || xcrun devicectl device process launch --device "$DEV" --terminate-existing br.eng.joaovictor.resort
