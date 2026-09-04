# 🤖 FortuneT V2 - Repository Guidelines

**Current Phase**: Phase 6 Go-Live — Beta Testing ✅
**Status**: Rust workspace live (Leptos CSR + workers-rs), production on Cloudflare

**Live URLs**:
- Frontend: https://fortunet.pages.dev
- Backend API: https://fortunet-api.yanggf.workers.dev
- Engine Worker: https://fortunet-engine.yanggf.workers.dev

---

## ⚠️ Critical Rules

### Wrangler Commands - Always Use OAuth
**For ANY wrangler command, always unset API token first:**
```bash
unset CLOUDFLARE_API_TOKEN && wrangler [command]
# then: wrangler whoami  # confirm OAuth
```

**Examples:**
```bash
unset CLOUDFLARE_API_TOKEN && wrangler deploy
unset CLOUDFLARE_API_TOKEN && wrangler deployments list
unset CLOUDFLARE_API_TOKEN && wrangler secret put IFLOW_API_KEY
unset CLOUDFLARE_API_TOKEN && wrangler pages deploy dist --project-name=fortunet
turso db shell fortunet "SELECT COUNT(*) FROM users"
```

**Why**: API tokens have permission issues. OAuth provides full access. One-person project, all work lands directly on `main`, deployment is manual (no token in CI).

**Rule**: Combine `unset CLOUDFLARE_API_TOKEN &&` with every `wrangler` command. Check `wrangler whoami` first.

### Turso / gwebcdb credentials
Ask `armo` (`armo`, `armo mint` …) — two token systems coexist (`gwebcdb-mint` = group token for every DB, CLI resolve = per-DB token). `401 does not have the permissions` = per-DB token, not a wrong flag. Do not use Cloudflare D1.

### Build / Deploy never auto-start
Ask before starting servers, `cargo install trunk` / Leptos first-build can hang the machine, and `cargo` must be wrapped with timeout. Never start servers automatically.

---

## 📋 Project Overview

FortuneT V2 is a **Rust** platform: Leptos CSR frontend + Cloudflare Workers (`workers-rs`) + isolated engine Worker + Turso.

**Primary Documents**:
- [CLAUDE.md](./CLAUDE.md) — workspace, routes, DOs, services, engine versions (authoritative)
- [MASTER_PLAN.md](./MASTER_PLAN.md) — migration timeline (historical, superseded by 98d3521)
- [README.md](./README.md) — live URLs, quick start, tech stack

---

## 🚀 Progress

### Engines ✅

| Engine | Status | Notes |
|--------|--------|-------|
| **ZiWei (紫微斗數)** | ✅ Complete | `x-iztro` via `crates/worker` → `crates/domain/ziwei`, 4×4 palace grid, Wuxing + BaZi |
| **Western Zodiac** | ✅ Complete | `solar-ephemeris` Moon + `vsop87` planets, via `crates/domain/western` |

**Generation Tags** (2026-09): `1940s–2010s` selectable as birth attribute (default from `birth_year`), stored as JSON array `users.generation_tags`, embedded into story prompt `【世代語境】`, 1930s/2020s added, prompt thickened to ~1072 chars (fourPillars, majorLimits, isLeap, brightness/sihua, ascendant/houses).

**F5 本週預測** (2026-09): `predictions` / `situation_checks` / `prediction_feedback` / `prediction_generations` 四表 + 4 端點已上線。cycle 級凍結（`prediction_generations` 一週一 profile 快照，空週也凍結）、forecast 遮罩（第 1 段收齊才吐全文）、F6 兩段式（第 2 段僅 occurred 後、一次性、situation 鎖定）、D2-A 全負面週例外；web `我的命格` PredictionsCard（§5.4.1 措辭回饋）。

### AI Integration ✅

| Priority | Provider | Model | 特點 |
|----------|----------|-------|------|
| Primary | iFlow | GLM-4.6 | 敘事最佳、溫柔專業 |
| Secondary | Groq | kimi-k2-instruct-0905 | 快速穩定、敘事柔順 |
| Tertiary | Cerebras | llama-3.3-70b | 冷備援、成本低 |

**AIMutexDO**: serialized (1 concurrent), `MAX_QUEUE_DEPTH=8`, `MAX_QUEUE_WAIT_MS=60000`, rpm/rpd limits, exresource tracking (`requests/tokens/errors/lastError/latencySum/failovers`), 45s provider timeout, offline stub when all fail. `SessionDO` key `session`, 7-day TTL.

### Chart & AI Endpoints (Birth-Data Centric)
```bash
PUT  /api/users/me/birth        # Save birth data + generation_tags (invalidates interpretations)
GET  /api/charts/:type          # Auto-calculate chart from stored birth (cached, birth_data_hash gated)
POST /api/charts/:type/interpret # AI interpretation (story cache, fromCache, 409 RECALC)
GET  /api/charts                # List cached interpretations
GET  /api/users/me              # Includes generation_tags (JSON array), billing, hasBirthData
```
`:type` is `ziwei` or `western`. `PUT /api/users/me/birth` deletes `interpretations` for user; `birth_data_hash` includes sorted `generation_tags`.

