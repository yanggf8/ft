# 🤖 FortuneT V2 - Repository Guidelines

**Current Phase**: Phase 5 (Pre-Migration) - Week 20 ✅
**Status**: Production live, beta testing materials ready

**Live URLs**:
- Frontend: https://fortunet.pages.dev
- Backend: https://fortunet-api.yanggf.workers.dev

---

## ⚠️ Critical Rules

### Wrangler Commands - Always Use OAuth
**For ANY wrangler command, always unset API token first:**
```bash
unset CLOUDFLARE_API_TOKEN && npx wrangler [command]
```

**Examples:**
```bash
# Deploy
unset CLOUDFLARE_API_TOKEN && npx wrangler deploy

# Check deployments
unset CLOUDFLARE_API_TOKEN && npx wrangler deployments list

# Manage secrets
unset CLOUDFLARE_API_TOKEN && npx wrangler secret put IFLOW_API_KEY

# D1 commands
unset CLOUDFLARE_API_TOKEN && npx wrangler d1 execute fortunet-db --remote --command "SELECT COUNT(*) FROM users"

# Pages deploy
unset CLOUDFLARE_API_TOKEN && npx wrangler pages deploy dist
```

**Why**: API tokens have permission issues. OAuth provides full access to all Cloudflare features.

**Rule**: Combine `unset CLOUDFLARE_API_TOKEN &&` with every `npx wrangler` command.

---

## 📋 Project Overview

FortuneT V2 is a Cloudflare-native migration with AI-powered storytelling features.

**Primary Document**: [MASTER_PLAN.md](./MASTER_PLAN.md) - Start here for all planning.

---

## 🚀 Phase 2 Progress

### Week 7-8: Calculation Engines ✅

| Engine | Status | Endpoint |
|--------|--------|----------|
| **ZiWei (紫微斗數)** | ✅ Complete | `POST /api/charts/calculate/ziwei` |
| **Western Zodiac** | ✅ Complete | `POST /api/charts/calculate/western` |

#### ZiWei Features
- Solar-to-lunar conversion (1900-2100)
- Four pillars calculation
- Life palace & body palace
- Five element determination
- 14 main stars placement
- Auxiliary stars (文昌、文曲、左輔、右弼、祿存、擎羊、陀羅)

#### Western Features
- Sun sign calculation
- Approximate moon sign
- Basic planetary positions

### Week 9: AI Integration ✅

#### Provider Strategy (3-tier failover)

| Priority | Provider | Model | 特點 |
|----------|----------|-------|------|
| Primary | iFlow | GLM-4.6 | 敘事最佳、溫柔專業 |
| Secondary | Groq | kimi-k2-instruct-0905 | 快速穩定、敘事柔順 |
| Tertiary | Cerebras | llama-3.3-70b | 冷備援、成本低 |

#### AI Mutex DO Features
- Serialized requests (1 concurrent)
- Auto failover on error
- exresource tracking per provider/day:
  - `requests` - 請求數
  - `tokens` - token 用量
  - `errors` - 錯誤數
  - `lastError` - 最後錯誤 (time, code, message)
  - `latencySum` - 延遲總和
  - `failovers` - failover 次數

#### Chart & AI Endpoints (Birth-Data Centric)
```bash
PUT  /api/users/me/birth        # Save birth data to user profile (invalidates cache)
GET  /api/charts/:type          # Auto-calculate chart from stored birth data (cached)
POST /api/charts/:type/interpret # AI interpretation with failover (cached)
GET  /api/charts                # List user's cached interpretations
```
`:type` is `ziwei` or `western`. Birth data is stored once on the user profile; charts are derived from it and cached per `(user_id, divination_type)` keyed by `birth_data_hash`. Updating birth data deletes all cached interpretations for that user.

### Deployed Infrastructure

