#!/bin/bash
# Builds Cranamp for iOS and assembles a `.app` bundle.
#
# Cranamp uses Cranpose's winit-based iOS backend, so winit starts
# `UIApplicationMain` itself: the app is a pure-Rust binary (`cranamp-ios`)
# with no Objective-C entry point and no Xcode project.
#
# Usage:
#   ./build-app.sh [target]
#     target: aarch64-apple-ios-sim (default) | aarch64-apple-ios (device)
#   PROFILE=release ./build-app.sh        # optimized build
#
# Prints the path to the assembled `.app` bundle on stdout.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TARGET="${1:-aarch64-apple-ios-sim}"
PROFILE="${PROFILE:-debug}"
APP_NAME="Cranamp"

case "$PROFILE" in
  release) PROFILE_FLAG="--release" ;;
  debug)   PROFILE_FLAG="" ;;
  *) echo "PROFILE must be 'debug' or 'release', got '$PROFILE'" >&2; exit 1 ;;
esac

# The deployment target, and it is not cosmetic. Without it the linker gives
# an `aarch64-apple-ios` binary the legacy `LC_VERSION_MIN_IPHONEOS 10.0` --
# the target's own historic floor, from before the iPhone X existed. iOS reads
# that as an app written for a 4.7-inch screen and runs it letterboxed: black
# bars top and bottom, and no amount of `UILaunchScreen` in the Info.plist
# changes it, because the decision is made from the Mach-O, not the plist.
#
# Setting it makes the linker emit `LC_BUILD_VERSION platform IOS minos 15.0`
# instead, which is what a full-screen app looks like. Keep it equal to
# `MinimumOSVersion` in Info.plist.
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-15.0}"

# shellcheck disable=SC2086
cargo build --manifest-path "$ROOT/Cargo.toml" \
  --bin cranamp-ios \
  --target "$TARGET" --no-default-features --features ios $PROFILE_FLAG >&2

BIN="$ROOT/target/$TARGET/$PROFILE/cranamp-ios"
APP="$ROOT/target/$TARGET/$PROFILE/$APP_NAME.app"

rm -rf "$APP"
mkdir -p "$APP"
cp "$BIN" "$APP/$APP_NAME"
cp "$SCRIPT_DIR/Info.plist" "$APP/Info.plist"

# Whatever the toolchain did with the deployment target above, say so out
# loud and refuse the bundle if it came out legacy. A letterboxed build is
# indistinguishable from a correct one until it is on a phone, and the load
# command is the whole difference.
build_version="$(xcrun vtool -show-build-version "$APP/$APP_NAME" 2>/dev/null || true)"
case "$build_version" in
  *LC_BUILD_VERSION*) ;;
  *)
    echo "$APP_NAME carries no LC_BUILD_VERSION -- iOS will run it letterboxed." >&2
    echo "$build_version" >&2
    exit 1
    ;;
esac
minos="$(printf '%s\n' "$build_version" | awk '/minos/{print $2; exit}')"
case "$minos" in
  1[0-3].*|[0-9].*)
    echo "$APP_NAME targets iOS $minos, below the 14.0 that UILaunchScreen needs." >&2
    exit 1
    ;;
esac

# Ad-hoc sign so the bundle runs on device/simulator without a developer team.
# Pass CODESIGN_IDENTITY for a real Developer ID / distribution identity.
codesign --force --sign "${CODESIGN_IDENTITY:--}" "$APP" >&2

echo "$APP"
