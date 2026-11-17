# 上手實驗 v1 設計(天賦線/模型面,路徑 c)— 2026-11-17

> **PARKED(2026-11-17)**:站長裁示——天賦理論在最小誤差預測下站不住(構念多餘),且「練習類」功能站長自己不覺得有趣(風水更有趣)。機制(自我預測校準)保留備查,不往下推。若重啟:拿掉「天賦」構念,定位改為「自我預測校準」。

方向:2026-11-17-talent-direction.md v3(四句根基、v2 十三條硬規則、§2 分層、§6 模板)。
流程:本 spec→自審→Kimi K3.1 Pro 審→Claude 核→站長核→多 AI 開工。
**排序**:站長 2026-11-17 裁示先於風水 v1(站長是第一個使用者;探索 > 算命)。

## 0. 定位與設計前提

- **v1 唯一設計目標使用者 = 站長本人**(dogfooding)。不做外部推廣、不放首頁入口;
  通過「站長自己連續使用 4 週」才評估開放與擴充。現況(2026-11-17 正式 DB):全站 2
  使用者、9 月後功能 0 筆——本功能的第一個成功指標是**站長本人產生真資料**。
- 內部構念「天賦 ≡ 相對學得快」(v2 句 1,產品操作定義,非科學定論);對外詞彙:
  **上手實驗**。介面禁「天賦」「擅長」「適合」(沿用 interest `BANNED_WORDS` 治理)。
- 核心機制:**你先預測自己學一件事要花多少,實際去做並記錄,最後對答案**。
  v1 的輸出是「你對自己的預測準不準」(自我模型校準),**不是**「你在哪個領域快」。

## 1. 迴圈模板(方向 v3 §6)

| 欄位 | 內容 |
|------|------|
| 假說提出者 | 使用者自己(事前鎖定的預測)|
| 體驗行動 | 實際練習,逐次記錄投入與具體產出 |
| 誤差訊號 | 預測投入 vs 實際投入;預測是否達標 vs 實際是否達標 |
| 更新對象 | 使用者的自我預測(校準),不更新任何「能力判定」|
| 保存位置 | Turso 兩表(§4);全部刪除+epoch |
| 宣稱層級 | 對答案層(滿足五要件,見 §2)|

## 2. 可檢驗五要件(方向 v3 §2)

1. **事前固定宣稱**:建立實驗時鎖定 `goal`(完成判準)、`predictedMinutes`(預估達標
   總投入分鐘)、`predictedReach`(預計截止前能否達標)、`deadline`;鎖定後不可修改
   (API 強制;同 F5 一次性精神)。
2. **明確結果欄位**:`actualMinutes`(各次紀錄加總,整數)、`reached`(布林,結案時記)。
3. **時間窗**:`deadline` = 建立日起 7–42 天(台北日期);到期未結案 → 自動視為結案
   `outcome=expired`,`reached=false`。
4. **缺失資料處理**:中途放棄 → `outcome=abandoned`,**保留在歷史並納入校準**(標示
   放棄,不靜默刪除;使用者可單筆刪除,刪除即不計入)。過期同理。
5. **誤差呈現**:結案頁顯示 `比值 = actualMinutes / predictedMinutes` 與
   「預測達標/實際達標」四格之一;歷史頁顯示每筆比值。

## 3. 硬規則落地(v2 十三條對照)

| v2 規則 | 本設計落地 |
|---------|------------|
| 證據=具體有界訊號,禁速度感自評 | 每次紀錄只收:日期、分鐘(整數)、具體產出(必填短文字,如「彈完第 1–8 小節」)、卡點(選填);**無任何 1–5 主觀評分欄** |
| 速度訊號禁轉推薦權重 | v1 無推薦、無「你該學什麼」;不跨實驗比較快慢 |
| 永不負面宣稱 | 無「你不擅長」類輸出;未達標文案只陳述事實與比值 |
| 任何「A 比 B 順」須排除難度/順序/新鮮感 | v1 **完全不輸出跨領域比較**(答不出混淆就不輸出)|
| 最低證據量 | 校準摘要須 ≥5 筆已結案(含放棄/過期)才出現;之前只顯示「目前觀察」|
| 宣稱粒度 | 校準摘要只談「你的時間預測傾向」,不類化到能力或領域 |
| 時間與條件標記 | 每筆比值附實驗期間與 goal 原文 |
| 對象=本人 | 只能記錄自己;無代測欄位 |
| AI | v1 無 AI |