| Component | Status | URL/ID |
|-----------|--------|--------|
| **Workers API** | ✅ Live | https://fortunet-api.yanggf.workers.dev |
| **D1 Database** | ✅ Ready | `88d074eb-7331-402b-bc76-1ac3cb0588da` |
| **R2 Storage** | ✅ Ready | `fortunet-storage` |
| **Session DO** | ✅ Working | DO storage (key-value) |
| **AI Mutex DO** | ✅ Working | DO storage, 3-provider failover |
| **CI/CD** | ✅ Configured | `.github/workflows/deploy.yml` |

### Cloudflare Secrets

| Secret | Purpose |
|--------|---------|
| `IFLOW_API_KEY` | Primary AI provider |
| `GROQ_API_KEY` | Secondary AI provider |
| `CEREBRAS_API_KEY` | Tertiary AI provider |

---

## 📁 Repository Structure

```
fortune-teller-v2/
├── MASTER_PLAN.md              # ⭐ Consolidated migration plan
├── README.md                   # Project overview
├── AGENTS.md                   # This file
├── FRONTEND_FIXES.md           # Frontend-backend contract fixes
├── STORYTELLING_ROADMAP.md     # Phase 7 storytelling features
│
├── .github/
│   └── workflows/
│       └── deploy.yml          # ✅ CI/CD pipeline
│
├── backend/                    # ✅ Cloudflare Workers
│   ├── src/
│   │   ├── index.ts            # Main entry (Hono)
│   │   ├── durable-objects/
│   │   │   ├── session-do.ts   # Session management
│   │   │   └── ai-mutex-do.ts  # AI failover & tracking
│   │   ├── services/
│   │   │   ├── billing.ts      # Trial & subscription logic
│   │   │   ├── ai/             # 3-provider failover (iFlow/Groq/Cerebras)
│   │   │   ├── ziwei/          # ZiWei calculation
│   │   │   └── western/        # Western calculation
│   │   ├── middleware/
│   │   │   ├── auth.ts         # Auth middleware
│   │   │   ├── security.ts     # Security headers
│   │   │   ├── cache.ts        # HTTP cache headers + ETag helpers
│   │   │   └── edgeCache.ts    # Cloudflare Cache API (health endpoint only)
│   │   └── routes/
│   │       ├── auth.ts         # register/login/logout
│   │       ├── users.ts        # /me, PUT /me/birth
│   │       └── charts.ts       # GET /:type, POST /:type/interpret
│   ├── scripts/
│   │   ├── schema.sql          # D1 schema (birth-data centric)
│   │   └── migrate-v2.sql      # v1→v2 migration
│   ├── wrangler.toml           # Cloudflare config
│   └── package.json
│
├── frontend/                   # ✅ React + Vite
│   ├── src/
│   │   ├── components/
│   │   │   ├── Layout.tsx
│   │   │   ├── ProtectedRoute.tsx
│   │   │   └── BirthDataForm.tsx  # Birth data entry (on profile)
│   │   ├── pages/
│   │   │   ├── HomePage.tsx
│   │   │   ├── LoginPage.tsx
│   │   │   ├── ProfilePage.tsx
│   │   │   └── DivinationPage.tsx # /divination/:type
│   │   ├── contexts/
│   │   │   └── AuthContext.tsx
│   │   ├── lib/
│   │   │   └── api.ts
│   │   ├── types/
│   │   │   └── index.ts
│   │   ├── App.tsx
│   │   ├── main.tsx
│   │   └── index.css
│   ├── index.html
│   ├── vite.config.ts
│   └── package.json
│
└── docs/
    ├── audit/                  # ✅ Phase -1 complete
    └── phase0/                 # ✅ Phase 0 complete (GO)
```

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

## 🎯 Phase 2 Tasks (Week 7-11)

### Week 7-8: Chart Calculation Engines ✅
- [x] Port ZiWei calculation engine
- [x] Port Western zodiac engine
- [x] Solar-to-lunar conversion
- [x] Main & auxiliary star placement

