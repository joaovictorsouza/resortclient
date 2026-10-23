#!/usr/bin/env bash
# Builds the game for a physical iPhone/iPad WITHOUT GoogleSignIn (the weak stub answers "unavailable", so
# the game falls back to the device login; use scripts/export-ipa.sh for the real Google login), signs it with your own (free) Apple development
# certificate and packs target/ios-device/new_game.ipa. Runs on a Mac only.
#
# One-time setup (free Apple ID / Personal Team):
#   1. Xcode > Settings > Accounts: add your Apple ID (creates the "Apple Development" certificate).
#   2. Xcode: create any empty iOS App project with Bundle Identifier == $BUNDLE_ID, team = Personal
#      Team, run it once on your device. This makes Xcode generate the provisioning profile that this
#      script picks up from ~/Library/Developer/Xcode/UserData/Provisioning Profiles (Xcode 16+) or
#      ~/Library/MobileDevice/Provisioning Profiles (older Xcode).
#   Personal Team profiles expire after 7 days: re-run this script (and install) to renew.
#
# Usage: scripts/build-ios-device.sh [--env dev|prod] [--install]
# Env:   BUNDLE_ID      must be unique per Apple ID (default br.eng.joaovictor.resort; free accounts often need
#                       something like dev.<yourname>.newgame)
#        SIGN_IDENTITY  codesign identity (default: first "Apple Development" in the keychain)
#        PROFILE_PATH   .mobileprovision to embed (default: newest one matching BUNDLE_ID)
#        ENVIRONMENT    dev (default) | prod: picks environments/<name>.env (same as --env)
#        NAKAMA_SERVER_KEY  the server's socket key (NAKAMA_SERVER_KEY in the VPS .env); without it the
#                       client uses "defaultkey" and the DEV server rejects it
#        NAKAMA_HOST, NAKAMA_PORT, GAME_DEV_MODE override the values from that file; GAME_ENV=$ENVIRONMENT
#                       is written to assets/dev.env (prod hides the in-game status panel)
#        PROFILE        release (default) | debug
#        DEVICE         devicectl device name/UDID for --install (default: the only paired iPhone/iPad)
set -euo pipefail

# The whole script lives in main() so bash parses it completely before running anything: bash reads
# scripts incrementally, and editing/checking out this file during a long build would otherwise
# make a running copy execute garbage ("syntax error near unexpected token").
main() {
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${PROFILE:-release}"
TARGET=aarch64-apple-ios
BUNDLE_ID="${BUNDLE_ID:-br.eng.joaovictor.resort}"
ENVIRONMENT="${ENVIRONMENT:-dev}"
INSTALL=0
while [ $# -gt 0 ]; do
  case "$1" in
    --env) ENVIRONMENT="${2:?--env needs dev or prod}"; shift ;;
    --install) INSTALL=1 ;;
    *) echo "Unknown argument: $1"; exit 1 ;;
  esac
  shift
done
source "$ROOT/scripts/load-environment.sh"
load_environment "$ENVIRONMENT"
require_server_key
OUT="$ROOT/target/ios-device"
APP="$OUT/Payload/new_game.app"
IPA="$OUT/new_game.ipa"

[ "$(uname)" = Darwin ] || { echo "This script needs macOS + Xcode."; exit 1; }
cd "$ROOT"
rustup target add "$TARGET" >/dev/null

# 1. Signing material.
IDENTITY="${SIGN_IDENTITY:-$(security find-identity -v -p codesigning | grep -m1 'Apple Development' | grep -oE '[0-9A-F]{40}' || true)}"
[ -n "$IDENTITY" ] || { echo "No 'Apple Development' identity in the keychain. See the setup notes at the top of this script."; exit 1; }

PROFILE_PATH="${PROFILE_PATH:-}"
if [ -z "$PROFILE_PATH" ]; then
  while IFS= read -r f; do
    if security cms -D -i "$f" 2>/dev/null | grep -qF ".$BUNDLE_ID</string>"; then PROFILE_PATH="$f"; break; fi
  done < <(ls -t "$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles/"*.mobileprovision \
                 "$HOME/Library/MobileDevice/Provisioning Profiles/"*.mobileprovision 2>/dev/null || true)
