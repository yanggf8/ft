# 設計：命理引擎現代化 ＋ Big5 人格×情境預測（rev.2）

日期：2026-08-26（rev.2：依外部審查修訂，見附錄 A）
狀態：待使用者審查
前置：引擎正確性修復已合入（`59b02f9`）

---

## 0. 決策紀錄（已與使用者確認）

| 決定點 | 選擇 |
|---|---|
| 引擎架構 | 導入 **iztro**（紫微）＋ **astronomy-engine**（西洋），自製版保留做對照基準 |
| 紫微 API 回應 | 升級為**完整盤**（四化、亮度、大限、三方四正） |
| 西洋範圍 | **完整本命盤**（十大行星＋ASC/MC＋主要相位） |
| 時辰處理 | 接 `users.timezone` ＋ **真太陽時**（僅用於斗數時辰判定，見 §3.3 切分原則） |
| Big5 測量 | TIPI 十題（中性題幹，命理包裝只出現在結果呈現） |
| TIPI 融合 | 先驗＋校正：預測以實測向量為準；落差分析當洞察 |
| 預測模型 | 人格 × 情境交互 |

## 1. 目標與非目標

### 目標
A1. 紫微核心改由 iztro 計算，輸出完整盤（星曜＋亮度＋生年四化＋大限＋三方四正）
A2. 西洋核心改由 astronomy-engine：十大行星黃道經度（UT）、ASC/MC、主要相位
A3. 出生時間管線升級：前端表單補收 分鐘／時區／城市座標；timezone→UTC；斗數時辰採真太陽時（邊界才生效）
B1. TIPI 十題（繁中標準題幹）→ OCEAN 實測向量
B2. 命盤先驗規則表 → 命格傾向向量；「命格 vs 真實自我」落差分析
B3. 人格×情境預測端點（新表、新 DO 請求形、規則錨點先行＋LLM 潤寫）

### 非目標
- 臨床級量表；宮位制選擇器（固定 Whole Sign）；流年流曜細部暴露；付費牆整合

## 2. 架構總覽與分期

```
A1 紫微換庫（含前端紫微盤重寫——現有顯示已讀取不存在的欄位，無回歸風險）
A2 西洋換庫（行星 UT ＋ ASC/MC；前端本命盤渲染）
A3 時間管線（表單欄位 → UTC → 斗數 TST；分三次 bump ENGINE_VERSION）
B1 人格側寫（schema + TIPI + 先驗 + 落差）
B2 預測（情境 + predict 端點 + 規則庫 + predictions 表）
```

每期獨立交付、獨立 bump 版本、獨立部署驗證。

## 3. Phase A：引擎現代化

### 3.1 紫微 → iztro（v2.6.0，MIT）

adapter：`backend/src/services/ziwei/iztro-adapter.ts`

- **一律走 `bySolar(solarDateStr, timeIndex, gender, fixLeap, language)`**——國曆輸入直接給 iztro，
  由其內部 lunar-lite 換農曆。禁止先過自製 `solarToLunar` 再餵 `byLunar`（兩套曆法疊加會使閏月邊界與 iztro 盤不一致；
  `isLeapMonth` 參數僅存在於 `byLunar`，不適用於本管線）
- `timeIndex`：hour+minute → 0–12（早子 0…晚子 12）。**晚子時歸屬必須顯式設定**：
  實作第一步核對 iztro `config.dayDivide` 預設值（v2.5.2+ 傳聞為 `forward`＝晚子按次日安星），
  在 adapter 內明確傳入並在 `meta.dayDivide` 回報，不得依賴隱含預設。
  以 23:00 案例同時產出自製版與 iztro 版各一，列入 §6 差異清單
- `fixLeap = true`（前十五日本月、後半月次月，與自製版一致）
- `language = 'zh-TW'`；體積假設：locale 不具 tree-shakable 性質，膨脹主要來自曆表而非字串——以 dry-run 量測為準
- 序列化：**不得直接 JSON.stringify FunctionalAstrolabe 類實例**；用 `toJSON()`（2.6.0+）或逐欄映射
- **座標系聲明（防腳槍）**：iztro 宮位陣列以寅起 0；本專案 V3 一律**地支序（子=0）**。
  adapter 負責映射，並提供單一 helper `branchIndexOf(palace)`