### F5 Predictions Endpoints (2026-09, spec: docs/superpowers/specs/2026-09-04-f5-api-predictions-design.md)
```bash
GET  /api/predictions?cycleId=   # 當週列表（checks/feedback/predictions；forecast 未收齊時遮罩為 null）
POST /api/predictions/generate   # 冪等週期生成（cycle_id=Asia/Taipei 週一起算；prediction_generations 凍結）
PUT  /api/predictions/checks     # F6 第 1 段 situation=absent|occurred（每週每 trigger 一次，去重）
POST /api/predictions/:id/feedback # F6 第 2 段 response=hit|miss|other（僅 occurred 後、一次性）
```
前置：最新 complete 人格側寫（`personality_profiles`）。`cycle_id` 為台北週一起算；寫入僅限當週（409 `STALE_CYCLE`）。

半自動 E2E：`./scripts/predictions-e2e.sh -t <session>`（自己 mint session：造 `login_tokens` 列 + 走 `/api/auth/verify`；已實測整鏈 2026-09-04，並據實測修正 F7 `db::batch` 的 Hrana v2 請求形狀 `{"type":"batch","batch":{"steps":[...]}}`）。

### Deployed Infrastructure

| Component | Status | URL/ID |
|-----------|--------|--------|
| **Workers API** (`ft-api`) | ✅ Live | https://fortunet-api.yanggf.workers.dev |
| **Engine Worker** (`ft-engine`, FT_ENGINE binding) | ✅ Live | https://fortunet-engine.yanggf.workers.dev |
| **Frontend** (Leptos CSR, Pages) | ✅ Live | https://fortunet.pages.dev |
| **Turso Database** | ✅ Ready | `libsql://fortunet-yanggf8.aws-ap-northeast-1.turso.io` |
| **Session DO** | ✅ Working | `SESSION_DO`, key `session` |
| **AI Mutex DO** | ✅ Working | `AI_MUTEX_DO`, queue 8 / 60s |
| **CI** | ✅ Configured | `.github/workflows/deploy.yml` — `cargo fmt --check` + `cargo test` (native) + `cargo clippy/build` (wasm) |
| **R2** | ✅ Ready | `fortunet-storage` |

### Cloudflare Secrets

| Secret | Purpose |
|--------|---------|
| `IFLOW_API_KEY` | Primary AI |
| `GROQ_API_KEY` | Secondary AI |
| `CEREBRAS_API_KEY` | Tertiary AI |
| `TURSO_URL` / `TURSO_AUTH_TOKEN` | Turso (vars + secret) |

---

## 📁 Repository Structure

```
ft/
├── CLAUDE.md                   # ⭐ Authoritative workspace guide
├── README.md                   # Live URLs & tech stack
├── AGENTS.md                   # This file
├── STORYTELLING_ROADMAP.md     # Storytelling roadmap
├── MASTER_PLAN.md              # Historical TS plan (superseded 98d3521)
├── Cargo.toml / Cargo.lock     # Workspace
├── .github/workflows/deploy.yml
├── scripts/
│   ├── schema.sql              # ⭐ Single source of truth (Turso)
│   ├── deploy-engine.sh        # worker-build + wrangler deploy (OAuth)
│   ├── deploy-web.sh           # build-web.sh + pages deploy
│   ├── verify-deployment.sh    # 部署後全服務驗證（含 F5 端點 401 probe）
│   └── predictions-e2e.sh      # F5 半自動 E2E：貼 session 跑 generate→checks→feedback 整鏈
├── vendor/solar-ephemeris/     # patch.crates-io
├── crates/
│   ├── schema/                 # ft-schema: api + storage DTOs (single source)
│   ├── domain/
│   │   ├── ziwei/              # ft-ziwei (x-iztro)
│   │   ├── western/            # ft-western (solar-ephemeris + vsop87)
│   │   └── big5/               # ft-big5
│   ├── worker/                 # ft-worker (fortunet-engine, /engine/ziwei + /engine/western)
│   ├── api/                    # ft-api (fortunet-api, routes/ + durable_objects/ + services/)
│   │   └── src/
│   │       ├── lib.rs          # #[event(fetch)] + CORS/headers
│   │       ├── routes/         # auth, users, charts, oauth, admin_invites, personality, predictions
│   │       ├── durable_objects/# SessionDO, AIMutexDO
│   │       └── services/       # billing, birth_hash (incl. generation_tags), engine, ai/prompts, generation, predictions (F5)
│   └── web/                    # ft-web (Leptos CSR, generation.rs synced with api)
│       ├── src/
│       │   ├── lib.rs          # App + Protected guard
│       │   ├── api.rs          # gloo-net client
│       │   ├── pages/          # Home, Login, Profile, Divination, Story
│       │   ├── components/     # BirthDataForm, PalaceGrid, etc.
│       │   └── generation.rs   # 1930s-2020s stories (synced)
│       ├── scripts/gen-stars.js
│       ├── galaxy.js
│       └── dist/               # build output
└── docs/
    ├── audit/ / phase0/        # Historical
    ├── phase5-summary.md
    ├── monitoring-setup.md
    └── rollback-procedures.md
```

