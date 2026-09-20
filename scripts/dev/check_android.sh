#!/usr/bin/env bash
# Check (or build) the Rust side of the Android app the way Gradle does.
#
#   check_android.sh [abi] [cargo-subcommand]
#
#   check_android.sh                  # check arm64-v8a
#   check_android.sh x86_64           # check the emulator's ABI
#   check_android.sh arm64-v8a build  # build the .so
#
# A plain `cargo check --target aarch64-linux-android` does not work here:
# the C dependencies want the NDK's clang, which is named for an API level
# (`aarch64-linux-android26-clang`) and is not on PATH, so `cc-rs` fails to
# find `aarch64-linux-android-clang` and the build dies in `aws-lc-sys`
# before any Rust of ours is compiled. `cargo-ndk` sets the compiler, the
# archiver and the linker for the ABI, which is what the Gradle plugin runs.
#
# The API level and the feature list are read from the Gradle build rather
# than repeated, because a check that compiles a different crate than the
# APK does is worse than no check. Note that cranamp's `android` feature
# does not imply `renderer-wgpu` the way `ios` does; leaving it out fails
# with a missing `AppLauncher::run` and a missing host-window API, which
# looks like broken code and is a missing feature.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
abi="${1:-arm64-v8a}"
action="${2:-check}"
gradle="$root/platform/android/app/build.gradle.kts"

command -v cargo-ndk > /dev/null || {
    echo "check_android.sh: cargo-ndk is required; install it with \`cargo install cargo-ndk\`" >&2
    exit 2
}
[ -f "$gradle" ] || { echo "check_android.sh: no $gradle to read the build from" >&2; exit 2; }

if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    sdk="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
    ANDROID_NDK_HOME="$(find "$sdk/ndk" -maxdepth 1 -mindepth 1 -type d 2> /dev/null | sort -V | tail -1)"
    [ -n "$ANDROID_NDK_HOME" ] || {
        echo "check_android.sh: no NDK under $sdk/ndk; set ANDROID_NDK_HOME" >&2
        exit 2
    }
fi
export ANDROID_NDK_HOME

min_sdk="$(sed -nE 's/.*minSdk[[:space:]]*=[[:space:]]*([0-9]+).*/\1/p' "$gradle" | head -1)"
features="$(sed -nE 's/.*features\.set\(listOf\((.*)\)\).*/\1/p' "$gradle" | head -1 |
    tr -d '" ' )"
[ -n "$min_sdk" ] || { echo "check_android.sh: no minSdk in $gradle" >&2; exit 2; }
[ -n "$features" ] || { echo "check_android.sh: no cargo features in $gradle" >&2; exit 2; }

echo "check_android.sh: ndk=$(basename "$ANDROID_NDK_HOME") api=$min_sdk abi=$abi features=$features"
cd "$root"
exec cargo ndk --platform "$min_sdk" -t "$abi" "$action" \
    --no-default-features --features "$features" --lib
