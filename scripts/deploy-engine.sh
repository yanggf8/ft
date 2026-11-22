#!/bin/bash
# Deploy the Rust ft-engine Worker (service-binding companion to fortunet-api).
# Requires OAuth (no API token).
#
# 上線前驗證（2026-11-22）：同一份產物先 wrangler deploy --env staging（fortunet-engine-staging），
# 對 staging 驗 health、紫微、流年 golden，過了才部署正式。
set -euo pipefail
cd "$(dirname "$0")/../crates/worker"
unset CLOUDFLARE_API_TOKEN 2>/dev/null || true

STAGING="https://fortunet-engine-staging.yanggf.workers.dev"
PROD="https://fortunet-engine.yanggf.workers.dev"

echo "🔧 worker-build --release ..."
worker-build --release

check() { # $1=engine base；流年 golden 同 verify-deployment.sh 8z
  local n body
  for n in 1 2 3 4 5; do
    body=$(curl -s --max-time 15 "$1/engine/ziwei/horoscope?date=1990-5-15&hour=12&gender=male&target=2026-7-1" || true)
    if echo "$body" | grep -q '"nominalAge":37' && echo "$body" | grep -q '"branch":"寅"' \
      && curl -s --max-time 15 "$1/health" | grep -q '"ok"' \
      && curl -s --max-time 15 "$1/engine/ziwei?date=1990-5-15&hour=12&gender=male" | grep -q '"chart"'; then
      return 0
    fi
    [ "$n" -lt 5 ] && sleep 4
  done
  return 1
}

echo "🧪 deploying fortunet-engine-staging ..."
wrangler deploy --env staging
if ! check "$STAGING"; then
  echo "❌ staging engine check failed — production NOT deployed"
  exit 1
fi
echo "  ✓ staging engine health / ziwei / horoscope golden"

echo "🚀 deploying fortunet-engine (production) ..."
wrangler deploy
if ! check "$PROD"; then
  echo "❌ production engine check failed — investigate now"
  exit 1
fi
echo "✅ done — $PROD"
