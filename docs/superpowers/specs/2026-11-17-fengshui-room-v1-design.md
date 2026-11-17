# 空間風水 v1 設計：房間格局 × 你的大腦 × 先猜後試

狀態：v1 草稿（已自審，待 Kimi 審）
日期：2026-11-17
依據：`docs/superpowers/reports/2026-11-17-fengshui-form-school-research.md`
取代：`2026-11-17-fengshui-v1-design.md`（八宅，parked）

## 0. 定位與站長裁示

- **風水負責引起興趣，預測大腦負責差異化。** 別家風水網站只告訴你「該怎麼擺」；本站把每條風水說法變成一個**你可以拿自己房間去對答案的假說**。
- 對照的是**現代風水**（港台與現代室內風水的通行說法），不追古籍、不做原典分級。
- 並列表達、互不背書：每張卡都有「**風水怎麼說**」與「**你的大腦怎麼看**」兩欄，兩欄之間不出現「所以」「證實」「科學解釋了」。
- 理氣成分（命卦、方位吉凶、財位度數、文昌桃花、流年飛星、缺角對應家人）一律不收。
- 站長是第一個使用者。v1 以站長自己的臥室和書房能實際走完全程為驗收標準。

三層體驗：

1. **看房間**（公開、匿名、純前端）：選房型 → 視覺化問答 → 結果卡。
2. **兩種說法並列**（同上）：每個格局特徵一張卡，附研究證據強度徽章。
3. **先猜後試**（登入）：從卡片挑一個零成本調整 → 先記下現在的感受、預測改了之後會怎樣 → 實際調整一週 → 回報 → 看預測和實際的差距。

## 1. 規則集（`FENGSHUI_ROOM_RULESET_VERSION = "1"`）

### 1.1 房型

`bedroom`（臥室）、`study`（書房／工作位）。兩種房型的題目各自獨立，不混用。

### 1.2 題目與判定

每題單選，必有「不確定」選項（判定為 `unknown`，卡片顯示「這題你選了不確定，先跳過」）。每題附示意 SVG（俯視簡圖），選項也附小圖。

**臥室**

| id | 題目 | 選項 → 狀態 | 對應主張 |
|---|---|---|---|
| `bed_back` | 床頭後面是什麼？ | 實牆 → ok；窗 → flag；門或走道 → flag；沒靠牆（懸空） → flag | 床頭靠實牆 |
| `bed_door_view` | 躺在床上，看得到房門嗎？ | 看得到、門在斜前方 → ok；門正對床尾 → flag；看不到門 → flag | 看得到門但不正對 |
| `bed_overhead` | 床的正上方有什麼？ | 沒有 → ok；外露的樑 → flag；吊櫃、層板或大型吊燈 → flag | 樑不壓床、頭頂無重物 |
| `bed_mirror` | 躺著時，鏡子照得到你嗎？ | 照不到 → ok；照得到 → flag | 鏡不對床 |
| `bed_corner` | 床邊兩公尺內，有沒有柱角、牆角或櫃角對著床？ | 沒有 → ok；有 → flag | 尖角不對人 |
| `bed_door_door` | 房門和其他房門正對嗎？ | 沒有 → ok；正對 → flag | 門對門 |
| `bed_light` | 晚上睡覺時，房間能暗下來嗎？ | 能（有遮光） → ok；外面的光會照進來 → flag | 明廳暗房 |
| `bed_clutter` | 地面和床底下的雜物多嗎？ | 很少 → ok；有一些 → mild；很多 → flag | 雜物堆積 |

**書房／工作位**

| id | 題目 | 選項 → 狀態 | 對應主張 |
|---|---|---|---|
| `seat_back` | 坐著時，椅背後面是什麼？ | 實牆 → ok；窗 → flag；門或走道 → flag | 座位背後有靠 |
| `seat_door_view` | 坐著時看得到房門嗎？ | 在視線內、不正對 → ok；正對著門 → flag；背對門 → flag | 看得到門但不正對 |
| `seat_overhead` | 座位上方有外露的樑嗎？ | 沒有 → ok；有 → flag | 樑不壓座位 |
| `seat_front` | 桌子前方是開闊的嗎？ | 開闊或有窗景 → ok；直接面對牆，很近 → flag | 明堂開闊 |
| `seat_corner` | 座位附近有尖角對著你嗎？ | 沒有 → ok；有 → flag | 尖角不對人 |
| `seat_light` | 白天座位的自然光夠嗎？ | 夠 → ok；不太夠 → flag | 採光 |
| `seat_clutter` | 桌面和周圍的雜物多嗎？ | 很少 → ok；有一些 → mild；很多 → flag | 雜物堆積 |

