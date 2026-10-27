# FortuneT V2 塔羅牌功能設計（2026-10-27 定稿；v1 已實作）

審查軌跡：Muse 初審（10 項 blocking）→ 二稿 Muse 複審＋Codex 初審 → 三稿 Opus 終審（4 項 blocking）→ 逐條核實全數屬實後修訂。零誤報紀錄：三家審查所有可查證主張經原始碼核實，全部成立。

## 第一部分：v1 設計（已實作於 c4e9828/4ad03f1/e20d3bc）

防護姿態：算命娛樂站，**不做防重擲**。防護僅兩類：資料衛生（冪等、無重複列）
與成本防護（限流防灌 DB／防 AI 費用）。

## 版本常數

- `TAROT_CORPUS_VERSION = "1"`：牌義語料版本（比照 `PRACTICE_CATALOG_VERSION`）。
- `TAROT_DRAW_ALGO_VERSION = "1"`：**種子編碼＋PRNG 演算法版本**。兩者任一改動
  都會讓「同日同裝置同牌」改變，故演算法變更必須 bump 此常數（文件明文，
  防改版後使用者同日換牌被當 bug）。

## 洗牌／隨機核心（v1a/v1b 共用，`crates/schema` tarot 模組）

- `shuffle(deck, entropy: &[u8]) -> Vec<DrawnCard>`（含逆位判定）。
- **熵流語意（定案）**：`entropy ≥ 32 bytes`；`state = fnv1a_64(entropy)`
  （純整數、native+wasm 皆可、站上 api 已有同型先例），之後每步
  `state = splitmix64(state)` 取 u64；取樣用拒絕取樣
  （`v < floor(2^64/n)*n` 才取 `v % n`），杜絕取模偏差；位元組切片**不循環
  重用**。
- **取用順序（定案）**：位置 0→2（現況→阻礙→建議），每位置先抽牌（自剩餘
  池）、後抽逆位（取該步 u64 最低位元：`state & 1 == 1` → 逆位，p=50%）。`daily_draw` 同法抽 1 張＋逆位。
- **v1a 含逆位（產品決策）**：今日抽牌同樣有正／逆位。
- 注入源：v1b 用 api `secure_bytes(32)`，`None` → 500 generic（fail-closed）；
  **明文禁止** `Math::random` 作為任何抽牌隨機源。
- 測試（native `cargo test -p ft-schema`）：固定 entropy 的 golden 斷言（整組
  輸出逐項釘死）；大樣本分佈 sanity（各牌頻率與逆位比例的均勻性範圍斷言）；
  `taipei_today` 邊界測試（台北 00:00 前後一秒）。

## v1a 今日抽牌（匿名、純前端）

- 命名：**「今日抽牌」**（避免暗示全站強制一日一次）。
- 零 API、零 DB、免登入；route `/tarot`（公開，比照 `/naming` 不包 `Protected`）。
- **台北日期推導（定案）**：新增 `ft_schema::tarot::taipei_today(utc_ms) -> "YYYY-MM-DD"`
  ——UTC+8 純算術（比照 `cycle.rs`，無 `js_sys`），前端以 `Date.now()` 餵入。
  不用裝置本地時區（美西裝置會在本地午夜換牌，違反契約）。
- **種子契約（定案）**：`seed = "{taipei_date}|{device_seed}"` 的 UTF-8 bytes；
  `device_seed` = 16 bytes → 32 hex 字（種子總長 10+1+32=43 bytes，滿足
  entropy ≥ 32 契約；`crypto.getRandomValues`，不可用時 fallback
  `Math.random`——**種子為體驗用途非安全邊界**，與「伺服端抽牌隨機源禁
  `Math::random`」不衝突），存 localStorage，首訪生成。同裝置同日同牌；
  換日自動換牌。
- **結果契約（定案）**：顯示牌代碼、牌名、正／逆位、牌義（≤120 字）、關鍵詞。
  **無歷史紀錄、昨日結果不保留**（明文取捨）；重載同日同牌。
- **降級（定案）**：localStorage 不可用（隱私模式）→ 種子改存 in-memory，
  該次工作階段內穩定、重載後換牌（明文降級取捨，不擋路）；頁面顯示
  「依台北時間每日更新」一行提示。