- 映射到 `ZiWeiChartV3`：palaces[12]（地支序；每星 name/brightness/化耀）、majorLimits[]、
  三方四正索引、fourPillars（沿用自製日柱邏輯或取自 iztro，二擇一併註明）、meta（dayDivide/isLeap/fixLeap/timeIndex/hourShifted/assumed）
- 自製 calculator 保留匯出作對照基準

### 3.2 西洋 → astronomy-engine（v2.1.19，MIT）

新檔：`backend/src/services/western/natal.ts`

- 行星：`GeoVector(body, t_UT, true)` —— 第三參數是 **aberration**（光行差），非座標系旗標；
  本命盤採 apparent（true）。回傳 J2000 赤道直角座標，再 `Ecliptic(vec)` 得真黃道經緯度。
  月亮直用 `EclipticGeoMoon(t_UT)`（ILE 模型，對高精度星曆可有數角分差，±1° 驗收綽綽有餘）
- 逆行判定：相鄰 **Δt = 1 天** 兩次取經度比較；角距優先用 `PairLongitude(a, b, t)`
  （API 存在性實作時確認，否則手算角距）
- ASC/MC（規格級公式，不得只寫「球面三角」）：
  ```
  GAST = SiderealTime(t_UT)            // 小時，[0,24)
  LST  = (GAST + eastLongitudeDeg/15) mod 24   // 東經為正
  RAMC = LST × 15°                     // 度
  MC   = atan2( sin(RAMC), cos(RAMC)·cos ε )   // 黃道經度，ε 用 obliquity 公式
  ASC  = atan2( cos(RAMC), −(sin(RAMC)·cos ε + tan φ·sin ε) )  // φ = 緯度
  ```
- 宮位：Whole Sign（ASC 所在星座為第一宮，十二宮即十二星座）——無極圈邊界問題
- 相位：{合0±8, 六合60±6, 刑90±7, 三合120±8, 沖180±8}，容許度常數集中定義
- 星座判定一律由黃道經度 ÷30，廢除固定日期表；舊近似函式刪除

### 3.3 時間管線（P0 切分原則：TST 只服務斗數時辰，絕不進入西洋星曆）

```
birth y/m/d/h/min + timezone(IANA)
  ├─→ UT（一次，Intl 換算）──────────────→ 西洋 GeoVector / EclipticGeoMoon（星曆以 UT 為準）
  │                                    └→ 西洋 ASC/MC：SiderealTime(UT)＋地理經度（經度在此處使用）
  └─→ 斗數時辰：LMT = UT + lon×4min
              TST = LMT + EoT           // 符號約定：EoT = 視太陽時 − 平太陽時（NOAA 閉式近似）
              僅當 TST 與鐘錶時間跨過時辰邊界（±20 分內）才採 TST 定 timeIndex
```

- 缺資料決策表（禁止「一律台北」）：

| 有 timezone？ | 有經緯度？ | 行為 |
|---|---|---|
| ✓ | ✓ | 全功能：UT＋TST＋ASC/MC |
| ✓ | ✗ | 西洋：行星可算（UT 正確）、**ASC/MC 降級不可算**（`ascendant:null`＋`assumed` 標記）；斗數：跳過 TST，用鐘錶時辰＋`assumed:true`。**不得拿台北經度配外國時區** |
| ✗ | — | 預設 `Asia/Taipei`（現行行為），全鏈 `assumed:true` 揭露 |

- `WesternBirthData` 增加 `minute`、`timezone`、`latitude`、`longitude`
- **A3 前置＝前端 BirthDataForm 升級**：補收 分鐘／時區下拉／城市選單→座標（內建縣市對照表為靜態設定資料），
  或 opt-in 瀏覽器定位。表單未升級前 A3 後半（TST/ASC/MC）不得上線

### 3.4 快取、版本與相容

- `ENGINE_VERSION` **分期遞增**：A1→`3.0.0`、A2→`3.1.0`、A3→`3.2.0`（機制沿用 engineVersion 內嵌失效）
- `POST /:type/interpret` 加守衛：解析既有 `chart_data.engineVersion` ≠ 當前 → 回 `409 RECALC_REQUIRED`
  （避免對舊盤燒 AI 後又被清掉）；ETag 改摻入 `ENGINE_VERSION`
- 回應採**加欄位**策略：V3 新欄位全部新增，V2 欄位（`lifePalaceIndex` 等）保留一個過渡期；
  頂層加 `chartSchemaVersion: 3`；前端同步改寫（現況紫微顯示讀的是後端不存在的鍵，等於重寫，無回歸面）

