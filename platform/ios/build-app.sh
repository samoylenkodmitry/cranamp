#!/bin/bash
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

export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-15.0}"

cargo build --manifest-path "$ROOT/Cargo.toml" \
  --bin cranamp-ios \
  --target "$TARGET" --no-default-features --features ios $PROFILE_FLAG >&2

BIN="$ROOT/target/$TARGET/$PROFILE/cranamp-ios"
APP="$ROOT/target/$TARGET/$PROFILE/$APP_NAME.app"

rm -rf "$APP"
mkdir -p "$APP"
cp "$BIN" "$APP/$APP_NAME"
cp "$SCRIPT_DIR/Info.plist" "$APP/Info.plist"

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

codesign --force --sign "${CODESIGN_IDENTITY:--}" "$APP" >&2

echo "$APP"
