#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project="$here/uitests/CranampUITests.xcodeproj"

device=""
if [ $# -gt 0 ] && [ "${1#-}" = "$1" ]; then
  device="$1"
  shift
fi
if [ -z "$device" ]; then
  device="$(xcrun devicectl list devices 2>/dev/null \
    | grep -i available \
    | grep -m1 -oE '[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}')"
fi
[ -n "$device" ] || {
  echo "No paired device. Pass a UDID, or check 'xcrun devicectl list devices'." >&2
  exit 1
}

results="${CRANAMP_UITEST_RESULTS:-$here/uitests/results.xcresult}"
rm -rf "$results"

echo "Running UI tests on $device" >&2
xcodebuild test \
  -project "$project" \
  -scheme CranampUITests \
  -destination "platform=iOS,id=$device" \
  -allowProvisioningUpdates \
  -allowProvisioningDeviceRegistration \
  -resultBundlePath "$results" \
  "$@"
