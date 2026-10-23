#!/usr/bin/env bash
# Regenerates ios/Resort.xcodeproj from ios/project.yml (XcodeGen). Needs: brew install xcodegen
set -euo pipefail
command -v xcodegen >/dev/null || { echo "Install XcodeGen: brew install xcodegen"; exit 1; }
cd "$(cd "$(dirname "$0")/.." && pwd)/ios"
xcodegen generate --spec project.yml
