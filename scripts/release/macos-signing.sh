#!/usr/bin/env bash
set -euo pipefail

prepare() {
  : "${P12_BASE64:?Missing CRANAMP_MACOS_DEVID_P12_BASE64}"
  : "${P12_PASSWORD:?Missing CRANAMP_MACOS_DEVID_P12_PASSWORD}"
  : "${RUNNER_TEMP:?Missing RUNNER_TEMP}"
  : "${GITHUB_ENV:?Missing GITHUB_ENV}"
  umask 077
  CRANAMP_SIGN_STATE=$(mktemp -d "$RUNNER_TEMP/cranamp-developer-id.XXXXXX")
  CRANAMP_SIGN_KEYCHAIN="$CRANAMP_SIGN_STATE/signing.keychain-db"
  CRANAMP_SIGN_KEYCHAIN_PASS=$(openssl rand -base64 32)
  export CRANAMP_SIGN_KEYCHAIN_PASS
  echo "::add-mask::$CRANAMP_SIGN_KEYCHAIN_PASS"
  printf 'CRANAMP_SIGN_STATE=%s\nCRANAMP_SIGN_KEYCHAIN=%s\nCRANAMP_SIGN_KEYCHAIN_PASS=%s\n' \
    "$CRANAMP_SIGN_STATE" "$CRANAMP_SIGN_KEYCHAIN" "$CRANAMP_SIGN_KEYCHAIN_PASS" >> "$GITHUB_ENV"
  trap 'rm -f "$CRANAMP_SIGN_STATE"/*.p12 "$CRANAMP_SIGN_STATE"/*.pem' EXIT
  printf '%s' "$P12_BASE64" | base64 -d > "$CRANAMP_SIGN_STATE/input.p12"

  # Import only the identity from the secret. Bundled CA certificates must not
  # shadow the runner's system trust anchors or Apple's current intermediate.
  # Older Keychain exports use RC2, which OpenSSL 3 reads via its legacy provider.
  local read_options=(-passin env:P12_PASSWORD)
  local pkcs12_help
  pkcs12_help=$(openssl pkcs12 -help 2>&1 || true)
  if [[ "$pkcs12_help" == *"-legacy"* ]]; then
    read_options+=(-legacy)
  fi
  openssl pkcs12 -in "$CRANAMP_SIGN_STATE/input.p12" "${read_options[@]}" \
    -clcerts -nokeys -out "$CRANAMP_SIGN_STATE/leaf.pem"
  test "$(grep -c 'BEGIN CERTIFICATE' "$CRANAMP_SIGN_STATE/leaf.pem")" = 1
  openssl pkcs12 -in "$CRANAMP_SIGN_STATE/input.p12" "${read_options[@]}" \
    -nocerts -out "$CRANAMP_SIGN_STATE/key.pem" -passout env:CRANAMP_SIGN_KEYCHAIN_PASS
  openssl pkcs12 -export -in "$CRANAMP_SIGN_STATE/leaf.pem" \
    -inkey "$CRANAMP_SIGN_STATE/key.pem" -passin env:CRANAMP_SIGN_KEYCHAIN_PASS \
    -out "$CRANAMP_SIGN_STATE/identity.p12" -passout env:CRANAMP_SIGN_KEYCHAIN_PASS

  security create-keychain -p "$CRANAMP_SIGN_KEYCHAIN_PASS" "$CRANAMP_SIGN_KEYCHAIN"
  security set-keychain-settings -ut 21600 "$CRANAMP_SIGN_KEYCHAIN"
  security unlock-keychain -p "$CRANAMP_SIGN_KEYCHAIN_PASS" "$CRANAMP_SIGN_KEYCHAIN"
  local keep=("$CRANAMP_SIGN_KEYCHAIN")
  while IFS= read -r existing; do
    [ -n "$existing" ] && keep+=("$existing")
  done < <(security list-keychains -d user | sed 's/^[[:space:]]*"//;s/"$//')
  security list-keychains -d user -s "${keep[@]}"
  security default-keychain -d user > "$CRANAMP_SIGN_STATE/default-keychain-before.txt"
  security default-keychain -d user -s "$CRANAMP_SIGN_KEYCHAIN"

  local intermediate="$CRANAMP_SIGN_STATE/DeveloperIDG2CA.cer"
  curl --proto '=https' --tlsv1.2 --retry 5 --retry-connrefused \
    --location --silent --show-error --fail --output "$intermediate" \
    https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer
  local expected=f16cd3c54c7f83cea4bf1a3e6a0819c8aaa8e4a1528fd144715f350643d2df3a
  test "$(shasum -a 256 "$intermediate" | cut -d' ' -f1)" = "$expected"
  security add-certificates -k "$CRANAMP_SIGN_KEYCHAIN" "$intermediate"
  security import "$CRANAMP_SIGN_STATE/identity.p12" -k "$CRANAMP_SIGN_KEYCHAIN" \
    -P "$CRANAMP_SIGN_KEYCHAIN_PASS" -T /usr/bin/codesign -f pkcs12
  security set-key-partition-list -S apple-tool:,apple:,codesign: \
    -s -k "$CRANAMP_SIGN_KEYCHAIN_PASS" "$CRANAMP_SIGN_KEYCHAIN" >/dev/null
  security find-certificate -a -Z -c 'Developer ID Certification Authority' "$CRANAMP_SIGN_KEYCHAIN" \
    | grep -E 'SHA-256 hash:|"labl"'

  # A codeSign-only evaluation can fetch a missing issuer and conceal a broken
  # local chain. Check local certificates, then actually exercise the private key.
  if ! security verify-cert -L -p basic -c "$CRANAMP_SIGN_STATE/leaf.pem" -k "$CRANAMP_SIGN_KEYCHAIN"; then
    date -u
    sw_vers
    command -v security codesign openssl
    openssl x509 -in "$CRANAMP_SIGN_STATE/leaf.pem" -noout -subject -issuer -dates -fingerprint -sha1
    security verify-cert -L -v -p basic -c "$CRANAMP_SIGN_STATE/leaf.pem" \
      -k "$CRANAMP_SIGN_KEYCHAIN" -k /System/Library/Keychains/SystemRootCertificates.keychain || true
    for domain in user admin; do
      local trust_options=()
      [ "$domain" = user ] || trust_options=(-d)
      security trust-settings-export ${trust_options[@]+"${trust_options[@]}"} \
        "$CRANAMP_SIGN_STATE/$domain-trust.plist" >/dev/null 2>&1 || true
    done
    python3 - "$CRANAMP_SIGN_STATE" <<'PY'
import hashlib, pathlib, plistlib, ssl, sys
state = pathlib.Path(sys.argv[1])
leaf = (state / 'leaf.pem').read_text()
leaf = leaf[leaf.index('-----BEGIN CERTIFICATE-----'):]
targets = {
    hashlib.sha1(ssl.PEM_cert_to_DER_cert(leaf)).hexdigest().upper(): 'Developer ID identity',
    hashlib.sha1((state / 'DeveloperIDG2CA.cer').read_bytes()).hexdigest().upper(): 'Developer ID G2',
    '611E5B662C593A08FF58D14AE22452D198DF6C60': 'Apple Root CA',
}
for domain in ['user', 'admin']:
    path = state / f'{domain}-trust.plist'
    trust = plistlib.loads(path.read_bytes()).get('trustList', {}) if path.exists() else {}
    for fingerprint, name in targets.items():
        entry = trust.get(fingerprint)
        print(domain, name, 'override:', entry.get('trustSettings') if entry else 'none')
PY
    return 1
  fi
  local identity
  identity=$(openssl x509 -in "$CRANAMP_SIGN_STATE/leaf.pem" -noout -fingerprint -sha1 | cut -d= -f2 | tr -d ':')
  security find-identity -v -p codesigning "$CRANAMP_SIGN_KEYCHAIN" \
    | grep -F "$identity" | grep -F 'Developer ID Application:'
  cp /usr/bin/true "$CRANAMP_SIGN_STATE/signing-probe"
  codesign --force --options runtime --timestamp --keychain "$CRANAMP_SIGN_KEYCHAIN" \
    --sign "$identity" "$CRANAMP_SIGN_STATE/signing-probe"
  codesign --verify --strict "$CRANAMP_SIGN_STATE/signing-probe"
  printf 'CRANAMP_SIGN_IDENTITY=%s\n' "$identity" >> "$GITHUB_ENV"
}

