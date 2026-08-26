# 設計：命理引擎現代化 ＋ Big5 人格×情境預測（rev.3）

日期：2026-08-26（rev.3：Grok＋Qwen 雙盲審裁決後定稿，見附錄 B）
狀態：待使用者最終核准
前置：引擎正確性修復已合入（`59b02f9`）

---

## 0. 決策紀錄（已與使用者確認）

| 決定點 | 選擇 |
|---|---|
| 引擎架構 | 導入 **iztro**（紫微）＋ **astronomy-engine**（西洋），自製版保留做對照基準（設 sunset 條件，§3.1） |
| 紫微 API 回應 | 升級為**完整盤**（四化、亮度、大限、三方四正） |
| 西洋範圍 | **完整本命盤**（十大行星＋ASC/MC＋主要相位，Whole Sign 宮位） |
| 時辰處理 | 接 `users.timezone` ＋ 真太陽時（僅用於斗數時辰判定，§3.3 切分原則）；晚子時採 iztro `dayDivide='forward'`（已對源碼證實） |
| Big5 測量 | TIPI 十題中性題幹；命理包裝只出現在結果呈現 |
| TIPI 融合 | 先驗＋校正：預測以實測向量為準；落差分析當洞察 |
| 預測模型 | 人格 × 情境交互 |

## 1. 目標與非目標

### 目標
A1. 紫微核心改由 iztro 計算，輸出完整盤（星曜＋亮度＋生年四化＋大限＋三方四正）
A2. 西洋核心改由 astronomy-engine：十大行星黃道經度（UT）、ASC/MC、主要相位
A3. 出生時間管線升級：前端表單補收 分鐘／時區／城市座標；timezone→UTC；斗數時辰採真太陽時（邊界才生效）
B1. TIPI 十題 → OCEAN 實測向量＋命盤象徵傾向（先驗）
B2. 人格×情境預測端點（新表、新 DO 請求形、規則錨點先行＋LLM 潤寫）

### 非目標
- 臨床級量表；宮位制選擇器；流年流曜細部暴露；付費牆整合
- **資料可攜（匯出）端點**——本期僅提供刪除權；匯出列為後續候選
- 圖表函式庫引入（雷達圖手繪 SVG：雷達圖僅五軸靜態，自繪約百行，避免在 gzip 預算敏感的 Workers bundle 上疊依賴；若實作工時超出預估一倍再重議）

## 2. 架構總覽與分期

```
A1 紫微換庫（含前端紫微盤重寫——現有顯示讀取不存在的欄位，無回歸風險）
A2 西洋換庫（行星 UT ＋ ASC/MC；前端本命盤渲染）
A3 時間管線（表單欄位 → UTC → 斗數 TST；per-type 版本遞增）
B1 人格側寫（schema + TIPI + 先驗 + 落差）
B2 預測（情境 + predict 端點 + 規則庫 + predictions 表）
```

每期獨立交付、獨立部署驗證。**舊前端相容**：PUT /me/birth 對缺席的 minute/timezone/lat/lng 一律走 assumed 預設路徑，永不回 400。

## 3. Phase A：引擎現代化

### 3.1 紫微 → iztro（v2.6.0，MIT）

adapter：`backend/src/services/ziwei/iztro-adapter.ts`

- API（已對 2.6.0 d.ts 核實）：`bySolar(solarDate: string, timeIndex: number, gender: GenderName, fixLeap?: boolean, language?: Language)`；
  `isLeapMonth` 僅存在於 `byLunar`。國曆輸入一律直接 `bySolar`，由 iztro 內部 lunar-lite 換農曆，
  **禁止**先過自製 solarToLunar 再餵 byLunar（兩套曆法疊加會使閏月邊界與 iztro 盤不一致）
- `timeIndex`：hour+minute → 0–12（早子 0…晚子 12）
- **晚子時**：iztro 內建預設 `dayDivide='forward'`（源碼 astro.js:39 `_dayDivide = 'forward'`，晚子按次日安星）。
  adapter 啟動時以 `config({ dayDivide: 'forward' })` **顯式設定**（不依賴隱含預設），
  `meta.dayDivide` 回報；23:00 案例列入 §5 測試。此決定使部分 23:00 出生用戶的盤與自製版不同屬預期修正
