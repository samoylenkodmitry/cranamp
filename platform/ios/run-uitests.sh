#!/usr/bin/env bash
# Run the on-device UI tests against the attached iPhone.
#
# These exist for the one class of question a headless test cannot answer: what
# a SYSTEM view decided. The document picker enables providers based on the
# content type the app asked for, and the app cannot read that back -- so the
# only check is to open the picker on a real device and look at it, which is
# what `CranampUITests` does.
#
# The app under test is not built here. It is the bundle `build-app.sh` makes
# and `install-ipa.sh` puts on the phone; the tests attach to it by bundle
# identifier and start it with `--open-picker=...`, so whatever is installed is
# what gets measured.
#
# REQUIRES, once, on the device:
#   Settings > Privacy & Security > Developer Mode   -- on
#   Settings > Developer > Enable UI Automation      -- on
# Without the second one the runner fails with "Timed out while enabling
# automation mode" before any test body runs. The device also has to be
# unlocked for the duration: a locked screen has no app to drive.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project="$here/uitests/CranampUITests.xcodeproj"

# A leading non-option argument names the device; everything else is handed to
# xcodebuild (`-only-testing:...` while iterating on one case, say).
device=""
if [ $# -gt 0 ] && [ "${1#-}" = "$1" ]; then
  device="$1"
  shift
fi
if [ -z "$device" ]; then
  # By the shape of the identifier rather than by column: the name and the
  # model both contain spaces, so counting fields picks the wrong one.
  device="$(xcrun devicectl list devices 2>/dev/null \
    | grep -i available \
    | grep -oE '[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}' \
    | head -1)"
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