No `backend/` / `frontend/` — removed in `98d3521 chore: Phase D cleanup`. `crates/*/build/` and `crates/web/dist/` are ignored build outputs; `galaxy.js` / `gen-stars.js` are current (not legacy).

---

## 📅 Timeline

```
Phase -1: System Audit        Week 0      ✅ COMPLETED
Phase 0:  Risk Assessment     Week 1-3    ✅ COMPLETED (GO)
Phase 1:  Foundation          Week 4-6    ✅ COMPLETED
Phase 2:  Core Features       Week 7-11   ✅ COMPLETED
Phase 3:  Frontend            Week 12-15  ✅ COMPLETED
Phase 4:  Integration/Test    Week 16-18  ✅ COMPLETED
Phase 5:  Pre-Migration       Week 19-20  ✅ COMPLETED
Phase 6:  Go-Live             Week 21     ✅ LIVE (Beta Testing)
Phase 7:  Storytelling        Week 26-33  ← P0/P1 generation chain live (2026-09)
```

Recent: 2026-09 rename `我的命盤→我的命格`, Wuxing+BaZi, personality merged, 4×4 palace, generation tags as selectable birth attribute, story prompt thickened.

---

## 💻 Development Commands

### Workspace (Cargo)
```bash
cargo build -p ft-api -p ft-worker -p ft-web   # add --target wasm32-unknown-unknown for worker/web
cargo check -p ft-api --target wasm32-unknown-unknown
cargo fmt --all                                # CI gates on --check
cargo clippy --target wasm32-unknown-unknown   # report-only
cargo test -p ft-schema -p ft-ziwei -p ft-western -p ft-big5 -p ft-api  # native only, no wasm runner
# Do NOT assume cargo test --target wasm32-unknown-unknown works (js_sys::Date panics outside wasm)
```

### Backend Worker (crates/api) & Engine Worker (crates/worker)
```bash
cd crates/api && worker-build --release && wrangler deploy   # OAuth only, unset token first
./scripts/deploy-engine.sh                                   # same

# Frontend (Leptos CSR)
./scripts/deploy-web.sh                                      # build-web.sh + pages deploy
cd crates/web && ./scripts/build-web.sh                      # build only (cargo + wasm-bindgen, no trunk)

# Verify
./scripts/verify-deployment.sh

# Database (single source of truth: scripts/schema.sql)
turso db shell fortunet < scripts/schema.sql
turso db shell fortunet "ALTER TABLE users ADD COLUMN generation_tags TEXT"  # idempotent, ignore duplicate column
gwebcdb-mint turso --tier write --db fortunet --export        # group token (covers every DB)
armo price / armo mint / ...                                  # Turso/gwebcdb toolset
```

**Deploy rule**: `unset CLOUDFLARE_API_TOKEN && wrangler ...`, prefer `wrangler whoami` first. CI does not deploy (OAuth only). One-person project, all work on `main`.

---

## 📝 Coding Standards

- **Rust**, 2-space indent, `snake_case` (wire keys `camelCase` via `serde(rename)`)
- `cargo fmt --all --check` is gating in CI; `clippy` is report-only — prefer not to add warnings
- Schema DTO field names are **semantic**: storage keys and wire keys must not be renamed (`crates/schema` is single source)
- Engine versions authoritative in `crates/api/src/services/engine_version.rs` (`ENGINE_VERSION_ZIWEI="3.0.0"`, `ENGINE_VERSION_WESTERN="4.0.0"`, `CHART_SCHEMA_VERSION=3`)

### Testing Philosophy
- `cargo test -p ft-schema -p ft-ziwei -p ft-western -p ft-big5 -p ft-api` — native only; `ft-worker`/`ft-web` validated via `verify-deployment.sh` + deployed Pages
- Integration over mocks; no default data/hardcoding

---

**Current Usage** (2026-09):
- Workers: ~10 req/day (testing)
- Turso: libSQL (no D1 cap)
- DO: session + AI mutex (queue 8 / 60s)
- AI: free tiers (iFlow/Groq/Cerebras) within $0

**Free Tier Limits**: Workers 100K/day, DO 400K/day, R2 10GB, Turso free.

---

## 📚 Key Documents

| Document | Status |
|----------|--------|
| [CLAUDE.md](./CLAUDE.md) | ⭐ Authoritative |
| [README.md](./README.md) | ✅ Synced 2026-09-02 |
| [MASTER_PLAN.md](./MASTER_PLAN.md) | ⚠️ Historical TS plan, superseded 98d3521 |
| [STORYTELLING_ROADMAP.md](./STORYTELLING_ROADMAP.md) | Storytelling roadmap |
| [docs/monitoring-setup.md](./docs/monitoring-setup.md) | Monitoring & alerts |
| [docs/rollback-procedures.md](./docs/rollback-procedures.md) | Emergency procedures |

---

**Last Updated**: 2026-09-04
**API URL**: https://fortunet-api.yanggf.workers.dev
**Workspace**: Cargo (`ft-api` / `ft-worker` / `ft-web` / `ft-schema` / `ft-ziwei` / `ft-western` / `ft-big5`)