**校準摘要**(≥5 筆後):已結案實驗比值的中位數,文案三選一:
「你的預估通常偏樂觀(實際多花約 X 倍)」/「你的預估大致準確」/「你的預估通常偏保守」
(門檻:中位數 >1.25 / 0.8–1.25 / <0.8;**產品選擇**,揭露於頁內小註)。
另列「達標預測命中 N/M」。不輸出任何領域層面結論。

## 4. 資料模型

`scripts/schema.sql`+`scripts/migrations/2026-11-xx-learn.sql`:

```sql
CREATE TABLE IF NOT EXISTS learn_experiments (
  id                TEXT PRIMARY KEY,
  user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  topic             TEXT NOT NULL,          -- ≤40 字:要學什麼
  goal              TEXT NOT NULL,          -- ≤120 字:怎樣算達標(具體、可判定)
  predicted_minutes INTEGER NOT NULL CHECK (predicted_minutes BETWEEN 10 AND 6000),
  predicted_reach   INTEGER NOT NULL CHECK (predicted_reach IN (0,1)),
  deadline          TEXT NOT NULL,          -- 台北日期 YYYY-MM-DD
  outcome           TEXT CHECK (outcome IN ('reached','not_reached','abandoned','expired')),
  closed_at         TEXT,
  protocol_version  TEXT NOT NULL,          -- LEARN_PROTOCOL_VERSION
  request_id        TEXT NOT NULL,
  created_at        TEXT NOT NULL,
  UNIQUE (user_id, request_id)
);
CREATE TABLE IF NOT EXISTS learn_sessions (
  id            TEXT PRIMARY KEY,
  experiment_id TEXT NOT NULL REFERENCES learn_experiments(id) ON DELETE CASCADE,
  user_id       TEXT NOT NULL,
  session_date  TEXT NOT NULL,              -- 台北日期,建立日..deadline 內
  minutes       INTEGER NOT NULL CHECK (minutes BETWEEN 1 AND 600),
  output        TEXT NOT NULL,              -- ≤120 字,具體產出
  stuck         TEXT,                       -- ≤120 字,選填
  request_id    TEXT NOT NULL,
  created_at    TEXT NOT NULL,
  UNIQUE (user_id, request_id)
);
-- users.learn_epoch INTEGER NOT NULL DEFAULT 0
```

- `reached` 由 `outcome` 導出(`reached` ⇔ `outcome='reached'`);`actualMinutes` 由 sessions
  加總計算,**不落庫**(避免冗餘不一致)。
- 同時進行中(`outcome IS NULL`)上限 **3**(API 檢查;站長自用的合理並行)。
- 過期結案:讀取時惰性判定(`deadline` < 今日台北日期 且 `outcome IS NULL` → 寫入
  `expired`),不需排程器。
- 紅線(route 檔頭):**上手實驗資料永不寫入 predictions / prediction_feedback /
  f8_assignments**;不讀命盤、人格分數、F5、F8、小練習資料。

## 5. API(crates/api/src/routes/learn.rs)

全部需登入;POST 先 `body_too_large` 再 `req.json()`;錯誤 generic `{code, message}`。

