#!/usr/bin/env bash
# Archives the game and exports a development-signed IPA: target/ios/export/new_game.ipa
# Env:  DEVELOPMENT_TEAM  Apple team id (or set it in ios/Config/Local.xcconfig)
#       CONFIGURATION     Release (default) or Debug
#       ENVIRONMENT       dev (default) | prod: environments/<name>.env gives the server and GAME_ENV
#                         (prod hides the in-game status panel); NAKAMA_HOST/NAKAMA_PORT/GAME_DEV_MODE override it
#       SIGNING           auto (default: automatic signing with your Apple ID in Xcode) | none (unsigned IPA, what the
#                         GitHub workflow builds; it does not install on a device until it is signed). Manual signing
#                         with a certificate and profile from secrets is a later step.
#       NAKAMA_SERVER_KEY the server's socket key; without it the app uses "defaultkey", which the DEV server rejects
# Needs the Apple ID signed in to Xcode (automatic signing, -allowProvisioningUpdates).
set -euo pipefail

# The whole script lives in main() so bash parses it completely before running anything: bash reads
# scripts incrementally, and editing/checking out this file during a long build would otherwise
# make a running copy execute garbage ("syntax error near unexpected token").
main() {
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONFIGURATION="${CONFIGURATION:-Release}"
OUT="$ROOT/target/ios"
cd "$ROOT"

SIGNING="${SIGNING:-auto}"
case "$SIGNING" in auto|none) ;; *) echo "SIGNING must be auto or none (got '$SIGNING')"; exit 1 ;; esac

TEAM_ARG=()
TEAM_ID=""
if [ "$SIGNING" = none ]; then
  # No Apple account, certificate or profile is involved: the archive is built unsigned.
  TEAM_ARG=(CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO CODE_SIGN_IDENTITY= DEVELOPMENT_TEAM=)
else
  [ -n "${DEVELOPMENT_TEAM:-}" ] && TEAM_ARG=("DEVELOPMENT_TEAM=$DEVELOPMENT_TEAM")
  if [ -z "${DEVELOPMENT_TEAM:-}" ] && ! grep -qE '^DEVELOPMENT_TEAM *= *[A-Z0-9]+' ios/Config/Local.xcconfig 2>/dev/null; then
    echo "Set DEVELOPMENT_TEAM (env or ios/Config/Local.xcconfig, see Local.xcconfig.example)"; exit 1
  fi
  TEAM_ID="${DEVELOPMENT_TEAM:-$(sed -nE 's/^DEVELOPMENT_TEAM *= *([A-Z0-9]+).*/\1/p' ios/Config/Local.xcconfig)}"
fi

[ -d ios/Resort.xcodeproj ] || scripts/gen-ios-project.sh

# The Xcode project bundles assets as-is, so the runtime config (assets/dev.env, read by
# ios/Sources/main.m) is written there for the build and removed afterwards (it may hold the key).
ENVIRONMENT="${ENVIRONMENT:-dev}"
source "$ROOT/scripts/load-environment.sh"
load_environment "$ENVIRONMENT"
require_server_key
DEV_ENV="$ROOT/assets/dev.env"
trap 'rm -f "$DEV_ENV"' EXIT
write_dev_env "$DEV_ENV"
rm -rf "$OUT/Resort.xcarchive" "$OUT/export"
PROVISION_ARG=()
[ "$SIGNING" = none ] || PROVISION_ARG=(-allowProvisioningUpdates)
xcodebuild -project ios/Resort.xcodeproj -scheme Resort -configuration "$CONFIGURATION" \
  -sdk iphoneos -destination 'generic/platform=iOS' -archivePath "$OUT/Resort.xcarchive" \
  -derivedDataPath "$OUT/derived" ${PROVISION_ARG[@]+"${PROVISION_ARG[@]}"} ${TEAM_ARG[@]+"${TEAM_ARG[@]}"} -quiet archive

if [ "$SIGNING" = none ]; then
  # An IPA is a zip with Payload/<app>.app; there is nothing to export or sign.
  APP="$(ls -d "$OUT"/Resort.xcarchive/Products/Applications/*.app | head -n 1)"
  [ -d "$APP" ] || { echo "No .app inside the archive"; exit 1; }
  PACK="$OUT/pack"
  rm -rf "$PACK" && mkdir -p "$PACK/Payload" "$OUT/export"
  cp -R "$APP" "$PACK/Payload/"
  (cd "$PACK" && /usr/bin/zip -qry "$OUT/export/new_game-unsigned.ipa" Payload)
  rm -rf "$PACK"
  echo "Unsigned IPA: $OUT/export/new_game-unsigned.ipa - env: $ENVIRONMENT, server: $NAKAMA_HOST:$NAKAMA_PORT"
  return 0
fi

mkdir -p "$OUT"
cat > "$OUT/ExportOptions.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>method</key><string>development</string>
  <key>teamID</key><string>$TEAM_ID</string>
  <key>signingStyle</key><string>automatic</string>
  <key>stripSwiftSymbols</key><true/>
</dict></plist>
PLIST
# exportArchive needs an Apple account that xcodebuild can see from the command line, and a free Personal
# Team often fails there ("No Accounts" / "No profiles"). The archive is already signed with the
# development profile the archive step created, so when the export fails an IPA is packed straight
# from the archive (an IPA is just a zip with Payload/<app>.app).
mkdir -p "$OUT/export"
if ! xcodebuild -exportArchive -archivePath "$OUT/Resort.xcarchive" -exportPath "$OUT/export" \
  -exportOptionsPlist "$OUT/ExportOptions.plist" -allowProvisioningUpdates; then
  echo "exportArchive failed; packing the IPA from the signed archive instead"
  APP="$(ls -d "$OUT"/Resort.xcarchive/Products/Applications/*.app | head -n 1)"
  [ -d "$APP" ] || { echo "No .app inside the archive"; exit 1; }
  codesign -dvv "$APP" 2>&1 | grep -E "Authority=Apple Development|TeamIdentifier" || {
    echo "The archive is not development-signed; add your Apple ID in Xcode > Settings > Accounts"; exit 1; }
  PACK="$OUT/pack"
  rm -rf "$PACK" && mkdir -p "$PACK/Payload" "$OUT/export"
  cp -R "$APP" "$PACK/Payload/"
  (cd "$PACK" && /usr/bin/zip -qry "$OUT/export/new_game.ipa" Payload)
  rm -rf "$PACK"
fi
echo "IPA: $OUT/export/new_game.ipa - env: $ENVIRONMENT, server: $NAKAMA_HOST:$NAKAMA_PORT"
}
main "$@"
