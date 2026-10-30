#!/usr/bin/env bash
# Run inside dbus-run-session and xvfb-run. Reads Cranamp through public MPRIS.
set -euo pipefail
binary="$(realpath "$1")"
mkdir -p "$2"
output="$(realpath "$2")"
"$binary" "$output" > "$output/player.log" 2>&1 &
player=$!
trap 'kill "$player" 2>/dev/null || true' EXIT
await_file() {
  for _ in $(seq 1 600); do
    test ! -f "$output/$1" || return 0
    kill -0 "$player" 2>/dev/null || { cat "$output/player.log"; return 1; }
    sleep .1
  done
  cat "$output/player.log"
  return 1
}
metadata() {
  gdbus call --session --dest org.mpris.MediaPlayer2.cranamp \
    --object-path /org/mpris/MediaPlayer2 \
    --method org.freedesktop.DBus.Properties.Get \
    org.mpris.MediaPlayer2.Player Metadata
}
await_file ready
metadata > "$output/with-cover.txt"
python3 - "$output/with-cover.txt" <<'PY'
import pathlib, re, sys, urllib.parse
metadata = pathlib.Path(sys.argv[1]).read_text()
assert all(text in metadata for text in ['Blue Song', 'Alice', 'Blue Album']), metadata
cover = re.search(r"file://[^']+", metadata)
assert cover, metadata
path = pathlib.Path(urllib.parse.unquote(urllib.parse.urlparse(cover[0]).path))
assert path.read_bytes().startswith(b'\x89PNG\r\n\x1a\n')
print('MPRIS title, artist, album and readable cover: passed')
PY
gdbus call --session --dest org.mpris.MediaPlayer2.cranamp \
  --object-path /org/mpris/MediaPlayer2 \
  --method org.mpris.MediaPlayer2.Player.Pause
touch "$output/next"
await_file cleared
metadata > "$output/without-cover.txt"
python3 - "$output/without-cover.txt" <<'PY'
import pathlib, sys
metadata = pathlib.Path(sys.argv[1]).read_text()
assert all(text in metadata for text in ['Bare Song', 'Bob', 'Bare Album']), metadata
assert 'file://' not in metadata, metadata
print('MPRIS next track clears the previous album cover: passed')
PY
touch "$output/exit"
wait "$player"
