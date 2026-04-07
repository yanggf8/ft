# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FortuneT V2 is an AI-powered Chinese/Western astrology platform. Backend runs on Cloudflare Workers (Hono), frontend is React+Vite, deployed to Cloudflare Pages.

- **Production Frontend**: https://fortunet.pages.dev
- **Production API**: https://fortunet-api.yanggf.workers.dev

## Commands

### Backend (from `backend/`)
```bash
npm run dev              # Local dev server (localhost:8787)
npm run typecheck        # TypeScript type checking
npm run test:integration # Integration tests (calls real APIs, needs RUN_INTEGRATION=true)
npm run db:init          # Apply schema.sql to remote D1
npm run db:init:local    # Apply schema.sql to local D1
```

### Frontend (from `frontend/`)
```bash
npm run dev              # Local dev server (localhost:5173)
npm test                 # Vitest tests
npm run build            # TypeScript check + Vite build (REQUIRED before deploy)
```

### Deployment (always unset API token first)
```bash
# Backend
cd backend && unset CLOUDFLARE_API_TOKEN && npx wrangler deploy

# Frontend (build is included in deploy script)
cd frontend && unset CLOUDFLARE_API_TOKEN && npm run deploy
```

**Critical**: Always prefix wrangler commands with `unset CLOUDFLARE_API_TOKEN &&` to force OAuth authentication. API tokens have permission issues.

## Architecture

### Backend (`backend/src/`)
- **Entry**: `index.ts` — Hono app with CORS, security headers, error handling. Exports `SessionDO` and `AIMutexDO` classes.
- **Routes**: `routes/auth.ts`, `routes/users.ts`, `routes/charts.ts` — API endpoints under `/api/`
- **Auth**: Passwordless email login. Sessions stored in `SessionDO` (Durable Object). Auth middleware validates `Bearer <sessionId>` header via DO lookup.
- **Durable Objects**:
  - `SessionDO` — Session storage (key-value in DO SQLite)
  - `AIMutexDO` — Serializes AI requests (1 concurrent), manages 3-provider failover, tracks usage metrics
- **AI Providers** (`services/ai/`): 3-tier failover: iFlow GLM-4.6 → Groq Kimi-K2 → Cerebras Llama-3.3-70b
- **Calculation Engines**: `services/ziwei/` (紫微斗數) and `services/western/` (Western zodiac)
- **Billing** (`services/billing.ts`): 30-day free trial, `checkUserAccess()`. Native IAP planned (no web checkout).
- **Database**: Cloudflare D1 (SQLite). Schema in `scripts/schema.sql`.
- **Storage**: R2 bucket `fortunet-storage`

### Frontend (`frontend/src/`)
- **Routing**: React Router v6 in `App.tsx`. Protected routes wrap with `ProtectedRoute`.
- **Auth**: `contexts/AuthContext.tsx` manages session state, stores sessionId for API calls.
- **API Client**: `lib/api.ts` — HTTP client with session header injection.
- **Pages**: HomePage, LoginPage, ProfilePage, DivinationPage (ZiWei/Western)

### Data Model: Birth-Data Centric

Birth data lives on the **user profile** (not per-request). Charts are derived from the user's stored birth data and cached per divination type.

- `users` table holds birth fields: `birth_year/month/day/hour/minute`, `gender`, `timezone`, `latitude`, `longitude`, `birth_data_hash`
- `interpretations` table caches one chart per `(user_id, divination_type)`, keyed by `birth_data_hash`
- When a user updates birth data via `PUT /api/users/me/birth`, all their cached interpretations are deleted
- `birth_data_hash` is computed in `routes/users.ts:computeBirthHash` and used as cache invalidation key

### API Endpoints
- `POST /api/auth/register | /login | /logout` — passwordless email auth
- `GET /api/users/me` — profile + billing + `hasBirthData` flag
- `PUT /api/users/me/birth` — save birth data (invalidates interpretations cache)
- `PUT /api/users/me` — update name/avatar
- `GET /api/charts/:type` — `:type` is `ziwei` or `western`. Auto-calculates from stored birth data, returns cached if available. Requires birth data on profile (400 `NO_BIRTH_DATA` otherwise).
- `POST /api/charts/:type/interpret` — runs AI interpretation for the cached chart. Requires `GET /api/charts/:type` to have been called first (404 otherwise).
- `GET /api/charts` — list all of user's cached interpretations

### Cache Headers Middleware
- `middleware/cache.ts:setCacheHeaders` — sets `Cache-Control` + `Vary: Authorization`. Used on `/me`, `/me/birth`, `/charts/:type`, `/charts/:type/interpret`. Routes manage their own ETags via `createETag(hash, timestamp)` for 304 responses.
- `middleware/edgeCache.ts:edgeCache` — Cloudflare Cache API wrapper, used only on `/health` (do not apply to authenticated routes — per-token cache key would grow unbounded).

### Cloudflare Bindings (wrangler.toml)
- `DB` → D1 database `fortunet-db`
- `SESSION_DO` → SessionDO class
- `AI_MUTEX` → AIMutexDO class
- `STORAGE` → R2 bucket
- Secrets: `IFLOW_API_KEY`, `GROQ_API_KEY`, `CEREBRAS_API_KEY`

## Coding Standards

- TypeScript strict mode, 2-space indent, single quotes, semicolons required
- File names: `kebab-case.ts`, variables: `camelCase`, constants: `UPPER_SNAKE_CASE`
- **Integration tests only** — no unit tests, no mocks. Tests must call real deployed services.
- No database constraints — design for flexibility
- No feature flags
- Frontend must be built before every deploy
