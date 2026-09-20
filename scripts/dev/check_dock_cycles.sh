#!/usr/bin/env bash
# Carries a pane out of the stack and puts it back, over and over.
#
#   check_dock_cycles.sh <out-dir> [rounds] [pane]
#
#   check_dock_cycles.sh /tmp/dock 3            # the playlist
#   check_dock_cycles.sh /tmp/dock 3 equalizer
#
# Tearing and docking both move the stack's lower edge, so the place a pane
# has to be let go to go back in is different every round. It is read from
# the window each time rather than worked out once, which is what made a
# hand-run check report failures that were the coordinates going stale.
#
# Exits 1 on a round where the pane did not come out, or did not go back in.
# macOS only; needs `cliclick`.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"
drag="${CRANPOSE_DRAG:-$root/../Cranpose-tabs/scripts/dev/drag_window.sh}"
out="${1:?usage: check_dock_cycles.sh <out-dir> [rounds] [pane]}"
rounds="${2:-3}"
pane="${3:-playlist}"
mkdir -p "$out"
status=0

[ -x "$drag" ] || { echo "check_dock_cycles.sh: no drag helper at $drag" >&2; exit 2; }

main_height=116 eq_height=116 grab=110 grip=6
case "$pane" in
    equalizer) title="Cranamp Winamp Equalizer" ;;
    playlist) title="Cranamp Winamp Playlist" ;;
    *) echo "check_dock_cycles.sh: pane is equalizer or playlist" >&2; exit 2 ;;
esac

stack() { "$drag" oswindows cranamp | grep 'Cranamp Winamp$'; }
torn() { "$drag" oswindows cranamp | grep -F "$title" || true; }

pkill -f 'target/debug/cranamp' 2> /dev/null || true
sleep 1
(cd "$root" && ./target/debug/cranamp > "$out/cranamp.log" 2>&1 &)
sleep 8
read -r sx sy _ _ _title <<< "$(stack)"
# The first press on a window that is not frontmost is spent focusing it.
"$drag" click $((sx + grab)) $((sy + 40)) > /dev/null 2>&1 || true
sleep 1

for round in $(seq 1 "$rounds"); do
    read -r sx sy _ sh _title <<< "$(stack)"
    # A docked pane sits under the panes above it; only the main pane is
    # always there, and the equalizer only when it has not been torn out.
    offset=$main_height
    if [ "$pane" = playlist ] && [ "$sh" -gt $((main_height + eq_height)) ]; then
        offset=$((main_height + eq_height))
    fi
    "$drag" drag "$((sx + grab)),$((sy + offset + grip))" "$((sx + 560)),$((sy + 200))" 24 25 > /dev/null
    sleep 1
    read -r tx ty _ _ _title <<< "$(torn)"
    if [ -z "${tx:-}" ]; then
        echo "round $round: FAIL the $pane did not come out of the stack"
        status=1
        break
    fi
    echo "round $round: the $pane came out to ${tx},${ty}"

    read -r sx sy _ sh _title <<< "$(stack)"
    "$drag" drag "$((tx + grab)),$((ty + grip))" \
        "$((sx + grab)),$((sy + sh + grip))" 24 25 > /dev/null
    sleep 1
    if [ -n "$(torn)" ]; then
        echo "round $round: FAIL the $pane stayed out after being put on the stack"
        status=1
        break
    fi
    read -r _ _ _ sh2 _title <<< "$(stack)"
    echo "round $round: it went back in and the stack is ${sh2} tall"
done

pkill -f 'target/debug/cranamp' 2> /dev/null || true
exit $status
