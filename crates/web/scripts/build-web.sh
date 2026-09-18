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

echo "🔧 strip name custom section（runtime 用不到；~900KB 白付傳輸）..."
# 只剝 'name' 自訂段：target-features 等其他自訂段必須保留（strip=true 全剝會
# 重演 workers-rs#1014 wasm-bindgen 失敗；profiles/Cargo.toml 註解有記錄）。
python3 - "$PWD/dist/wasm/ft_web_bg.wasm" <<'PYEOF'
import sys

path = sys.argv[1]
data = open(path, "rb").read()
assert data[:4] == b"\x00asm", "not a wasm file"

def read_u32(buf, p):
    n = 0
    shift = 0
    while True:
        b = buf[p]
        p += 1
        n |= (b & 0x7F) << shift
        shift += 7
        if not (b & 0x80):
            return n, p

out = bytearray(data[:8])
p = 8
stripped = 0
while p < len(data):
    sid = data[p]
    p += 1
    size, p = read_u32(data, p)
    payload = data[p : p + size]
    name = None
    if sid == 0:
        nlen, q = read_u32(payload, 0)
        name = payload[q : q + nlen].decode("utf-8")
    if name == "name":
        stripped += size
        p += size  # 必須前進，否則 payload 會被當新段重新解析
        continue  # 整段丟棄
    out.append(sid)
    # 重寫 size（LEB128，原值不變，直接回寫原 bytes 即可）
    # 這裡保守地重新編碼 size，避免截錯
    n = size
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out.append(b | 0x80)
        else:
            out.append(b)
            break
    out += payload
    p += size

open(path, "wb").write(bytes(out))
print(f"  stripped name section: {stripped:,} bytes")
PYEOF
ls -lh dist/wasm/ft_web_bg.wasm | awk '{print "  wasm now:", $5}'

echo "🔧 copy index.html + style.css + galaxy.js + boot.js + _headers ..."
cp index.html dist/index.html   # (already references ./wasm/ft_web.js)
cp style.css dist/style.css
cp galaxy.js dist/galaxy.js
cp boot.js dist/boot.js         # wasm 啟動器(外部檔;CSP script-src 'self' 擋 inline)
cp _headers dist/_headers       # Pages 安全標頭(CSP/HSTS/框架防護;P3)

echo "✅ dist/ ready"
ls -lh dist
