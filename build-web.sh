#!/usr/bin/env bash
set -euo pipefail

if command -v wasm-pack >/dev/null 2>&1; then
  WASM_PACK="wasm-pack"
elif [[ -x "$HOME/.cargo/bin/wasm-pack" ]]; then
  WASM_PACK="$HOME/.cargo/bin/wasm-pack"
else
  echo "wasm-pack is required. Install it with: cargo install wasm-pack" >&2
  exit 1
fi

rm -rf pkg dist

"$WASM_PACK" build \
  --target web \
  --release \
  --no-default-features \
  --features web,renderer-wgpu

mkdir -p dist
cp index.html dist/index.html
cp -R pkg dist/pkg
mkdir -p dist/demo-music
cp assets/demo-music/generated/*.mp3 dist/demo-music/
cp assets/demo-music/generated/cranamp-demo-playlist.m3u dist/demo-music/
# The FM set streams from its own host, so only the playlist is published -
# the app reads its compiled-in copy, this one is for other players.
mkdir -p dist/fm-music
cp assets/fm-music/cranamp-fm-playlist.m3u dist/fm-music/

echo "WASM example written to dist/index.html"
