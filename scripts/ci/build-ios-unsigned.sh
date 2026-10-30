#!/usr/bin/env bash
# Builds the KMP-hosted iOS app unsigned and packs target/ios/export/new_game-unsigned.ipa.
# Env: ENVIRONMENT (dev|prod, default dev), NAKAMA_SERVER_KEY (never committed).
# Needs: Xcode, xcodegen, JDK 17, Rust with the aarch64-apple-ios target.
set -euo pipefail
cd "$(dirname "$0")/../.."
export ENVIRONMENT="${ENVIRONMENT:-dev}"
(cd kmp/iosApp && xcodegen generate &&
  xcodebuild -project iosApp.xcodeproj -scheme iosApp -configuration Release \
    -destination generic/platform=iOS -derivedDataPath build \
    CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO CODE_SIGN_IDENTITY="" build | tail -20)
APP=$(find kmp/iosApp/build/Build/Products -name 'iosApp.app' -maxdepth 3 | head -1)
[ -n "$APP" ] || { echo "iosApp.app not found"; exit 1; }
rm -rf target/ios/export && mkdir -p target/ios/export/Payload
cp -R "$APP" target/ios/export/Payload/
(cd target/ios/export && zip -qry new_game-unsigned.ipa Payload)
echo "IPA: target/ios/export/new_game-unsigned.ipa"
