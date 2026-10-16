#!/usr/bin/env bash
# Build the Android app: the Rust library for each ABI into jniLibs, then the APK.
#
#   mobile/build-android.sh [debug|release]   (default: release — a debug Rust build is
#                                              slow on a phone and large)
#
# Needs ANDROID_HOME (or ANDROID_SDK_ROOT), an NDK under it (or ANDROID_NDK_HOME),
# JAVA_HOME pointing at a JDK 17, cargo-ndk, and the Rust targets
# aarch64-linux-android and x86_64-linux-android. ABIS overrides the ABIs built
# (e.g. ABIS=x86_64 for the emulator only).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
profile="${1:-release}"
abis="${ABIS:-arm64-v8a x86_64}"

targets=()
for abi in $abis; do targets+=(-t "$abi"); done
cargo_profile=()
[ "$profile" = release ] && cargo_profile=(--release)

cd "$here/.."
cargo ndk "${targets[@]}" -P 26 -o "$here/android/app/src/main/jniLibs" \
    build -p sapphire-sync-mobile "${cargo_profile[@]}"

# cargo-ndk copies every cdylib in the graph (iroh builds its own); the app loads one.
find "$here/android/app/src/main/jniLibs" -name '*.so' ! -name 'libsapphire_sync_mobile.so' -delete

cd "$here/android"
./gradlew --quiet assembleDebug
echo "built $here/android/app/build/outputs/apk/debug/app-debug.apk"
