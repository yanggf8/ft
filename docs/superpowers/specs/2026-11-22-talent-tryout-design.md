# 上手小實驗（天賦線 v1，路徑 c）— 設計 v1

日期：2026-11-22　狀態：v1.2 已上線（2026-11-22 正式站 /try；§13 優先於上文）
來源：站長 2026-11-22「天賦方向照你推薦的做」＝方向 v3（`2026-11-17-talent-direction.md`）§3：
路徑 c（宣稱者＝使用者自己）、對外詞彙「上手」、D6 不 block（v1 即做）。v2 的 13 條硬規則全適用。

## 1. 一句話

使用者從一個跨領域的小實驗池挑一個 15 分鐘的小挑戰，**先猜自己會做到哪一步並鎖定**，
實際去做，回來勾實際做到哪一步，看「猜的 vs 實際」的差距。累積幾次後看到的是
「我對自己的猜測傾向」與「比我以為的順的地方」——都是本人、這幾次、這些條件下的觀察。

## 2. 迴圈模板（方向 v3 §6）

| 欄位 | 內容 |
|---|---|
| 假說提出者 | 使用者自己（「15 分鐘內我會做到第幾步」） |
| 體驗行動 | 線下實際做 15 分鐘的小挑戰 |
| 誤差訊號 | 實際做到的步數 − 猜的步數（整數，−3…+3） |
| 更新對象 | 使用者對自己的預測（校準）；不更新任何推薦權重 |
| 保存位置 | Turso `talent_trials`；使用者可逐筆刪除或全部刪除（硬刪） |
| 宣稱層級 | 對答案層——**只針對使用者自己的猜測**（五要件見 §6）；題目文案本身是素材層 |

## 3. 範圍／非範圍

**v1 做**：12 題小實驗目錄、先猜鎖定、7 天內回報、單次結果卡、我的紀錄（列表＋最低證據量
後的猜測傾向）、刪除。需登入（資料跟人走；對象＝本人）。

**v1 不做**：
- 命盤呼應（路徑 b 元素）——v1 不碰 symbolic，測量線保持乾淨；日後再議。
- AI 解讀（硬規則 8：v1 預設不接 AI）。
- 領域之間的比較或排名（風險 2：答不出「是不是難度／順序／新鮮感造成」就不輸出比較）。
- 依結果推薦題目（硬規則 4：速度訊號禁轉推薦權重）。
- 速度感自評（硬規則 2：只收具體、可列舉的訊號）。
- 匿名模式、分享、提醒推播。

## 4. 使用者流程

1. **`/try` 首頁**：一句說明（「挑一個 15 分鐘的小挑戰，先猜自己會做到哪，做完再對答案」）＋
   「這是給你自己的小實驗」（硬規則 7）。
   - 「抽三個」：從目錄抽 3 題，**至少 1 題來自你還沒試過的領域**（多樣性是結構要求，硬規則 4）；
     抽法與過去結果無關（純亂數）。可再抽，也可「看全部 12 題」自選。
   - 有進行中的實驗時，頁首顯示那一筆（只能同時一筆進行中）。
2. **題目頁**：標題、要準備的東西（只限紙筆／手機／家裡常見物品）、三個步驟（checkpoint）、
   「計時 15 分鐘，到時間就停」。
3. **先猜**：「15 分鐘內，你猜你會做到第幾步？」四選一：還沒到第 1 步／第 1 步／第 2 步／第 3 步。
   送出即鎖定（不能改；要重來只能刪掉這筆）。顯示「7 天內回來對答案」。
4. **實際做**：頁面提供 15 分鐘倒數（純前端，可不用）；不強制在站上做。
5. **對答案**（一次性）：
   - 「實際做到第幾步？」同四選一；或「這次沒做」（→ 不算數，該筆結束）。
   - 「最卡的是哪裡？」（可選、單選）：看不懂說明／手或身體跟不上／記不住／時間不夠／中途不想做了／沒有卡。
6. **結果卡**：並排「你猜：第 X 步 → 實際：第 Y 步」與一句誤差描述（§7 文案），附條件標記
   「這是你在 {日期} 這一次的 15 分鐘」。永不出現能力宣稱。