狀態 `ok | mild | flag | unknown`。**沒有分數、沒有總評、沒有吉凶等級**：結果只是一組卡片，flag 卡排在前面。

### 1.3 純邏輯位置

`crates/schema/src/fengshui/`：`ruleset.rs`（題目、選項、判定）、`corpus.rs`（卡片語料）、`evidence.rs`（證據等級與文獻鍵）、`mod.rs`（`assess(room, answers) -> Vec<Card>`）。全部是純函式，native 可測；web 透過 ft-schema 直接呼叫，v1 的看房間與卡片**不經 API**（塔羅 v1a 先例）。

## 2. 卡片語料（`FENGSHUI_ROOM_CORPUS_VERSION = "1"`）

每題對應一組卡片，每張卡片四段：

1. **風水怎麼說**（≤100 字）：現代風水通行說法，中性描述。說法分歧時照實寫「各家說法不一」。ok 狀態也有卡（「這一項符合風水常見的建議」），各狀態卡片結構對稱。
2. **你的大腦怎麼看**（≤140 字）：只陳述研究測了什麼、強度多少；沒研究就寫「目前沒有研究直接測過」。
3. **證據徽章**：取自 `EvidenceLevel` 列舉，對應研究報告 §2：
   - `supported`「有研究支持（效果小到中）」
   - `mixed`「研究結果不一致」
   - `indirect`「只有間接研究」
   - `untested`「目前沒有研究直接測過」
4. **可以試試看**（≤80 字，僅 flag／mild）：零成本調整，例如移床、加屏風、隨手關門、整理、遮光簾。這一段也是先猜後試的入口。

徽章初值（實作時以研究報告為準）：

| 題目 | 徽章 |
|---|---|
| `bed_back`、`bed_door_view` | `indirect`（床位無直接研究；瞭望－庇護研究只測過座位） |
| `seat_back`、`seat_door_view` | `mixed` |
| `bed_overhead`、`seat_overhead` | `indirect` |
| `bed_mirror`、`bed_door_door` | `untested` |
| `bed_corner`、`seat_corner` | `mixed` |
| `bed_light`、`seat_light` | `supported` |
| `seat_front` | `indirect`（研究測的是窗景與自然，不是「面牆」） |
| `bed_clutter`、`seat_clutter` | `indirect` |

**另有一張固定的說明卡，放在結果最後**：預測大腦是什麼、為什麼它只是解讀的框架不是證據、以及「你的預期本身就會改變你的感受，這也是預測大腦的一部分」。

**文獻**：`evidence.rs` 存文獻鍵（作者、年份、期刊、URL），卡片可展開「看研究」。

### 2.1 文案紅線（測試守門）

- `corpus_banned_words` 測試：語料不得出現
  - 背書詞：「證實」「證明」「科學解釋」「所以風水」
  - 恐嚇詞：「凶」「血光」「破財」「離婚」「生病」「出殯」「相罵」
  - 化煞商品：「八卦鏡」「山海鎮」「五帝錢」「葫蘆」「水晶」「貔貅」「風鈴」
- 「煞」只能出現在「風水怎麼說」段，作為術語引用（如「風水稱為壓樑煞」），不得出現在大腦段與調整段。
- 長度上限以 `chars().count()` 測試。
- 每題每個非 unknown 狀態都必須有卡（完整性測試）。

## 3. 先猜後試（登入）

### 3.1 流程

1. 在 flag／mild 卡按「試試看」，帶入 `room`、`itemId`、`adjustmentId`。
2. **起點**：回答兩題，1–5 分：
   - 安心感：「待在這個位置，你覺得多安心？」
   - 功能感：臥室問「最近睡得好嗎？」，書房問「最近在這裡能專心嗎？」
