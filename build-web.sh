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
# The page asks for this build's player by a name of its own, and the service
# worker keeps it under that name, so a page never runs with a player an
# earlier build left in the cache.
build="$(cat pkg/cranamp_bg.wasm pkg/cranamp.js | { sha256sum 2>/dev/null || shasum -a 256; } | cut -c1-12)"
versioned="s#\./pkg/cranamp\.js\"#./pkg/cranamp.js?v=${build}\"#; s#\./pkg/cranamp_bg\.wasm\"#./pkg/cranamp_bg.wasm?v=${build}\"#"
sed "$versioned" index.html > dist/index.html
cp assets/icon/favicon.png assets/icon/apple-touch-icon.png dist/
cp assets/icon/icon-192.png assets/icon/icon-512.png assets/icon/icon-maskable-512.png dist/
cp manifest.webmanifest dist/manifest.webmanifest
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
sed "$versioned; s/__CRANAMP_CACHE__/cranamp-${version}-${build}/" sw.js > dist/sw.js
cp -R pkg dist/pkg
mkdir -p dist/demo-music
cp assets/demo-music/generated/*.mp3 dist/demo-music/
cp assets/demo-music/generated/cranamp-demo-playlist.m3u dist/demo-music/

echo "WASM example written to dist/index.html"
