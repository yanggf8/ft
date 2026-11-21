# Architecture & Design Notes

從 CLAUDE.md 遷出(2026-09-11)— CLAUDE.md 只留規則與指標,實作細節與設計敘事集中在這。
內容描述 commit `e75e07f`(2026-09-07)當下的程式狀態;更動時請同步更新本文件對應小節。

## Workspace (Cargo)

- `crates/schema` — shared DTOs. **`api`** (request/response contract both Worker and Web
  deserialize) and **`storage`** (DO storage key/format for bit-compat), plus pure feature
  logic / static truth tables: `items` (IPIP-15), `symbolic`, `predict`, `anchors`, `cycle`,
  and **`naming`** (姓名學 五格＋三才＋康熙筆畫表, 2026-09-18). This crate is the single source
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
  overlay + 資格矩陣), `predictions.rs`, `tarot.rs`, `palm.rs`, `zodiac.rs`, `ziwei.rs`
  （紫微流年大運／今年運勢）, `admin_invites.rs`, `oauth.rs`
  (+ `common.rs` shared helpers: `auth_user`, rate limit, `body_too_large` 64 KiB cap,
  `embed_meta`, `extracted_version`, `page_query` 歷史分頁 query 解析（tarot／palm 共用）).
- **Durable Objects** (`durable_objects/`):
  - `SessionDO` — key `"session"`, 7-day TTL. `get` deletes expired; `refresh` extends.
  - `AIMutexDO` — true serialization (1 concurrent via `queue`+`oneshot`), `MAX_QUEUE_DEPTH=8`,
    `MAX_QUEUE_WAIT_MS=25000`, exresource metrics. **文字解讀全走 Cloudflare Workers AI**
    （`[ai]` binding，2026-11-20 起；取代 Groq→Cerebras——Cerebras 已 402 失效）：
    `TEXT_CHAIN` 主環 `@cf/google/gemma-4-26b-a4b-it`（`enable_thinking:false`）→ 備環
    `@cf/meta/llama-3.3-70b-instruct-fp8-fast`；整筆請求自入列起 88s 預算（避開 Cloudflare
    100s 524），主環上限 55s、剩不到 15s 不進備環；`length`／`incomplete` 截斷換環；
    每日免費額度錯誤（3036／4006）立即回 503 `AI_QUOTA`、不走 stub；全環失敗才回離線 stub
    （`charts.rs` 不把 stub 寫進快取，舊 stub 視同未命中）。手相走同一 binding 的 vision 兩環
    （契約不變）。選型實測與 neurons 估算見 `docs/superpowers/plans/2026-11-19-zodiac-match.md` §1。
  - `RateLimitDO` — cross-isolate persistent rate limiting, 8 shards (`rl:0..rl:7`).
- **Services** (`services/`): `billing` (30-day trial), `birth_hash` (bit-for-bit JS-compatible),
  `engine` (service-binding client + `jd_from_birth` tz conversion), `ai` (prompts + providers；紫微／西洋／人生故事
  system prompt 共用 `GUARDRAIL_ZH/EN`：命盤只談傾向、不把天賦寫成注定、不借遺傳／腦科學／IQ／研究數據背書、
  禁醫療法律財務確定性斷言；塔羅另有專屬 prompt 不疊加),
  `predictions` (F5: 週期生成/遮罩/鎖定), `chart_resolver` (overlay 的型別化命盤解析 — 唯讀、
  結構驗證、engine 失敗降級), `clock`, `uuid`, `db` (Turso client over Hrana HTTP + `batch`
  原子批次 + the bind helpers), `oauth` (Google flow helpers + one-time exchange),
  `invite` (邀請碼 + `valid_expires_iso`), `login_token` (magic-link tokens).
- **Database** — `users`, `interpretations`, `personality_profiles` + F5 四表
  (`predictions` / `situation_checks` / `prediction_feedback` / `prediction_generations`) +
  `invites` + `oauth_exchanges`. `practice_plans`/`practice_attempts`/`interest_results`
  與 `users.practice_epoch`/`users.interest_epoch` 為已下架功能殘留（程式碼不再讀寫，待另案 drop）。
  `subscriptions`/`usage_tracking`/`ai_quota` are provisioned
  but unused. `services/db.rs` speaks Hrana over `worker::Fetch`; the `libsql` crate's own
  `cloudflare` feature is unusable here because it pins `worker ^0.6` against our 0.8.

