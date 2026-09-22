# Architecture & Design Notes

從 CLAUDE.md 遷出(2026-09-11)— CLAUDE.md 只留規則與指標,實作細節與設計敘事集中在這。
內容描述 commit `e75e07f`(2026-09-07)當下的程式狀態;更動時請同步更新本文件對應小節。

## Workspace (Cargo)

- `crates/schema` — shared DTOs. **`api`** (request/response contract both Worker and Web
  deserialize) and **`storage`** (DO storage key/format for bit-compat), plus pure feature
  logic / static truth tables: `items` (IPIP-15), `symbolic`, `predict`, `anchors`, `cycle`,
  **`naming`** (姓名學 五格＋三才＋康熙筆畫表, 2026-09-18). This crate is the single source
  of truth that removes TS↔Rust drift.
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
  (ziwei/western), `Story`, `Admin`, `Naming`＋`Glossary`（2026-09-18 起公開頁，免登入）。
- **Components**: `BirthDataForm`, `ZiWeiPalaceGrid`, `Layout`; `Profile` 內含 `PredictionsCard`
  （F5 本週預測 — F6 兩段式動線）；`glossary.rs` 靜態詞條（名詞解釋頁渲染）。
- Uses `ft-schema::api` types directly — no wire-type drift.
- `crates/web/_headers` — Pages 安全標頭(HSTS/CSP/X-Frame-Options/nosniff/Referrer/
  Permissions-Policy);`build-web.sh` 會複製進 dist。

## Cosmic-silver theme + 星空藍 ground (2026-08-30 / 2026-09-16)

All styling is plain CSS in `crates/web/style.css`; no Rust owns a token.
Spec: `docs/superpowers/specs/2026-08-30-cosmic-silver-theme-design.md` §1
carries the token table and every contrast derivation — **read it before
changing any colour.**

- **Ground** = body `linear-gradient(135deg, var(--void), var(--deep-space))`.
  Since 2026-09-16 that pair is `#0A1733 → #14224A` (星空藍, user direction);
  it replaced the hesocial midnight-black `#0C0C0C → #10141F`, which read as
  夜空黑. The hue shift ~doubled ground luminance, which is what forced the
  two value moves below.
- **AA floor is the glass COMPOSITE, never the bare ground.** Text sits on
  plates: `.card` (`white .05`), `.feature` (deep-space gradient
  `rgba(10,23,51,.32) → rgba(5,13,31,.55)`, 2026-09-17 issue#1 — 凸顯白字，
  silver-dim 由 6.2 升到 ~7.6–8.8:1), `.ocean-dim` (`.04` over a card),
  `input` (`.05` over a card), `.palace`/`.quiz-choice` (`navy .55` over a
  card), `.star` pills, `.palace.life` (`white .07`), `.nav`/`.quiz-submit`
  (`navy .72`). Sampling the bare gradient overstates every ratio.
- **Two values moved for the hue shift**: `--silver-faint` `#8A919C` →
  `#9AA3B2` (was 4.2:1 on card glass / 3.8:1 on ocean-dim, both under the
  floor) and the lg mirror's horizon band `#6F7787` → `#7B8494`.
- **`.palace.life` is the trap.** It swaps the navy plate for a near-white
  highlight, so semi-transparent `.star` pills resolve against a *brighter*
  plate there: silver-dim fell to 4.05:1 and rose to 3.48:1. Fixed by
  re-laying the navy `.55` plate under the role tint inside the life palace
  only; the rules also pair each background with its `color` so a future
  `.on-light` nesting cannot split them.
- **`card:hover` / `.feature:hover` is not a floor case.**
  `filter: brightness(1.12)` scales the element's whole rendered output —
  plate and glyph together — so the ratio rises. Do not model a brightened
  plate against an unbrightened fill.
- **Tightest passing text on the site**: `rose` star pills in the life palace
  at 4.6:1. `--silver-faint`'s floor is `input::placeholder` at 4.55:1.
- Reviewed by an adversarial pass (Kimi K3, 2026-09-17) that re-derived every
  number; its corrections are recorded in the spec's "K3 adversarial review"
  block, including three claims in the earlier 09-16 note that were wrong.

## F5 Predictions (2026-09)

- **端點**（`routes/predictions.rs` → `services/predictions.rs`）：
  `GET /api/predictions?cycleId=`（列當週，含 checks/feedback/`generated`/`strengths`）、
  `POST /api/predictions/generate`（冪等週期生成；**F4 起必帶 body** `{"strengths":{五域 0–3}}`，
  缺/越界 → 400 `INVALID_STRENGTHS`）、`PUT /api/predictions/checks`（F6 第 1 段 absent|occurred）、
  `POST /api/predictions/:id/feedback`（F6 第 2 段 hit|miss|other）。
