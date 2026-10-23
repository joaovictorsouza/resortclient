#!/usr/bin/env bash
# Builds libgame.so (arm64-v8a) with cargo-ndk into target/android/jniLibs, where Gradle picks it up.
# Usage: scripts/build-android-lib.sh [debug|release]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${1:-debug}"
SDK="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-${ANDROID_NDK:-$(ls -d "$SDK"/ndk/* 2>/dev/null | sort -V | tail -1)}}"
MIN_SDK=26   # AAudio (used by the audio backend) needs API 26

[ -n "$ANDROID_NDK_HOME" ] || { echo "ANDROID_NDK_HOME is not set"; exit 1; }
command -v cargo-ndk >/dev/null || { echo "Install cargo-ndk: cargo install cargo-ndk --locked"; exit 1; }

cd "$ROOT"
rustup target add aarch64-linux-android >/dev/null

CARGO_FLAGS=(-p game --lib)
[ "$PROFILE" = release ] && CARGO_FLAGS+=(--release)
rm -rf "$ROOT/target/android/jniLibs"
export CARGO_PROFILE_DEV_DEBUG=0   # the .so is stripped anyway; debuginfo only eats disk (GBs)
cargo ndk -P "$MIN_SDK" -t arm64-v8a -o "$ROOT/target/android/jniLibs" build "${CARGO_FLAGS[@]}"
"$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-strip --strip-debug \
  "$ROOT/target/android/jniLibs/arm64-v8a/libgame.so"
