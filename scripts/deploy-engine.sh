#!/bin/bash
# Deploy the Rust ft-engine Worker (service-binding companion to fortunet-api).
# Deterministic: worker-build → wrangler deploy. Requires OAuth (no API token).
set -euo pipefail
cd "$(dirname "$0")/../crates/worker"

echo "🔧 worker-build --release ..."
worker-build --release

echo "🚀 deploying fortunet-engine ..."
unset CLOUDFLARE_API_TOKEN 2>/dev/null || true
wrangler deploy

echo "✅ done — engine endpoint:"
echo "   https://fortunet-engine.yanggf.workers.dev/health"