7. **我的紀錄**（`/try` 下半部）：
   - 每筆：題目、日期、猜／實際、誤差、狀態（進行中／已對答案／沒做／過期），依日期新到舊排（**不依誤差排序**）。
   - **比你以為的順的地方**：實際 > 猜 的題目清單＋固定註記「只列出比你猜的順的題目；沒列出的不代表不適合你，
     很多題你還沒試」（硬規則 5）。
   - **你的猜測傾向**：已對答案 ≥ 3 筆才顯示（硬規則 6）：「猜少了 a 次、剛好 b 次、猜多了 c 次」＋
     一句描述（偏保守／差不多／偏樂觀，規則見 §5.3）。未達門檻顯示「再對 N 次答案就能看到你的猜測傾向」。

## 5. 資料與規則（`ft_schema::talent`）

### 5.1 目錄（`CATALOG`，`TALENT_CATALOG_VERSION = "1"`）

`TrialItem { id, domain, title, prep, steps: [&str; 3] }`；`id` 是 wire／儲存鍵**不可改名**
（下架＝從 CATALOG 移除＋bump）。領域 `Domain`（wire 小寫）：
`movement` 身體動作、`hands` 手作、`language` 語言、`memory` 記憶、`music` 音樂、
`space` 空間、`words` 文字、`drawing` 畫畫、`logic` 邏輯、`speaking` 表達。

| id | 領域 | 標題 | 三步 |
|---|---|---|---|
| `juggle` | movement | 用兩三顆捲好的襪子學拋接 | 兩顆單手連續拋接 10 下／兩顆交叉拋接連續 10 下／三顆完成一輪（每顆各拋一次） |
| `dance-8` | movement | 跟影片學一段 8 拍舞步 | 跟著影片做完一遍／不看影片做完一遍／不看影片連做兩遍不出錯 |
| `knots` | hands | 用鞋帶學三個繩結 | 打出平結／打出八字結／不看說明打出稱人結 |
| `crane` | hands | 摺一隻紙鶴 | 摺出正方基本形／摺出鶴的基本形／完成一隻紙鶴 |
| `hangul` | language | 認一套沒學過的字母（例如韓文） | 認得 5 個字母／認得 10 個字母／用它拼出自己的名字 |
| `digits` | memory | 背一串 30 位的數字 | 背出前 10 位／背出前 20 位／30 位全對 |
| `song` | music | 學一首沒聽過的歌的第一段 | 哼得出旋律／看歌詞唱完／不看歌詞唱完 |
| `cube-draw` | space | 畫出立體方塊 | 畫出一個看起來立體的方塊／從三個不同角度各畫一個／畫三個疊在一起的方塊 |
| `haiku` | words | 寫 5-7-5 的短詩 | 寫出一首字數對的／寫出三首／挑一首念給一個人聽 |
| `cup-sketch` | drawing | 畫眼前的一個杯子 | 畫出輪廓／加上明暗／不看紙只看杯子畫一張 |
| `scratch` | logic | 用 Scratch 讓角色動起來 | 讓角色往前走／用方向鍵控制角色／碰到邊緣會反彈 |
| `one-minute` | speaking | 用一分鐘介紹一個你喜歡的東西 | 寫下三個重點／計時一分鐘講完不超時／講給一個人聽 |

`prep`（準備）：只寫一般家中物品、紙筆、手機或電腦；需要外部教學的題目寫「自己找一支教學影片」，
**不放外部連結**。

文案測試（同 shop／interest 作法）：禁字（天賦、天分、擅長、適合、不適合、研究、科學、證實、準確、
大腦、潛能、命中、注定、IQ、智商）不得出現在目錄與所有固定文案；每題 3 步、id 小寫 kebab、id 不重複、
每個 Domain 至少 1 題。

### 5.2 抽題（純函式，web 端用）

`draw_three(tried_domains: &[Domain], rand: impl FnMut(usize) -> usize) -> [&TrialItem; 3]`：
若有未試領域，第一題從未試領域的題目中取；其餘兩題從剩下題目取、不重複、不同於第一題的領域優先。
**輸入只有「試過哪些領域」，沒有任何結果欄位**（型別上就拿不到誤差，守硬規則 4）。

