# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FortuneT V2 is an AI-powered Chinese/Western astrology platform. Backend runs on Cloudflare Workers (Hono), frontend is React+Vite, deployed to Cloudflare Pages.

- **Production Frontend**: https://fortunet.pages.dev
- **Production API**: https://fortunet-api.yanggf.workers.dev

## Commands

### Backend (from `backend/`)
```bash
npm run dev                      # Local dev server (localhost:8787)
npm run typecheck                # TypeScript type checking
npm run test:integration         # Integration tests (calls real APIs, needs RUN_INTEGRATION=true)
npm run test:integration:staging # Integration tests against staging
npm run db:init                  # Apply schema.sql to remote D1
npm run db:init:local            # Apply schema.sql to local D1
npm run deploy                   # Deploy (unsets CLOUDFLARE_API_TOKEN, then wrangler deploy)
```

### Frontend (from `frontend/`)
```bash
npm run dev              # Local dev server (localhost:5173)
npm test                 # Vitest tests
npm run build            # TypeScript check + Vite build (REQUIRED before deploy)
npm run deploy           # build + unset token + deploy to Pages
npm run deploy:prod      # Same, deploying to the main branch
```

### Deployment (always unset API token first)
```bash
cd backend  && npm run deploy   # Builds nothing; unsets token then wrangler deploy
cd frontend && npm run deploy   # Builds, unsets token, then wrangler pages deploy
```

The `deploy` / `deploy:prod` scripts already wrap `unset CLOUDFLARE_API_TOKEN` — prefer them. `deploy:unsafe` variants skip the unset (do not use unless you know why).

**Critical**: OAuth auth is required; API tokens have permission issues. If running `wrangler` directly, always prefix with `unset CLOUDFLARE_API_TOKEN &&`.

### Helper Scripts (from project root)
```bash
./scripts/deploy-backend.sh     # Typecheck + deploy backend + health check
./scripts/deploy-frontend.sh    # Build + deploy frontend with checks
./scripts/verify-deployment.sh  # Verify production services are healthy
```

## Architecture

### Backend (`backend/src/`)
- **Entry**: `index.ts` — Hono app with CORS, security headers, error handling. Exports `SessionDO` and `AIMutexDO` classes.
- **Routes**: `routes/auth.ts`, `routes/users.ts`, `routes/charts.ts` — API endpoints under `/api/`
- **Auth**: Passwordless email login. Sessions stored in `SessionDO` (Durable Object). Auth middleware validates `Bearer <sessionId>` header via DO lookup.
- **Durable Objects**:
  - `SessionDO` — Session storage (key-value in DO SQLite). Sessions expire after 7 days; `refreshSession()` extends TTL on use.
  - `AIMutexDO` — Serializes AI requests (1 concurrent via in-memory queue), manages 3-provider failover, tracks per-provider/per-day "exresource" metrics (requests, tokens, errors, latency, failovers) in DO storage.
- **AI Providers**: 3-tier failover defined by the `PROVIDERS` array in `durable-objects/ai-mutex-do.ts`: iFlow `GLM-4.6` (rpm 1) → Groq `moonshotai/kimi-k2-instruct-0905` (rpm 30, rpd 14400) → Cerebras `llama-3.3-70b` (rpm 30, rpd 14400). A provider is skipped when its API key is missing, its daily quota (rpd) is exhausted, or its per-minute limit (rpm) is hit; on error the DO fails over to the next. All providers failing returns 503 `ALL_PROVIDERS_FAILED`.
  - **Implementation detail**: Only iFlow has a dedicated adapter class (`services/ai/iflow.ts`). Groq and Cerebras are called inline in `ai-mutex-do.ts:callProvider` via the OpenAI-compatible `/chat/completions` endpoint. `services/ai/cerebras.ts` exists but is **not** used by the DO. Shared prompts live in `services/ai/prompts.ts` (`getSystemPrompt`, `buildPrompt`).
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
- Cached `chart_data` also embeds `engineVersion` from `services/engine-version.ts`; when it mismatches `ENGINE_VERSION`, `GET /api/charts/:type` recalculates and clears the stale `ai_interpretation` — bump that constant whenever any calculation algorithm changes
- `scripts/schema.sql` also defines `subscriptions`, `usage_tracking`, and `ai_quota` tables that are **not yet used** by any code (provisioned for planned billing/quota features). Only `users` and `interpretations` are live.

