# Deploy Frontend to Cloudflare Pages

## Quick Deploy

```bash
cd frontend
npm run build
unset CLOUDFLARE_API_TOKEN
npx wrangler pages deploy dist --project-name=fortunet
```

**Note**: This will prompt for OAuth login in your browser.

## Expected Result

Your frontend will be available at:
```
https://fortunet.pages.dev
```

## CORS Already Configured ✅

Backend already allows requests from:
- `*.pages.dev` domains
- `localhost` (for development)
- `*.workers.dev` domains

No additional configuration needed!

## After Deployment

1. Visit `https://fortunet.pages.dev`
2. Test registration and login
3. Create a chart
4. Request AI interpretation

## Custom Domain (Optional)

To use a custom domain like `app.fortunet.com`:

1. Go to Cloudflare Dashboard → Pages → fortunet
2. Click "Custom domains"
3. Add your domain
4. Update DNS records as instructed

---

**Backend API**: https://fortunet-api.yanggf.workers.dev
**Frontend**: https://fortunet.pages.dev (after deployment)
