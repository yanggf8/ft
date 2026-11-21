# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this
repository. It is **rules and pointers only** — architecture walkthrough, feature design
notes, and engine-version narrative live in **docs/ARCHITECTURE.md** (keep that file in sync
when code changes).

## Project Overview

FortuneT V2 is an AI-powered Chinese/Western astrology platform. The backend, chart engine,
and frontend are all **Rust**, deployed as Cloudflare Workers (workers-rs) and a Leptos CSR
frontend on Cloudflare Pages.

- **Production Frontend**: https://ahexagram.com（canonical，2026-09-16 起；`www.` 轉址同站、
  `fortunet.pages.dev` 為備援 mirror）
- **Production API**: https://api.ahexagram.com（canonical；`fortunet-api.yanggf.workers.dev`
  為備援，`verify-deployment.sh` 預設仍打它）
- **Engine Worker** (service binding): https://fortunet-engine.yanggf.workers.dev
- 網域相關：CORS 生產白名單寫死在 `lib.rs is_allowed_origin`（ahexagram.com/www/pages.dev）；
  magic-link/OAuth/邀請連結指向 `WEB_ORIGIN` var；Pages custom domain 與 Worker custom domain
  都在 Cloudflare dashboard 管理（wrangler OAuth 無 DNS 寫入權）；`ahexagram.com` 同時是
  Resend 寄件網域（`MAIL_FROM=noreply@ahexagram.com`）與 Google OAuth redirect URI 的 host。

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
./scripts/f8-acceptance.sh [--cleanup]      # F8 對照組驗收全自動（DB 自造 login token 登入、
                                            #   測驗自舉、事前盲/遮罩/解盲、turso 帳本交叉比對）
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
  logic (`symbolic`, `predict`, `anchors`, `cycle`, `naming` 姓名學 v1, `fengshui` 空間風水 v1); single source of truth, no TS drift.
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
- 命理 AI 解讀（紫微／西洋／人生故事）system prompt 一律帶 `GUARDRAIL_ZH/EN`
  （`services/ai/prompts.rs`）：傾向非注定、不借遺傳／腦科學／IQ／研究數據背書；
  新增 chart type 時必須沿用，測試 `guardrail_matches_prompt_language` 守門。
- **CSP 紅線（2026-09-11 事故）**：`_headers` 的 `script-src 'self' 'wasm-unsafe-eval'`
  **擋 inline script** — wasm 啟動器必須留在外部檔 `boot.js`（`index.html` 以
  `<script type="module" src="/boot.js">` 引用，`build-web.sh` 負責複製）。
  把啟動器改回 inline 會重演「app 不 mount」的全站停擺。改 `index.html`/`_headers`
  後必須在瀏覽器載入 `dist/`（套用同款 header）驗證再部署。
- **快取紅線（2026-11-20 白屏事故）**：ahexagram.com zone 的 Browser Cache TTL=4h 會覆寫
  `_headers` 對 `.js/.css` 的 `Cache-Control`（pages.dev 不受影響）。`build-web.sh` 對
  `boot.js`／`ft_web.js`／`style.css` 加內容 hash `?v=` 是唯一防線——拿掉就會在部署後讓
  舊 `ft_web.js` 配新 wasm、app 不 mount。驗證白屏要用**有舊快取**的瀏覽器，全新
  Playwright 測不出來。
- **塔羅（2026-10-27 全案）**：三版本常數（`TAROT_CORPUS_VERSION`／
  `DRAW_ALGO_VERSION`／`PROMPT_VERSION`）在 `crates/schema/src/tarot`，改語料／
  演算法／prompt 必 bump；塔羅資料永不寫入 predictions/prediction_feedback/
  f8_assignments；星曜 type 的 wire 值是 `main`（非 iztro 的 `major`）；AI 路徑
  限流一律 `rate_limit_strict`（fail-closed）、AIMutexDO 走專屬實例
  `id_from_name("tarot")`；卡面在 `crates/web/assets/tarot/`（build-web.sh 複製）。
- `predict::GATE_ORDER`（work→money→love→family→health）同時決定 F4 `gated_domains()`
  槽位順序與 predictions list 的 SQL ORDER BY — 三處耦合，改動前先讀
  `crates/schema/src/predict.rs` 頂部 doc；generate **必帶** body
  `{"strengths":{五域 0–3}}`（缺/越界 → 400 `INVALID_STRENGTHS`）。
- F5 generate = **F4 原子 batch**（steps[0] strengths 快照 + steps[1] freeze +
  steps[2..] predictions，併發敗者以 steps[1] affected==0 偵測）× **F8 逐槽 25%
  對照籤**（`f8_assignments` 帳本在 batch 外、回饋收齊前 `isControl` 盲測）。
  部署順序 **web 先**：新 API 會拒絕舊 web 無 strengths 的 generate（400）；
  細節見 docs/ARCHITECTURE.md §F5。