sign() {
  local app="${1:?Usage: macos-signing.sh sign PATH_TO_APP}"
  : "${CRANAMP_SIGN_STATE:?Missing signing state}"
  : "${CRANAMP_SIGN_KEYCHAIN:?Missing signing keychain}"
  : "${CRANAMP_SIGN_KEYCHAIN_PASS:?Missing keychain password}"
  : "${CRANAMP_SIGN_IDENTITY:?Missing signing identity}"
  : "${ASC_P8_BASE64:?Missing CRANAMP_ASC_API_KEY_P8_BASE64}"
  : "${ASC_KEY_ID:?Missing CRANAMP_ASC_API_KEY_ID}"
  : "${ASC_ISSUER_ID:?Missing CRANAMP_ASC_API_ISSUER_ID}"
  security unlock-keychain -p "$CRANAMP_SIGN_KEYCHAIN_PASS" "$CRANAMP_SIGN_KEYCHAIN"
  codesign --force --options runtime --timestamp --keychain "$CRANAMP_SIGN_KEYCHAIN" \
    --sign "$CRANAMP_SIGN_IDENTITY" "$app"
  codesign --verify --deep --strict --verbose=2 "$app"
  umask 077
  trap 'rm -f "$CRANAMP_SIGN_STATE/asc.p8" "$CRANAMP_SIGN_STATE/notarize.zip"' EXIT
  printf '%s' "$ASC_P8_BASE64" | base64 -d > "$CRANAMP_SIGN_STATE/asc.p8"
  ditto -c -k --keepParent "$app" "$CRANAMP_SIGN_STATE/notarize.zip"
  xcrun notarytool submit "$CRANAMP_SIGN_STATE/notarize.zip" \
    --key "$CRANAMP_SIGN_STATE/asc.p8" --key-id "$ASC_KEY_ID" --issuer "$ASC_ISSUER_ID" \
    --wait --output-format json > "$CRANAMP_SIGN_STATE/notarization.json"
  python3 - "$CRANAMP_SIGN_STATE/notarization.json" <<'PY'
import json, sys
result = json.load(open(sys.argv[1]))
print('Notarization:', result.get('id'), result.get('status'))
if result.get('status') != 'Accepted':
    raise SystemExit('Apple did not accept the notarization submission')
PY
  xcrun stapler staple "$app"
  xcrun stapler validate "$app"
  spctl --assess --type execute --verbose=2 "$app"
}

