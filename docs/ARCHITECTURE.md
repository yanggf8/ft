# Architecture & Design Notes

從 CLAUDE.md 遷出(2026-09-11)— CLAUDE.md 只留規則與指標,實作細節與設計敘事集中在這。
內容描述 commit `e75e07f`(2026-09-07)當下的程式狀態;更動時請同步更新本文件對應小節。

## Workspace (Cargo)

- `crates/schema` — shared DTOs. **`api`** (request/response contract both Worker and Web
  deserialize) and **`storage`** (DO storage key/format for bit-compat). This crate is the
  single source of truth that removes TS↔Rust drift.
- `crates/domain/ziwei` — ZiWei engine (wraps `x-iztro`).
- `crates/domain/western` — Western engine (hybrid: `solar-ephemeris` Moon + `vsop87` planets).
- `crates/domain/big5` — F1 personality scoring (`scoring`/`careless`/`norm`).
- `crates/worker` — `fortunet-engine` Worker, exposed via service binding (`FT_ENGINE`).
- `crates/api` — `fortunet-api` Worker: routes, durable objects, Turso, AI failover.
- `crates/web` — Leptos CSR frontend (replaces the old React app).

## Backend Worker (crates/api/src)

- **Entry**: `lib.rs` — `#[event(fetch)]`, OPTIONS preflight, security headers, CORS
  (exact-hostname allowlist), x-request-id, JSON 404 normalization.
- **Routes** (`routes/`): `auth.rs`, `users.rs`, `charts.rs`, `personality.rs` (F1 + F3
  overlay + 資格矩陣), `predictions.rs`, `admin_invites.rs`, `oauth.rs`
  (+ `common.rs` shared helpers: `auth_user`, rate limit, `body_too_large` 64 KiB cap,
  `embed_meta`, `extracted_version`).
- **Durable Objects** (`durable_objects/`):
  - `SessionDO` — key `"session"`, 7-day TTL. `get` deletes expired; `refresh` extends.
  - `AIMutexDO` — true serialization (1 concurrent via `queue`+`oneshot`), `MAX_QUEUE_DEPTH=8`,
    `MAX_QUEUE_WAIT_MS=60000`, rpm/rpd limits, exresource metrics, 3-provider failover
    (iFlow → Groq → Cerebras) with a 45s provider timeout, and an offline stub when all
    providers fail.
  - `RateLimitDO` — cross-isolate persistent rate limiting, 8 shards (`rl:0..rl:7`).
- **Services** (`services/`): `billing` (30-day trial), `birth_hash` (bit-for-bit JS-compatible),
  `engine` (service-binding client + `jd_from_birth` tz conversion), `ai` (prompts + providers),
  `predictions` (F5: 週期生成/遮罩/鎖定), `chart_resolver` (overlay 的型別化命盤解析 — 唯讀、
  結構驗證、engine 失敗降級), `clock`, `uuid`, `db` (Turso client over Hrana HTTP + `batch`
  原子批次 + the bind helpers), `oauth` (Google flow helpers + one-time exchange),
  `invite` (邀請碼 + `valid_expires_iso`), `login_token` (magic-link tokens).
- **Database** — `users`, `interpretations`, `personality_profiles` + F5 四表
  (`predictions` / `situation_checks` / `prediction_feedback` / `prediction_generations`) +
  `invites` + `oauth_exchanges`. `subscriptions`/`usage_tracking`/`ai_quota` are provisioned
  but unused. `services/db.rs` speaks Hrana over `worker::Fetch`; the `libsql` crate's own
  `cloudflare` feature is unusable here because it pins `worker ^0.6` against our 0.8.

## Engine Worker (crates/worker/src/lib.rs)

`fortunet-engine` — `#[event(fetch)]` routing `/engine/ziwei` + `/engine/western`. `jd_utc` is
validated as finite (a bad JD would panic the ephemeris math). Emits `engineVersionZiwei="4.0.0"`
(decorative — see Engine Versions below).

## Frontend (crates/web/src)

- **`lib.rs`** — `#[component] App`, `wasm_bindgen(start)` mount. Router with a `Protected` guard.
- **`api.rs`** — gloo-net client (session in localStorage, structured `ApiErr`,
  `exchange_oauth_code`, `fetch_overlay`).
- **Pages**: `Home`, `Login`, `Profile`, `Personality` (quiz + F3 overlay), `Divination`
  (ziwei/western), `Story`, `Admin`.
- **Components**: `BirthDataForm`, `ZiWeiPalaceGrid`, `Layout`; `Profile` 內含 `PredictionsCard`
  （F5 本週預測 — F6 兩段式動線）。
- Uses `ft-schema::api` types directly — no wire-type drift.
- `crates/web/_headers` — Pages 安全標頭(HSTS/CSP/X-Frame-Options/nosniff/Referrer/
  Permissions-Policy);`build-web.sh` 會複製進 dist。

## F5 Predictions (2026-09)