- `fixLeap = true`（前十五日本月、後半月次月，與自製版一致）
- `language = 'zh-TW'`；locale 不具 tree-shakable 性質，體積以 dry-run 實測為準
- 序列化：iztro Functional* 類別帶實例級 `toJSON()`（自訂 serialize，不觸發巢狀呼叫）；
  但 V3 形狀與 iztro 不同，adapter **一律逐欄映射**，不直接 stringify 類實例
- **座標系**：iztro 宮位陣列寅起 0；V3 一律地支序（子=0）。映射 helper：
  `branchIndexOf(palaceIndex) = (palaceIndex + 2) % 12`（寅=2），以 adapter 錨點測試釘死
- 映射到 `ZiWeiChartV3`：palaces[12]（地支序；每星 name/brightness/化耀）、majorLimits[]、
  三方四正索引、fourPillars（取自 iztro）、meta（dayDivide/isLeap/fixLeap/timeIndex/hourShifted/assumed）
- 自製 calculator 保留匯出；**sunset 條件**：A1/A2 錨點測試全綠且上線滿兩週無 parity 異常後，
  自製版退出 bundle（保留於 git 歷史供對照）

### 3.2 西洋 → astronomy-engine（v2.1.19，MIT）

新檔：`backend/src/services/western/natal.ts`

- 行星：`GeoVector(body, t_UT, true)` —— 第三參數是 **aberration**（光行差，apparent 位置），非座標系旗標；
  回傳 J2000 赤道直角座標，再 `Ecliptic(vec)` 得真黃道經緯度。月亮直用 `EclipticGeoMoon(t_UT)`
- 角距優先用 **`PairLongitude(body1, body2, date)`**（d.ts:1321 已核實）；逆行判定：相鄰 Δt=1 天經度比較
- ASC/MC（施工級規格）：
  ```
  GAST = SiderealTime(t_UT)                       // Greenwich *apparent* sidereal time，小時 [0,24)
  LST  = (GAST + eastLongitudeDeg/15) mod 24      // 東經為正
  RAMC = LST × 15°                                // 度
  ε    = 23.4392911° − 0.0130042·T − 1.64e-7·T²   // IAU 低階多項式，T = 自 J2000 起的儒略世紀數
  MC   = atan2( sin RAMC, cos RAMC · cos ε )
  ASC  = atan2( cos RAMC, −(sin RAMC · cos ε + tan φ · sin ε) )   // φ = 緯度
  ```
- 宮位：Whole Sign（ASC 所在星座為第一宮）
- 相位容許度：{合0±8, 六合60±6, 刑90±7, 三合120±8, 沖180±8}，常數集中定義
- 星座判定一律黃道經度 ÷30；廢除固定日期表與舊近似函式

### 3.3 時間管線（P0 切分原則：TST 只服務斗數時辰，絕不進入西洋星曆）

```
birth y/m/d/h/min + timezone(IANA)
  ├─→ UT（一次，Intl 換算）──────────────→ 西洋 GeoVector / EclipticGeoMoon / SiderealTime（星曆以 UT 為準）
  └─→ 斗數時辰：LMT = UT + lon×4min
              TST = LMT + EoT           // EoT = 視太陽時 − 平太陽時（NOAA 閉式近似公式實作於 services/western/eot.ts）
              僅當 TST 與鐘錶時間跨過時辰邊界 ±20 分內才採 TST 定 timeIndex
```

- 缺資料決策表（禁止「一律台北」）：

| 有 timezone？ | 有經緯度？ | 行為 |
|---|---|---|
| ✓ | ✓ | 全功能：UT＋TST＋ASC/MC |
| ✓ | ✗ | 西洋：行星可算、**ASC/MC 降級不可算**（null＋assumed 標記）；斗數：跳過 TST，用鐘錶時辰＋assumed:true。**不得拿台北經度配外國時區** |
| ✗ | — | 預設 `Asia/Taipei`，全鏈 assumed:true 揭露 |

- 城市座標：台灣**鄉鎮市區**級靜態對照表放前端、隨請求上送（鄉鎮中心誤差 ≈ 時間 2 分內，遠小於 ±20 分邊界窗，可接受）
- **A3 前置＝前端 BirthDataForm 升級**（分鐘／時區下拉／鄉鎮座標或 opt-in 定位）；未升級前 TST/ASC/MC 不得上線