- **`cycle_id`**：Asia/Taipei 週一起算（`crates/schema/src/cycle.rs` 純函數，毫秒 ISO 解析、週一格式驗證）。
- **週期生成冪等（F4 原子 batch）**：`prediction_generations` 一週一 profile 快照；generate 的寫入是
  **單一 `db::batch` 原子交易**（steps[0]=`prediction_strengths` 快照、steps[1]=freeze、
  steps[2..]=predictions `WHERE NOT EXISTS`）；併發敗者以 steps[1] affected==0 偵測回現況；
  空週也凍結；週中重測不補 domain、strengths 凍結後不改（防混 profile）。
- **F4 領域閘門 × F8 對照籤（2026-09-13 併）**：`predict::GATE_ORDER=[Work,Money,Love,Family,Health]`
  決定 `gated_domains()` 槽位（強度 ≥1 且該域有錨點；work+money+love 目錄每格 ≥2=`rules-2`，
  family/health 恆 0）；每槽獨立 25% 中籤（`draw_control_default`，crypto 不可用=不中籤 fail-closed），
  中籤槽以其他使用者最新 complete 側寫走完全相同管線（`plan_slot`/`draw_shuffled_profile`）；
  對照失敗一律誠實降級真實組、freeze 前後不報錯。帳本 `f8_assignments`（batch 外純簿記：
  assigned+prediction_id / suppressed）；**事前盲**：回饋未收齊前 list 的 `isControl` 恆 false；
  **事後解盲**：web 於回饋收齊後顯示週級對照聚合（逐條指認已裁決不做）。
- **F6 測量保護（API 強制）**：forecast 遮罩（`redact_view`：`distinct(trigger) ⊆ checks` 才吐全文；
  GET/generate 共用）；第 2 段僅 `occurred` 後、一次性（`FEEDBACK_EXISTS`）；有 feedback 後情境鎖定
  （`SITUATION_LOCKED`，單句原子 `INSERT…SELECT…WHERE NOT EXISTS`）；寫入僅限當週（409 `STALE_CYCLE`）。
- **`filter_negative_half` 已推廣（2026-09-11，D2-A 特例廢除）**：嚴格「負面不過半」+ floor
  「永不丟最後一條」（全負週保留最佳 1 條，任何 n）。**tie 方向是承重牆**：drop-argmax 首同分勝
  （手動掃描嚴格 `>`），不得改用 `max_by_key`。`RULES_VERSION="rules-2"`（`anchors.rs`）。
- **Web**：`我的命格` PredictionsCard — 未生成週顯示 F4 五領域 0–3 輸入（`NeedStrengths`），
  使用者送出才 generate（**無自動生成，latch 已移除**）；Stage 1 全收齊才 refetch 全文進 Stage 2；
  按鈕閘門 `stage1_complete ∧ 已 refetch ∧ forecast.is_some()`；回饋三句用 §5.4.1 措辭；
  換週偵測走 `cycle_seen`（任何狀態比對，rollover 重置 strengths）。
- **F7 資料刪除**：`DELETE /api/personality/me` 六句一次 `db::batch`（Hrana v2 隱式交易，失敗整批 rollback；
  含 `prediction_strengths`）。
- **E2E**：`scripts/predictions-e2e.sh -t <session>` 半自動整鏈（generate 帶 F4 strengths→checks→feedback，
  含遮罩閘門①②驗證）；`scripts/f8-acceptance.sh` F8 驗收**全自動**（turso 自造 login token
  登入 → 測驗自舉 → generate → 事前盲 → checks/feedback → 事後解盲 API≡DB 交叉比對 →
  `--cleanup` F7+SQL 硬清測試列）。F8 首輪驗收已過（2026-09-13：work 槽中籤 control、
  D2-A 壓除記 suppressed、money real assigned、解盲一致，測試帳號已清）。
- **部署順序 web 先**：新 API 拒絕舊 web 無 strengths 的 generate（400）；反之舊 API 容忍新 web 的 body。
- 設計文件：`docs/superpowers/specs/2026-09-04-f5-api-predictions-design.md`、
  `docs/superpowers/specs/2026-09-04-f5-web-predictions-ui-design.md`、
  `docs/superpowers/specs/2026-09-11-f4-love-expansion-design.md`（F4/love/rules-2）、
  `docs/superpowers/specs/2026-09-13-f8-control-design.md`（對照籤/帳本/盲測；
  事前登記 `docs/preregistration/f8-d6.md`）。

