#!/usr/bin/env bash
# Resizes the playlist, docked in the stack and in a window of its own.
#
#   check_playlist_resize.sh <out-dir>
#
# A docked pane has no window of its own: the window around it belongs to the
# stack, and that window is not resizable, so dragging its corner has to grow
# the pane and let the stack's window follow. Undocked, the same corner is an
# ordinary window resize the platform does. This checks both, and that the
# height a pane was given survives being carried out and put back.
#
# Exits 1 when a drag moved the corner and nothing followed it.
# macOS only; needs `cliclick`.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
drag="${CRANPOSE_DRAG:-$root/../Cranpose-tabs/scripts/dev/drag_window.sh}"
out="${1:?usage: check_playlist_resize.sh <out-dir>}"
mkdir -p "$out"
status=0

[ -x "$drag" ] || { echo "check_playlist_resize.sh: no drag helper at $drag" >&2; exit 2; }
command -v cliclick > /dev/null || { echo "check_playlist_resize.sh: cliclick is required" >&2; exit 2; }

main_height=116 eq_height=116 grab=110 grip=6 corner=8
min_playlist=145

stack() { "$drag" oswindows cranamp | grep 'Cranamp Winamp$'; }
torn() { "$drag" oswindows cranamp | grep -F 'Cranamp Winamp Playlist' || true; }

pkill -f 'target/debug/cranamp' 2> /dev/null || true
sleep 1
trap 'pkill -f "target/debug/cranamp" 2> /dev/null || true' EXIT
(cd "$root" && CRANPOSE_DEMO_TRACE=1 ./target/debug/cranamp > "$out/cranamp.log" 2>&1 &)
sleep 6

report() {
    local what="$1" want="$2" got="$3"
    if [ "$want" = "$got" ]; then
        echo "$what: $got"
    else
        echo "FAIL $what: wanted $want, got $got"
        status=1
    fi
}

# Grow the docked stack by dragging the playlist's corner down.
read -r x y w h _ <<< "$(stack)"
[ -n "${h:-}" ] || { echo "check_playlist_resize.sh: the stack never appeared" >&2; exit 1; }
"$drag" drag "$((x + w - corner)),$((y + h - corner))" "$((x + w - corner)),$((y + h - corner + 90))" 14 40 > /dev/null
sleep 1
read -r _ _ _ grown _ <<< "$(stack)"
report "the docked playlist grew with the corner" "$((h + 90))" "$grown"

# And shrink it again.
read -r x y w h _ <<< "$(stack)"
"$drag" drag "$((x + w - corner)),$((y + h - corner))" "$((x + w - corner)),$((y + h - corner - 150))" 14 40 > /dev/null
sleep 1
read -r _ _ _ shrunk _ <<< "$(stack)"
report "and shrank with it" "$((h - 150))" "$shrunk"

# The stack stops where the pane's own floor is.
read -r x y w h _ <<< "$(stack)"
"$drag" drag "$((x + w - corner)),$((y + h - corner))" "$((x + w - corner)),$((y + h - corner - 300))" 16 40 > /dev/null
sleep 1
read -r _ _ _ floored _ <<< "$(stack)"
report "the stack stops at the pane's floor" "$((main_height + eq_height + min_playlist))" "$floored"

# Carry the playlist out and resize the window it becomes.
read -r x y w h _ <<< "$(stack)"
"$drag" drag "$((x + grab)),$((y + main_height + eq_height + grip))" \
    "$((x + grab + 420)),$((y + main_height + eq_height + grip + 160))" 20 30 > /dev/null
sleep 1
read -r px py pw ph _ <<< "$(torn)"
if [ -z "${ph:-}" ]; then
    echo "FAIL the playlist would not come out"
    exit 1
fi
"$drag" drag "$((px + pw - corner)),$((py + ph - corner))" \
    "$((px + pw - corner + 60)),$((py + ph - corner + 120))" 14 40 > /dev/null
sleep 1
read -r _ _ nw nh _ <<< "$(torn)"
report "the torn playlist resized as a window" "$((pw + 60))x$((ph + 120))" "${nw}x${nh}"

# Put it back: the stack owns the width, the pane keeps the height.
read -r sx sy sw sh _ <<< "$(stack)"
read -r px py pw ph _ <<< "$(torn)"
"$drag" drag "$((px + grab)),$((py + grip))" "$((sx + grab)),$((sy + sh + grip))" 20 30 > /dev/null
sleep 1
read -r _ _ back_w back_h _ <<< "$(stack)"
report "docking keeps the height it was given" "${sw}x$((sh + ph))" "${back_w}x${back_h}"

exit $status
