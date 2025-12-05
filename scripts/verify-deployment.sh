#!/bin/bash
# Deployment Verification Script
# Run after each deployment to verify all services are working

set -e

API_URL="${API_URL:-https://fortunet-api.yanggf.workers.dev}"
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "🔍 Verifying deployment at $API_URL"
echo ""

# Test 1: Health check
echo -n "1. Health check... "
HEALTH=$(curl -s "$API_URL/health")
if echo "$HEALTH" | grep -q '"status":"ok"'; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
  echo "Response: $HEALTH"
  exit 1
fi

# Test 2: Database health
echo -n "2. Database health... "
DB_HEALTH=$(curl -s "$API_URL/health/db")
if echo "$DB_HEALTH" | grep -q '"status":"ok"'; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
  echo "Response: $DB_HEALTH"
  exit 1
fi

# Test 3: ZiWei calculation
echo -n "3. ZiWei calculation... "
ZIWEI=$(curl -s -X POST "$API_URL/api/charts/calculate/ziwei" \
  -H "Content-Type: application/json" \
  -d '{"year":1990,"month":5,"day":15,"hour":14,"gender":"male"}')
if echo "$ZIWEI" | grep -q '"palaces"'; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
  echo "Response: $ZIWEI"
  exit 1
fi

# Test 4: Western calculation
echo -n "4. Western calculation... "
WESTERN=$(curl -s -X POST "$API_URL/api/charts/calculate/western" \
  -H "Content-Type: application/json" \
  -d '{"year":1990,"month":3,"day":25,"hour":12}')
if echo "$WESTERN" | grep -q '"sunSign"'; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
  echo "Response: $WESTERN"
  exit 1
fi

# Test 5: Security headers
echo -n "5. Security headers... "
HEADERS=$(curl -sI "$API_URL/health")
if echo "$HEADERS" | grep -q "x-frame-options" && \
   echo "$HEADERS" | grep -q "x-content-type-options"; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC}"
  echo "Missing security headers"
  exit 1
fi

# Test 6: Rate limiting
echo -n "6. Rate limiting... "
for i in {1..35}; do
  curl -s -X POST "$API_URL/api/charts/calculate/ziwei" \
    -H "Content-Type: application/json" \
    -d '{"year":1990,"month":5,"day":15,"hour":14,"gender":"male"}' \
    > /dev/null 2>&1
done
RATE_LIMIT=$(curl -s -X POST "$API_URL/api/charts/calculate/ziwei" \
  -H "Content-Type: application/json" \
  -d '{"year":1990,"month":5,"day":15,"hour":14,"gender":"male"}' \
  -w "%{http_code}" -o /dev/null)
if [ "$RATE_LIMIT" = "429" ]; then
  echo -e "${GREEN}✓${NC}"
else
  echo -e "${RED}✗${NC} (Expected 429, got $RATE_LIMIT)"
fi

echo ""
echo -e "${GREEN}✅ All checks passed!${NC}"
echo ""
echo "Deployment verified successfully at $(date)"
