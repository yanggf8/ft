#!/bin/bash
# Deploy the Rust fortunet-api Worker (routes / DOs / Turso / AI failover).
# Requires OAuth (no API token).
#
# 上線前驗證（2026-11-22）：同一份 worker-build 產物
#   1. wrangler deploy --env staging → fortunet-api-staging（獨立 Turso fortunet-staging、DO、R2，
#      service binding → fortunet-engine-staging）
#   2. 對 staging 跑 route sanity＋verify-deployment.sh（API／engine 全套；前端另由 deploy-web.sh 驗）
#      失敗 → 停，不部署正式
#   3. wrangler deploy（正式）→ route sanity
# DB migration 要先在 fortunet-staging 跑、再在 fortunet 跑，都要早於本腳本。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/crates/api"
unset CLOUDFLARE_API_TOKEN 2>/dev/null || true

STAGING_API="https://fortunet-api-staging.yanggf.workers.dev"
STAGING_ENGINE="https://fortunet-engine-staging.yanggf.workers.dev"
PROD_API="https://fortunet-api.yanggf.workers.dev"

echo "🔧 worker-build --release ..."
worker-build --release

# 部署後健檢:未認證探測,最多 5 次、間隔 4s — Cloudflare 邊緣傳播可能落後數秒,
# 立即探測會誤報 404(2026-09-07 實測)。探到期望碼即收工。
probe_get() { # $1=api $2=path $3=expected-code
  local url="$1$2" got="" n
  for n in 1 2 3 4 5; do
    got=$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 "$url")
    [ "$got" = "$3" ] && { echo "$got"; return 0; }
    [ "$n" -lt 5 ] && sleep 4
  done
  echo "$got"
  return 1
}

probe_exchange() { # $1=api；POST 壞 code:期望 401(無效)或 400(形狀錯)
  local got="" n
  for n in 1 2 3 4 5; do
    got=$(curl -s -o /dev/null -w '%{http_code}' --max-time 15 -X POST \
      -H 'Content-Type: application/json' -d '{"code":"sanity"}' \
      "$1/api/auth/oauth/exchange")
    { [ "$got" = "401" ] || [ "$got" = "400" ]; } && { echo "$got"; return 0; }
    [ "$n" -lt 5 ] && sleep 4
  done
  echo "$got"
  return 1
}

sanity() { # $1=api
  local fail=0 overlay exchange
  overlay=$(probe_get "$1" /api/personality/overlay 401) &&
    echo "  ✓ /api/personality/overlay -> $overlay" ||
    { echo "  ✗ overlay -> $overlay (期望 401;404 = 舊碼或傳播未收斂)"; fail=1; }
  exchange=$(probe_exchange "$1") &&
    echo "  ✓ /api/auth/oauth/exchange -> $exchange" ||
    { echo "  ✗ exchange -> $exchange (期望 401/400;404 = 舊碼或傳播未收斂)"; fail=1; }
  return $fail
}

echo "🧪 deploying fortunet-api-staging ..."
wrangler deploy --env staging

echo "🩺 staging route sanity ..."
if ! sanity "$STAGING_API"; then
  echo "❌ staging sanity failed — production NOT deployed"
  exit 1
fi

echo "🧪 staging verify-deployment (API/engine) ..."
if ! API_URL="$STAGING_API" ENGINE_URL="$STAGING_ENGINE" WEB_BASE=skip "$ROOT/scripts/verify-deployment.sh"; then
  echo "❌ staging verify failed — production NOT deployed"
  exit 1
fi

echo "🚀 deploying fortunet-api (production) ..."
wrangler deploy

echo "🩺 production route sanity ..."
if ! sanity "$PROD_API"; then
  echo "⚠️  sanity 未過 — 數分鐘後重跑 verify-deployment.sh 再判(邊緣傳播),或檢查 worker-build 產物。"
  exit 1
fi

echo "✅ done — api endpoint:"
echo "   $PROD_API/health"
