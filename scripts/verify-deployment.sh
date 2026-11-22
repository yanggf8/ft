#!/bin/bash
# Deployment Verification Script
# Run after each deployment to verify all services are working
#
# Chart-calculation tests hit the public engine worker directly: the old
# POST /api/charts/calculate/* debug routes were removed during the security
# hardening, and the API worker's real calculation path (POST /api/users/me/birth)
# requires an authenticated session, which a smoke test cannot script.
#
# Test 7 (rate limiting) is opt-in: it hammers POST /api/auth/login past its
# per-minute limit, which writes dead login-token rows to prod Turso and rate-limits
# the caller's own IP for the 60 s window. Set RUN_RATE_LIMIT_TEST=1 to include it.

set -e

API_URL="${API_URL:-https://fortunet-api.yanggf.workers.dev}"
ENGINE_URL="${ENGINE_URL:-https://fortunet-engine.yanggf.workers.dev}"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'
FAILED=0

# curl with a hard timeout, so a dead endpoint prints ✗ instead of hanging
# or tripping set -e mid-script
hcurl() { curl -s --max-time 15 "$@"; }

echo "🔍 Verifying deployment at $API_URL"
echo "   Engine worker at $ENGINE_URL"
echo ""

# Test 1: API health check
echo -n "1. API health check... "
HEALTH=$(hcurl "$API_URL/health") || true
if echo "$HEALTH" | grep -q '"status":"ok"'; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
  echo "Response: $HEALTH"
  exit 1
fi

# Test 2: Database health
echo -n "2. Database health... "
DB_HEALTH=$(hcurl "$API_URL/health/db") || true
if echo "$DB_HEALTH" | grep -q '"status":"ok"'; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
  echo "Response: $DB_HEALTH"
  exit 1
fi

# Test 3: Engine worker health
echo -n "3. Engine worker health... "
ENGINE_HEALTH=$(hcurl "$ENGINE_URL/health") || true
if echo "$ENGINE_HEALTH" | grep -q '"status":"ok"'; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
  echo "Response: $ENGINE_HEALTH"
  exit 1
fi

# Test 4: ZiWei calculation (engine worker)
echo -n "4. ZiWei calculation... "
ZIWEI=$(hcurl "$ENGINE_URL/engine/ziwei?date=1990-5-15&hour=14&gender=male") || true
if echo "$ZIWEI" | grep -q '"palaces"'; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
  echo "Response: $ZIWEI"
  exit 1
fi

# Test 5: Western calculation (engine worker)
# JD from a fixed UTC instant: JD = epoch_seconds/86400 + 2440587.5 (1970-01-01T00:00Z).
# Epoch conversion: GNU date uses -d, BSD/macOS date uses -j -f; a failure here
# must fail loud (an empty JD would only surface as a confusing engine 400).
# LC_ALL=C pins the awk radix point regardless of the caller's locale.
echo -n "5. Western calculation... "
EPOCH=$(date -u -d "1990-03-25 12:00:00" +%s 2>/dev/null \
  || date -u -j -f "%Y-%m-%d %H:%M:%S" "1990-03-25 12:00:00" +%s 2>/dev/null) || {
  echo -e "${RED}✗${NC} (cannot compute test epoch — no supported date -d/-j)"
  exit 1
}
JD=$(LC_ALL=C awk -v e="$EPOCH" 'BEGIN { printf "%.5f", e/86400 + 2440587.5 }')
WESTERN=$(hcurl "$ENGINE_URL/engine/western?jdUtc=$JD") || true
if echo "$WESTERN" | grep -q '"sunSign"'; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC} (jdUtc=$JD)"
  echo "Response: $WESTERN"
  exit 1
fi

# Test 6: Security headers
echo -n "6. Security headers... "
HEADERS=$(hcurl -I "$API_URL/health") || true
if echo "$HEADERS" | grep -qi "x-frame-options" && \
   echo "$HEADERS" | grep -qi "x-content-type-options"; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC} (headers missing)"
  FAILED=1
fi

# Test 7: Rate limiting (opt-in — see header note)
echo -n "7. Rate limiting... "
if [ "${RUN_RATE_LIMIT_TEST:-0}" != "1" ]; then
  echo -e "SKIPPED (set RUN_RATE_LIMIT_TEST=1 to run)"
