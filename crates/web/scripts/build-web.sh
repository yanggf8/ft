#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."   # crates/web
ROOT="$(cd ../.. && pwd)"   # repo root (workspace target 在此)

echo "🔧 wasm32-unknown-unknown release build ..."
cargo build --locked -p ft-web --target wasm32-unknown-unknown --release

echo "🔧 wasm-bindgen --target web ..."
mkdir -p dist
wasm-bindgen "$ROOT/target/wasm32-unknown-unknown/release/ft_web.wasm" \
  --target web \
  --out-dir dist/wasm

echo "🔧 copy index.html + style.css ..."
cp index.html dist/index.html   # (already references ./wasm/ft_web.js)
cp style.css dist/style.css

echo "✅ dist/ ready"
ls -lh dist
