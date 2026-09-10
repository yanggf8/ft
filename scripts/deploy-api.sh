#!/bin/bash
# Deploy the Rust fortunet-api Worker (routes / DOs / Turso / AI failover).
# Deterministic: worker-build → wrangler deploy. Requires OAuth (no API token).
set -euo pipefail
cd "$(dirname "$0")/../crates/api"

echo "🔧 worker-build --release ..."
worker-build --release

echo "🚀 deploying fortunet-api ..."
unset CLOUDFLARE_API_TOKEN 2>/dev/null || true
wrangler deploy

API="https://fortunet-api.yanggf.workers.dev"

# 部署後健檢:未認證探測,最多 5 次、間隔 4s — Cloudflare 邊緣傳播可能落後數秒,
# 立即探測會誤報 404(2026-09-07 實測)。探到期望碼即收工。
probe_get() { # $1=path $2=expected-code
  local url="$API$1" got="" n
  for n in 1 2 3 4 5; do
    got=$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 "$url")
    [ "$got" = "$2" ] && { echo "$got"; return 0; }
    [ "$n" -lt 5 ] && sleep 4
  done
  echo "$got"
  return 1
}

probe_exchange() { # POST 壞 code:期望 401(無效)或 400(形狀錯)
  local got="" n
  for n in 1 2 3 4 5; do
    got=$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 -X POST \
      -H 'Content-Type: application/json' -d '{"code":"sanity"}' \
      "$API/api/auth/oauth/exchange")
    { [ "$got" = "401" ] || [ "$got" = "400" ]; } && { echo "$got"; return 0; }
    [ "$n" -lt 5 ] && sleep 4
  done
  echo "$got"
  return 1
}

echo "🩺 route sanity (unauthenticated probes, retrying for edge propagation) ..."
fail=0
overlay=$(probe_get /api/personality/overlay 401) &&
  echo "  ✓ /api/personality/overlay -> $overlay" ||
  { echo "  ✗ overlay -> $overlay (期望 401;404 = 舊碼或傳播未收斂)"; fail=1; }
exchange=$(probe_exchange) &&
  echo "  ✓ /api/auth/oauth/exchange -> $exchange" ||
  { echo "  ✗ exchange -> $exchange (期望 401/400;404 = 舊碼或傳播未收斂)"; fail=1; }

if [ "$fail" = "1" ]; then
  echo "⚠️  sanity 未過 — 數分鐘後重跑本腳本再判(邊緣傳播),或檢查 worker-build 產物。"
  exit 1
fi

echo "✅ done — api endpoint:"
echo "   https://fortunet-api.yanggf.workers.dev/health"
