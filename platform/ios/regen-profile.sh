#!/usr/bin/env bash
# Mint (or renew) the iOS development provisioning profile for this app.
#
# Xcode's automatic signing is what creates it: given the team and the bundle
# identifier it registers the App ID, registers the attached device if it is
# new, and writes the profile into
# ~/Library/Developer/Xcode/UserData/Provisioning Profiles. install-ipa.sh then
# picks it up by bundle identifier.
#
# Needs an Apple ID with the team in Xcode > Settings > Accounts, and the
# iPhone attached (USB or paired over the network).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

device="${1:-${DEVICE:-}}"
if [ -z "$device" ]; then
  device="$(xcrun devicectl list devices 2>/dev/null \
    | grep -iE 'iPhone|iPad' \
    | grep -oE '[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}' \
    | head -1)"
fi
[ -n "$device" ] || { echo "No paired device found (xcrun devicectl list devices)." >&2; exit 1; }

exec xcodebuild -project "$here/profile-minter/ProfileMinter.xcodeproj" -scheme Minter \
  -destination "id=$device" \
  -allowProvisioningUpdates -allowProvisioningDeviceRegistration \
  build