## Engine Worker (crates/worker/src/lib.rs)

`fortunet-engine` — `#[event(fetch)]` routing `/engine/ziwei` + `/engine/ziwei/horoscope`
（流年大運，回應 key `horoscope`）+ `/engine/western`. `jd_utc` is
validated as finite (a bad JD would panic the ephemeris math). Emits `engineVersionZiwei="4.0.0"`
(decorative — see Engine Versions below).

## Frontend (crates/web/src)

- **`lib.rs`** — `#[component] App`, `wasm_bindgen(start)` mount. Router with a `Protected` guard.
- **`api.rs`** — gloo-net client (session in localStorage, structured `ApiErr`,
  `exchange_oauth_code`, `fetch_overlay`).
- **Pages**: `Home`, `Login`, `Profile`, `Personality` (quiz + F3 overlay), `Divination`
  (ziwei/western), `Story`, `Admin`, `Naming`＋`Glossary`（2026-09-18 起公開頁，免登入）。
- **Components**: `BirthDataForm`, `ZiWeiPalaceGrid`, `Layout`; `Profile` 內含 `PredictionsCard`（F5 本週預測 — F6 兩段式動線）；`glossary.rs`
  靜態詞條（名詞解釋頁渲染）。
- Uses `ft-schema::api` types directly — no wire-type drift.
- `crates/web/_headers` — Pages 安全標頭(HSTS/CSP/X-Frame-Options/nosniff/Referrer/
  Permissions-Policy);`build-web.sh` 會複製進 dist。

## 本週小練習（2026-10-20 上線）

已下架（2026-11-17，站長裁示；資料表保留待另案 drop）。

## 塔羅牌（2026-10-27，v1＋v2 已上線）

兩段式：v1a「今日抽牌」公開頁（`/tarot`，免登入零 API——決定性抽牌
`ft_schema::tarot::daily_draw`，種子＝台北日期＋localStorage 32-hex 裝置種子，
明文不防重擲）與 v1b 三張牌陣（`/tarot/spread`，登入）。`ft_schema::tarot` 是
前後端共用契約：78 張 ZH-TW 語料（`catalog.rs`，v2 經 Claude 校對 8 條修正；
v3 見下方子單元）、注入式洗牌 `shuffle_three`（fnv1a_64→SplitMix64＋
拒絕取樣，演算法由 `TAROT_DRAW_ALGO_VERSION` 鎖定，golden test 釘死）、
`taipei_today`（重用 cycle.rs UTC+8 純算術）。API 五個 `/api/tarot` 路由在
`routes/tarot.rs`：draw 處理順序＝IP 限流→auth→body cap→parse→驗證→冪等
（`UNIQUE(user_id, request_id)` 重放回原結果／異 body 409）→uid 24h 限流→
`secure_bytes` CSPRNG（fail-closed，明文禁 `Math::random`）→寫入；DELETE 404
不洩存在性。**v2 AI 命盤感知解牌**（同日上線）：`POST .../:id/interpret`——
auth→讀取（done 冪等不計次）→IP/user **strict** 限流（fail-closed，3＋10 次/
24h，進入 AI 呼叫即計）→AIMutexDO **專屬實例** `id_from_name("tarot")`→
`stub:true` 一律視為 failed→`ai_*` 欄位（規則式 interpretation 永不覆寫；
UPDATE first-wins 防雙分頁競態）；`?chart=0`＝純塔羅（紫微＋F4 全不外送）；
chartSummary 組裝在 route 層（isLifePalace 主星——wire 值 `main` 非 `major`）。
AIMutexDO 佇列已從 `thread_local!` 遷 `QueueState` 實例欄位（per-DO-instance；
thread_local 是 per-isolate 會跨實例共享）。`rate_limit_strict` 在 common.rs。
DB：`tarot_readings`（migrations：2026-10-27-tarot / -tarot-ai / -tarot-ai-context、
2026-11-21-tarot-topic）。
**美術**：78 張卡面＋卡背在 `crates/web/assets/tarot/`（museimage 產製、
cosmic-silver 方向、PIL 降 1024/q82 WebP 約 12.7MB、`build-web.sh` 複製）；
卡面 CSS 1:2 原生比例、逆位 180°、直向 ≤480px 斷點（RWD）。首頁 feature-grid
塔羅卡排最前（chrome ring 主打位）＋NEW 徽章——2026-10-27 用戶指示，取代
原「紫微第一」決策。塔羅資料永不寫入 predictions／prediction_feedback／
f8_assignments。完整設計與三家（Muse/Codex/Opus）七輪審查軌跡：
`docs/superpowers/specs/2026-10-27-tarot-v1-design.md`。