fi
[ -n "$PROFILE_PATH" ] || {
  echo "No provisioning profile for $BUNDLE_ID in the Xcode profile folders. See the setup notes at the top of this script."
  echo "Pass PROFILE_PATH=<file.mobileprovision> if Xcode stored it elsewhere."
  exit 1
}
echo "Identity: $IDENTITY"; echo "Profile:  $PROFILE_PATH"

mkdir -p "$OUT"
security cms -D -i "$PROFILE_PATH" > "$OUT/profile.plist"
/usr/libexec/PlistBuddy -x -c 'Print :Entitlements' "$OUT/profile.plist" > "$OUT/entitlements.plist"

# 2. Native static library.
CARGO_FLAGS=(-p game --lib --target "$TARGET")
[ "$PROFILE" = release ] && CARGO_FLAGS+=(--release)
# Keep C dependencies (blake3, ...) on the same minimum iOS as the final link.
# Staticlib only: the cdylib cannot link without the Google bridge symbols (weak stubs in ios/Sources).
IPHONEOS_DEPLOYMENT_TARGET=16.0 cargo rustc "${CARGO_FLAGS[@]}" --crate-type staticlib
LIB="$ROOT/target/$TARGET/$PROFILE/libgame.a"

# 3. .app bundle (same launcher and frameworks as the simulator build, device SDK).
rm -rf "$OUT/Payload" && mkdir -p "$APP"
SDK="$(xcrun --sdk iphoneos --show-sdk-path)"
xcrun --sdk iphoneos clang -target arm64-apple-ios16.0 -isysroot "$SDK" \
  -fobjc-arc ios/Sources/main.m ios/Sources/GoogleSignInStub.m "$LIB" \
  -framework UIKit -framework Foundation -framework Metal -framework QuartzCore \
  -framework CoreGraphics -framework CoreAudio -framework AudioToolbox -framework AVFoundation \
  -framework CoreMedia -framework CoreVideo -framework CoreFoundation -framework Security \
  -framework SystemConfiguration -framework GameController -framework CoreHaptics -framework CoreMotion \
  -framework OpenGLES -framework UserNotifications -framework UniformTypeIdentifiers \
  -lc++ -liconv -o "$APP/new_game"
cp ios/Info.plist "$APP/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $BUNDLE_ID" "$APP/Info.plist"
rsync -a assets/ "$APP/assets/"
# The server key lives on the server (/opt/resort/.env), never in the repo: pass it when building.
write_dev_env "$APP/assets/dev.env"
cp "$PROFILE_PATH" "$APP/embedded.mobileprovision"

# 4. Sign and zip as .ipa.
codesign --force --sign "$IDENTITY" --entitlements "$OUT/entitlements.plist" --timestamp=none "$APP"
codesign --verify --strict "$APP"
rm -f "$IPA" && (cd "$OUT" && zip -qry "$IPA" Payload)
echo "IPA: $IPA (env: $ENVIRONMENT, server: $NAKAMA_HOST:$NAKAMA_PORT)"

# 5. Optional install on a connected device (iOS 17+/Xcode 15+).
if [ "$INSTALL" = 1 ]; then
  if [ -z "${DEVICE:-}" ]; then
    # devicectl needs --device: pick the paired iPhone/iPad when there is exactly one.
    xcrun devicectl list devices --json-output "$OUT/devices.json" >/dev/null 2>&1 || true
    DEVICE="$(python3 - "$OUT/devices.json" <<'PY' 2>/dev/null || true
import json, sys
devs = json.load(open(sys.argv[1]))["result"]["devices"]
ids = [d["identifier"] for d in devs
       if d.get("hardwareProperties", {}).get("platform") == "iOS"
       and d.get("connectionProperties", {}).get("pairingState") == "paired"]
print(ids[0] if len(ids) == 1 else "")
PY
)"
    [ -n "$DEVICE" ] || { echo "Could not pick a single device. Run 'xcrun devicectl list devices' and pass DEVICE=<name or id>."; exit 1; }
  fi
  xcrun devicectl device install app --device "$DEVICE" "$APP"
  echo "Installed. First launch: Settings > General > VPN & Device Management > trust your developer profile."
fi
}
main "$@"
