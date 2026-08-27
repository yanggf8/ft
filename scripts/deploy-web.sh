#!/bin/bash
# Build the Leptos frontend and deploy to Cloudflare Pages.
# Requires OAuth (no CLOUDFLARE_API_TOKEN); must be run after build-web.sh.
set -euo pipefail
cd "$(dirname "$0")/../crates/web"

echo "🔧 building frontend ..."
./scripts/build-web.sh

echo "🚀 deploying to Cloudflare Pages ..."
unset CLOUDFLARE_API_TOKEN 2>/dev/null || true
wrangler pages deploy dist --project-name=fortunet --branch=main

echo "✅ done — https://fortunet.pages.dev"