**子單元：感情／事業（2026-11-21，sub-units spec §B）**。`/tarot` 今日抽牌下方
「選一個主題」三張 `.feature` 卡（綜合三張→`/tarot/spread`、感情→`/tarot/love`、
事業→`/tarot/career`；未登入連 `/login?next=…`）。三路由共用 `TarotSpreadPage`
（`topic` prop，預設 general）：標題／引導／提問 placeholder／牌位副標依主題，
歷史清單三主題共用、顯示主題標籤。牌陣仍固定三張 現況／阻礙／建議。
- `ft_schema::tarot::TarotTopic { General, Love, Career }`（wire `general|love|career`）；
  `TarotDrawRequest.topic` 省略＝general（舊 web 相容）、`TarotReadingView.topic`
  缺欄＝general（新 web 對舊 API 相容）。
- 規則式：`position_intro(topic, pos, reversed)`——general 與 v2 逐字相同，love／career
  各 6 段新引言；`TAROT_CORPUS_VERSION="3"`（`TAROT_DRAW_ALGO_VERSION` 不動）。
  測試：引言 ≤40 字、塔羅禁字表（一定／注定／復合／分手／升職／錄取…）、三主題組裝 ≤600。
- DB：`tarot_readings.topic TEXT`（migration `2026-11-21-tarot-topic.sql`，無 CHECK）；
  新寫入一律存字串（含 `'general'`），舊列 NULL ≡ general（讀取與冪等比對皆然；
  topic 不一致 → 409 `IDEMPOTENCY_CONFLICT`）。
- AI：interpret 由 DB 列讀 topic 放進 chartData；`build_tarot_prompt` 對 love／career
  加一行主題框架（不判定對方心意／關係結局；不判定升遷、錄取、收入）並換無提問預設句；
  general 輸出與 v1 逐字相同（golden 測試 `tarot_general_prompt_frozen_v1_*`）。
  `TAROT_PROMPT_VERSION="2"`（只寫進 `ai_prompt_version`，舊解讀不失效）。
- 限流、AIMutexDO 實例 `"tarot"`、chartType `"tarot"`、純塔羅勾選不變；三主題共用次數。

## 手相（2026-10-28，v1 已上線）