### 3.4 版本、快取與相容（rev.3：per-type 版本）

- **分類版本**：`ENGINE_VERSION_ZIWEI` 與 `ENGINE_VERSION_WESTERN` 各自內嵌於所屬 chart_data；
  bump 只失效對應類型，避免 A2 上線把全站紫微盤連帶重算（並連帶清掉有效的 AI 解讀）
- 初值皆 `'3.0.0'`；語意決策樹：**計算演算法變** → bump 該類 ENGINE_VERSION；
  **回應 JSON 形狀變** → bump 頂層 `chartSchemaVersion`（前端適配用，不觸發重算）；兩者同時變就都 bump
- `POST /:type/interpret` 守衛：解析既有 `chart_data.engineVersion` ≠ 當前該類版本 → `409 RECALC_REQUIRED`
  ；ETag 摻入對應版本。**前端行為定義**：收 409 → 自動 GET /:type 重算一次 → 成功後重送 interpret；
  再失敗才 toast 錯誤（不自動循環）
- 回應採加欄位策略：V3 新欄位全新增、V2 欄位保留過渡期、頂層 `chartSchemaVersion: 3`

## 4. Phase B：Big5 人格×情境預測

### 4.1 資料模型（D1，no-constraints 慣例）

users 表欄位已齊備（birth_minute/timezone/latitude/longitude 均存在於 schema.sql，無需 ALTER）。新增三表：

```sql
CREATE TABLE IF NOT EXISTS personality_profiles (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  tipi_answers TEXT NOT NULL,        -- JSON [10] int 1–7
  ocean_measured TEXT NOT NULL,      -- JSON 五維 0–100，(mean−1)/6×100
  ocean_prior TEXT,                  -- JSON 同尺度「命盤象徵傾向」
  prior_source TEXT,                 -- 'ziwei'|'western'|null
  measurement_status TEXT NOT NULL DEFAULT 'complete',  -- 'complete'|'skipped_prior_only'
  item_duration_ms INTEGER,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS situation_checks (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  domains TEXT NOT NULL,             -- JSON {work,love,family,money,health} 0–3
  target TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS predictions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  profile_id TEXT NOT NULL,
  situation_id TEXT NOT NULL,
  divination_type TEXT NOT NULL,     -- 'ziwei'|'western'
  anchors TEXT NOT NULL,
  prediction_text TEXT NOT NULL,
  cache_key TEXT NOT NULL,
  rules_version TEXT NOT NULL DEFAULT 'rules-1',
  prompt_version TEXT NOT NULL DEFAULT 'prompt-1',
  created_at TEXT DEFAULT (datetime('now'))
);
```

遷移路徑：`backend/scripts/migrations/*.sql`，以 `unset CLOUDFLARE_API_TOKEN && npx wrangler d1 execute fortunet-db --file <file> --remote|--local` 施行。
`interpretations` 的 CHECK 殘留 `'bazi'`：predictions 有獨立表不受影響；SQLite 移除 CHECK 需重建表，風險大於收益，**保留殘留並記錄**。

版本遞增策略：RULES_VERSION/PROMPT_VERSION 手動語意遞增（規則集或 prompt 模板實質變更才動）；舊 predictions 保留但被新版本取代。

### 4.2 方法論（雙審修訂版）

- **題幹中性**：TIPI 十題公開繁中版語意對，作答畫面零命盤資訊（防 priming）；命理包裝只在結果頁
- **先驗命名與出處**：UI 稱「**命盤象徵傾向**」（不稱命格人格）；`priors.ts` 每條規則附來源等級標注
  （古典文本歸納／設計者判斷），透明可 review
- **落差門檻**：TIPI test-retest 信度有限（±15 分量級噪声），落差 <15 分不呈現差異敘事，僅顯示兩向量
- **亂答偵測**：作答時長／全端作答／正反題矛盾三項；觸發行為＝提示重測一次，仍失敗才存為
  `measurement_status='skipped_prior_only'`；使用者主動跳過亦同。predict 對 skipped 狀態回 `409 MEASUREMENT_REQUIRED`
- **量尺**：每維兩題均分（反向翻轉）→ `(mean−1)/6×100`；先驗調整量 ±20/±10/0（100 分制），多星命中取平均，clamp [0,100]；
  兩盤並存時 prior_source 以 ziwei 為準

