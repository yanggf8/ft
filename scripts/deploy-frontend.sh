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
    echo "$CURRENT_COMMIT" > dist/.build-info
  fi
else
  echo -e "${YELLOW}⚠ No build info found, rebuilding...${NC}"
  npm run build
  echo "$CURRENT_COMMIT" > dist/.build-info
fi

echo ""
echo "📦 Build verified and ready"
echo ""
echo "☁️  Deploying to Cloudflare Pages..."
echo "   (This will prompt for OAuth login)"
echo ""

unset CLOUDFLARE_API_TOKEN
npx wrangler pages deploy dist --project-name=fortunet

echo ""
echo -e "${GREEN}✅ Deployment complete!${NC}"
echo ""
echo "Frontend: https://fortunet.pages.dev"
echo "Backend:  https://fortunet-api.yanggf.workers.dev"
echo ""