3. **預測**：「調整一週後，你猜這兩題會變幾分？」同樣 1–5 分。
4. 實際調整，等一週。
5. **回報**：建立後滿 7 天開放、30 天內有效。回報時**先答實際分數**，送出後才揭曉當初的預測。
6. **對答案**：顯示「你預測 +2，實際 +1」之類的差距，附固定說明：「差距本身就是資訊：大腦在學的就是這個差距。一個人一次的結果不能證明風水有沒有用；你的預期也會影響感受。」

### 3.2 為什麼先答實際再看預測

如果先看到自己的預測，回報時會被它拉過去，誤差就失真。這和 F5 預測的遮蔽原則一致，由 **API 強制**：未回報前，`GET` 不回傳預測值。

### 3.3 限制

- 同時進行中的實驗最多 3 個，同一 `itemId` 同時最多 1 個。
- 超過 30 天未回報 → 狀態 `expired`，仍可看到預測值，不能再回報。
- 不收自由文字（隱私與審查成本）。

## 4. API

全部 `/api/fengshui/experiments` 前綴，DTO camelCase，`Cache-Control: no-store`。

| 方法 | 路徑 | 說明 |
|---|---|---|
| POST | `/api/fengshui/experiments` | 建立。body `{requestId, room, itemId, adjustmentId, baseline:{calm, func}, predicted:{calm, func}}` |
| GET | `/api/fengshui/experiments` | 列表，`page_query` 分頁；未回報者 `predicted` 省略 |
| POST | `/api/fengshui/experiments/:id/report` | body `{actual:{calm, func}}`；回傳含 `predicted` 的完整結果 |
| DELETE | `/api/fengshui/experiments/:id` | 不存在、他人、重刪一律 404 |
| DELETE | `/api/fengshui/experiments` | 全刪，回 `{deleted: n}` |

處理順序（塔羅先例）：IP 便宜限流 → auth → `body_too_large`（64 KiB）→ parse → 驗證 → 冪等查詢 → uid 限流 → 寫入。

驗證：
- `room` ∈ `bedroom|study`；`itemId` 必須屬於該房型；`adjustmentId` 必須屬於該題的 flag／mild 狀態；分數整數 1–5。失敗 400 `INVALID_EXPERIMENT`。
- 進行中已滿 3 個或同題已有 → 409 `EXPERIMENT_LIMIT`。
- 回報：未滿 7 天 → 409 `REPORT_TOO_EARLY`（回 `opensAt`）；已回報 → 409 `ALREADY_REPORTED`；過期 → 409 `EXPIRED`。
- 冪等：同 `requestId` 同 body → 200 回原結果；異 body → 409 `IDEMPOTENCY_CONFLICT`。
- 錯誤一律不夾帶 DB 細節（`"db error"`）。

限流：`fengshui:ip:{ip}` 20/min（fail-open）、`fengshui:create:{uid}` 10/24h。無 AI，不用 strict。

伺服端時間：`created_at`、`reported_at` 一律由伺服端以 `clock::now_iso()` 寫入；7 天與 30 天的門檻在 Rust 端算成同格式 ISO 字串後傳入 SQL 比較（不在 SQL 內混用 `datetime('now')`，避免格式不一致的字串比較）。

## 5. DB（`schema.sql` ＋ `scripts/migrations/2026-11-XX-fengshui.sql`）

```sql
CREATE TABLE IF NOT EXISTS fengshui_experiments (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  request_id TEXT NOT NULL,
  room TEXT NOT NULL,
  item_id TEXT NOT NULL,
  adjustment_id TEXT NOT NULL,
  ruleset_version TEXT NOT NULL,
  corpus_version TEXT NOT NULL,
  baseline_calm INTEGER NOT NULL,
  baseline_func INTEGER NOT NULL,
  predicted_calm INTEGER NOT NULL,
  predicted_func INTEGER NOT NULL,
  actual_calm INTEGER,
  actual_func INTEGER,
  created_at TEXT NOT NULL,
  reported_at TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_fengshui_user_request ON fengshui_experiments(user_id, request_id);
CREATE INDEX IF NOT EXISTS idx_fengshui_user_created ON fengshui_experiments(user_id, created_at);
```