### 4.3 先驗規則表與領域映射（純規則，LLM 不打分）

- `services/personality/priors.ts`：主星/星座 → 特質調整透明對照表（含口訣出處）
- `services/personality/rules.ts`：trait×domain 錨點（高N×money高壓→壓力放大等），`RULES_VERSION` 常數；
  領域→宮位固定映射：work→官祿、love→夫妻、family→田宅、money→財帛、health→疾厄

### 4.4 API 與 AI 接線

| 端點 | 方法 | 備註 |
|---|---|---|
| `/api/personality/tipi` | POST | 驗證：長度恰 10、整數 1–7、伺服器端最短作答 ≥5s |
| `/api/personality/me` | GET | 最新側寫 |
| `/api/personality/me` | DELETE | 個資刪除權：清 personality_profiles/situation_checks/predictions 三表 |
| `/api/personality/situation` | POST | domains 0–3 校驗 |
| `/api/charts/:type/predict` | POST | authMiddleware＋獨立 rate limit（10/min/IP） |

predict 流程：錨點 JSON（OCEAN＋domains＋宮位活化＋規則命中；**不上傳原始十題**）→ AIMutexDO 新增
`{ kind:'predict', payload }` 分派（現有 handleRequest 只認 interpretRequest）。**佇列策略明示**：
v1 共享單一 FIFO、不設優先級；DO metrics 加 per-kind queue depth；p95 等待超標再議分流。
潤寫輸出過 schema 校驗（禁止新增錨點外因果）→ 存 predictions。
快取鍵 = hash(ocean_measured + domains + target + divination_type + birth_data_hash + RULES_VERSION + PROMPT_VERSION + 對應 ENGINE_VERSION)；
`birth_data_hash` 由 predict 當下讀 users 表後呼叫**共用的 computeBirthHash**（自 routes/users.ts 抽至 `services/birth-hash.ts`，兩處共用同一實作）取得，禁止重寫第二份演算法。

個資合規（上線前檢核）：同意基礎（主動作答視為同意，UI 聲明目的）、保存期限（跟隨刪除權即時清除）、
LLM 傳輸揭露（僅傳錨點不含原始答案）。註：TIPI 答案未必落入台灣個資法 §6 特種個資法定類別，
惟產品自我要求按高標準處理。

### 4.5 前端

- `/personality`＋ProtectedRoute：問卷（中性題幹）→ 結果頁（SVG 雷達疊加實測/象徵傾向＋落差文字；落差<15 分僅並列）
- DivinationPage 重寫：紫微十二宮卡片（亮度/四化徽章/大限列）、西洋行星表＋相位列表；情境勾選→預測區塊
- 統一錯誤處理：409 RECALC_REQUIRED（自動重算流程）、409 MEASUREMENT_REQUIRED（導向問卷）、429/400 toast 對照表
- BirthDataForm 升級（§3.3）；隱私文案（目的限定、刪除入口、「趨勢參考非診斷」免責）

## 5. 驗證策略

- **已提交 integration tests**（RUN_INTEGRATION=true 才跑）：
  - iztro 錨點：出版範例盤（固定輸入→預期命宮主星＋四化），不用名人生辰
  - 西洋錨點：astro.com 對照，鎖 tropical/geocentric/apparent；行星 ±1°、ASC/MC ±2°；**不比宮頭**（Placidus≠Whole Sign）
  - 23:00 案例（dayDivide='forward' 行為）、閏月案例、interpret 對 stale engineVersion 的 409 流程
- 拋棄式腳本續用於開發期曆法錨點（JDN 法）；擋迴歸靠上述已提交 tests
- 上線門檻：`wrangler deploy --dry-run` gzip<3MB；CPU 以實測為準（目標 <10ms/request），超標拆 DO 或升付費
- 部署前 commit；`wrangler dev` 本地驗證需使用者確認

## 6. 已知差異清單（自製 vs iztro）

| 情境 | 自製版 | iztro | 備註 |
|---|---|---|---|
| 23:00 出生 | 子時、當日日柱 | dayDivide='forward'：次日安星 | 已定案，§3.1 |
| 閏月 | 十五日界 | fixLeap=true 相同 | 應一致 |
| 四化/亮度/大限 | 無 | 有 | 功能增量非衝突 |

