#!/bin/bash
set -euo pipefail

# Puts the macOS signing material into this repository's GitHub secrets, so the
# release job can sign and notarize the .app without anything local.
#
#   scripts/export_macos_signing_secrets.sh \
#     --p12 ~/Desktop/developer-id.p12 \
#     --asc-key ~/Downloads/AuthKey_ABC123.p8 --asc-key-id ABC123 \
#     --asc-issuer 11111111-2222-3333-4444-555555555555
#
# It asks for the .p12 password on the terminal and does not echo it. Every
# value reaches gh over a pipe, so none of them shows up in the process list
# or in the shell history.
#
# The .p12 holds a "Developer ID Application" certificate and its private key.
# Make it in Keychain Access: export the identity, pick a password.
# The .p8 is an App Store Connect API key with the Developer role or higher;
# notarytool signs in with it.

P12="" P12_PASSWORD="" ASC_KEY="" ASC_KEY_ID="" ASC_ISSUER=""
while [ $# -gt 0 ]; do
  case "$1" in
    --p12)          P12="$2"; shift 2 ;;
    --p12-password) P12_PASSWORD="$2"; shift 2 ;;
    --asc-key)      ASC_KEY="$2"; shift 2 ;;
    --asc-key-id)   ASC_KEY_ID="$2"; shift 2 ;;
    --asc-issuer)   ASC_ISSUER="$2"; shift 2 ;;
    *) echo "unknown option: $1" >&2; exit 1 ;;
  esac
done

[ -f "$P12" ] || { echo "--p12 needs a file" >&2; exit 1; }
if [ -z "$P12_PASSWORD" ]; then
  printf 'password for %s: ' "$(basename "$P12")" >&2
  read -rs P12_PASSWORD
  printf '\n' >&2
fi
[ -n "$P12_PASSWORD" ] || { echo "the .p12 password is required" >&2; exit 1; }
[ -f "$ASC_KEY" ] || { echo "--asc-key needs a file" >&2; exit 1; }
[ -n "$ASC_KEY_ID" ] || { echo "--asc-key-id is required" >&2; exit 1; }
[ -n "$ASC_ISSUER" ] || { echo "--asc-issuer is required" >&2; exit 1; }

command -v gh >/dev/null || { echo "gh CLI not found." >&2; exit 1; }
gh auth status >/dev/null 2>&1 || { echo "gh is not authenticated. Run: gh auth login" >&2; exit 1; }

p12_certificates() {
  openssl pkcs12 -in "$P12" -passin fd:3 -nokeys -clcerts -legacy 3<<< "$P12_PASSWORD" 2>/dev/null \
    || openssl pkcs12 -in "$P12" -passin fd:3 -nokeys -clcerts 3<<< "$P12_PASSWORD" 2>/dev/null
}

p12_certificates | openssl x509 -noout -subject 2>/dev/null | grep -q "Developer ID Application" \
  || { echo "$P12 holds no Developer ID Application certificate, or the password is wrong" >&2; exit 1; }

base64 -i "$P12" | gh secret set CRANAMP_MACOS_DEVID_P12_BASE64
printf '%s' "$P12_PASSWORD" | gh secret set CRANAMP_MACOS_DEVID_P12_PASSWORD
base64 -i "$ASC_KEY" | gh secret set CRANAMP_ASC_API_KEY_P8_BASE64
printf '%s' "$ASC_KEY_ID" | gh secret set CRANAMP_ASC_API_KEY_ID
printf '%s' "$ASC_ISSUER" | gh secret set CRANAMP_ASC_API_ISSUER_ID

echo "five secrets set. The next tag builds a signed and notarized Cranamp.app."