### API Endpoints
- `GET /health` — liveness (status, timestamp, `ENVIRONMENT` var); `GET /health/db` — checks D1 connectivity. Used by deploy/verify scripts.
- `POST /api/auth/register | /login | /logout` — passwordless email auth
- `GET /api/users/me` — profile + billing + `hasBirthData` flag
- `PUT /api/users/me/birth` — save birth data (invalidates interpretations cache)
- `PUT /api/users/me` — update name/avatar
- `GET /api/charts/:type` — `:type` is `ziwei` or `western`. Auto-calculates from stored birth data, returns cached if available. Requires birth data on profile (400 `NO_BIRTH_DATA` otherwise).
- `POST /api/charts/:type/interpret` — runs AI interpretation for the cached chart. Requires `GET /api/charts/:type` to have been called first (404 otherwise).
- `GET /api/charts` — list all of user's cached interpretations

### Cross-Cutting Middleware (`backend/src/middleware/`)
- **Cache** (`cache.ts:setCacheHeaders`) — sets `Cache-Control` + `Vary: Authorization`. Per-endpoint TTLs: `GET /me` 300s, `PUT /me/birth` 0s (no-store), `GET /charts/:type` 3600s, `POST /charts/:type/interpret` 86400s + `must-revalidate` — all `private`. ETag-based 304 responses (`createETag(hash, timestamp)`) are implemented only on the two `/charts/:type` routes.
- **Auth** (`auth.ts`) — validates `Bearer <sessionId>` against `SessionDO`. `optionalAuth` exists but is currently unused.
- **Security** (`security.ts`) — sets `X-Content-Type-Options`, `X-Frame-Options`, `X-XSS-Protection`, `Referrer-Policy`, `Permissions-Policy`, CSP.
- **Rate limiting** — in-route, per-IP, 10 req/min on auth endpoints (`routes/auth.ts`) and AI interpretation (`routes/charts.ts`); returns 429 when exceeded.
- `index.ts` adds CORS (allows localhost, `*.pages.dev`, `*.workers.dev` with credentials) and an `x-request-id` header on every response.
- `middleware/validate.ts` holds Zod schemas but is **not** wired into routes (routes validate manually).

### Cloudflare Bindings (wrangler.toml)
- `DB` → D1 database `fortunet-db`
- `SESSION_DO` → SessionDO class
- `AI_MUTEX` → AIMutexDO class
- `STORAGE` → R2 bucket
- Secrets: `IFLOW_API_KEY`, `GROQ_API_KEY`, `CEREBRAS_API_KEY`

## Coding Standards

- TypeScript strict mode, 2-space indent, single quotes, semicolons required
- File names: `kebab-case.ts`, variables: `camelCase`, constants: `UPPER_SNAKE_CASE`
- **Integration tests only** — no unit tests, no mocks. Tests must call real deployed services. See `.testing-rules` for the full testing philosophy.
- No database constraints — design for flexibility
- No feature flags
- Frontend must be built before every deploy

## Git & CI

- **Pre-push hook** (`.githooks/pre-push`, installed via `bash .githooks/install.sh`): verifies GitHub push access, then runs **both** backend and frontend `typecheck`, and warns if the branch is behind `origin/main`. A push is blocked on type errors.
- **CI** (`.github/workflows/deploy.yml`): on push/PR to `main` runs **backend typecheck only** (no frontend typecheck, no integration tests, no staging env); deploys to production only on push to `main`. The pre-push hook — not CI — is what guards frontend types, so do not rely on CI to catch them.
- `frontend npm run build` writes the current commit SHA to `dist/.build-info`.
