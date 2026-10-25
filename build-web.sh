#!/usr/bin/env bash
set -euo pipefail

if ! command -v cranpose-build >/dev/null 2>&1; then
  echo "cranpose-build 0.3.0 or later is required: https://github.com/samoylenkodmitry/cranpose-build/releases" >&2
  exit 1
fi

rm -rf pkg dist

# cranpose-build records the pipelines the first screen draws with in headless
# Chrome and builds the module again with them embedded, so on WebGL the page
# builds them as it starts. Without Chrome it keeps the module it built first.
OUT="target/cranpose-web-page"
cranpose-build build --platform web --release --output-dir "$OUT"
cp -R "$(ls -dt "$OUT"/web/release/build-*/application | head -1)/pkg" pkg

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
cp -R site/privacy site/support dist/
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
sed "$versioned; s/__CRANAMP_CACHE__/cranamp-${version}-${build}/" sw.js > dist/sw.js
cp -R pkg dist/pkg
mkdir -p dist/demo-music
cp assets/demo-music/generated/*.mp3 dist/demo-music/
cp assets/demo-music/generated/cranamp-demo-playlist.m3u dist/demo-music/

echo "WASM example written to dist/index.html"