### Week 9: AI Integration ✅
- [x] 3-provider failover: iFlow → Groq → Cerebras
- [x] iFlow GLM-4.6 (primary, best narrative)
- [x] Groq kimi-k2-instruct-0905 (secondary, fast)
- [x] Cerebras llama-3.3-70b (tertiary, stable)
- [x] AI Mutex DO (serialized requests, failover)
- [x] exresource tracking (usage, errors, latency, failovers)

### Week 10-11: Billing ✅
- [x] Trial period (30 days free)
- [x] `trial_ends_at` field in users table
- [x] `billing.ts` service (checkUserAccess)
- [x] `/api/users/me` returns billing status
- [ ] Native app IAP integration (planned - Apple / Google store billing)

### Week 12-15: Frontend ✅
- [x] Vite + React + TypeScript setup
- [x] Passwordless auth (email-only, sessionId)
- [x] API client with session management
- [x] Auth context & protected routes
- [x] Pages: Home, Login, Profile, Chart
- [x] Chart creation form (ZiWei/Western)
- [x] AI interpretation UI
- [x] Mobile responsive design
- [x] Build: 179KB (57KB gzipped)

---

## 💻 Development Commands

### Backend
```bash
cd backend
npm run dev                   # Local dev (localhost:8787)
npm run typecheck             # TypeScript check
npm test                      # Safe: Does nothing (prevents accidental API calls)
npm run test:integration      # Run integration tests (calls production API)

# Deploy (IMPORTANT: Always use OAuth, not API token)
unset CLOUDFLARE_API_TOKEN    # Must unset token first
npx wrangler deploy           # Will prompt for OAuth login

# Secrets management
npx wrangler secret put IFLOW_API_KEY
npx wrangler secret put GROQ_API_KEY
npx wrangler secret put CEREBRAS_API_KEY

# Database
npm run db:init               # Apply schema to remote D1
npm run db:init:local         # Apply schema to local D1
```

**⚠️ Deployment Rule**: Always `unset CLOUDFLARE_API_TOKEN` before deploying. Wrangler should use OAuth authentication, not API tokens, to avoid permission issues.

### CI/CD Setup (GitHub)
Required secrets:
- `CLOUDFLARE_API_TOKEN` - API token with Workers/D1/R2 permissions
- `CLOUDFLARE_ACCOUNT_ID` - Your Cloudflare account ID

---

## 📝 Coding Standards

- **TypeScript**: Strict mode enabled
- **Files**: `kebab-case.ts`
- **Variables**: `camelCase`
- **Constants**: `UPPER_SNAKE_CASE`
- **Indentation**: 2 spaces
- **Quotes**: Single quotes
- **Semicolons**: Required

### Testing Philosophy
- **Integration tests only** - No unit tests, no mocks, no placeholders
- **Real APIs** - Tests must call actual deployed services
- **Real data** - No fake data or stubs
- **Real environment** - Test against production or staging only
- **Rule**: If you can't test it with real integration, don't write the test

---

**Current Usage** (Phase 2):
- Workers: ~10 requests/day (testing)
- D1: 0.09 MB / 5 GB limit
- DO: Minimal (session + AI mutex)
- R2: 0 MB / 10 GB limit
- AI: Free tiers (iFlow/Groq/Cerebras)

**Free Tier Limits**:
- Workers: 100K requests/day
- D1: 5GB storage, 25M reads/day
- DO: 400K requests/day
- R2: 10GB storage

---

## 📚 Key Documents

| Document | Status |
|----------|--------|
| [MASTER_PLAN.md](./MASTER_PLAN.md) | ⭐ Primary reference |
| [docs/phase0/d1_compatibility_report.md](./docs/phase0/d1_compatibility_report.md) | ✅ 100% Pass |
| [docs/phase0/go_no_go_decision.md](./docs/phase0/go_no_go_decision.md) | ✅ GO |

---

**Last Updated**: 2025-12-04
**API URL**: https://fortunet-api.yanggf.workers.dev
**Frontend Build**: 179KB (57KB gzipped)