- 房間問答答案**不落庫**，只存實驗用到的 `itemId`／`adjustmentId`。
- 回報用條件式 UPDATE（`WHERE id=? AND user_id=? AND reported_at IS NULL AND created_at <= ?`），以 affected 判定成敗，防併發重複回報。
- 進行中上限用單一條件式 INSERT（`INSERT … SELECT … WHERE (SELECT COUNT(*) …) < 3 AND NOT EXISTS (同 itemId 進行中)`），避免先查後寫的競態。
- 帳號刪除：靠 `ON DELETE CASCADE`（站上無刪帳路由，各表皆此慣例）。
- **風水資料永不寫入** `predictions`／`prediction_feedback`／`f8_assignments`（route 檔頭註明）。

## 6. 前端

- `/fengshui`（公開）：開場（一段話說明三層體驗與「兩種說法並列」）→ 選房型 → 問答（一題一頁，可返回）→ 結果卡。答案存 `localStorage`（`fengshui_room_{room}`），可清除。
- 結果卡：flag 在前；每張卡兩欄並列（手機上下排），證據徽章；「看研究」展開文獻；flag／mild 卡有「試試看」。
- `/fengshui/lab`（Protected）：我的實驗（進行中、可回報、已完成、過期）、回報流程、對答案畫面、刪除與全刪（二次確認、回報筆數）。
- 首頁 feature-grid 加入口；名詞解釋加「風水」分類（樑壓、明堂、穿堂等術語），bump glossary 版本。
- 示意 SVG 以內嵌 Leptos `view!` 繪製，不引入外部 JS（CSP）。
- 色彩沿用現有 token，新色票須照 §色彩紅線重採玻璃底對比。

## 7. 測試與驗收

**ft-schema（native）**
- T1 規則集完整：每題選項皆有狀態；每房型題目 id 不重複。
- T2 語料完整：每題每個非 unknown 狀態都有卡；flag／mild 都有調整段。
- T3 `corpus_banned_words` 與「煞」位置限制。
- T4 長度上限。
- T5 `assess` 排序（flag → mild → ok → unknown）與 unknown 跳過語意。
- T6 證據徽章每題都有、文獻鍵都能解析。

**ft-api（native，純函式部分）**
- T7 驗證函式：房型、題目歸屬、調整歸屬、分數範圍。
- T8 遮蔽：未回報的 view 序列化結果不含 `predicted`。
- T9 回報時窗判定（7 天開放、30 天過期，邊界含等號）。

**部署驗證**：`verify-deployment.sh` 加 `GET /api/fengshui/experiments` 未登入 401。

**站長驗收**：站長用自己的臥室或書房走完看房間 → 開一個實驗；七天後回報並看到對答案畫面。

## 8. 分批

1. **Batch 1（schema）**：規則集、語料、證據、`assess`、T1–T6。語料先出草稿給站長看。
2. **Batch 2（web 看房間）**：`/fengshui` 問答與結果卡、首頁入口、名詞解釋。可以單獨部署（純前端）。
3. **Batch 3（API＋DB＋lab）**：migration、routes、`/fengshui/lab`、T7–T9、verify。部署順序：先 migration，再 API，最後 web。

## 9. 不做（v1）

照片與 AI 判讀（v2，需先 spike）、平面圖拖拉、全屋格局（穿堂、入門見灶等）、窗外形煞、任何理氣內容、統計彙總或跨使用者比較。

## 10. 待決

1. 回報開放時間 7 天、有效 30 天是否合適。
2. 兩個 1–5 分量尺的措辭。
3. 結果卡的「說明卡」要放最後，還是開場就先講。
4. 資料表約束：站長全域規則為「資料庫不加約束」，但本專案各表皆有 FK cascade 與 UNIQUE（冪等依賴 UNIQUE 撞鍵回查）。本 spec 依專案先例；若要依全域規則，冪等改為條件式 INSERT，刪帳改由路由清理。