cleanup() {
  [ -n "${CRANAMP_SIGN_STATE:-}" ] || return 0
  case "$CRANAMP_SIGN_STATE" in
    "${RUNNER_TEMP:?}"/cranamp-developer-id.*) ;;
    *) echo 'Refusing to clean an unexpected signing directory' >&2; return 1 ;;
  esac
  local result=0
  if [ -f "${CRANAMP_SIGN_KEYCHAIN:-}" ]; then
    local current_default previous_default
    current_default=$(security default-keychain -d user | sed 's/^[[:space:]]*"//;s/"$//')
    if [ "$current_default" = "$CRANAMP_SIGN_KEYCHAIN" ] && [ -f "$CRANAMP_SIGN_STATE/default-keychain-before.txt" ]; then
      previous_default=$(sed 's/^[[:space:]]*"//;s/"$//' "$CRANAMP_SIGN_STATE/default-keychain-before.txt")
      security default-keychain -d user -s "$previous_default" || result=1
    fi
    local keep=()
    while IFS= read -r existing; do
      [ "$existing" = "$CRANAMP_SIGN_KEYCHAIN" ] || keep+=("$existing")
    done < <(security list-keychains -d user | sed 's/^[[:space:]]*"//;s/"$//')
    if [ "${#keep[@]}" -gt 0 ]; then
      security list-keychains -d user -s "${keep[@]}" || result=1
    else
      security list-keychains -d user -s || result=1
    fi
    security delete-keychain "$CRANAMP_SIGN_KEYCHAIN" || result=1
  fi
  rm -rf "$CRANAMP_SIGN_STATE"
  return "$result"
}

case "${1:-}" in
  prepare) prepare ;;
  sign) sign "${2:-}" ;;
  cleanup) cleanup ;;
  *) echo 'Usage: macos-signing.sh prepare|sign PATH_TO_APP|cleanup' >&2; exit 2 ;;
esac
