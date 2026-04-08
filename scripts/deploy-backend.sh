#!/bin/bash
# Deploy backend with pre-flight typecheck and post-deploy health check
set -e

cd "$(dirname "$0")/../backend"

echo "🔍 Typecheck..."
npx tsc --noEmit --excludeDirectories src/__tests__

echo "🚀 Deploying backend..."
unset CLOUDFLARE_API_TOKEN
npx wrangler deploy

echo "⏳ Waiting for propagation..."
sleep 3

echo "🏥 Health check..."
HEALTH=$(curl -sf https://fortunet-api.yanggf.workers.dev/health 2>/dev/null || echo "FAIL")

if echo "$HEALTH" | grep -q '"status":"ok"'; then
  echo "✅ Backend deployed and healthy"
  echo "   $HEALTH"
else
  echo "❌ Health check failed!"
  echo "   Response: $HEALTH"
  echo "   Check: https://fortunet-api.yanggf.workers.dev/health"
  exit 1
fi