- 語料改版後同日重載會顯示新文案（牌不變）——明文可接受。

## v1b 三張牌陣（登入）

### 產品契約（定案）

- 固定三張：**現況／阻礙／建議**；78 張不重複抽取；正逆位獨立 50%（寫死）。
- `question` 選填，**以字元數計**（`chars().count() ≤ 200`，trim 後空字串視為
  未填存 NULL）。
- 解讀為規則式：組裝函數放 `crates/schema`（純邏輯，native 可測），模板 = 每張「位置引言（6 條：3 位置×正逆，每條 ≤40 字）
  ＋牌義（≤120 字）」；`interpretation` 總長上限 600 字（入庫前驗證）。

### API（皆帶 `/api` 前綴；DTO wire keys camelCase，`serde(rename)` 慣例）

- `POST /api/tarot/draw`，body `TarotDrawRequest { spread, question?, requestId }`。
  **處理順序（定案，比照 practice／predictions 全站先例——IP 限流最先）**：
  `rate_limit("tarot:ip:{ip}", 20, 60_000)` → auth → body cap → parse → 驗證
  （`requestId` UUID 形狀檢查、`spread` ∈ `"three"`、question 字數；失敗 400
  `INVALID_TAROT`）→ **冪等查詢**（同 `requestId`：**同 body**（比對
  `spread`＋正規化後 `question`：trim-NULL 後比對）→ **200** 回原結果；異
  body → 409 `IDEMPOTENCY_CONFLICT`；比照 practice `matches_plan` 先例）→
  uid 24h 限流（放這裡是為讓冪等重放不受 429 擋）→ 抽牌 → 寫入（UNIQUE 衝突
  → 回查比對，同 practice）→ **201** `TarotReadingView`。401 先於 body cap／
  parse（rate-limit-first 不影響 `verify-deployment.sh` 401 probe——每端點
  僅打一次，predictions 已證可行）。
- 限流（定案）：`rate_limit("tarot:draw:{uid}", 20, 86400000)`——**「約 24 小時
  固定視窗內 20 次的濫用緩衝」**（RateLimitDO 為首次命中起算的固定視窗、
  fail-open；非曆日、非硬上限——v1b 無 AI 成本，接受此取捨；真硬上限留 v2
  AI 路徑處理）。
- `GET /api/tarot/readings`：分頁**比照 practice `page_query` 實作**（以實際
  程式碼為準）；`no-store` 不快取；回 `{items, nextCursor}`。
- `TarotReadingView { id, question, spread, cards: [TarotCardView { id, name,
  reversed, position }], interpretation, corpusVersion, createdAt }`。
  牌 `id` 為字串代碼（如 `major_08`、`cups_05`，語義化且穩定）；`position` ∈
  `situation|obstacle|advice`。
- `DELETE /api/tarot/readings/:id`：僅本人紀錄；不存在／他人／重刪一律 **404**
  （不洩漏存在性）。`DELETE /api/tarot/readings`（整批）→ `{deleted: n}`（與 practice 的 `{"deleted":true}` 不同形：n 供前端提示「已刪 N 筆」，屬刻意差異）。
  question 屬私密自述，刪除為一等公民。

### DB（`schema.sql` ＋ `scripts/migrations/2026-XX-XX-tarot.sql` 雙更新）

```sql
CREATE TABLE IF NOT EXISTS tarot_readings (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  request_id TEXT NOT NULL,
  question TEXT,                    -- 選填，≤200 字元，可刪
  spread TEXT NOT NULL CHECK (spread IN ('three')),
  cards TEXT NOT NULL,              -- JSON 字串（站上慣例）
  interpretation TEXT NOT NULL,     -- 規則式成品快照
  corpus_version TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(user_id, request_id)
);
CREATE INDEX IF NOT EXISTS idx_tarot_user_created ON tarot_readings(user_id, created_at);
```

- 儲存語意：存成品快照＋版本戳（歷史即時呈現；重渲染留 v2 評估）。

### 五域映射

- **v1 不含五域映射**（v1 無任何消費者——審查意見成立）。tarot 自有對映表
  （不共用 `predict` 型別）留待 v2+ 真正需要時定義。塔羅資料永不寫入
  predictions／prediction_feedback／f8_assignments（紅線，保留明文）。