else
  for i in {1..35}; do
    hcurl -X POST "$API_URL/api/auth/login" \
      -H "Content-Type: application/json" \
      -d '{"email":"ratelimit-probe@example.invalid"}' \
      > /dev/null 2>&1 || true
  done
  RATE_LIMIT=$(hcurl -X POST "$API_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"email":"ratelimit-probe@example.invalid"}' \
    -w "%{http_code}" -o /dev/null) || true
  if [ "$RATE_LIMIT" = "429" ]; then
    echo -e "${GREEN}✓${NC}"
  else
    echo -e "${RED}✗${NC} (Expected 429, got $RATE_LIMIT)"
    FAILED=1
  fi
fi

# Test 8: F5 predictions endpoints are routed and auth-guarded (401 without a session).
# Auth-guard check is the scriptable part — a full F6 flow needs a magic-link login.
echo -n "8. F5 predictions endpoints (401 without auth)... "
PRED_OK=1
for probe in \
  "GET|/api/predictions" \
  "POST|/api/predictions/generate" \
  "PUT|/api/predictions/checks" \
  "POST|/api/predictions/_smoke_/feedback"
do
  METHOD="${probe%%|*}"
  PATHNAME="${probe##*|}"
  CODE=$(hcurl -X "$METHOD" "$API_URL$PATHNAME" -w "%{http_code}" -o /dev/null) || true
  if [ "$CODE" != "401" ]; then
    echo ""
    echo "   ✗ $METHOD $PATHNAME -> $CODE (expected 401)"
    PRED_OK=0
  fi
done
if [ "$PRED_OK" = "1" ]; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC} (see above)"
  FAILED=1
fi

# Test 8b: tarot endpoints are routed and auth-guarded (401 without a session).
# (auth fires before any rate limit on these paths — 401 probes consume no quota)
echo -n "8b. Tarot endpoints (401 without auth)... "
TAROT_OK=1
for probe in \
  "POST|/api/tarot/draw" \
  "GET|/api/tarot/readings" \
  "POST|/api/tarot/readings/00000000-0000-0000-0000-000000000000/interpret" \
  "DELETE|/api/tarot/readings"
do
  METHOD="${probe%%|*}"
  PATHNAME="${probe##*|}"
  CODE=$(hcurl -X "$METHOD" "$API_URL$PATHNAME" -w "%{http_code}" -o /dev/null) || true
  if [ "$CODE" != "401" ]; then
    echo ""
    echo "   ✗ $METHOD $PATHNAME -> $CODE (expected 401)"
    TAROT_OK=0
  fi
done
if [ "$TAROT_OK" = "1" ]; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC} (see above)"
  FAILED=1
fi

# Test 8z: 紫微子單元 endpoints are routed and auth-guarded (401 without a session),
# plus an engine golden for the 流年 horoscope (spec 2026-11-21-sub-units §5).
echo -n "8z. Ziwei sub-unit endpoints (401 without auth)... "
ZW_OK=1
for probe in \
  "GET|/api/ziwei/horoscope" \
  "GET|/api/ziwei/year" \
  "POST|/api/ziwei/year/interpret"
do
  METHOD="${probe%%|*}"
  PATHNAME="${probe##*|}"
  CODE=$(hcurl -X "$METHOD" "$API_URL$PATHNAME" -w "%{http_code}" -o /dev/null) || true
  if [ "$CODE" != "401" ]; then
    echo ""
    echo "   ✗ $METHOD $PATHNAME -> $CODE (expected 401)"
    ZW_OK=0
  fi
done
# golden: 1990-05-15 午時 男, target 2026-7-1 → 流年 丙午、流年命宮 午、虛歲 37、大限 寅、小限 辰
HORO=$(hcurl "$ENGINE_URL/engine/ziwei/horoscope?date=1990-5-15&hour=12&gender=male&target=2026-7-1") || true
if ! echo "$HORO" | python3 -c '
import json,sys
h=json.load(sys.stdin)["horoscope"]
y=h["yearly"]
assert (y["stem"],y["branch"])==("丙","午"), y
assert h["nominalAge"]==37 and h["luckYear"]==2026, h
assert h["decadal"]["branch"]=="寅" and h["age"]["branch"]=="辰", h
assert [y["mutagen"][k] for k in ("lu","quan","ke","ji")]==["天同","天機","文昌","廉貞"], y["mutagen"]
' 2>/dev/null; then
  echo ""
  echo "   ✗ engine horoscope golden mismatch: ${HORO:0:300}"
  ZW_OK=0
fi
if [ "$ZW_OK" = "1" ]; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC} (see above)"
  FAILED=1
fi

