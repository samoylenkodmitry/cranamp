#!/usr/bin/env bash
# Starts Cranamp the way a Wayland session with XWayland does and checks that
# the player comes up as an X11 window that its title bar can drag.
#
#   check_xwayland_session.sh <out-dir> [binary]
#
# A Wayland session sets both WAYLAND_DISPLAY and DISPLAY. The player places
# and drags its windows in global screen coordinates, which only X11 has, so
# it has to choose the X display. With no WAYLAND_DISPLAY of its own the check
# names a socket that does not exist, so a build that goes to Wayland cannot
# come up at all. Pictures of the player before and after the drag, cut to the
# window, land in <out-dir>.
#
# Exits 1 when the player does not appear on the X display, when the hidden
# host window is on screen, when the player carries no icon for the panel, or
# when dragging the title bar does not move the player. Linux only; needs an X
# display (Xvfb is enough), xdotool, xprop and ImageMagick's `import`.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="${1:?usage: check_xwayland_session.sh <out-dir> [binary]}"
binary="${2:-$root/target/debug/cranamp}"
mkdir -p "$out"

[ -n "${DISPLAY:-}" ] || { echo "check_xwayland_session.sh: DISPLAY names no X display" >&2; exit 2; }
[ -x "$binary" ] || { echo "check_xwayland_session.sh: no cranamp binary at $binary" >&2; exit 2; }
for tool in xdotool xprop import; do
    command -v "$tool" > /dev/null || { echo "check_xwayland_session.sh: needs $tool" >&2; exit 2; }
done

grab=110 grip=6 dx=180 dy=120 steps=12 slack=4
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-cranamp-check-absent-wayland-socket}"

fail() {
    echo "FAIL: $*"
    exit 1
}

picture() {
    import -window root -crop "${WIDTH}x${HEIGHT}+${X}+${Y}" +repage "$out/$1.png"
}

"$binary" > "$out/cranamp.log" 2>&1 &
pid=$!
trap 'kill "$pid" 2> /dev/null || true' EXIT

player="$(timeout 30 xdotool search --sync --onlyvisible --name '^Cranamp Winamp$' | head -n 1)" \
    || fail "the player did not appear on X display $DISPLAY with WAYLAND_DISPLAY=$WAYLAND_DISPLAY (log: $out/cranamp.log)"
sleep 2
eval "$(xdotool getwindowgeometry --shell "$player")"
echo "the player is X11 window $player at ${X},${Y}, ${WIDTH}x${HEIGHT}"
picture before

host="$(xdotool search --onlyvisible --name '^Cranamp$' || true)"
[ -z "$host" ] || fail "the hidden host window is on screen as X11 window $host"

icon="$(xprop -id "$player" _NET_WM_ICON | grep -o 'Icon ([0-9]* x [0-9]*)' | head -n 1 || true)"
[ -n "$icon" ] || fail "the player window carries no _NET_WM_ICON, so panels show a generic icon"
echo "the player carries a panel icon: $icon"

start_x=$X start_y=$Y
xdotool mousemove "$((X + grab))" "$((Y + grip))" mousedown 1
for step in $(seq 1 "$steps"); do
    xdotool mousemove "$((X + grab + dx * step / steps))" "$((Y + grip + dy * step / steps))"
    sleep 0.03
done
xdotool mouseup 1
sleep 1

eval "$(xdotool getwindowgeometry --shell "$player")"
moved_x=$((X - start_x)) moved_y=$((Y - start_y))
echo "dragging the title bar by ${dx},${dy} moved the player by ${moved_x},${moved_y}"
picture after
if [ "$moved_x" -lt $((dx - slack)) ] || [ "$moved_x" -gt $((dx + slack)) ] \
    || [ "$moved_y" -lt $((dy - slack)) ] || [ "$moved_y" -gt $((dy + slack)) ]; then
    fail "the player did not follow its title bar"
fi

echo "PASS: in a Wayland session the player runs on XWayland and its title bar drags it"