### 語料

- 156 條牌義（≤120 字／條）＋6 條位置引言；AI 起草、人工終校；逆位不得僅是
  正位加否定；不翻譯、不改寫 ekelen/Tarotoo 等英文資料集。

### 前端

- 路由：`/tarot`（公開：今日抽牌＋三張牌陣 CTA）；`/tarot/spread`（Protected：
  抽牌＋同頁下方歷史列表，點列展開完整解讀——單頁設計，少一條 route）。
- **登入回流（定案）**：公開頁 CTA → `/login?next=/tarot/spread`；login 頁支援
  `next` 參數（**僅接受本站相對路徑：開頭 `/` 且非 `//` 與 `/\`，並測 `%2F%2F` 與 `/%5C` 案例，否則 fallback
  `/profile`**——防 open redirect；未帶參數行為不變）。
- 卡面符號化：**先交付樣本**（大牌×2、小牌×2、正逆位樣式、手機尺寸）再估
  全套工時——獨立設計交付，非 CSS 順手之作。
- 翻牌 a11y：卡為 button（Enter/Space 可翻）、翻後 focus 移至結果卡、
  aria-label 含牌名與正逆位；`prefers-reduced-motion` 下直接揭牌不播動畫
  （`style.css` 現行 media query 需補卡面規則）。
- 語料體積：粗估 156×120 字 ≈ 55 KiB（UTF-8 未壓縮）＋關鍵詞＋模板；原驗收
  標準為「實建後 gzip 增量 >30 KiB 則改為 `/tarot` route 進入時動態載入」。
  **2026-10-27 實測修訂**：整體 gzip 增量 49 KiB（500.1→550.9 KiB，+10%）。
  拆解顯示頁面／客戶端／DTO 程式碼本身即約 35 KiB——單 bundle SPA 架構下，
  語料無論如何載入都無法使增量低於 30 KiB 線；語料 lazy-load 至多再省約
  1/3，卻引入 runtime fetch 失敗面與 build-time 匯出步驟。**決策：接受
  49 KiB，不採動態載入**；若日後 bundle 增長實際影響首屏，再一併重議 SPA
  分包（屆時塔羅語料與頁面一起切，而非單獨處理語料）。
- 首頁：feature-grid **末位**新增第七卡（紫微維持第一、既有卡序不動；桌面
  `auto-fit` 末列孤卡可接受）；整卡可點（比照姓名學卡）；文案標明
  「今日抽牌免登入／三張牌陣需登入」；敘事定位為**當下問題的反思工具**
  （紫微／西洋是命盤、Big5 是人格、F5 是本週預測——塔羅不引用命盤資料，
  界線保留）；**不得暗示 AI**（v1 無 AI，與既有「AI 解讀」卡區隔）。
- 用語統一：正位／逆位、現況／阻礙／建議、解牌。

### 驗收與部署

- `verify-deployment.sh` 加 401 probe（同 F5 先例；handler 順序保證 401 先於
  body cap／限流）。
- `docs/ARCHITECTURE.md` 同步。
- v1a 與 v1b 的 **web 同包部署**（v1b API 先上，web 一次包今日抽牌＋牌陣
  兩頁——消除 v1a 先上時 CTA 指向未存在 route 的死鏈）。schema 語料會同時
  內嵌 api worker bundle（api 依賴 schema；v2 AI prompt 本來就要用，取捨
  可接受，動態載入方案只救 web 的 wasm 體積）。首頁 hero-actions 列不動
  （入口只在 feature-grid）。

---

## 第二部分：v2 契約（AI 命盤感知解牌——v2 開工前凍結版）

前置：v1 上線後才動工。以下為 v2 實作前須定案的契約（吸收 Muse 二輪＋Codex 初審）。

1. **AI 欄位遷移（與 v1 schema 相容，定案）**：新增可空欄位 `ai_interpretation
   TEXT`、`ai_status TEXT CHECK (pending|done|failed)`、`ai_prompt_version TEXT`、
   `ai_provider TEXT`、`ai_model TEXT`、`ai_generated_at TEXT`。**規則式
   `interpretation` 永不覆寫**（歷史快照原則）；AI 失敗＝`ai_status=failed`＋
   空 `ai_interpretation`＋前端重試入口；AI 成功前顯示規則式解讀。附
   `scripts/migrations/` 遷移檔。
2. **計次語意（定案）**：AI 呼叫**嘗試即計次**（含失敗——防無限重試燒額度）；
   每用戶每日 3 次（`tarot:ai:{uid}`）＋每 IP 疊加（`tarot:ai:ip:{ip}`）——
   user 鍵擋單帳號、IP 鍵擋多帳號。
3. **fail-closed（定案）**：AI 成本端點不得沿用 fail-open 的 `rate_limit()`；
   新增 `rate_limit_strict`（限流器／DO 不可用時**拒絕**而非放行）。
4. **AIMutexDO 隔離（定案，v2 前置改動）**：現行 `QUEUE`／`PROCESSING` 是
   `std::thread_local!`（per-isolate，**不是** per-DO-instance）——僅改
   `id_from_name("tarot")` 不保證獨立佇列（同 isolate 共置即共享）。須先把
   佇列遷到 DO struct 的 per-instance 欄位（`RefCell`，instance 存續期間
   保留），再以 `id_from_name("tarot")` 取專屬實例；上線時驗證兩佇列互不
   影響。
5. **Prompt 邊界**：question 以明確分隔符包入；system prompt 釘死「塔羅為
   娛樂與自我反思定位，不得給醫療／法律／財務確定性斷言」。
6. **question 外送知情（定案）**：question 原文會送第三方 AI（iFlow／Groq／
   Cerebras failover 鏈）——AI 解牌按鈕旁常駐小字「你的問題將傳送至第三方
   AI 服務生成解讀」；首次使用時再明示一次。分隔符處理的是注入，不是知情，
   兩者都要。
7. **命盤感知為可控制契約（定案）**：抽牌頁提供「純塔羅解牌」選項（不含
   命盤 context）；使用者事後修改生辰，**舊 AI 解讀照舊顯示**（歷史快照
   原則，不失效）。
8. **命盤摘要定義（定案）**：紫微命宮主星等級短文（非整盤截斷 JSON）；
   F4 五域＝**僅當週 `cycle_id` 的 `prediction_strengths` 自評快照**，無當週
   列＝無此 context（不得拿上週自評餵本週解牌）；摘要組裝放 routes/services
   層，**不進 `services::ai`**（`services::ai` 不得 import `symbolic` 紅線）。
9. **降級矩陣**：無生辰 → 純塔羅 AI 解牌；有生辰無快取 → 觸發解析或跳過
   該 context；engine 失敗 → 不擋抽牌與規則式解讀。文案區分「未填生辰」與
   「暫時無法取得」（chart_resolver「不得共用文案」教訓）。
10. **資料最小化**：摘要僅帶解牌所需欄位，**不帶原始出生時地**。
11. **offline stub 禁入庫（含偵測機制）**：AIMutexDO 全 failover 失敗時回
    **200＋stub 文案＋`stub:true`**（現行 chart 路徑不檢查該旗標）。塔羅
    呼叫端**必檢查回應 `stub` 旗標，`stub:true` 一律視為 failed**
    （`ai_status=failed`、不寫 `ai_interpretation`）；建議 chart 路徑一併
    補檢查（既有行為變更，另立單評估）。
12. **端點形狀（定案）**：`POST /api/tarot/readings/:id/interpret`（同步）；
    `ai_status` 只有 `done|failed`（同步下 pending 不可見，移除）；重試＝
    再打同端點。AIMutex wire：`chartType:"tarot"`、`chartData`＝
    `{cards, question, chartSummary?}` JSON、`max_tokens` 1024。
13. **限值與措辭**：每用戶 3 次＋每 IP 10 次（皆首命中起算 24h 固定視窗，
    非「曆日」——同 v1 措辭）；計次邊界＝進入 AI 呼叫即計（含 queue-full
    503 與失敗）。
14. **RWS 圖像**：v3+ 議題——自托管 Pages 靜態資產＋逐檔核對掃描授權標示，
    v1/v2 不碰。
