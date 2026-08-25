#!/usr/bin/env bash
# Sign the release .ipa with the local development identity and install it on
# the attached iPhone.
#
# The release .ipa is built unsigned -- CI has no development identity and the
# device set is not CI's business -- so sideloading is a local step: take the
# .app out of the Payload, embed the development profile, re-sign, install.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ipa="${1:?usage: install-ipa.sh <path to .ipa>}"

bundle_id="$(plutil -extract CFBundleIdentifier raw -o - "$here/Info.plist")"
[ -n "$bundle_id" ] || { echo "No CFBundleIdentifier in $here/Info.plist" >&2; exit 1; }

identity="$(security find-identity -v -p codesigning | awk '/Apple Development/{print $2; exit}')"
[ -n "$identity" ] || { echo "No 'Apple Development' identity in the keychain. Add your Apple ID in Xcode > Settings > Accounts." >&2; exit 1; }

# A profile written for this bundle identifier is the better match, but the
# team wildcard Xcode mints by default covers it too -- take whichever is
# there, exact first.
profiles="$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"
profile=""
wildcard=""
while IFS= read -r candidate; do
  candidate_id="$(security cms -D -i "$candidate" 2>/dev/null | plutil -extract Entitlements.application-identifier raw - 2>/dev/null || true)"
  case "$candidate_id" in
    *".$bundle_id") profile="$candidate" ;;
    *.\*) wildcard="$candidate" ;;
  esac
done < <(find "$profiles" -name '*.mobileprovision' 2>/dev/null)
[ -n "$profile" ] || profile="$wildcard"
[ -n "$profile" ] || { echo "No provisioning profile covering $bundle_id. Run regen-profile.sh first." >&2; exit 1; }

team_id="$(security cms -D -i "$profile" | plutil -extract Entitlements.com\\.apple\\.developer\\.team-identifier raw -)"
app_id="$team_id.$bundle_id"
device="${DEVICE:-$(xcrun devicectl list devices 2>/dev/null \
  | grep -iE 'iPhone|iPad' \
  | grep -oE '[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}' \
  | head -1)}"
[ -n "$device" ] || { echo "No paired device found (xcrun devicectl list devices)." >&2; exit 1; }

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
unzip -q "$ipa" -d "$stage"
app="$(find "$stage/Payload" -maxdepth 1 -name '*.app' | head -1)"
[ -n "$app" ] || { echo "$ipa holds no .app under Payload" >&2; exit 1; }

cp "$profile" "$app/embedded.mobileprovision"

entitlements="$stage/entitlements.plist"
cat > "$entitlements" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>application-identifier</key><string>$app_id</string>
  <key>com.apple.developer.team-identifier</key><string>$team_id</string>
  <key>get-task-allow</key><true/>
</dict></plist>
PLIST

echo "identity=$identity team=$team_id app-id=$app_id device=$device profile=$(basename "$profile")" >&2
xattr -cr "$app"
codesign --force --sign "$identity" --entitlements "$entitlements" --timestamp=none \
  --generate-entitlement-der "$app" >&2
codesign --verify --strict "$app" >&2

xcrun devicectl device install app --device "$device" "$app" >&2