## 4. Phase B：Big5 人格×情境預測

### 4.1 資料模型（D1，no-constraints 慣例）

```sql
CREATE TABLE IF NOT EXISTS personality_profiles (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  tipi_answers TEXT NOT NULL,        -- JSON [10] int 1–7
  ocean_measured TEXT NOT NULL,      -- JSON 五維 0–100，公式見 §4.3
  ocean_prior TEXT,                  -- JSON 同尺度
  prior_source TEXT,                 -- 'ziwei'|'western'|null
  measurement_status TEXT NOT NULL DEFAULT 'complete',  -- 'complete'|'skipped_prior_only'
  item_duration_ms INTEGER,          -- 作答時長（亂答偵測用）
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
  anchors TEXT NOT NULL,             -- 規則庫輸出的結構化錨點 JSON
  prediction_text TEXT NOT NULL,     -- LLM 潤寫結果
  cache_key TEXT NOT NULL,
  rules_version TEXT NOT NULL,
  prompt_version TEXT NOT NULL,
  created_at TEXT DEFAULT (datetime('now'))
);
```

遷移路徑：migration 檔放 `backend/scripts/migrations/`，以
`unset CLOUDFLARE_API_TOKEN && npx wrangler d1 execute fortunet-db --file ... --remote` 上遠端、`--local` 上本地。
不動 `interpretations`（其 CHECK 殘留 `'bazi'` 屬歷史包袱，另案清理）。

### 4.2 方法論修正（外部審查 P0）

- **題幹中性**：TIPI 十題採公開繁中版原文語意對，畫面不出現任何命盤資訊（避免 priming 污染「實測」）。
  命理包裝移到**結果頁**：「你的命格傾向 vs 你的自我評估」雷達疊加＋落差文字
- **亂答偵測不用先驗背離**（那會把洞察當作弊）：用 作答時長過短／全端作答／正反向題矛盾 三項
- **量尺對齊**：TIPI 每維兩題均分（反向先翻轉）得 1–7 → `(mean−1)/6×100` 得 0–100。
  先驗調整量定義為 100 分制上的 ±20/±10/0（對應規則表 −2/−1/0），多星命中取平均後套用，上下限 [0,100]。
  `prior_source` 同時有兩盤時以 ziwei 為準（另一份存檔不套用）
- **跳過者**：`measurement_status='skipped_prior_only'`；predict 端點對此狀態回 409 `MEASUREMENT_REQUIRED`，
  UI 不得呈現為「你的行為預測」

### 4.3 先驗規則表與領域映射（純規則，LLM 不參與打分）

- `services/personality/priors.ts`：主星/星座 → 特質調整量的透明對照表（常數＋口訣出處註釋），可被 review 校驗
- `services/personality/rules.ts`：trait×domain 錨點規則（如 高N×money高壓→壓力反應放大），
  附 `RULES_VERSION`；領域→宮位固定映射：work→官祿、love→夫妻、family→田宅、money→財帛、health→疾厄

### 4.4 API 與 AI 接線

| 端點 | 方法 | 備註 |
|---|---|---|
| `/api/personality/tipi` | POST | 驗證：長度恰 10、整數 1–7、伺服器端最短作答時長（≥5s，bot 門檻） |
| `/api/personality/me` | GET | 最新側寫 |
| `/api/personality/me` | DELETE | **個資刪除權**：清除人格/情境/預測三表資料 |
| `/api/personality/situation` | POST | domains 0–3 校驗 |
| `/api/charts/:type/predict` | POST | authMiddleware ＋ 獨立 rate limit（與 interpret 同型 10/min/IP） |

predict 流程：組錨點 JSON（OCEAN＋domains＋宮位活化＋規則命中）→ **prompt 只送錨點，不上傳原始十題答案**
→ AIMutexDO **新增請求形** `{ kind:'predict', payload }`（現有 handleRequest 只認
`interpretRequest:{chartType,chartData}`，需擴充分派；注意與 interpret 共享全域佇列，iFlow rpm=1 下的排隊行為要在監控上可辨識）
→ schema 校驗潤寫輸出（禁止新增錨點外的因果）→ 存 predictions。
快取鍵 = hash(ocean_measured + domains + target + divination_type + birth_data_hash + RULES_VERSION + PROMPT_VERSION + ENGINE_VERSION)。

