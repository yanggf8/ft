#!/bin/bash
# 把 staging（fortunet-api-staging）缺的 secrets 從 ~/.secrets 寫進 Cloudflare（2026-11-22）。
#
# 先在 ~/.secrets 加上（值自己貼，不要貼進 AI 對話）：
#   export FORTUNET_RESEND_API_KEY=…        # Resend → API Keys（建議另開一把 staging 用）
#   export FORTUNET_GOOGLE_CLIENT_SECRET=…  # Google Cloud Console → OAuth 用戶端 892069949445-…
#   export FORTUNET_CEREBRAS_API_KEY=…      # Cerebras Cloud → API Keys
#   export FORTUNET_GROQ_API_KEY=…          # Groq Console → API Keys
# 再跑：./scripts/staging-secrets.sh
#
# Google 登入另需在同一個 OAuth 用戶端的「已授權的重新導向 URI」加：
#   https://fortunet-api-staging.yanggf.workers.dev/api/auth/google/callback
# 值只經 stdin 進 wrangler，不印出、不進命令列參數。缺哪把就跳過哪把並列出。
set -uo pipefail
cd "$(dirname "$0")/../crates/api"
unset CLOUDFLARE_API_TOKEN 2>/dev/null || true
# shellcheck disable=SC1090
. ~/.secrets

missing=()
for pair in RESEND_API_KEY:FORTUNET_RESEND_API_KEY \
            GOOGLE_CLIENT_SECRET:FORTUNET_GOOGLE_CLIENT_SECRET \
            CEREBRAS_API_KEY:FORTUNET_CEREBRAS_API_KEY \
            GROQ_API_KEY:FORTUNET_GROQ_API_KEY; do
  name="${pair%%:*}"; src="${pair##*:}"
  val="${!src:-}"
  if [ -z "$val" ]; then
    missing+=("$src")
    continue
  fi
  if printf '%s' "$val" | wrangler secret put "$name" --env staging >/dev/null 2>&1; then
    echo "✓ $name → fortunet-api-staging"
  else
    echo "✗ $name 寫入失敗（wrangler whoami 檢查登入）"
  fi
done

if [ "${#missing[@]}" -gt 0 ]; then
  echo "⚠ ~/.secrets 還沒有：${missing[*]}"
fi
echo "目前 staging secrets："
wrangler secret list --env staging 2>/dev/null | grep -o '"name": "[^"]*"' | sed 's/"name": /  /'
