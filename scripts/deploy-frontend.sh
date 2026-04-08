#!/bin/bash
# Frontend Deployment Script with Build Verification
# Ensures dist/ matches current code before deploying

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "🚀 FortuneT Frontend Deployment"
echo ""

cd "$(dirname "$0")/../frontend"

# Check if dist exists
if [ ! -d "dist" ]; then
  echo -e "${YELLOW}⚠ dist/ not found, building...${NC}"
  npm run build
fi

# Get last commit hash
CURRENT_COMMIT=$(git rev-parse HEAD)

# Check if dist was built from current commit
if [ -f "dist/.build-info" ]; then
  DIST_COMMIT=$(cat dist/.build-info)
  if [ "$DIST_COMMIT" = "$CURRENT_COMMIT" ]; then
    echo -e "${GREEN}✓ dist/ is up to date (commit: ${CURRENT_COMMIT:0:7})${NC}"
  else
    echo -e "${YELLOW}⚠ dist/ is outdated${NC}"
    echo "  Current commit: ${CURRENT_COMMIT:0:7}"
    echo "  Built from:     ${DIST_COMMIT:0:7}"
    echo ""
    echo "Rebuilding..."
    npm run build
  fi
else
  echo -e "${YELLOW}⚠ No build info found, rebuilding...${NC}"
  npm run build
fi

echo ""
echo "📦 Build verified and ready"
echo ""
echo "☁️  Deploying to Cloudflare Pages..."
echo ""

# Use wrangler with OAuth (unset API token)
unset CLOUDFLARE_API_TOKEN

# Deploy with wrangler
npx wrangler pages deploy dist --project-name=fortunet

EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
  echo ""
  echo "⏳ Waiting for propagation..."
  sleep 5

  echo "🏥 Smoke test..."
  HTTP_CODE=$(curl -sf -o /dev/null -w "%{http_code}" https://fortunet.pages.dev 2>/dev/null || echo "000")
  if [ "$HTTP_CODE" = "200" ]; then
    echo -e "${GREEN}✅ Deployment complete and site is live!${NC}"
  else
    echo -e "${YELLOW}⚠ Deployed but site returned HTTP $HTTP_CODE (may need a few more seconds)${NC}"
  fi
  echo ""
  echo "Frontend: https://fortunet.pages.dev"
  echo "Backend:  https://fortunet-api.yanggf.workers.dev"
  echo ""
else
  echo ""
  echo -e "${RED}❌ Deployment failed with exit code $EXIT_CODE${NC}"
  echo ""
  exit $EXIT_CODE
fi