### 4.5 前端

- `/personality` 路由＋ProtectedRoute；問卷（中性題幹）→ 結果頁（雷達圖疊加實測/先驗＋落差文字；SVG 手繪雷達，不引入圖表庫）
- DivinationPage 重寫：紫微十二宮卡片（亮度/四化徽章/大限列）、西洋行星表＋相位列表；情境勾選→預測區塊
- BirthDataForm：分鐘／時區／城市座標（§3.3）
- 圖表樣式遵循 dataviz 規範（實作時載入 dataviz skill）
- 隱私文案：目的限定、刪除入口、TIPI 為趨勢參考非心理診斷

## 5. 驗證策略

- **已提交的 integration tests**（`RUN_INTEGRATION=true` 才跑，打 staging/local worker）：
  - iztro 錨點：出版範例盤（固定輸入→預期命宮主星＋四化），**不用名人生辰**（Rodden 等級不可考）
  - 西洋錨點：對照 astro.com 數枚案例，**鎖定 tropical/geocentric/apparent 設定**；比行星度數 ±1° 與 ASC/MC ±2°，
    **不比宮頭**（astro.com 預設 Placidus，與 Whole Sign 必然不同）
  - 23:00 案例（dayDivide 行為）、閏月案例、interpret 對 stale engineVersion 的 409 路徑
- 拋棄式腳本續用於開發期曆法錨點（JDN 法），但**擋迴歸靠上述已提交 tests**
- 上線門檻：`wrangler deploy --dry-run` 量 gzip（<3MB）與 CPU（目標 <10ms/request，實測為準，不做假設）
- 部署前 commit；`wrangler dev` 本地驗證需使用者確認後執行

## 6. 已知差異清單（自製 vs iztro，對照基準解讀指引）

| 情境 | 自製版 | iztro 預期 | 備註 |
|---|---|---|---|
| 23:00 出生 | 子時、當日日柱 | 視 dayDivide 設定，可能次日安星 | §3.1 顯式設定＋測試 |
| 閏月 | 十五日界 | fixLeap=true 相同 | 應一致 |
| 四化/亮度/大限 | 無 | 有 | 功能增量非衝突 |

## 7. 風險與緩解（rev.2 更新）

| 風險 | 緩解 |
|---|---|
| iztro 依賴鏈（dayjs/i18next/lunar-lite/lunar-typescript）觸 Node API 或超過 gzip 上限 | dry-run 量測門檻；必要時 alias/stub 或降級方案（保留自製引擎為 fallback） |
| Workers 10ms CPU 免費層 | 上線前基準測試 cache-miss 全路徑；超標則拆計算到 DO 或升付費 |
| 真太陽時讓老用戶的盤變了 | meta.hourShifted 揭露＋release note；TST 只在邊界 ±20 分生效，影響面最小化 |
| LLM 幻覺 | 錨點先行＋schema 校驗＋prompt 只含結構化錨點 |
| 心理資料保護 | 刪除端點、目的限定、原始答案不出本地（prompt 送錨點）、UI 免責文案 |
| 前後端切換窗口 | 加欄位策略＋chartSchemaVersion（§3.4） |

---

## 附錄 A：rev.2 修訂紀錄（外部審查裁決）

審查者：Grok Build（run-mt9qxw61-8mvijz，30 條發現）。佐證結果：**28 條採納、2 條列「實作時對源碼驗證」**
（iztro `config.dayDivide` 預設值、astronomy-engine `PairLongitude` 存在性——離線不可考，但建議做法不受影響）。

主要採納（P0）：①真太陽時與西洋星曆時間軸切分（§3.3）②缺經緯度/分鐘的決策表與表單前置（§3.3、A3 範圍修正）
③TIPI 題幹去 priming（§4.2）。其餘 P1/P2 修訂散見 §3.1（bySolar/dayDivide/座標系/toJSON）、§3.4（分期 bump/
interpret 409 守衛/ETag）、§4.1–4.4（predictions 表/rulesVersion/skipped 狀態/DO 新請求形/個資刪除）、§5（測試盲點）。

四項事實核驗（我方獨立確認）：wrangler.toml 無 nodejs_compat ✓；
DivinationPage.tsx:105 讀取後端不存在的欄位（紫微顯示現況即壞）✓；schema.sql:37 CHECK 含殘留 'bazi' ✓；BirthDataForm 未收集 minute/lat/lng ✓。
