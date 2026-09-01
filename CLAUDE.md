# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FortuneT V2 is an AI-powered Chinese/Western astrology platform. The backend, chart
engine, and frontend are all **Rust**, deployed as Cloudflare Workers (workers-rs + Hono-free
`worker` crate) and a Leptos CSR frontend on Cloudflare Pages.

- **Production Frontend**: https://fortunet.pages.dev
- **Production API**: https://fortunet-api.yanggf.workers.dev
- **Engine Worker** (service binding): https://fortunet-engine.yanggf.workers.dev

## Commands

### Build & check (Rust workspace)

```bash
cargo build -p ft-api   -p ft-worker -p ft-web   # any; worker/web need --target wasm32-unknown-unknown
cargo check -p ft-api --target wasm32-unknown-unknown
cargo fmt --all                                # format (CI gates on --check)
cargo clippy --target wasm32-unknown-unknown   # lints (report only, not gating)
```

### Backend Worker (crates/api) — deploy

```bash
cd crates/api && worker-build --release && wrangler deploy   # requires OAuth, no API token
# or: scripts/deploy-engine.sh (engine worker) / manual for api
```

### Engine Worker (crates/worker) — deploy

```bash
./scripts/deploy-engine.sh   # worker-build --release + wrangler deploy (OAuth only)
```

### Frontend (crates/web) — build & deploy

```bash
./scripts/deploy-web.sh      # build-web.sh + wrangler pages deploy dist --project-name=fortunet
# build only: cd crates/web && ./scripts/build-web.sh   (cargo build + wasm-bindgen, no trunk)
```

### Database (Turso / libSQL)

**No project here uses Cloudflare D1** — its free-tier database cap is a hard
blocker, so everything is on Turso. `TURSO_URL` is a `[vars]` entry;
`TURSO_AUTH_TOKEN` is a secret.

```bash
# schema.sql is the single source of truth for the database shape.
turso db shell fortunet < scripts/schema.sql
# Mint a token (one group token covers every DB in the group):
gwebcdb-mint turso --tier write --db fortunet --export
```

### Helper Scripts

```bash
./scripts/verify-deployment.sh  # verify production services are healthy
```

## Architecture

### Workspace (Cargo)

- `crates/schema` — shared DTOs. **`api`** (request/response contract both Worker and Web
  deserialize) and **`storage`** (DO storage key/format for bit-compat). This crate is the
  single source of truth that removes TS↔Rust drift.
- `crates/domain/ziwei` — ZiWei engine (wraps `x-iztro`).
- `crates/domain/western` — Western engine (hybrid: `solar-ephemeris` Moon + `vsop87` planets).
- `crates/worker` — `fortunet-engine` Worker, exposed via service binding (`FT_ENGINE`).
- `crates/api` — `fortunet-api` Worker: routes, durable objects, Turso, AI failover.
- `crates/web` — Leptos CSR frontend (replaces the old React app).

### Backend Worker (crates/api/src)

- **Entry**: `lib.rs` — `#[event(fetch)]`, OPTIONS preflight, security headers, CORS
  (exact-hostname allowlist), x-request-id, JSON 404 normalization.
- **Routes** (`routes/`): `auth.rs`, `users.rs`, `charts.rs` (+ `common.rs` shared helpers).
- **Durable Objects** (`durable_objects/`):
  - `SessionDO` — key `"session"`, 7-day TTL. `get` deletes expired; `refresh` extends.
  - `AIMutexDO` — true serialization (1 concurrent via `queue`+`oneshot`), `MAX_QUEUE_DEPTH=8`,
    `MAX_QUEUE_WAIT_MS=60000`, rpm/rpd limits, exresource metrics, 3-provider failover
    (iFlow → Groq → Cerebras) with a 45s provider timeout, and an offline stub when all
    providers fail.
- **Services** (`services/`): `billing` (30-day trial), `birth_hash` (bit-for-bit JS-compatible),
  `engine` (service-binding client + `jd_from_birth` tz conversion), `ai` (prompts + providers),
  `clock`, `uuid`, `db` (Turso client over Hrana HTTP + the bind helpers).
- **Database** — `users` + `interpretations` tables live. `subscriptions`/`usage_tracking`/`ai_quota`
  are provisioned but unused. `services/db.rs` speaks Hrana over `worker::Fetch`; the `libsql`
  crate's own `cloudflare` feature is unusable here because it pins `worker ^0.6` against our 0.8.

### Engine Worker (crates/worker/src/lib.rs)

`fortunet-engine` — `#[event(fetch)]` routing `/engine/ziwei` + `/engine/western`. `jd_utc` is
validated as finite (a bad JD would panic the ephemeris math). Emits `engineVersionZiwei="4.0.0"`.

### Frontend (crates/web/src)

- **`lib.rs`** — `#[component] App`, `wasm_bindgen(start)` mount. Router with a `Protected` guard.
- **`api.rs`** — gloo-net client mirroring the old `lib/api.ts` (session in localStorage,
  interpret 409 retry, structured `ApiErr`).
- **Pages**: `Home`, `Login`, `Profile`, `Divination` (ziwei/western), `Story`.
- **Components**: `BirthDataForm`, `ZiWeiPalaceGrid`, `Layout`.
- Uses `ft-schema::api` types directly — no wire-type drift.

## Engine Versions

The authoritative constants live in `crates/api/src/services/engine_version.rs`:
`ENGINE_VERSION_ZIWEI = "3.0.0"`, `ENGINE_VERSION_WESTERN = "4.0.0"` (real ephemeris +
top-level `sunSign`/`moonSign`), `CHART_SCHEMA_VERSION = 3`. Cache freshness is judged by
these: stored charts whose `meta.engineVersion*` differs are stale and recalculated. The
engine worker's own response field `engineVersionZiwei` says `"4.0.0"` but is decorative —
the api side stamps and compares against its own constant. Keep them in sync when bumping.
Bump `WESTERN` only after the §8.2 event-table validation. When bumped, stored caches with
an older `meta.engineVersion*` are treated as stale and recalculated.

## Testing

`cargo test -p ft-schema -p ft-ziwei -p ft-western -p ft-big5 -p ft-api` — **native target
only** (CI step `test (native)`); `js_sys::Date`/clock calls panic outside wasm, so
api-side tests pin pure logic (extraction like `trial_access_for`) rather than the js_sys
call sites. `ft-worker` and `ft-web` have no unit tests: the engine worker is validated
through `scripts/verify-deployment.sh` against production, the frontend through the
deployed Pages site. Do not assume `cargo test --target wasm32-unknown-unknown` works —
the wasm test binaries cannot execute without a wasm-bindgen test runner.

## Coding Standards

- Rust, 2-space indent, snake_case (camelCase only for JSON wire keys via `serde(rename)`).
- Existing warnings: some legacy clippy lints remain; CI gates on `fmt` + build, `clippy` is
  report-only. Prefer not to add new warnings.
- Schema DTO field names are **semantic**: storage keys and wire keys must not be renamed.

## Git & CI

- `.github/workflows/deploy.yml` — on push/PR to `main`: `cargo fmt --check`, `cargo clippy`,
  `cargo build` the wasm crates. **Deployment is manual** (OAuth only, no API token in CI).
- One-person project; all work lands directly on `main` (no feature branching).
- Must `unset CLOUDFLARE_API_TOKEN` before any `wrangler` command (OAuth preferred; API tokens
  have permission issues).
