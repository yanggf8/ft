# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this
repository. It is **rules and pointers only** — architecture walkthrough, feature design
notes, and engine-version narrative live in **docs/ARCHITECTURE.md** (keep that file in sync
when code changes).

## Project Overview

FortuneT V2 is an AI-powered Chinese/Western astrology platform. The backend, chart engine,
and frontend are all **Rust**, deployed as Cloudflare Workers (workers-rs) and a Leptos CSR
frontend on Cloudflare Pages.

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

### Deploy

```bash
./scripts/deploy-api.sh      # api: worker-build --release + wrangler deploy + route sanity
./scripts/deploy-engine.sh   # engine: worker-build --release + wrangler deploy
./scripts/deploy-web.sh      # web: build-web.sh (copies _headers) + wrangler pages deploy
# build only (web): cd crates/web && ./scripts/build-web.sh   (cargo build + wasm-bindgen, no trunk)
# manual equivalent (api): cd crates/api && worker-build --release && wrangler deploy
```

### Database (Turso / libSQL)

**No project here uses Cloudflare D1** — its free-tier database cap is a hard blocker, so
everything is on Turso. `TURSO_URL` is a `[vars]` entry; `TURSO_AUTH_TOKEN` is a secret.

```bash
# schema.sql is the single source of truth for the database shape.
turso db shell fortunet < scripts/schema.sql
# Mint a token (one group token covers every DB in the group):
gwebcdb-mint turso --tier write --db fortunet --export
```

### Helper Scripts

```bash
./scripts/verify-deployment.sh              # production health + new-route sanity
./scripts/predictions-e2e.sh -t <session>   # F5 本週預測半自動 E2E（generate→checks→feedback 整鏈）
```

## Cloudflare Permissions (rules)

- `wrangler` runs on **OAuth only** — `unset CLOUDFLARE_API_TOKEN` before any wrangler
  command; when unsure what an operation needs, run `wrangler whoami` first and check the
  printed scope list against it.
- **wrangler OAuth has NO DNS write access** (`zone` is read-only): DNS record changes
  (Resend SPF/DKIM, etc.) go through the Cloudflare dashboard manually, or with a separate
  scoped API token (Zone → DNS → Edit). Workers deploys are unaffected (`workers (write)`).
- `wrangler secret list` confirms a secret exists but **never exposes its value** — keep a
  local copy of every secret value in `~/.secrets` at creation time, or the value is gone.

## Architecture (map)

Detailed walkthrough — routes, DOs, services, engines, frontend, F5 design notes, F2/F3
notes, engine-version narrative — lives in **docs/ARCHITECTURE.md**. Crate map:

- `crates/schema` — shared DTOs (`api` wire contract, `storage` DO format) + pure feature
  logic (`symbolic`, `predict`, `anchors`, `cycle`); single source of truth, no TS drift.
- `crates/domain/{ziwei,western,big5}` — engines (ziwei wraps x-iztro; western = real
  ephemeris; big5 = F1 scoring).
- `crates/worker` — `fortunet-engine` Worker (service binding `FT_ENGINE`).
- `crates/api` — `fortunet-api` Worker: routes, DOs, Turso, AI failover.
- `crates/web` — Leptos CSR frontend.

## Rules worth remembering

- Engine-version constants are authoritative in
  `crates/api/src/services/engine_version.rs` — the api side stamps chart caches and
  compares against them; the engine worker's own version field is decorative. Bump policy
  and sync notes: docs/ARCHITECTURE.md §Engine Versions.
- F5/F6 measurement protections (forecast redaction, one-shot feedback, situation lock,
  current-week-only writes) are API-enforced — do not bypass; details:
  docs/ARCHITECTURE.md §F5.
- `INVITE_REQUIRED=false` since 2026-09-06 (business model undecided) — flipping it back
  re-arms the invite gate on BOTH OAuth and magic-link registration.
- Route-level request bodies are capped at 64 KiB (`body_too_large` in `routes/common.rs`)
  — new body-parsing routes must call it before `req.json()`.
- Route code must not echo DB/engine error detail to clients (generic `"db error"` only).
- **CSP 紅線（2026-09-11 事故）**：`_headers` 的 `script-src 'self' 'wasm-unsafe-eval'`
  **擋 inline script** — wasm 啟動器必須留在外部檔 `boot.js`（`index.html` 以
  `<script type="module" src="/boot.js">` 引用，`build-web.sh` 負責複製）。
  把啟動器改回 inline 會重演「app 不 mount」的全站停擺。改 `index.html`/`_headers`
  後必須在瀏覽器載入 `dist/`（套用同款 header）驗證再部署。
- `galaxy.js` 的拒絕迴圈用 `for(;;)`，**不要**用 `do-while` + `continue`（`g` 未定義
  時會退出迴圈回傳 undefined → NaN → 動畫迴圈死亡；見 gauss() 註解與 commit 324bb73）。
- `predict::GATE_ORDER`（work→money→love→family→health）同時決定 F4 `gated_domains()`
  槽位順序與 predictions list 的 SQL ORDER BY — 三處耦合，改動前先讀
  `crates/schema/src/predict.rs` 頂部 doc；generate **必帶** body
  `{"strengths":{五域 0–3}}`（缺/越界 → 400 `INVALID_STRENGTHS`）。
- F5 generate = **F4 原子 batch**（steps[0] strengths 快照 + steps[1] freeze +
  steps[2..] predictions，併發敗者以 steps[1] affected==0 偵測）× **F8 逐槽 25%
  對照籤**（`f8_assignments` 帳本在 batch 外、回饋收齊前 `isControl` 盲測）。
  部署順序 **web 先**：新 API 會拒絕舊 web 無 strengths 的 generate（400）；
  細節見 docs/ARCHITECTURE.md §F5。

## Testing

`cargo test -p ft-schema -p ft-ziwei -p ft-western -p ft-big5 -p ft-api` — **native target
only**; the wasm test binaries cannot execute without a wasm-bindgen test runner.
`ft-worker` and `ft-web` have no unit tests: the engine worker is validated through
`scripts/verify-deployment.sh` against production, the frontend through the deployed
Pages site.

## Coding Standards

- Rust, 2-space indent, snake_case (camelCase only for JSON wire keys via `serde(rename)`).
- Existing warnings: some legacy clippy lints remain; CI gates on `fmt` + build, `clippy` is
  report-only. Prefer not to add new warnings.
- Schema DTO field names are **semantic**: storage keys and wire keys must not be renamed.

## Git & CI

- `.github/workflows/deploy.yml` — on push/PR to `main`: `cargo fmt --check`, `cargo clippy`,
  `cargo build` the wasm crates. **Deployment is manual** (OAuth only, no API token in CI).
- One-person project; all work lands directly on `main` (no feature branching).
- Must `unset CLOUDFLARE_API_TOKEN` before any `wrangler` command (OAuth preferred; API
  tokens have permission issues).