# Test 8c: palm endpoints are routed and auth-guarded (401 without a session).
# (auth fires before any rate limit on these paths — 401 probes consume no quota)
echo -n "8c. Palm endpoints (401 without auth)... "
PALM_OK=1
for probe in \
  "POST|/api/palm/analyze" \
  "GET|/api/palm/readings" \
  "DELETE|/api/palm/readings/00000000-0000-0000-0000-000000000000" \
  "DELETE|/api/palm/readings"
do
  METHOD="${probe%%|*}"
  PATHNAME="${probe##*|}"
  CODE=$(hcurl -X "$METHOD" "$API_URL$PATHNAME" -w "%{http_code}" -o /dev/null) || true
  if [ "$CODE" != "401" ]; then
    echo ""
    echo "   ✗ $METHOD $PATHNAME -> $CODE (expected 401)"
    PALM_OK=0
  fi
done
if [ "$PALM_OK" = "1" ]; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC} (see above)"
  FAILED=1
fi

# Test 8d: zodiac match AI endpoint is routed and auth-guarded (401 without a session).
# (auth fires before any rate limit — 401 probe consumes no quota)
echo -n "8d. Zodiac match AI endpoint (401 without auth)... "
CODE=$(hcurl -X POST "$API_URL/api/zodiac/match/interpret" -H "Content-Type: application/json" -d '{"target":"Leo"}' -w "%{http_code}" -o /dev/null) || true
if [ "$CODE" = "401" ]; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC} POST /api/zodiac/match/interpret -> $CODE (expected 401)"
  FAILED=1
fi

# Test 8e: 開運小物 interest endpoints are routed and validate before any write
# (spec 2026-11-22-shop-interest §6). Neither probe inserts a row: an unknown itemId is
# rejected before the DB, and /me with no token and no anonId is rejected as NO_IDENTITY.
# (the POST probe counts once against the caller IP's 30/hour shop limit)
echo -n "8e. Shop interest endpoints (validation, no writes)... "
SHOP_OK=1
BODY=$(hcurl -X POST "$API_URL/api/shop/interest" -H "Content-Type: application/json" -d '{"itemId":"nope"}' -w "|%{http_code}") || true
if [ "${BODY##*|}" != "400" ] || ! echo "${BODY%|*}" | grep -q '"INVALID_ITEM"'; then
  echo ""
  echo "   ✗ POST /api/shop/interest {itemId:nope} -> ${BODY##*|} (expected 400 INVALID_ITEM)"
  SHOP_OK=0
fi
BODY=$(hcurl "$API_URL/api/shop/interest/me" -w "|%{http_code}") || true
if [ "${BODY##*|}" != "400" ] || ! echo "${BODY%|*}" | grep -q '"NO_IDENTITY"'; then
  echo ""
  echo "   ✗ GET /api/shop/interest/me (no identity) -> ${BODY##*|} (expected 400 NO_IDENTITY)"
  SHOP_OK=0
fi
if [ "$SHOP_OK" = "1" ]; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC} (see above)"
  FAILED=1
fi

# Test 9: 前端真瀏覽器驗證（scripts/web-smoke.mjs；需 node＋本機 playwright-core／chromium）。
# 正式站、匿名模式（不建帳號）。WEB_BASE 可覆寫；環境沒有 playwright 時只警告不算失敗。
WEB_BASE="${WEB_BASE:-https://ahexagram.com}"
echo -n "9. Frontend browser smoke ($WEB_BASE)... "
if [ "$WEB_BASE" = "skip" ]; then
  echo "skipped (WEB_BASE=skip)"
elif command -v node >/dev/null 2>&1; then
  SMOKE_OUT=$(node "$(dirname "$0")/web-smoke.mjs" --base "$WEB_BASE" 2>&1); SMOKE_RC=$?
  if [ "$SMOKE_RC" = "0" ]; then
    echo -e "${GREEN}✓${NC}"
  elif [ "$SMOKE_RC" = "2" ]; then
    echo -e "${YELLOW:-}⚠ skipped${NC} (no playwright/chromium: ${SMOKE_OUT##*$'\n'})"
  else
    echo -e "${RED}✗${NC}"
    echo "$SMOKE_OUT" | grep -E "❌" | sed 's/^/   /'
    FAILED=1
  fi
else
  echo "⚠ skipped (node not found)"
fi

echo ""
if [ "$FAILED" -ne 0 ]; then
  echo -e "${RED}❌ Verification FAILED${NC} (see ✗ above)"
  exit 1
fi
echo -e "${GREEN}✅ All checks passed!${NC}"
echo ""
echo "Deployment verified successfully at $(date)"
