# 🚀 FortuneT V2 - Cloudflare Migration

**Status**: Phase 4 (Testing) - Week 16-18 ✅
**Timeline**: 24 weeks core + 8 weeks storytelling

---

## 📋 Quick Start

**Start Here** → [MASTER_PLAN.md](./MASTER_PLAN.md)
**Dev Guide** → [AGENTS.md](./AGENTS.md)

---

## 📊 Project Summary

| Metric | Current | After Migration |
|--------|---------|-----------------|
| **Monthly Cost** | $437-1,062 | $0-303 |
| **Performance (p95)** | 500-2000ms | <200ms |
| **Infrastructure** | Render + Vercel | Cloudflare (unified) |

---

## 🎯 Current Progress

### Phase 4 Complete ✅
- ✅ Unit tests (15/15 passing)
- ✅ Integration test suite ready
- ✅ Security headers middleware
- ✅ Rate limiting (auth/calc/AI)
- ✅ Load test script (k6)
- ✅ E2E test plan documented

### Phase 3 Complete ✅
- ✅ Vite + React + TypeScript frontend
- ✅ Passwordless auth (email-only, sessionId)
- ✅ Chart creation form (ZiWei/Western)
- ✅ AI interpretation UI
- ✅ Mobile responsive design
- ✅ Build: 179KB (57KB gzipped)

### Phase 2 Complete ✅
- ZiWei (紫微斗數) calculation engine
- Western Zodiac calculation engine
- AI interpretation with 3-provider failover:
  - iFlow GLM-4.6 (primary)
  - Groq kimi-k2-instruct-0905 (secondary)
  - Cerebras llama-3.3-70b (tertiary)
- Billing: 30-day free trial for new users

### Live API
```
https://fortunet-api.yanggf.workers.dev
```

---

## 📁 Documentation

| Document | Purpose |
|----------|---------|
| **[MASTER_PLAN.md](./MASTER_PLAN.md)** | ⭐ Migration plan & timeline |
| **[AGENTS.md](./AGENTS.md)** | ⭐ Dev guide & coding standards |
| **[FRONTEND_FIXES.md](./FRONTEND_FIXES.md)** | Frontend-backend contract fixes |
| [STORYTELLING_ROADMAP.md](./STORYTELLING_ROADMAP.md) | Phase 7 storytelling features |
| [docs/phase0/](./docs/phase0/) | Risk assessment (GO decision) |

---

## 📅 Timeline

```
Phase -1: System Audit        Week 0      ✅ COMPLETED
Phase 0:  Risk Assessment     Week 1-3    ✅ COMPLETED (GO)
Phase 1:  Foundation          Week 4-6    ✅ COMPLETED
Phase 2:  Core Features       Week 7-11   ✅ COMPLETED
Phase 3:  Frontend            Week 12-15  ✅ COMPLETED
Phase 4:  Integration/Test    Week 16-18  ✅ COMPLETED
Phase 5:  Pre-Migration       Week 19-20  ← NEXT
Phase 6:  Go-Live             Week 21
Stabilization                 Week 22-25
Phase 7:  Storytelling        Week 26-33
```

---

## 🛠️ Tech Stack

- **Frontend**: React + TypeScript + Vite + Tailwind
- **Backend**: Cloudflare Workers + Hono
- **Database**: D1 (SQLite)
- **Cache**: Durable Objects
- **Storage**: R2
- **AI**: iFlow / Groq / Cerebras (free tiers)
- **Payments**: Stripe (Week 10-11)

---

## 💻 Development

### Backend
```bash
cd backend
npm run dev          # Local dev (localhost:8787)
npm run typecheck    # TypeScript check

# Deploy (use OAuth)
unset CLOUDFLARE_API_TOKEN
npx wrangler deploy
```

### Frontend
```bash
cd frontend
npm run dev          # Local dev (localhost:5173)
npm run build        # Production build
```

---

**Last Updated**: 2025-12-04
