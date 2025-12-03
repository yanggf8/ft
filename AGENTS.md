# 🤖 FortuneT V2 - Repository Guidelines

**Current Phase**: Phase 2 (Core Features) - Week 9 ✅
**Status**: AI integration complete, ready for Week 10-11 (Payments)

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

### Deployed Infrastructure

| Component | Status | URL/ID |
|-----------|--------|--------|
| **Workers API** | ✅ Live | https://fortunet-api.yanggf.workers.dev |
| **D1 Database** | ✅ Ready | `88d074eb-7331-402b-bc76-1ac3cb0588da` |
| **R2 Storage** | ✅ Ready | `fortunet-storage` |
| **Session DO** | ✅ Working | SQLite-backed |
| **CI/CD** | ✅ Configured | `.github/workflows/deploy.yml` |

### Phase 1 Exit Criteria

- [x] Repository structure created
- [x] Wrangler configured and working locally
- [x] D1 database created with schema
- [x] R2 bucket created
- [x] Session DO working
- [x] Health endpoint responding
- [x] CI/CD pipeline configured

### Implemented Endpoints

```bash
# Health
GET  /health
GET  /health/db

# Auth
POST /api/auth/register
POST /api/auth/login
POST /api/auth/logout

# Users
GET  /api/users/me
PUT  /api/users/me

# Charts
GET  /api/charts
POST /api/charts
GET  /api/charts/:id
PUT  /api/charts/:id
DELETE /api/charts/:id
```

---

## 📁 Repository Structure

```
fortune-teller-v2/
├── MASTER_PLAN.md              # ⭐ Consolidated migration plan
├── README.md                   # Project overview
├── AGENTS.md                   # This file
│
├── .github/
│   └── workflows/
│       └── deploy.yml          # ✅ CI/CD pipeline
│
├── backend/                    # ✅ Cloudflare Workers
│   ├── src/
│   │   ├── index.ts            # Main entry (Hono)
│   │   ├── durable-objects/
│   │   │   └── session-do.ts   # Session management
│   │   ├── middleware/
│   │   │   ├── auth.ts         # Auth middleware
│   │   │   └── validate.ts     # Zod validation
│   │   └── routes/
│   │       ├── auth.ts
│   │       ├── users.ts
│   │       └── charts.ts
│   ├── scripts/
│   │   └── schema.sql          # D1 schema
│   ├── wrangler.toml           # Cloudflare config
│   └── package.json
│
├── docs/
│   ├── audit/                  # ✅ Phase -1 complete
│   └── phase0/                 # ✅ Phase 0 complete (GO)
│
└── phase0-tests/               # Phase 0 validation
```

---

## 📅 Timeline

```
Phase -1: System Audit        Week 0      ✅ COMPLETED
Phase 0:  Risk Assessment     Week 1-3    ✅ COMPLETED (GO)
Phase 1:  Foundation          Week 4-6    ✅ COMPLETED
Phase 2:  Core Features       Week 7-11   ← NEXT
Phase 3:  Frontend            Week 12-15
Phase 4:  Integration/Test    Week 16-18
Phase 5:  Pre-Migration       Week 19-20
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
- [x] Groq API integration
- [x] Interpretation generation
- [x] Rate limiting (10 req/min/IP)

### Week 10-11: Payments ← NEXT
- [ ] Stripe integration
- [ ] Subscription management
- [ ] Webhook handling

---

## 💻 Development Commands

### Backend
```bash
cd backend
npm run dev                   # Local dev (localhost:8787)
npm run deploy                # Deploy to Cloudflare
npm run typecheck             # TypeScript check
npm run db:init               # Apply schema to remote D1
npm run db:init:local         # Apply schema to local D1
```

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

---

## 🎯 Zero-Cost Strategy

**Current Usage** (Phase 1):
- Workers: ~10 requests/day (testing)
- D1: 0.09 MB / 5 GB limit
- DO: Minimal (session tests)
- R2: 0 MB / 10 GB limit

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

**Last Updated**: 2025-12-03
**API URL**: https://fortunet-api.yanggf.workers.dev