| 端點 | 行為 | 錯誤 |
|------|------|------|
| `GET /api/learn` | 進行中實驗(含 sessions 與 actualMinutes)+校準摘要(或「目前觀察」)| — |
| `POST /api/learn` | 建立並鎖定預測;body `{topic, goal, predictedMinutes, predictedReach, deadline, requestId, epoch}` | 3 個進行中 → 409 `TOO_MANY_ACTIVE`;值域 → 400 `INVALID_EXPERIMENT`;epoch 過期 → 409 `STALE_EPOCH` |
| `POST /api/learn/{id}/sessions` | 新增一次紀錄;body `{sessionDate, minutes, output, stuck?, requestId}` | 已結案 → 409 `CLOSED`;日期越界 → 400 `INVALID_SESSION` |
| `POST /api/learn/{id}/close` | 結案;body `{outcome: reached\|not_reached\|abandoned}` | 已結案 → 409 `CLOSED` |
| `DELETE /api/learn/{id}` | 刪單筆(含其 sessions) | 非本人/不存在 → 404 |
| `GET /api/learn/history?page=` | 已結案分頁,新→舊,每筆含比值 | — |
| `DELETE /api/learn` | 全部刪除+`learn_epoch` 加一(比照 practice) | — |

- 冪等:`(user_id, request_id)` UNIQUE,重送回既有列。
- 預測鎖定:無任何修改 topic/goal/predicted*/deadline 的端點。
- 版本:`LEARN_PROTOCOL_VERSION`(schema 常數;改五要件語義、門檻、欄位語義必 bump),
  寫入每筆實驗;校準摘要只納入與現行版本相同的列(不混版)。

## 6. Web(crates/web)

- 「我的命格」頁新卡 `LearnCard`(小練習卡之後);**v1 不放首頁卡**(§0)。
- 建立流程:要學什麼 → 怎樣算達標(提示:「寫成別人也能判斷有沒有做到的樣子」)
  → 預估總投入分鐘 → 截止日前能不能達標 → 截止日 → **確認鎖定**(明示鎖定後不可改)。
- 進行中:每個實驗一列,顯示已投入/預估、剩餘天數;「記一次」(日期、分鐘、做出了
  什麼、卡在哪)、「結案」(達標/未達標/放棄)。
- 結案頁:四格對答案(預測達標×實際達標)、比值、goal 原文與期間。
- 歷史頁:分頁;校準摘要(§3)+門檻小註;全部刪除(前顯示筆數)。
- 文案:不出現天賦/擅長/適合/大腦/預測誤差/研究;「猜錯是資訊,不是失敗」級中性語。

## 7. 測項(TDD;native)

- **schema**:比值計算;校準摘要門檻(<5 筆→目前觀察;中位數三區間含邊界 0.8/1.25);
  過期判定(台北日期邊界);protocol 版本過濾;禁詞掃描(沿用 `BANNED_WORDS`+本功能
  文案)。
- **API**:冪等(兩種 POST 重送);第 4 個進行中 → 409;鎖定欄位無修改路徑(路由表斷言);
  結案後記錄/再結案 → 409;session 日期越界 → 400;他人 id → 404;惰性過期寫入
  `expired`;放棄/過期列出現在歷史且納入摘要;單筆刪除後不納入;全部刪除+epoch 後
  舊 epoch 建立 → 409;body cap;全流程後 predictions/prediction_feedback/f8_assignments
  row-count 差值為 0。
- **E2E**:`scripts/learn-e2e.sh`(仿 interest-e2e:自造 login token、API 段+瀏覽器段不攔截、
  結束硬清測試帳號);`verify-deployment.sh` 加未登入 401 probe。

## 8. 批次

| 批 | 內容 | 驗收閘 |
|----|------|--------|
| 1 | schema:`ft_schema::learn` 契約、比值/摘要/過期純函式、版本常數 | schema 測項 |
| 2 | DDL+遷移+`routes/learn.rs`+verify probe | API 測項 |
| 3 | web `LearnCard`+流程+E2E 腳本+ARCHITECTURE.md 章節 | E2E+分層 release gate(方向 v3 §5)|

部署:Turso 遷移 → API → web。上線後即開始站長自用 4 週;4 週後依實際紀錄回顧。

## 9. 開放問題(自審與 Kimi 審重點)

1. 「預估總投入分鐘」對站長是否好預測?替代:預估次數。v1 取分鐘(較具體)。
2. 中位數門檻 0.8/1.25 為產品選擇,4 週後依實際分布檢討。
3. 進行中上限 3 是否夠用。
4. 與本週小練習的關係:小練習=日常行動+主觀幫助;上手實驗=學習目標+具體投入。
   兩者並存,不互讀。