- **端點**（`routes/predictions.rs` → `services/predictions.rs`）：
  `GET /api/predictions?cycleId=`（列當週，含 checks/feedback）、`POST /api/predictions/generate`
  （冪等週期生成）、`PUT /api/predictions/checks`（F6 第 1 段 absent|occurred）、
  `POST /api/predictions/:id/feedback`（F6 第 2 段 hit|miss|other）。
- **`cycle_id`**：Asia/Taipei 週一起算（`crates/schema/src/cycle.rs` 純函數，毫秒 ISO 解析、週一格式驗證）。
- **週期生成冪等**：`prediction_generations` 一週一 profile 快照；`generate` **先寫 freeze 再插 predictions**
  （單寫者 + `UNIQUE(user_id, cycle_id, domain)` 防呆）；空週也凍結；週中重測不補 domain（防混 profile）。
- **F6 測量保護（API 強制）**：forecast 遮罩（`redact_view`：`distinct(trigger) ⊆ checks` 才吐全文；
  GET/generate 共用）；第 2 段僅 `occurred` 後、一次性（`FEEDBACK_EXISTS`）；有 feedback 後情境鎖定
  （`SITUATION_LOCKED`，單句原子 `INSERT…SELECT…WHERE NOT EXISTS`）；寫入僅限當週（409 `STALE_CYCLE`）。
- **`filter_negative_half` D2-A 例外**：v1 僅 2 領域，全負面週保留較佳 1 條（coverage 高者勝、同則 priority 小者）；
  F8 登記「三領域落地後廢除」。`RULES_VERSION="rules-1"`（`anchors.rs`）。
- **Web**：`我的命格` PredictionsCard — Stage 1 全收齊才 refetch 全文進 Stage 2；按鈕閘門
  `stage1_complete ∧ 已 refetch ∧ forecast.is_some()`；回饋三句用 §5.4.1 措辭；generate per-mount latch；
  window focus 重比 `cycleId` 偵測換週。
- **F7 資料刪除**：`DELETE /api/personality/me` 五句一次 `db::batch`（Hrana v2 隱式交易，失敗整批 rollback）。
- **E2E**：`scripts/predictions-e2e.sh -t <session>` 半自動整鏈（generate→checks→feedback，含遮罩閘門①②驗證）；
  已實測通過（2026-09-04，測試帳號用完即清，F8 零污染）。
- 設計文件：`docs/superpowers/specs/2026-09-04-f5-api-predictions-design.md`、
  `docs/superpowers/specs/2026-09-04-f5-web-predictions-ui-design.md`。

## F2/F3 命盤象徵向量 + 落差洞察 (2026-09-07)

- 規格 rev.3：`docs/superpowers/specs/2026-09-07-f2-f3-design.md`;計畫:`docs/superpowers/plans/2026-09-07-f2-f3.md`。
- **F2**(`crates/schema/src/symbolic.rs`):純規則對照表(紫微命宮主星含借宮 clamp、西洋元素),
  全盤制(紫微優先、西洋 fallback、皆無缺席),基準 50 + 平均 + clamp,五檔帶 UI 不出裸分。
- **F3**(`GET /api/personality/overlay`):資格矩陣(最新列狀態治理,fail-closed)、
  `|gap| ≥ 20` 且實測非最低檔才敘事、opt-in 三段動線。
- **紅線**:`predictions.rs`/`services::ai` 不得 import `symbolic` — 由 golden 回歸測試釘死
  (`crates/schema/src/predict.rs` 的 F5 選則輸出逐欄位 fixture)。
- **chart_resolver**(`crates/api/src/services/chart_resolver.rs`):唯讀解析(Value 層新鮮度:
  birth hash + engine version + schema version;結構驗證;engine 失敗降級;gender 缺失跳紫微;
  紫微早返回、西洋 fallback)。

## Engine Versions

The authoritative constants live in `crates/api/src/services/engine_version.rs`:
`ENGINE_VERSION_ZIWEI = "3.0.0"`, `ENGINE_VERSION_WESTERN = "4.0.0"` (real ephemeris +
top-level `sunSign`/`moonSign`), `CHART_SCHEMA_VERSION = 3`. Cache freshness is judged by
these: stored charts whose `meta.engineVersion*` differs are stale and recalculated. The
engine worker's own response field `engineVersionZiwei` says `"4.0.0"` but is decorative —
the api side stamps and compares against its own constant. The `engine_version.rs` module
comment's "bump WESTERN only after §8.2" line is stale (bump already done); WESTERN's inline
comment is the accurate one.

## Email (Resend)

- Magic-link 登入信經 Resend 寄送;`MAIL_FROM` var + `RESEND_API_KEY` secret。
- 2026-09-11:`ahexagram.com` 已建入 Resend(id `beda6a15-…`,ap-northeast-1),DNS 紀錄
  (DKIM TXT `resend._domainkey`、MX/SPF `send`、CNAME `rsend`,全 DNS only)待加/驗證;
  verified 後 `MAIL_FROM` 換 `noreply@ahexagram.com`(或等效)並部署,再以非本人信箱實寄驗收。
- `~/.secrets` 的 `RESEND_FULL_API_KEY` 為 full-access key(domain 寫操作用);
  線上 `wrangler secret RESEND_API_KEY` 為舊 key(值未留本地,功能正常)。
