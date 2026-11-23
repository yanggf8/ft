#!/bin/bash
# Build the Leptos frontend and deploy to Cloudflare Pages, gated by a real-browser smoke test.
# Requires OAuth (no CLOUDFLARE_API_TOKEN).
#
# 本專案沒有本機伺服器，上線前驗證跑在固定的 staging 分支：
#   1. build-web.sh
#   2. 部署 dist 到 --branch=staging → https://staging.fortunet.pages.dev
#      （網頁依 host 自動改打 fortunet-api-staging，資料寫進 fortunet-staging DB，不碰正式資料）
#   3. scripts/web-smoke.mjs --base staging（本機 turso 已登入時加 --login 驗登入後頁面）
#      失敗 → 停，不部署正式站
#   4. 同一份 dist 部署到 --branch=main（正式站）
#   5. web-smoke 打 https://ahexagram.com（上線後驗證）
# 環境沒有 playwright／chromium（web-smoke 回 2）時停下，不跳過驗證直接上線。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/crates/web"
unset CLOUDFLARE_API_TOKEN 2>/dev/null || true

STAGING="https://staging.fortunet.pages.dev"
PROD="https://ahexagram.com"
LOGIN_FLAG=""
STAGING_API="https://fortunet-api-staging.yanggf.workers.dev"
if command -v turso >/dev/null 2>&1 && turso db list >/dev/null 2>&1; then
  LOGIN_FLAG="--login"
fi

# Pages 新部署切換有延遲（2026-11-23／24 三次：剛上線的 smoke 讀到舊 CSS／舊頁／新圖 404，
# 隔幾秒重跑即全過）：smoke 失敗（exit 1）時等 30 秒重跑一次；exit 2（環境沒有 playwright）不重試。
smoke() {
  local rc=0
  "$@" || rc=$?
  if [ "$rc" -eq 1 ]; then
    echo "⏳ smoke failed once — waiting 30s for Pages propagation, then retrying ..."
    sleep 30
    rc=0
    "$@" || rc=$?
  fi
  return "$rc"
}

echo "🔧 building frontend ..."
./scripts/build-web.sh

echo "🧪 deploying to staging ($STAGING) ..."
npx wrangler pages deploy dist --project-name=fortunet --branch=staging --commit-dirty=true

echo "🧪 smoke test on staging ${LOGIN_FLAG:+(with login)} ..."
if ! FT_API="$STAGING_API" FT_DB=fortunet-staging smoke node "$ROOT/scripts/web-smoke.mjs" --base "$STAGING" $LOGIN_FLAG; then
  echo "❌ staging smoke failed — production NOT deployed"
  exit 1
fi

echo "🚀 deploying to production ..."
npx wrangler pages deploy dist --project-name=fortunet --branch=main --commit-dirty=true

echo "🔎 smoke test on production ..."
if ! smoke node "$ROOT/scripts/web-smoke.mjs" --base "$PROD"; then
  echo "❌ production smoke failed — investigate now (previous deployment can be restored in the Pages dashboard)"
  exit 1
fi

echo "✅ done — $PROD"
