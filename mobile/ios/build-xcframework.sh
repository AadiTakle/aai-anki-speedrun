#!/usr/bin/env bash
#
# Build SpeedrunFFI.xcframework from the shared Rust engine, for iOS device
# (aarch64-apple-ios) + iOS simulator (aarch64-apple-ios-sim). Drop the
# resulting xcframework into your Xcode app target (see README.md).
#
# Usage:  ./build-xcframework.sh [--release]
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
ffi="$here/../speedrun-ffi"
out="$here/build"

profile_flag=""
profile_dir="debug"
if [[ "${1:-}" == "--release" ]]; then
    profile_flag="--release"
    profile_dir="release"
fi

cd "$ffi"

# Device: build ONLY the staticlib. A full `cargo build` also links a cdylib,
# which currently fails on iOS device with an undefined `___chkstk_darwin`
# symbol from blake3's NEON assembly. That link step is irrelevant for an app
# (apps link the .a), so we skip it here.
cargo rustc --lib --crate-type staticlib --target aarch64-apple-ios $profile_flag

# Simulator: the normal build (staticlib + cdylib) works.
cargo build --target aarch64-apple-ios-sim $profile_flag

td="${CARGO_TARGET_DIR:-target}"
device_lib="$td/aarch64-apple-ios/$profile_dir/libspeedrun_ffi.a"
sim_lib="$td/aarch64-apple-ios-sim/$profile_dir/libspeedrun_ffi.a"

rm -rf "$out/SpeedrunFFI.xcframework"
mkdir -p "$out"
xcodebuild -create-xcframework \
    -library "$device_lib" -headers "$ffi/include" \
    -library "$sim_lib" -headers "$ffi/include" \
    -output "$out/SpeedrunFFI.xcframework"

echo "Wrote $out/SpeedrunFFI.xcframework"