兩段式：L0 教學公開頁（`/palm`，免登入零 API——掌形 SVG 線稿教學圖
（currentColor，兼作拍攝導引 overlay，一物三用）＋提問式示範＋隱私卡）與
`/palm/capture`（Protected：P6 二段式知情 localStorage `palm_ai_notice`
fail-closed→左右手→`getUserMedia` facingMode environment（`_headers`
`camera=(self)`；`camera_open_gen` 世代守衛防 pending 競態）＋掌形導引
overlay→拍照 ≤768px WebP q80 退 jpeg（canvas toDataURL）→data URL 預覽→
P22 結果（規則式快照＋特徵中文標籤，absent＝此線不明顯／unknown＝未能判讀）
→歷史兩段刪除、刪後重取列表；fallback `<input capture>`→img EXIF 自動轉正）。
**AI 鏈純 Workers AI 兩環**（spec §6.2 選型終局，使用者裁示 OR／外部直連皆
不採）：`@cf/meta/llama-4-scout-17b`（max_tokens 2048；品質閘
`GATE_THRESHOLD_SCOUT=0.3`）→`@cf/meta/llama-3.2-11b-vision`（3072；品質閘
只靠 quality 欄——confidence 不可校準）；AIMutexDO 專屬實例
`id_from_name("palm")`、`env.ai("AI")` binding 就地不出站、回應**雙形**
（`result.response`／`choices`）都要解、全敗 503 `AI_ALL_FAILED` fail-closed
絕不 stub（rpm 觸頂同碼、log `rpm_blocked=` 區分）、`finish_reason=length`→
500 `AI_TRUNCATED`（終局不 failover）；模型常數 `SCOUT_MODEL` 在
`ft_schema::palm::wire` 單一來源（DO PALM_CHAIN 與 route 品質閘共用）。
API 四端點 `routes/palm.rs`：analyze 順序＝cheap IP fail-open 20/min→auth→
512KiB 專用 cap（`body_too_large_with`）→驗證（單一實質約束 imageBase64
≤682,667 字）→sha256 指紋→冪等（比對 hand＋image_sha256，UNIQUE(user_id,
request_id)）→IP strict 15/24h→uid strict 3/24h **refundable**→DO→**P8a
退款閉環**（Queue 結局退且不占額度；422 品質閘占 `palm:refund:{uid}` 3/24h
才退；AI_PARSE_FAILED／AI_TRUNCATED／全敗／成功不退——RateLimitDO `/refund`
＋check 回 resetAt 票根）→P8b 圍籬剝除 strict parse→品質閘 422
`PALM_QUALITY_LOW`→`assemble_palm_interpretation` 規則式快照落庫；DO outcome
code 轉發封閉集（四契約碼白名單）。schema `ft_schema::palm`：列舉全含
unknown、`deny_unknown_fields` 擋幻覺、`PALM_FEATURES_VERSION`（抽取 prompt
＋特徵 schema 同版）；DB `palm_readings`（corpus/features 兩版欄；
`ai_chart_context` 三段留 v2）。**紅線**：照片永不落庫（指紋＝sha256(base64
字串) 不進 wire/log）；手相資料永不寫入 predictions／prediction_feedback／
f8_assignments；抽取 prompt 含 P21 注入防線句、不附 GUARDRAIL（結構化抽取
非命理散文——tarot 同款例外，v2 interpret 上線時補回）。L0 示意為站內 SVG
線稿（標示「示意圖，非真人手掌」）。驗證：`verify-deployment.sh` Test 8c
（四端點 401）；名詞解釋含手相 9 詞（glossary-1.3）。完整設計（三輪外部
審查＋spike 11 模型實測）：
`docs/superpowers/specs/2026-10-27-palm-v1-design.md`。

## 興趣探索 v1（2026-10-31 上線）

已下架（2026-11-17，站長裁示；資料表保留待另案 drop）。

## 空間風水 v1（2026-11-19，Batch 1＋2 上線）

形勢派室內格局（臥室 8 題／書房（在家）7 題／辦公室座位 7 題），對照現代風水；spec：`docs/superpowers/specs/2026-11-17-fengshui-room-v1-design.md`；辦公室座位子單元見 `docs/superpowers/specs/2026-11-21-sub-units-design.md` §C。

- **辦公室座位（2026-11-21，規則集 v2／語料 v4）**：甲案——`Room::Study` 標籤改「書房（在家）」，新增 `Room::Office`（wire `office`、題目前綴 `office_`、標籤「辦公室座位」）有自己完整題組（背後有靠、門沖／正對主管、明堂、樑壓／燈管、壁刀、採光、雜物），只收有現代形家說法的題目。研究欄共用既有文獻（`sailer2021` 是唯一在辦公室做、也直接量座位位置的研究），沒有研究的標「沒人測過」。辦公室卡的調整只給桌面與椅子上做得到的事（測試擋 `move_`），問答與「可以試試看」都顯示 `OFFICE_SEAT_NOTE`「公司座位通常不能自己換，以桌面上能動的東西為主」。
- **網址**：`/fengshui` 是選房型開場（三欄，辦公室標 NEW）；`/fengshui/:room`（`bedroom|study|office`＝`Room::wire`）直達該房型問答，非法 room 以 `<Redirect>` 回 `/fengshui`。兩條路由共用 `FengshuiPage`，`:room` 變動由 Effect 重新開始該房型；「換房間」與清除答案都導回 `/fengshui`。