- **星座配對（2026-11-20）**：規則層 `ft_schema::zodiac_match`（`ZODIAC_MATCH_CORPUS_VERSION`
  改文案必 bump）；AI 解讀 `POST /api/zodiac/match/interpret` 伺服器自讀西洋盤
  （`resolve_western_chart`，**不可**用 `resolve_chart`），`ZODIAC_MATCH_PROMPT_VERSION` 改 prompt
  必 bump；快取鍵＝(user, target, sign_key, corpus_version, prompt_version)；**失敗不落庫、
  任何 AI 失敗（含輸出檢查不過）都退兩把次數**（與 tarot 不同）；migration 先於 API 部署；AIMutexDO 專屬實例 `id_from_name("zodiac-match")`；配對資料永不寫入
  predictions/prediction_feedback/f8_assignments。
- **紫微今年運勢（2026-11-21）**：流年引擎 `ft_ziwei::horoscope` 輸出**子=0**（x-iztro 是寅=0），
  改語意或 x-iztro divide 設定必 bump `ENGINE_VERSION_ZIWEI_HOROSCOPE`；禁用 `horoscope_now()`。
  生辰一律走 `resolve_ziwei_birth`（**不可**用 `resolve_chart`、不可預設午時）——沒時辰回 400
  `NO_BIRTH_HOUR`。AI 解讀 `POST /api/ziwei/year/interpret`（chartType `ziwei_year`，
  `ZIWEI_YEAR_PROMPT_VERSION` 改 prompt 必 bump）；快取鍵＝(user, luck_year, engine_version_ziwei,
  horoscope_version, prompt_version)；**任何 AI 失敗（含 `output_ok` 不過）不落庫、退兩把、503
  `AI_UNAVAILABLE`**；結尾提醒句由 route 附加；migration 先於 API 部署；生辰 PUT 刪
  `ziwei_year_readings`；AIMutexDO 專屬實例 `id_from_name("ziwei-year")`；資料永不寫入
  predictions/prediction_feedback/f8_assignments。`/divination/ziwei` 轉址 Route 必須在
  `/divination/:type` 之前。
- **Workers AI 呼叫一律走 `providers.rs run_plain_json`**：workers-rs `Ai::run` 會把
  `serde_json::Value` 物件序列化成 JS `Map`，binding 讀不到欄位（2026-11-20 正式站 AI 全滅事故）。

- **色彩/對比紅線（2026-11-19 kawaii 亮底，取代 cosmic-silver）**：色值只能寫在
  `style.css` 的 `:root`（Rust 行內樣式與其餘 CSS 一律 `var(--token)`），改色後必跑
  `python3 scripts/css-guard.py --allow-local-vars`（寫死色值白名單制＋`var()` 存在性）
  與 `python3 scripts/contrast-check.py`（token 配對；新配對加進檔頭 `PAIRS`）。
  AA 基準＝實際底（白卡、`--void`/`--deep-space` 頁底、`--surface-tint`、紫板）；
  最緊：摩羯深端配深字 4.80、`--success` 在頁底光暈最深處 4.40（只准放卡內）。
  控制項邊框用 `--metal-lo`（≥3:1），`--glass-border` 只做裝飾。實畫面稽核：
  `node scripts/contrast-audit.mjs --base <本機 pages dev>`。細節見
  docs/ARCHITECTURE.md §Kawaii light theme。

## Testing

`cargo test -p ft-schema -p ft-ziwei -p ft-western -p ft-big5 -p ft-api` — **native target
only**; the wasm test binaries cannot execute without a wasm-bindgen test runner.
`ft-worker` and `ft-web` have no unit tests: the engine worker is validated through
`scripts/verify-deployment.sh` against production, the frontend through the deployed
Pages site.

## Coding Standards

- Rust, 2-space indent, snake_case (camelCase only for JSON wire keys via `serde(rename)`).
- Existing warnings: some legacy clippy lints remain; CI gates on `fmt` + test + build +
  **clippy (wasm32, no continue-on-error — 2026-09-18 修正：clippy 實為硬閘，非 report-only)**.
  Prefer not to add new warnings.
- Schema DTO field names are **semantic**: storage keys and wire keys must not be renamed.

## Git & CI

- `.github/workflows/deploy.yml` — on push/PR to `main`: `cargo fmt --check`,
  `cargo test --locked`（native：schema/ziwei/western/big5/api）, `cargo clippy`
  (wasm32 `--all-targets`), `cargo build` the wasm crates. **Deployment is manual**
  (OAuth only, no API token in CI).
- One-person project; all work lands directly on `main` (no feature branching).
- Must `unset CLOUDFLARE_API_TOKEN` before any `wrangler` command (OAuth preferred; API
  tokens have permission issues).
