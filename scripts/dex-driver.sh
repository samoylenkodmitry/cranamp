#!/usr/bin/env bash
#
# dex-driver.sh — drive a cranpose/cranamp app over adb in ANY Android window
# mode, including Samsung DeX desktop mode and freeform windows.
#
# Why this exists
# ---------------
# In a freeform / DeX window the activity's GPU render buffer is LARGER than its
# on-screen frame: the system adds a shadow/resize margin (the window
# surfaceInsets), so e.g. the buffer is 525x879 px while the visible frame is
# only 447x801. cranpose already corrects pointer input by that inset (see
# `surface_inset_px` in crates/cranpose/src/android.rs), so a real touch — or an
# `adb input tap` — at the on-screen FRAME pixel lands on the right control.
#
# The only thing that trips you up when driving over adb is arithmetic: you must
# work in the window's own pixel space, not eyeball coordinates off a downscaled
# full-screen screenshot (that is how taps "miss"). This helper removes the
# guesswork:
#   * `shot`  saves a 1:1 PNG of JUST the app window — find a control's (x,y) in it
#   * `tap x y` taps that window-relative pixel (it adds the frame origin for you)
#
# It is window-mode agnostic: in fullscreen the frame origin is (0,0) and it
# behaves like a plain `adb input tap`.
#
# Usage
# -----
#   PKG=com.cranamp.app ADB_SERIAL=<serial> scripts/dex-driver.sh shot win.png
#   PKG=com.cranamp.app ADB_SERIAL=<serial> scripts/dex-driver.sh tap 248 144
#   scripts/dex-driver.sh bounds
#   scripts/dex-driver.sh swipe 200 600 200 200 250   # frame-relative swipe
#
# PKG defaults to com.cranamp.app. ADB_SERIAL is optional (omit for a single
# device). Requires adb and ImageMagick (`magick` or `convert`) for `shot`.
set -euo pipefail

PKG="${PKG:-com.cranamp.app}"

adb_cmd() { adb ${ADB_SERIAL:+-s "$ADB_SERIAL"} "$@"; }

# Resolve the focused PKG window's on-screen frame as "L T R B" (physical px).
# `am stack list` carries the freeform window bounds most reliably; fall back to
# `dumpsys activity activities` where it does not.
win_bounds() {
    {
        adb_cmd shell am stack list 2>/dev/null || true
        adb_cmd shell dumpsys activity activities 2>/dev/null || true
    } \
        | grep -F "$PKG" | grep "visible=true" \
        | grep -oE 'bounds=\[[0-9-]+,[0-9-]+\]\[[0-9-]+,[0-9-]+\]' | head -1 \
        | grep -oE -- '-?[0-9]+' | head -4 | paste -sd' '
}

cmd="${1:-}"
shift || true

read -r L T R B <<<"$(win_bounds)"
if [ -z "${B:-}" ]; then
    echo "dex-driver: could not resolve a visible '$PKG' window (is it running?)" >&2
    exit 1
fi
W=$((R - L))
H=$((B - T))

case "$cmd" in
    bounds)
        echo "frame: ${L},${T} -> ${R},${B}  (${W}x${H})"
        ;;
    tap)
        [ $# -ge 2 ] || { echo "usage: $0 tap <x> <y>" >&2; exit 2; }
        adb_cmd shell input tap "$((L + $1))" "$((T + $2))"
        ;;
    swipe)
        [ $# -ge 4 ] || { echo "usage: $0 swipe <x1> <y1> <x2> <y2> [ms]" >&2; exit 2; }
        adb_cmd shell input swipe \
            "$((L + $1))" "$((T + $2))" "$((L + $3))" "$((T + $4))" "${5:-200}"
        ;;
    shot)
        out="${1:-win.png}"
        im="$(command -v magick || command -v convert || true)"
        [ -n "$im" ] || { echo "dex-driver: ImageMagick (magick/convert) not found" >&2; exit 1; }
        tmp="$(mktemp --suffix=.png)"
        adb_cmd exec-out screencap -p >"$tmp"
        "$im" "$tmp" -crop "${W}x${H}+${L}+${T}" +repage "$out"
        rm -f "$tmp"
        echo "$out  (${W}x${H}) — locate a control, then: $0 tap <x_in_image> <y_in_image>"
        ;;
    *)
        echo "usage: $0 {bounds | shot [out.png] | tap <x> <y> | swipe <x1> <y1> <x2> <y2> [ms]}" >&2
        exit 2
        ;;
esac
