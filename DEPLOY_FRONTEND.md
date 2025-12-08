# Deploy Frontend to Cloudflare Pages

## ✅ Current Deployment

**Production**: https://fortunet.pages.dev
**Backend**: https://fortunet-api.yanggf.workers.dev

---

## 🚀 Deploy Updates

```bash
cd frontend
unset CLOUDFLARE_API_TOKEN
npm run deploy
```

This will:
1. Build frontend (with TypeScript check)
2. Save commit hash to `dist/.build-info`
3. Deploy to Cloudflare Pages

---

## 📜 Available Scripts

```bash
npm run deploy        # Build and deploy to Cloudflare Pages
npm run deploy:prod   # Deploy to production branch
npm run build         # Build only (no deploy)
npm run preview       # Preview build locally
npm run dev           # Local development
npm test              # Run tests
```

---

## 🔧 Backend Deployment

```bash
cd backend
unset CLOUDFLARE_API_TOKEN
npm run deploy
```

---

## ✅ CORS Configuration

Already configured! Backend allows:
- `*.pages.dev` domains
- `localhost` (for development)
- `*.workers.dev` domains

No additional setup needed.

---

## 🔍 Verify Deployment

```bash
./scripts/verify-deployment.sh
```

Checks:
- Health endpoints
- Database connectivity
- Chart calculations
- Security headers
- Rate limiting

---

## 🌐 Custom Domain (Optional)

To use `app.fortunet.com`:

1. Go to Cloudflare Dashboard → Pages → fortunet
2. Click "Custom domains"
3. Add your domain
4. Update DNS records as instructed

---

**Last Updated**: 2025-12-08