- **全案純前端**：`/fengshui` 公開頁，無 API、無 DB、無 migration。答案只存 localStorage `fengshui_room_v1_{room}`；翻卡猜測只在頁面狀態。
- 純邏輯與語料在 `crates/schema/src/fengshui/`（`ruleset`／`corpus`／`evidence`／`observe`／`date`）；版本常數 `FENGSHUI_ROOM_RULESET_VERSION`、`FENGSHUI_ROOM_CORPUS_VERSION`，使用者可見文字或判定改動必 bump。
- 卡片並列「風水怎麼說／研究怎麼說」，互不背書；研究徽章＝立場 `stance`（同方向／相反／沒差異或不一致／沒人測過）＋`reach`（direct／related／none）＋limitation，每張作答過的卡（含 ok）都能翻（Batch 2.1；語料版本 3）。預測大腦由「翻卡先猜」（猜研究站在哪一邊）互動承載，卡片內容不得用預測處理補故事。
- `evidence.rs` 是文獻唯一來源（`full_text_read` 由 T6 守門：direct 卡引用文獻必須全讀過）；核對筆記在 `docs/superpowers/reports/fengshui-research/evidence-ledger.md`。
- 紅線由 `corpus.rs` 測試守門：分段禁用詞（背書／恐嚇／化煞商品／理氣）、「煞」只在風水欄、長度上限。
- 閘門 A：站長自用後決定是否做 Batch 3（先猜後試，純前端）；API＋DB 版待閘門 B。

## 星座配對 v1（2026-11-20，階段 A 規則版＋階段 B AI 解讀，皆已上線）

點選對方太陽星座，看「你的太陽／月亮／上升 × 對方的太陽」互動傾向；plan：`docs/superpowers/plans/2026-11-19-zodiac-match.md`（§2.2–2.3、§6）。

