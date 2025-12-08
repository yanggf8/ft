#!/bin/bash
# Deploy Frontend to Cloudflare Pages
# Run this script to deploy

set -e

echo "🚀 Deploying FortuneT Frontend to Cloudflare Pages"
echo ""

cd frontend

echo "📦 Building frontend..."
npm run build

echo ""
echo "☁️  Deploying to Cloudflare Pages..."
echo "   (This will open your browser for OAuth login)"
echo ""

unset CLOUDFLARE_API_TOKEN
npx wrangler pages deploy dist --project-name=fortunet

echo ""
echo "✅ Deployment complete!"
echo ""
echo "Your frontend is now live at:"
echo "   https://fortunet.pages.dev"
echo ""
echo "Backend API:"
echo "   https://fortunet-api.yanggf.workers.dev"
echo ""
echo "Next steps:"
echo "1. Visit https://fortunet.pages.dev"
echo "2. Test registration and login"
echo "3. Create a chart"
echo "4. Request AI interpretation"
echo ""
