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
./scripts/deploy-api.sh      # api: build → staging（sanity＋verify 全套）→ 驗過才正式 → sanity
./scripts/deploy-engine.sh   # engine: build → staging（health／golden）→ 驗過才正式
./scripts/deploy-web.sh      # web: build → staging 分支 → web-smoke（過了才）→ 正式站 → web-smoke
# build only (web): cd crates/web && ./scripts/build-web.sh   (cargo build + wasm-bindgen, no trunk)
# manual equivalent (api): cd crates/api && worker-build --release && wrangler deploy
```

### Database (Turso / libSQL)

**No project here uses Cloudflare D1** — its free-tier database cap is a hard blocker, so
everything is on Turso. `TURSO_URL` is a `[vars]` entry; `TURSO_AUTH_TOKEN` is a secret.

```bash
# schema.sql is the single source of truth for the database shape.
turso db shell fortunet-staging < scripts/schema.sql   # staging 先
turso db shell fortunet < scripts/schema.sql
# Mint a token (one group token covers every DB in the group):
gwebcdb-mint turso --tier write --db fortunet --export
```

### Helper Scripts

```bash
./scripts/verify-deployment.sh              # production health + new-route sanity + 9. 前端真瀏覽器 smoke
node scripts/web-smoke.mjs --base <url> [--login]  # 前端真瀏覽器驗證（staging 或正式站；--login 自造帳號、硬清）
./scripts/predictions-e2e.sh -t <session>   # F5 本週預測半自動 E2E（generate→checks→feedback 整鏈）
python3 -I scripts/subunits-selftest.py      # 子單元正式站自測（紫微流年/今年運勢、塔羅主題；自造帳號、預設硬清）
python3 -I scripts/shop-selftest.py          # 開運小物正式站自測（匿名／登入登記、轉登入不重算、me 聯集、取消、錯誤碼；結束硬清）
python3 scripts/talent-selftest.py [--api <url> --db <name>]  # 上手小實驗自測（建立冪等、單一進行中、回報一次性、過期 410、傾向門檻、刪除；結束硬清）
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
  logic (`symbolic`, `predict`, `anchors`, `cycle`, `naming` 姓名學 v1, `fengshui` 空間風水 v1 臥室／書房／辦公室座位); single source of truth, no TS drift.
- `crates/domain/{ziwei,western,big5}` — engines (ziwei wraps x-iztro; western = real
  ephemeris; big5 = F1 scoring).
- `crates/worker` — `fortunet-engine` Worker (service binding `FT_ENGINE`).
- `crates/api` — `fortunet-api` Worker: routes, DOs, Turso, AI failover.
- `crates/web` — Leptos CSR frontend.

## Rules worth remembering

- **上線前驗證紅線（2026-11-22）**：沒有本機伺服器，三個部署腳本都是「staging 先、驗過才上正式」：
  `deploy-engine.sh`（fortunet-engine-staging：health／紫微／流年 golden）→ `deploy-api.sh`
  （fortunet-api-staging：sanity＋`verify-deployment.sh` 全套）→ `deploy-web.sh`（Pages 分支
  `staging.fortunet.pages.dev`＋`scripts/web-smoke.mjs --login`）。staging 完全隔離：Turso
  `fortunet-staging`、獨立 DO／R2，網頁依 host 自動打 staging API（`api.rs api_base`）。
  **任何 DB migration 先跑 `fortunet-staging` 再跑 `fortunet`**；schema.sql 變更兩邊都要套。
  staging 只需要 `TURSO_AUTH_TOKEN`（程式另外只讀 RESEND_API_KEY／GOOGLE_CLIENT_SECRET，staging 驗證用不到）。新增頁面或互動時同步在
  web-smoke 加檢查；smoke 測試資料必須淨零。
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
  **子單元（2026-11-21）**：`tarot_readings.topic`（general|love|career，無 CHECK）
  新寫入一律存字串、舊列 **NULL ≡ general**（讀取與冪等比對皆然）；語料 v3
  （主題牌位引言）、prompt v2（love／career 框架行）——general prompt 輸出被
  golden 測試凍結為 v1，改 general 必先改 golden 並 bump；migration 先於 API 部署。
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
- **開運小物（2026-11-22）**：目錄 `ft_schema::shop`——品項 `id` 是 wire／儲存鍵**不可改名**
  （下架＝自 CATALOG 移除＋bump），改文字或增刪品項必 bump `SHOP_CATALOG_VERSION`；文案受禁字／
  價格測試守門，**不得宣稱任何改運效果**。`routes/shop.rs` 登入可選（`auth_user_opt`）＋`anonId`；
  migration 先於 API 部署；資料永不寫入 predictions/prediction_feedback/f8_assignments；
  日後加帳號刪除路由須一併刪 `shop_interest`。細節見 docs/ARCHITECTURE.md §開運小物。
- **上手小實驗（天賦線 v1，2026-11-22）**：`/try`，宣稱者＝使用者自己（先猜 15 分鐘做到第幾步→鎖定→
  7 天內回報→看差距）。目錄 `ft_schema::talent`：題目 `id`／標題／領域／步驟**永不改**（改＝新 id、舊 id 下架＋
  bump `TALENT_CATALOG_VERSION`）；所有固定文案在 schema，雙語禁字測試守門（對外不說「天賦」、不做能力宣稱、
  不做跨題比較）；`draw_three` 只吃「試過的領域」，結果**不得**進推薦。過期判斷唯一來源 `talent::is_expired`；
  「同時一筆進行中」靠原子 `INSERT … WHERE NOT EXISTS`；回報一次性。資料永不寫入
  predictions/prediction_feedback/f8_assignments、不 import `symbolic`；日後加帳號刪除路由須一併刪 `talent_trials`。
- **風水縮景（2026-11-22）**：結果頁縮景零件由 `scripts/sprite-stack/fengshui_parts.py` 產生
  （切片＋`crates/schema/src/fengshui/diorama_parts.rs`，勿手改）；改零件或 ruleset 題目／選項後必重跑產生器、
  跑 `--check`（CI 沒 PIL），並更新 `fengshui/diorama.rs` 的 `MAPPING`、bump `DIORAMA_VERSION`
  （`every_question_option_is_mapped` 測試會擋漏寫的選項）。
- **引路貓（2026-11-23）**：吉祥物頭永遠是月牙（月相由首頁王座椅背的月亮表現）、台詞跟月相，純邏輯在 `ft_schema::moon`（台詞受禁字測試守門，
  **不得**做運勢宣稱）；`MoonPhase::key()` 是椅背月亮圖檔名一部分不可改名；改模型跑
  `python3 scripts/sprite-stack/mascot.py`（產生 4 張圖集＋`components/mascot_stacks.rs`，勿手改）；首頁王座版另跑
  `python3 scripts/sprite-stack/throne.py`（`components/throne_stacks.rs` 勿手改；月亮蓋到前方體素會直接報錯；星星層遮罩也由它產生）。
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