### 5.3 誤差與傾向（純函式）

- `error = reached − predicted`（皆 0..=3；`reached` 為 None＝沒做，不算）。
- `tendency(errors)`：n＝已對答案筆數；n < 3 → `NotEnough { need: 3 − n }`；
  否則 mean = Σerror / n：mean ≥ 0.5 → `Conservative`（猜得比做到的少）；mean ≤ −0.5 → `Optimistic`；
  其餘 `Close`。只描述「猜」不描述「能力」。

### 5.4 狀態

`open`（已猜未回報，未過期）／`reported`／`skipped`（回報「這次沒做」）／`expired`（猜後 7 天未回報；
由讀取時計算，不寫回）。過期與沒做都不進誤差與傾向（缺失資料規則）。
「進行中」只算未回報且**未過期**的——過期那筆不擋新實驗。
舊紀錄的 `item_id` 若已不在目前 CATALOG（下架），標題顯示「已下架的題目」，誤差與傾向照算。

## 6. 對答案層五要件放行表（方向 v3 §5 第 4 點）

| 要件 | 內容 |
|---|---|
| 事前宣稱與版本 | `predicted`（0–3）於 POST 時寫入，之後無任何改寫路由；附 `catalog_version` |
| 結果欄位與型別 | `reached`（0–3 整數）、`stuck`（enum 可空）；一次性，回報後不可改 |
| 時間窗與時區 | 猜的時間起 7×24 小時（UTC 時間戳比較，與時區無關）；過期後回報回 410 `EXPIRED` |
| 缺失資料規則 | 「這次沒做」＝skipped、過期＝expired，兩者都不進誤差、傾向、「比你以為的順」 |
| 誤差呈現位置 | 單筆結果卡（§4.6）與我的紀錄（§4.7） |

五要件齊 → 這條線可對外說「上手小實驗提供對答案機制」（只說這一條功能，不做全站宣稱）。

## 7. 固定文案（schema 常數，受禁字測試守門）

- 誤差 +k（k>0）：「比你猜的多走了 k 步。」
- 0：「跟你猜的一樣。」
- −k：「比你猜的少走了 k 步。這次的條件已經記下來了。」（不加任何評價）
- 卡點中文：看不懂說明／手或身體跟不上／記不住／時間不夠／中途不想做了／沒有卡。
- 傾向：Conservative「你目前常常把自己猜得保守一點。」／Close「你目前猜得跟實際差不多。」／
  Optimistic「你目前常常把自己猜得樂觀一點。」，下面固定一行「這只是這幾次 15 分鐘的觀察，換一天、
  換題目都可能不同。」
- 「比你以為的順的地方」註記（§4.7）。

## 8. API（`routes/talent.rs`，全部需登入）

| 方法／路徑 | 說明 |
|---|---|
| `GET /api/talent/trials` | 本人所有紀錄（新到舊），含計算後的 `status`、`error`；另回 `tendency` |
| `POST /api/talent/trials` | `{itemId, predicted, requestId}` → 建立。錯誤：`INVALID_ITEM`、`INVALID_PREDICTION`、`OPEN_TRIAL_EXISTS`（409，已有進行中）、`RATE_LIMITED`。同 requestId 重送回原筆（冪等，先查後插） |
| `POST /api/talent/trials/:id/report` | `{reached: 0..3 \| null, stuck?, requestId}` → `reached=null`＝這次沒做。錯誤：`NOT_FOUND`（非本人也回 404）、`ALREADY_REPORTED`（409；同 requestId 重送回原結果）、`EXPIRED`（410）、`INVALID_REACHED`、`INVALID_STUCK` |
| `DELETE /api/talent/trials/:id` | 硬刪本人一筆 |
| `DELETE /api/talent/trials` | 硬刪本人全部 |

- 每個讀 body 的路由先 `body_too_large`；錯誤不回 DB 細節；限流 `talent:user:{uid}` 每小時 60 次寫入
  （`rate_limit` 一般版，fail-open 與其他非 AI 路由一致）。
- DTO 放 `ft_schema::api`（camelCase）：`TalentTrialCreate`、`TalentTrialReport`、`TalentTrial`、
  `TalentTrialsResponse { trials, tendency }`。

