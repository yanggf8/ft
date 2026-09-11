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

echo "🔧 copy index.html + style.css + galaxy.js + boot.js + _headers ..."
cp index.html dist/index.html   # (already references ./wasm/ft_web.js)
cp style.css dist/style.css
cp galaxy.js dist/galaxy.js
cp boot.js dist/boot.js         # wasm 啟動器(外部檔;CSP script-src 'self' 擋 inline)
cp _headers dist/_headers       # Pages 安全標頭(CSP/HSTS/框架防護;P3)

echo "✅ dist/ ready"
ls -lh dist
