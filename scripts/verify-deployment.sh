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

echo ""
if [ "$FAILED" -ne 0 ]; then
  echo -e "${RED}❌ Verification FAILED${NC} (see ✗ above)"
  exit 1
fi
echo -e "${GREEN}✅ All checks passed!${NC}"
echo ""
echo "Deployment verified successfully at $(date)"
