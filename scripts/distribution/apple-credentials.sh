#!/usr/bin/env bash
set -euo pipefail
: "${P12_BASE64:?Missing CRANAMP_APPLE_STORE_P12_BASE64}"
: "${P12_PASSWORD:?Missing CRANAMP_APPLE_STORE_P12_PASSWORD}"
: "${PROFILE_BASE64:?Missing App Store provisioning profile}"
: "${CRANAMP_APPLE_SIGN_IDENTITY:?Missing Apple Distribution identity}"
keychain="$RUNNER_TEMP/cranamp-store.keychain-db"
password=$(openssl rand -base64 32)
echo "::add-mask::$password"
umask 077
printf '%s' "$P12_BASE64" | base64 -d > "$RUNNER_TEMP/store.p12"
printf '%s' "$PROFILE_BASE64" | base64 -d > "$RUNNER_TEMP/store-profile.mobileprovision"
security create-keychain -p "$password" "$keychain"
security set-keychain-settings -lut 21600 "$keychain"
security unlock-keychain -p "$password" "$keychain"
security import "$RUNNER_TEMP/store.p12" -k "$keychain" -P "$P12_PASSWORD" -T /usr/bin/codesign -T /usr/bin/productbuild
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$password" "$keychain" >/dev/null
security list-keychains -d user -s "$keychain" "$HOME/Library/Keychains/login.keychain-db"
profile_dir="$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"
mkdir -p "$profile_dir"
uuid=$(security cms -D -i "$RUNNER_TEMP/store-profile.mobileprovision" | python3 -c 'import plistlib,sys; print(plistlib.load(sys.stdin.buffer)["UUID"])')
cp "$RUNNER_TEMP/store-profile.mobileprovision" "$profile_dir/$uuid.mobileprovision"
printf 'CRANAMP_SIGN_KEYCHAIN=%s\nCRANAMP_PROVISIONING_PROFILE=%s\nCRANAMP_INSTALLED_PROFILE=%s\n' \
  "$keychain" "$RUNNER_TEMP/store-profile.mobileprovision" "$profile_dir/$uuid.mobileprovision" >> "$GITHUB_ENV"
rm -f "$RUNNER_TEMP/store.p12"