- **規則版純前端**：`/zodiac` 公開頁，規則在 wasm 內算（`ft_schema::zodiac_match`），免登入零 API。登入且 `hasBirthData` → `GET /api/charts/western`（no-cache 走 ETag）帶入三星座；`birth_hour` 為空則不列上升、月亮列加註。
- 規則層：星座距離 d → 七相位（台灣術語，前端白話成「隔 N 個星座（X相）」）→ 6 類質性標籤（不給分數）＋元素 10 句＋模式 6 句；同星座的星體合併成一列（`match_rows`）。版本常數 `ZODIAC_MATCH_CORPUS_VERSION`，文案改動必 bump。
- 紅線由測試守門：12×12 全組合、對稱、d=0..6 表格、禁用字（注定／不合／速配／分手／%，含 `zodiac.rs`／`home.rs` 字面文案）。
- 前端：載入三態（Idle／Loading／Loaded／Failed）；await 後比對 uid 防登出殘留，且不得讀已卸載元件的 signal；結果變動即重播翻牌並捲入視窗；`aria-live` 只放一句 sr-only 摘要。
- **階段 B AI 解讀**（2026-11-20）：`POST /api/zodiac/match/interpret`（`routes/zodiac.rs`），body `{"target":"Leo"}`。順序：auth → 64KiB → 每分鐘 20 次 → 驗 target（400 `INVALID_SIGN`）→ **伺服器自讀西洋盤** `chart_resolver::resolve_western_chart`（不用 `resolve_chart`——紫微成功時它的 western 為 None；未填生辰 400 `NO_BIRTH_DATA`／DB 500／引擎 502，皆在計次前）→ 查快取（命中不計次）→ `rate_limit_strict_refundable` IP 10／user 3 per 24h（user 上限 429 `AI_DAILY_LIMIT`，其餘限流 429 `RATE_LIMIT`）→ AIMutexDO 專屬實例 `id_from_name("zodiac-match")`。
  - **失敗不落庫、一律退次**：AI 呼叫任何失敗（binding／stub 取得失敗、DO 例外、DO 非 200、stub、空內容、輸出檢查不過）都退 IP＋user 兩把票根（各退一次）；`AI_QUOTA`／`AI_QUEUE_FULL`／`AI_QUEUE_TIMEOUT` 回 503 原碼，其餘 503 `AI_UNAVAILABLE`。**與 tarot 不同**（tarot 的 stub／空內容計次）。
  - 落庫前輸出檢查 `output_ok`（純函式）：必含 `## 核心價值`；無 rising 不得出現「上升」；無 moon 不得有 `## 情感需求`；不得出現 你們的月亮／你們的上升／對方的月亮／對方的上升／%／％。
  - 只有成功才寫 `zodiac_match_readings`（除 `id` PRIMARY KEY 外無任何 constraint、無 NOT NULL，值由程式保證；讀取端容忍 NULL）；寫入失敗仍回結果並 `console_error!`（無細節）。**部署順序：migration `scripts/migrations/2026-11-19-zodiac-match.sql` 必須先於 API 部署**。生辰變更時刪除快取為 best-effort（失敗只記 log；sign_key 本就不會被新星座命中）。
  - 快取鍵＝`(user_id, target_sign, sign_key, corpus_version, prompt_version)`；`sign_key`＝`"Taurus|Aquarius|Virgo"`（無上升／不明為 `-`），target 先正規化。生辰變更（`PUT /api/users/me/birth`）一併刪除（best-effort）。
  - chartData 只含星座與規則層判讀（`rules[].pair` 明寫「你的月亮（水瓶座）× 對方的太陽（獅子座）」），不含生日／出生地／時辰；未填時辰不送 rising。prompt（`prompts.rs` `zodiac_match`）預設中文＋`GUARDRAIL_ZH`；有上升輸出四節（核心價值／情感需求／第一印象／小提醒）、無上升去第一印象、無月亮去情感需求；各節「本節談什麼」只供模型理解、明令不得抄進內文；禁「你們的月亮／上升」、禁斷言成敗與建議交往分手。**關係中立**（v2）：對方可能是伴侶／朋友／同事／家人，不預設戀愛／曖昧與性別，不用吸引力／來電／感情升溫等戀愛詞。`ZODIAC_MATCH_PROMPT_VERSION`（現為 "2"）改 prompt 必 bump。
  - 前端：只對「登入＋星盤 Loaded＋已選對方」顯示「產生 AI 解讀」，按鈕前揭露「星座組合會傳送至第三方 AI 服務生成，僅供娛樂與自我反思」；在途請求以 `ai_gen` 世代綁定——換對方／星盤即世代 +1、清結果、`ai_loading=false`（按鈕立即可用），舊回應世代不符整包丟棄（不碰 loading）；只有 `AI_DAILY_LIMIT` 顯示 3 次上限文案，其他 429 走通用「請求太頻繁」；AI 區不進 live region。
  - 實測（2026-11-20，gemma-4 不思考）：3 組 prompt 3–7 秒、約 17–21 neurons／次。

## 紫微子單元（2026-11-21：命盤／流年大運／今年運勢）

spec：`docs/superpowers/specs/2026-11-21-sub-units-design.md` §A（推翻 2026-08-26 spec 的流年非目標，
只開放大限＋小限＋流年＋流年四化；流月以下仍不做）。

- **網址**：`/ziwei` 子單元選單（公開；未登入卡片連 `/login?next=/ziwei/…`）；`/ziwei/chart`
  ＝`DivinationPage chart_type="ziwei"`；`/ziwei/luck`、`/ziwei/year`（Protected）。
  `DivinationPage` 改吃 `chart_type` prop，`/divination/:type` 走 `DivinationParamPage` wrapper；
  `/divination/ziwei` 靜態轉址 `/ziwei/chart`，**必須宣告在 `/divination/:type` 之前**。
- **引擎**：`ft_ziwei::horoscope(birth…, target)` → `ft_schema::ziwei_luck::ZiWeiHoroscopeV1`。
  x-iztro horoscope 索引是 **寅=0**，輸出一律重編為 **子=0**（與 `ZiWeiChartV3` 一致）並帶地支；
  干支／宮名／四化星轉 zh-TW；目標時辰固定午；`luckYear`＝出生農曆年＋虛歲−1（僅
  `AgeDivide::Normal` 成立，測試 `config_default_divides_pinned` 釘住三個 divide）；起運前
  `decadal.isChildhood=true`、無年齡區間。**禁用 `horoscope_now()`**（讀本機時鐘）。
  golden：1990-05-15 午時男 2026 丙午（流年命宮午、天同祿／天機權／文昌科／廉貞忌、虛歲 37、
  大限寅 35–44、小限辰）、春節前日期、春節前出生、童限年。