## 9. 資料表（migration `scripts/migrations/2026-11-22-talent-trials.sql`，同步進 schema.sql）

依站長全域規則：只有 PRIMARY KEY，不加其他 constraint；欄位必有值由程式保證。

```sql
CREATE TABLE IF NOT EXISTS talent_trials (
  id TEXT PRIMARY KEY,
  user_id TEXT, item_id TEXT, catalog_version TEXT,
  predicted INTEGER, created_at TEXT, create_request_id TEXT,
  reached INTEGER, skipped INTEGER, stuck TEXT, reported_at TEXT, report_request_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_talent_trials_user_created ON talent_trials(user_id, created_at);
```

- 「同時只有一筆進行中」由程式先查後插（併發雙寫最多多一筆，讀取時以最舊的 open 為準，另一筆照常可回報）。
- 回報用條件更新 `UPDATE … WHERE id=? AND user_id=? AND reported_at IS NULL`，affected=0 再查一次區分
  已回報／不存在。
- **紅線**：天賦線資料永不寫入 predictions／prediction_feedback／f8_assignments；不 import `symbolic`。
- 日後若加帳號刪除路由，須一併刪 `talent_trials`。
- migration 順序：fortunet-staging → fortunet → deploy-api（staging 先）→ deploy-web。

## 10. 前端（`pages/talent.rs`，路由 `/try`）

- 未登入：說明＋登入按鈕（`/login?next=/try`）。
- 首頁新增卡片「上手小實驗」（FeatureIcon 新增 `try`），文案取自 schema 常數。
- 抽題／全部題目／題目頁／先猜／對答案／結果卡／我的紀錄 皆在同一頁以狀態切換（不另開路由），
  15 分鐘倒數用 `setInterval`，離開頁面即停（不存）。
- 樣式只用既有 token；新配對（如有）加進 contrast-check。
- web-smoke `--login`：開 `/try` → 抽三個（3 張卡）→ 選第一題猜第 1 步 → 對答案第 2 步 → 結果卡出現
  「多走了 1 步」→ 刪除這筆 → 列表為空；清理清單加 `talent_trials`。匿名：`/try` mount、有登入鈕。

## 11. 驗收

- `cargo test -p ft-schema -p ft-api`（目錄／禁字／抽題多樣性／誤差與傾向／DTO 序列化）、fmt、clippy wasm32。
- verify-deployment 新增 8t：四條路由未登入 401。
- `scripts/talent-selftest.py`（正式站／staging 皆可）：自造帳號；建立→重送冪等→第二筆 409→回報→
  重回報 409→列表 error／tendency NotEnough→再兩筆達門檻看到 tendency→刪單筆→刪全部；結束硬清。
- 部署走三段 staging 閘門。

## 12. 風險對策對照（v2 §5）

1. 狀態相依快照 → 每筆帶日期與「這一次的 15 分鐘」條件標記；可刪。
2. 個人內對比方法論坑 → v1 不輸出任何跨題比較；只輸出「猜 vs 實際」（同一題、同一次，難度基線一致）。
3. 馬太效應 → 抽題不看結果、強制未試領域。
4. 補集推論與代測 → 固定註記「沒列出的不代表不適合」；「給你自己的小實驗」。
5. 努力不對稱 → 最小迴圈＝一次 15 分鐘就有結果卡（非空洞：具體的猜／實際對照）。
6. 文案治理 → 對外不用「天賦」；禁字測試。
7. 資料劃界 → 獨立表、紅線聲明、硬刪。

## 13. 獨立審查修正（v1.2，2026-11-22；Gemini 審 6 blocker＋should-fix 全核，**本節優先於上文**）

站長 2026-11-22「審完直接開工，做完部署給我看」＝方向 v3 §5 第 5 點的簽核（文案若有疑義上線後回報）。

### 13.1 宣稱與文案
1. 「沒列出的不代表不適合」含禁字且預設「有列＝適合」→ 改為：「只列出實際做到比你猜的多的那幾次；
   沒列出的，只代表那次沒超過你的猜測，或你還沒試過。」