## 7. 風險與緩解

| 風險 | 緩解 |
|---|---|
| iztro 依賴鏈超過 gzip 上限 | dry-run 量測門檻；Workers 不支援動態 import、單 bundle 無 code-split 可救——超標即啟用自製引擎 fallback（架構保險，非可選項） |
| Workers 10ms CPU 免費層 | 上線前基準測試 cache-miss 全路徑；超標拆計算到 DO 或升付費 |
| 真太陽時讓老用戶的盤變了 | meta.hourShifted 揭露＋release note；TST 只在邊界 ±20 分生效 |
| LLM 幻覺 | 錨點先行＋schema 校驗＋prompt 只含結構化錨點 |
| 心理資料保護 | 刪除端點、目的限定、原始答案不出本地、UI 免責 |
| 前後端切換窗口 | 加欄位策略＋chartSchemaVersion |

---

## 附錄 A：rev.2 修訂紀錄（Grok 審查裁決）

30 條發現：28 採納、2 留驗。P0 三項全數證實並修入：真太陽時時間軸切分（§3.3）、缺輸入資料決策表與表單前置（§3.3/A3）、
TIPI 去 priming（§4.2）。我方獨立驗證四項事實：wrangler.toml 無 nodejs_compat ✓；DivinationPage.tsx:105 讀取後端不存在欄位 ✓；
schema.sql:37 CHECK 殘留 'bazi' ✓；BirthDataForm 未收集 minute/lat/lng ✓。

## 附錄 B：rev.3 修訂紀錄（Qwen 3.8 Max 盲審 × 套件實物驗證的三方裁決）

Qwen 30 條：約 20 條採納/部分採納、2 條 **P0 級幻覺被實物證據駁回**、其餘由原始碼探針解案。

**實物驗證結果**（npm pack 拆 2.6.0/2.1.19 tarball 直查源碼）：
| 待定點 | 結論 | 裁決 |
|---|---|---|
| iztro bySolar 簽名（Qwen 稱 rev.2 寫錯，P0） | d.ts:63 與 rev.2 完全一致 | **Qwen 幻覺，駁回** |
| users 表缺 timezone（Qwen 稱需 ALTER，P0） | schema.sql:18-21 timezone/latitude/longitude/birth_minute 全都在 | **Qwen 幻覺，駁回** |
| dayDivide 預設值（懸案） | astro.js:39 `_dayDivide='forward'` | **定案：採 forward、顯式設定** |
| toJSON 存在性 | FunctionalStar/FunctionalSurpalaces 帶實例級 toJSON | 定案：存在，但仍逐欄映射 |
| PairLongitude 存在性 | d.ts:1321 `PairLongitude(body1, body2, date)` | 定案：採用（Qwen 給的四參簽名也不對） |
| obliquity 來源（Qwen 提 e.Tilt(t)） | astronomy-engine 無地球 obliquity API | 改 IAU 多項式寫死 §3.2 |

**Qwen 實質貢獻（採納）**：per-type 引擎版本號避免跨類型無謂重算（§3.4，Grok 與我都漏了）、先驗來源等級標注與
「命盤象徵傾向」命名（§4.2）、TIPI 噪声→落差<15 分不敘事（§4.2）、skipped 狀態機觸發條件（§4.2）、
409 前端行為與統一錯誤處理（§3.4/§4.5）、佇列 FIFO 明示＋queue depth 監控（§4.4）、cache_key 共用 computeBirthHash（§4.4）、
自製引擎 sunset 條件（§3.1）、版本語意決策樹（§3.4）、gzip 無 code-split 出路的風險措辭（§7）、資料可攛列非目標（§1）。

**部分駁回**：個資法特種個資分類（法定類別不含心理測驗答案，採高標準自我要求但不引用錯誤法條）、
雷達圖改用圖表庫（bundle 預算理由成立，維持手繪並設重議門檻）、bazi CHECK 重建清理（SQLite 需重建表，風險>收益，記錄保留）。

教訓：Qwen 的兩條 P0 若未經拆包驗證直接採信，會改錯正確的 API 呼叫並寫出多餘的資料庫遷移——
**外部審查的事實性主張必須以實物證據裁決，方法論主張按品質裁決**。