- **API**（`routes/ziwei.rs`）：生辰一律經 `chart_resolver::resolve_ziwei_birth`（**不可**用
  `resolve_chart`：缺紫微退西洋、缺性別略過；也不可用 `birth_hour.unwrap_or(12)`）→ 400
  `NO_BIRTH_DATA`／`NO_GENDER`／`NO_BIRTH_HOUR`。
  - `GET /api/ziwei/horoscope?year=Y`：省略＝台北今天（春節前屬前一年）；Y 以 `Y-7-1` 算；
    範圍出生農曆年～+100（400 `INVALID_YEAR`）；引擎失敗 502 `ENGINE_ERROR`（不 echo 細節）；
    不落庫，`private, max-age=300`＋`Vary: Authorization`；外層蓋 `engineVersionZiweiHoroscope`
    （`ENGINE_VERSION_ZIWEI_HOROSCOPE`，api 權威常數）。
  - `GET /api/ziwei/year` → `{summary, reading|null}`（no-store）。
  - `POST /api/ziwei/year/interpret`（無 body）：auth → 每分鐘 IP 20 次預限流 → 生辰 → 今天流年
    → 查快取（命中不計次）→ 本命盤（`resolve_ziwei_natal`）→ `rate_limit_strict_refundable`
    IP 10／user 3 per 24h（user 上限 429 `AI_DAILY_LIMIT`）→ AIMutexDO 專屬實例
    `id_from_name("ziwei-year")`，chartType `ziwei_year`、`language` 一律 `zh`。
    **任何失敗（stub、空字串、`output_ok` 不過、佇列滿／逾時、DO 例外）不落庫、退兩把、
    503 `AI_UNAVAILABLE`**。`output_ok`：禁「一定／注定／必然／保證」、`[一二…十０-９0-9]+月`、
    「預測」「本週」，去空白後 ≥200 字。結尾提醒「流年是傳統的觀察框架，不是預測。」由
    route 附加（AI 原文入庫，回應時附加）。
  - chartData：本命十二宮主星＋亮度＋生年四化、五行局、性別、目前大限（或童限）、流年命宮、
    流年十二宮名、流年四化（含落在本命哪宮）；**不送**生日、出生地、時辰、虛歲。資料在 routes
    層組裝，`services::ai` 不 import `symbolic`。`ZIWEI_YEAR_PROMPT_VERSION`（schema）改 prompt 必 bump。
  - 表 `ziwei_year_readings`（migration `scripts/migrations/2026-11-21-ziwei-year.sql`，只有
    PRIMARY KEY＋`(user_id, luck_year)` INDEX）；快取鍵＝(user, luck_year, engine_version_ziwei,
    horoscope_version, prompt_version)，先查後插、讀最新。`PUT /api/users/me/birth` 一併刪除
    （快取鍵不含生辰，刪除失敗回 500）。**不讀寫** predictions／prediction_feedback／f8_assignments／F4 strengths。
- **前端**：`pages/ziwei.rs`。流年大運＝大限時間軸（童限年前置「童限」段）、年份切換（預設 API
  回的今天 luckYear，範圍出生農曆年～+100）、`ZiWeiPalaceGrid` 選用 `overlay: LuckOverlay`（大限
  命宮／童限、流年命宮、小限徽章以地支定位；流祿／流權／流科／流忌小標；中心格年干支・虛歲・
  大限・流年四化）、規則說明（`ft_schema::ziwei_luck::RULE_*`，禁字測試）。今年運勢＝摘要 chips
  ＋AI 卡。`NO_BIRTH_HOUR` → 擋板＋前往 /profile；API 404 → 「服務準備中」。樣式 `zw-*`
  （style.css，色值全走 token；徽章配對在 `contrast-check.py` PAIRS「紫微流年」段）。
- 順帶修正：ziwei／story prompt 的性別比對原本只認 `male`／`female`，盤面存「男／女」→ 永遠送
  「未知」；改 `gender_zh` 兩種都認（不 bump，舊快取不追溯）。