2. 「比你以為的順的地方」＝error>0 的**逐次**列表（不是題目清單），新到舊，每筆帶日期、猜→實際、
   「這一次的 15 分鐘」；同題多次各自列，不合併、不依差距排序。
3. −k 文案改「比你猜的少走了 k 步。」（不再提條件）；列表欄名「差距」（不用「誤差」）。
4. 0 步一律顯示「還沒到第 1 步」。日期＝`created_at`（猜的時候），以瀏覽器本地時區顯示到日。
5. 所有使用者可見固定字串（含首頁卡、門檻提示「再對 N 次答案就能看到你的猜測傾向」、卡點標籤、
   登入提示）都是 `ft_schema::talent` 常數；禁字測試掃全部，雙語清單：
   天賦、天分、天生、資質、潛能、擅長、適合、不適合、能力、優勢、強項、弱項、研究、科學、理論、證實、
   證明、準確、預測誤差、大腦、測出、診斷、命中、注定、IQ、智商、talent、gifted、aptitude、brain、
   theory、science、research、proven、accurate、potential、destiny、fate。
6. 分層 release gate 清單（方向 v3 §5）：
   - (c) 句清單：只有 §7 的誤差句與傾向句，宣稱對象都是「使用者這次的猜測 vs 這次的實際」，屬對答案層；
     目錄與其餘說明屬素材層／操作說明，不含 (c) 句。
   - 正確／錯誤對照：✔「比你猜的多走了 2 步。」✘「你在手作方面很有天分。」；
     ✔「你目前常常把自己猜得保守一點。」✘「你比自己以為的更有能力。」
   - 五要件放行表見 §6；站長簽核見上。

### 13.2 防作弊／五要件
7. 「同時一筆進行中」改**原子**：`INSERT INTO talent_trials (...) SELECT ... WHERE NOT EXISTS (SELECT 1
   FROM talent_trials WHERE user_id=? AND reported_at IS NULL AND created_at > ?cutoff)`；affected==0 →
   先查同 requestId（冪等）再回 409 `OPEN_TRIAL_EXISTS`。
8. 刪掉「要重來只能刪掉這筆」；中途放棄用「這次沒做」。刪除只做資料自主（§6 加註：刪除會讓傾向重算，
   這是本人的自我觀察，不做對外統計，接受）。
9. 每個 `id` 的 title／domain／steps **永不改**；要改就新 id、舊 id 下架，並 bump `TALENT_CATALOG_VERSION`。
10. 回報順序：驗 body → `SELECT` (id,user_id)（無 → 404 `NOT_FOUND`）→ 已回報：同 `report_request_id`
    回 200 原結果、否則 409 `ALREADY_REPORTED` → 過期 410 `EXPIRED` → `UPDATE … WHERE id=? AND user_id=?
    AND reported_at IS NULL AND created_at > ?cutoff`；affected==0 重查一次依上述回應。
11. 過期判斷唯一來源：`ft_schema::talent::is_expired(created_at, now)`（RFC3339 UTC，≥ 7×86400 秒），
    GET／POST／report 共用。時間戳一律 `clock::now_iso()`。

### 13.3 API／資料
12. `requestId`：8–64 字 `[A-Za-z0-9_-]`，不合 400 `INVALID_REQUEST_ID`。POST 先查 (user_id,
    create_request_id)：有且 item／predicted 相同 → 200 原筆；不同 → 409 `REQUEST_ID_CONFLICT`。
13. 數字欄位以 `serde_json::Value` 取出再驗（整數 0–3，否則 `INVALID_PREDICTION`／`INVALID_REACHED`）。
    report 的 `reached` key **必填**（缺 → `INVALID_REACHED`），`null`＝這次沒做；`reached=null` 時帶
    `stuck` → 400 `INVALID_STUCK`。`skipped` 存 0／1，未回報為 NULL。
