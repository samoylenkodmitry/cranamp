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

echo "WASM example written to dist/index.html"