## F2/F3 命盤象徵向量 + 落差洞察 (2026-09-07)

- 規格 rev.3：`docs/superpowers/specs/2026-09-07-f2-f3-design.md`;計畫:`docs/superpowers/plans/2026-09-07-f2-f3.md`。
- **F2**(`crates/schema/src/symbolic.rs`):純規則對照表(紫微命宮主星含借宮 clamp、西洋元素),
  全盤制(紫微優先、西洋 fallback、皆無缺席),基準 50 + 平均 + clamp,五檔帶 UI 不出裸分。
- **F3**(`GET /api/personality/overlay`):資格矩陣(最新列狀態治理,fail-closed)、
  `|gap| ≥ 20` 且實測非最低檔才敘事、opt-in 三段動線。
- **紅線**:`predictions.rs`/`services::ai` 不得 import `symbolic` — 由 golden 回歸測試釘死
  (`crates/schema/src/predict.rs` 的 F5 選則輸出逐欄位 fixture)。
- **Runtime 驗收已過(2026-09-11,測試帳號實測後 F7+SQL 清除、F8 零污染)**:
  401 session 強制、409 三態(`NO_MEASUREMENT`/`F3_DISABLED`/`MEASUREMENT_PENDING` —
  分別對應無測量 / skip / careless_suspected)、過期快取重算(塗改
  `meta.engineVersionWestern` → GET 回寫 4.0.0)、gender 缺失跳紫微
  (`priorSource:"western"`)、F3 敘事閘門(gap≥20 敘事、最低檔豁免壓過大 gap)、
  8 路並發重複計算容忍(全 200、last-write-wins)、**§0.4 紅線:四態命盤
  (新鮮/過期/失敗/缺席)下 F5 generate 輸出正規化後逐位一致**。
  Engine worker 真斷線的降級僅 native T6 覆蓋,未在 prod 演練。
- **已知舊帳(已結,2026-09-13)**:`/api/charts/:type` cache-hit 與 fresh 兩路回應外殼
  曾不一致(cached 版 `birth_data_hash` 恆 null — SELECT 漏欄、缺 top-level
  `engineVersion`/`chartSchemaVersion`)— cache-hit 回應現與 fresh 逐欄一致
  (僅 `fromCache` 旗標有別),前端容錯保留。
- **chart_resolver**(`crates/api/src/services/chart_resolver.rs`):唯讀解析(Value 層新鮮度:
  birth hash + engine version + schema version;結構驗證;engine 失敗降級;gender 缺失跳紫微;
  紫微早返回、西洋 fallback)。

## 姓名學 v1 ＋ 名詞解釋 (2026-09-18, issue #2)

純 client-side 功能：**零 API、零 DB、免登入**——api/worker/schema.sql 全不動，部署只動 web。

- **引擎**：`crates/schema/src/naming/`（`mod` 管線／`sancai` 五行生剋＋三才簡表／`luck` 81
  數理表／`strokes` 康熙筆畫表 5410 字）。`/naming` 頁同步呼叫 `analyze()` 在瀏覽器計算
  （輸入不傳送不儲存）。設計與**產品資料鎖定版**（81 表全文、golden 筆畫、22 詞條、
  肉部=6／數目字形等決策）：`docs/superpowers/specs/2026-09-18-naming-v1-design.md`。
- **筆畫表產製**：`scripts/gen-kangxi.py`（一次性離線，不進 CI）——Big5 常用 5401 字＋
  姓氏補充，14 個簡化部首規則（含 肉部之月=6、罒→网6、飠→食9；residual==0 例外讓 王=4），
  generator 內建 golden 對帳、不符即中止。表版本 `STROKE_TABLE_VERSION`（資訊性，非
  api engine_version 體系——無快取可比對）。
- **版面**：首頁 feature-grid 6 卡 3+3（紫微維持 DOM 第一保 chrome ring）；姓名學與
  名詞解釋為整卡連結 `a.feature`（style.css 兩行擋全域連結樣式）；`/glossary` 為獨立
  靜態頁（詞條在 `crates/web/src/glossary.rs`；`glossary-1` 已定案，2026-09-22 修訂 `glossary-1.1`
  —「日月並明」雙棟之才→棟梁之才，後續意見以修訂版處理並 bump）。
- 已知刻意簡化（spec §2/§3 列冊）：三才為生剋簡表非 125 組古表——失真案例的情境註記
  `sancai::distortion_note` 依 elements 計算、單一來源在 schema（金金金/木木土/洩氣局五
  順生鏈），web 只渲染，snapshot 測試鎖定；81 數理分歧條目採四源投票共識。

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