14. 限流：先驗登入，所有寫入路由共用 `talent:user:{uid}` 每小時 60 次（`rate_limit`，fail-open）。
15. DTO（camelCase）：
    - `TalentTrial { id, itemId, catalogVersion, predicted: u8, createdAt, status: "open"|"reported"|"skipped"|"expired",
      reached: Option<u8>, stuck: Option<String>, reportedAt: Option<String>, gap: Option<i8> }`
    - `TalentTendency { reportedCount: u32, need: u32 /*0＝已達門檻*/, under: u32 /*猜少了*/, exact: u32, over: u32,
      kind: Option<"conservative"|"close"|"optimistic"> }`
    - `TalentTrialsResponse { trials: Vec<TalentTrial>, tendency: TalentTendency }`
    - POST 201（冪等重送 200）回 `TalentTrial`；report 200 回 `TalentTrial`；DELETE 單筆 200 `{"deleted":1}`，
      不存在或非本人 404；DELETE 全部 200 `{"deleted":n}`。
16. 卡點 wire：`unclear` 看不懂說明／`body` 手或身體跟不上／`memory` 記不住／`time` 時間不夠／
    `motivation` 中途不想做了／`none` 沒有卡。
17. `draw_three` 的「試過的領域」＝已回報（reported）且題目仍在 CATALOG 的領域；有進行中的實驗時仍可抽題、
    看題，只有 POST 會 409。verify 8t 為五條路由未登入 401。

### 13.4 目錄 v1（取代 §5.1 表；全部一個人、15 分鐘、步驟由易到難且可自己判定）
| id | 領域 | 標題 | 準備 | 三步 |
|---|---|---|---|---|
| `juggle` | movement | 用捲好的襪子學拋接 | 三雙捲成球的襪子 | 一顆單手連續拋接 10 下／兩顆交叉拋接連續 10 下／三顆連續拋接一輪（每顆各拋一次都接住） |
| `dance-8` | movement | 跟影片學一段 8 拍舞步 | 手機或電腦，自己找一支教學影片 | 跟著影片做完一遍／關掉影片做完一遍／關掉影片連做兩遍不停 |
| `knots` | hands | 用鞋帶學三個繩結 | 一條鞋帶或繩子，自己找教學 | 不看說明打出平結／不看說明打出八字結／不看說明打出稱人結 |
| `crane` | hands | 摺一隻紙鶴 | 一張正方形的紙，自己找教學 | 摺到「正方基本形」／摺到「鶴的基本形」（菱形、兩側已收窄）／摺出能立起來的紙鶴 |
| `hangul` | language | 認韓文的子音 | 手機或電腦，自己找一張韓文子音表 | 蓋住讀音認出 5 個子音／蓋住讀音認出 10 個子音／蓋住讀音把 14 個子音全部認對 |
| `digits` | memory | 背一串 30 位的數字 | 這頁會給你一串數字 | 背出前 10 位／背出前 20 位／30 位全對 |
| `song` | music | 學一首沒聽過的歌的第一段 | 手機或電腦 | 跟著原唱哼完旋律／關掉原唱、看歌詞唱完／關掉原唱、不看歌詞唱完 |
| `cube-draw` | space | 畫立體方塊 | 紙和筆 | 畫出一個能看到三個面的方塊／畫出同一個方塊的另一個角度（看到不同的面）／畫出三個方塊疊成階梯 |
| `haiku` | words | 寫 5-7-5 的短詩 | 紙和筆 | 寫出一首字數 5-7-5 的／寫出三首／把其中一首的最後一句改寫兩種不同的版本 |
| `cup-sketch` | drawing | 畫眼前的一個杯子 | 一個杯子、紙和筆 | 畫出輪廓／加上明暗（亮的一面、暗的一面）／把杯子收起來，憑記憶再畫一張 |
| `scratch` | logic | 用 Scratch 讓角色動起來（需電腦） | 電腦瀏覽器打開 Scratch 官網 | 讓角色往前走／碰到舞台邊緣會反彈／按方向鍵可以控制角色 |
| `one-minute` | speaking | 用一分鐘介紹一個你喜歡的東西 | 手機錄音 | 寫下三個重點／不看稿錄一段 45–60 秒／重聽一遍，沒有超過 3 秒的停頓 |

`digits` 的數字由 `ft_schema::talent::digits_for(trial_id)` 以試驗 id 做確定性雜湊產生 30 位（同一筆永遠同一串，
不另存）；猜之前不顯示，猜完才顯示。`Domain` 的中文標籤也是常數。
