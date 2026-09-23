#!/usr/bin/env bash
# Runs test/windows-shell.ps1 on a Windows desktop reached over SSH.
#
#   test/windows-shell.sh <cranamp.exe> <out-dir> [ssh-host]
#
# Copies the executable and the check into ~/cranamp-shell-audit on the host,
# starts the check in the signed-in desktop session, waits for its verdict and
# brings back audit.log, failure.txt and the pictures of the executable's and
# the player window's icons, then removes the CranampShellAudit scheduled task
# the check reached the desktop through. The host defaults to `win`.
#
# Exits 1 when the check fails or does not finish within a minute.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exe="${1:?usage: windows-shell.sh <cranamp.exe> <out-dir> [ssh-host]}"
out="${2:?usage: windows-shell.sh <cranamp.exe> <out-dir> [ssh-host]}"
host="${3:-win}"
remote="cranamp-shell-audit"
mkdir -p "$out"

ssh "$host" "New-Item -ItemType Directory -Force -Path $remote | Out-Null"
scp -q "$exe" "$host:$remote/cranamp.exe"
scp -q "$here/windows-shell.ps1" "$host:$remote/windows-shell.ps1"
ssh "$host" "powershell -NoProfile -ExecutionPolicy Bypass -File $remote\\windows-shell.ps1 -Executable \$HOME\\$remote\\cranamp.exe -OutputDirectory \$HOME\\$remote\\out -Schedule"

verdict=""
sleep 14
for _ in $(seq 1 30); do
    sleep 2
    verdict="$(ssh "$host" "Get-Content $remote\\out\\result.txt -ErrorAction SilentlyContinue" | tr -d '\r' || true)"
    [ -n "$verdict" ] && break
done
sleep 2
ssh "$host" "Unregister-ScheduledTask -TaskName CranampShellAudit -Confirm:\$false -ErrorAction SilentlyContinue"

for file in audit.log failure.txt exe-icon.png window-icon.png; do
    rm -f "$out/$file"
    scp -q "$host:$remote/out/$file" "$out/$file" 2> /dev/null || true
done
grep -E '^(PE subsystem|File description|Icons in|Process started|Console window|Visible windows|Window |Taskbar icon)' "$out/audit.log" 2> /dev/null || true
[ -f "$out/failure.txt" ] && sed -n '1,3p' "$out/failure.txt"

case "$verdict" in
    PASS) echo "PASS: no terminal, and the executable and every window carry the icon" ;;
    FAIL) echo "FAIL: see $out/failure.txt"; exit 1 ;;
    *) echo "FAIL: the check did not finish on $host"; exit 1 ;;
esac