- **部署順序**：Turso migration → engine → api → web。

## Kawaii light theme (2026-11-19)

Plan + review record: `docs/superpowers/plans/2026-11-19-kawaii-light-redesign.md`
(Kimi K01–K12, Fable F01–F21). Replaces cosmic-silver below.

- **Tokens**: every colour lives in `style.css` `:root`; old names kept with new
  values (`--heading/--text/--silver-dim/--silver-faint` = 標題/內文/次要/最淡),
  plus `--primary*`, `--accent`, `--ink-on-primary`, `--surface(-tint)`,
  `--success(-soft)`, `--rose(-soft)`, `--sign-<12>-deep/-light` + `--ink-on-sign`.
  Fixed-colour exceptions (camera overlay `--cam-*`, dark tarot back
  `--tarot-back-*`) are deliberate: their backdrop is not themed.
- **Ground**: `--void #fff5fa → --deep-space #f4eeff` + two faint glows; white
  opaque cards (no glass/blur), 20px radius, purple 3D primary buttons. Starfield
  (`galaxy.js`, sky divs, `gen-stars.js`) removed; `build-web.sh` wipes `dist/`.
- **Gold card rim (2026-11-20)**: `.card`/`.feature` border = 2px polished gold
  (`--gold-edge`, layered under a `padding-box` white fill so only the rim shows)
  + `--gold-glow`; a warm-white `--gold-glint` band sweeps the rim every 120s
  (~2s, `@keyframes gold-rim-glint`, 30s delay), off under reduced-motion. Decor
  only (not a control edge, not in contrast pairs). The first home feature card
  keeps its rainbow featured ring on top.
- **Guards**: `scripts/css-guard.py` (no literal colours outside `:root`, allowlist
  `scripts/css-guard-allowlist.txt` with reasons; every `var()` defined),
  `scripts/contrast-check.py` (token pairs incl. each sign stop),
  `scripts/contrast-audit.mjs` (Playwright, real composited pixels, text-fill /
  SVG / opacity / non-text). Tightest: Capricorn deep 4.80; `--success` only
  inside cards (4.40 on the deepest glow).
- **Western flip cards** (`divination.rs` `WesternView`, CSS `zc-*` at end of
  `style.css`): sun/moon/asc as 3D buttons, auto-flip on load (350/500/650ms),
  `aria-pressed` + stable sr-only name, faces `aria-hidden`, reduced-motion shows
  backs with no animation. Sign/planet/element names via `ft_schema::zodiac`
  (`sign_zh`/`sign_class`/`planet_zh`, case-insensitive, unknown → raw string).
- **reduced-motion** blocks sit at the end of the stylesheet (after the rules they
  override) — earlier placement silently disabled the tarot opt-out.

## Cosmic-silver theme + 星空藍 ground (2026-08-30 / 2026-09-16) — RETIRED 2026-11-19

*Superseded by §Kawaii light theme; kept for history.*

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
  `POST /api/predictions/generate`（同週可新增多個 prediction run；**F4 起必帶 body** `{"strengths":{五域 0–3}}`，
  缺/越界 → 400 `INVALID_STRENGTHS`）、`PUT /api/predictions/checks`（F6 第 1 段 absent|occurred）、
  `POST /api/predictions/:id/feedback`（F6 第 2 段 hit|miss|other）。
- **`cycle_id`**：Asia/Taipei 週一起算（`crates/schema/src/cycle.rs` 純函數，毫秒 ISO 解析、週一格式驗證）。
- **週期多批次生成（F4 原子 batch）**：`prediction_runs` 每次保存一份 profile＋五域強度快照，
  同週重新生成會新增 predictions，不覆蓋舊 prediction/feedback；`prediction_generations` 僅保留
  「本週曾生成」的相容摘要，`prediction_strengths` 回傳最新一批輸入。
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
top-level `sunSign`/`moonSign`), `CHART_SCHEMA_VERSION = 3`,
`ENGINE_VERSION_ZIWEI_HOROSCOPE = "1"`（紫微流年；改 `ft_ziwei::horoscope` 語意或任一 x-iztro
divide 設定必 bump，今年運勢快取以它為鍵）. Cache freshness is judged by
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
