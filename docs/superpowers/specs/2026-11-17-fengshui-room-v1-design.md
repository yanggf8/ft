# 空間風水 v1 設計：房間格局 × 你的大腦 × 先猜後試

狀態：v2（自審 → Kimi K3.1 Pro 審 15 條〔7 Blocking〕經 Claude 核實後吸收）→ 待 Fable 5.2 審
日期：2026-11-17
依據：`docs/superpowers/reports/2026-11-17-fengshui-form-school-research.md`
審查紀錄：`docs/superpowers/reports/2026-11-17-fengshui-room-kimi-review.md`
取代：`2026-11-17-fengshui-v1-design.md`（八宅，parked）

## 0. 定位與站長裁示

- **風水負責引起興趣，預測大腦負責差異化。** 別家風水網站只告訴你「該怎麼擺」；本站把每條風水說法變成一個你可以拿自己房間觀察看看的問題。
- 對照的是**現代風水**（港台與現代室內風水的通行說法），不追古籍、不做原典分級。
- 並列表達、互不背書：每張卡都有「**風水怎麼說**」與「**你的大腦怎麼看**」兩欄。
- **預測處理（PP）只能當解讀語言，不能當證據**：不得提升任何證據徽章；某條主張沒有研究時，不得改用 PP 編一個「大腦理論上會警戒」的合理故事來補（Kimi K02）。
- 理氣成分（命卦、方位吉凶、財位、文昌、桃花、流年飛星、羅盤、五行、缺角對應家人）一律不收。
- 站長是第一個使用者。v1 以站長自己的臥室和書房能實際走完、並正確理解（見 §7 站長驗收）為標準。

三層體驗：

1. **看房間**（公開、匿名、純前端）：選房型 → 視覺化問答 → 結果卡。
2. **兩種說法並列**（同上）：每個格局特徵一張卡。
3. **先猜後試**（登入）：挑一個低成本調整 → 記下過去一週的感受、猜調整後一週會是幾分 → 實際調整 → 回報 → 看預測與自評的差距，當作自我觀察的線索。

## 1. 規則集（`FENGSHUI_ROOM_RULESET_VERSION = "1"`）

### 1.1 房型

`bedroom`（臥室）、`study`（書房／工作位）。兩種房型的題目各自獨立。

### 1.2 題目與判定

每題單選，必有「不確定」選項（狀態 `unknown`）。每題附俯視示意 SVG，選項也附小圖；選項可直接點圖選。

**臥室**

| id | 題目 | 選項 → 狀態 | 對應主張 |
|---|---|---|---|
| `bed_back` | 床頭後面是什麼？ | 實牆 → ok；窗 → flag；門或走道 → flag；沒靠牆 → flag | 床頭靠實牆 |
| `bed_door_view` | 躺在床上，看得到房門嗎？ | 看得到、門在斜前方 → ok；門正對床尾 → flag(`facing`)；看不到門 → flag(`hidden`) | 看得到門但不正對 |
| `bed_overhead` | 床的正上方有什麼？ | 沒有 → ok；外露的樑 → flag(`beam`)；吊櫃、層板或大型吊燈 → flag(`heavy`) | 樑不壓床、頭頂無重物 |
| `bed_mirror` | 躺著時，鏡子照得到你嗎？ | 照不到 → ok；照得到 → flag | 鏡不對床 |
| `bed_corner` | 床旁邊有沒有柱角、牆角或櫃角，尖端朝著床？ | 沒有 → ok；有 → flag | 尖角不對人 |
| `bed_door_door` | 房門和其他房門正對嗎？ | 沒有 → ok；正對 → flag | 門對門 |
| `bed_light` | 晚上睡覺時，房間能暗下來嗎？ | 能 → ok；外面的光會照進來 → flag | 明廳暗房 |
| `bed_clutter` | 地面和床底下的雜物多嗎？ | 很少 → ok；有一些 → mild；很多 → flag | 雜物堆積 |

**書房／工作位**

| id | 題目 | 選項 → 狀態 | 對應主張 |
|---|---|---|---|
| `seat_back` | 坐著時，椅背後面是什麼？ | 實牆 → ok；窗 → flag；門或走道 → flag | 座位背後有靠 |
| `seat_door_view` | 坐著時看得到房門嗎？ | 在視線內、不正對 → ok；正對著門 → flag(`facing`)；背對門 → flag(`hidden`) | 看得到門但不正對 |
| `seat_overhead` | 座位上方有外露的樑嗎？ | 沒有 → ok；有 → flag | 樑不壓座位 |
| `seat_front` | 桌子前方是開闊的嗎？ | 開闊或有窗景 → ok；直接面對很近的牆 → flag | 明堂開闊 |
| `seat_corner` | 座位旁邊有尖角朝著你嗎？ | 沒有 → ok；有 → flag | 尖角不對人 |
| `seat_light` | 白天座位的自然光夠嗎？ | 夠 → ok；不太夠 → flag | 採光 |
| `seat_clutter` | 桌面和周圍的雜物多嗎？ | 很少 → ok；有一些 → mild；很多 → flag | 雜物堆積 |

狀態 `ok | mild | flag | unknown`；同為 flag 但證據不同的選項以變體區分（如 `facing`／`hidden`），各有自己的卡。**沒有分數、沒有總評、沒有吉凶等級。**題目中不使用無依據的精確距離（Kimi K15）。

### 1.3 純邏輯位置

`crates/schema/src/fengshui/`：`ruleset.rs`（題目、選項、判定）、`corpus.rs`（卡片語料）、`evidence.rs`（徽章與文獻）、`wire.rs`（實驗 DTO，前後端共用）、`mod.rs`（`assess(room, answers) -> Vec<Card>`）。純函式，native 可測；web 直接呼叫 ft-schema，看房間與卡片**不經 API**（塔羅 v1a 先例）。

## 2. 卡片語料（`FENGSHUI_ROOM_CORPUS_VERSION = "1"`）

卡片以 `(itemId, 狀態或變體)` 為單位。每張卡三段：

1. **風水怎麼說**（≤100 字）：現代風水通行說法，中性描述；分歧時照實寫「各家說法不一」。ok 卡也有（「這一項符合風水常見的建議」）。
2. **你的大腦怎麼看**（≤140 字）：只陳述研究測了什麼、在什麼結果上（睡眠、情緒、注意力、偏好評分……）、強度多少；沒有研究就寫「目前沒有研究直接測過」，不再補任何理論推想。**證據徽章只放在這一欄內**，標題固定為「相關環境研究」，下方固定一行「這些研究不是在檢驗風水」（Kimi K02）。
3. **可以試試看**（≤80 字，僅 flag／mild）：**不買化煞商品的低成本調整**，優先順序：關門、用現有的布簾或家具、整理一小區、調整枕頭方向；涉及大型家具時加「量力而為，不要獨自搬重物」（Kimi K13）。這一段是先猜後試的入口。

### 2.1 證據徽章（兩軸，Kimi K01）

`EvidenceBadge { reach, state }`：

- `reach`：`direct`「研究直接測過這個特徵」／`related`「只測過相關的空間特徵」／`none`「目前沒有研究直接測過」
- `state`（`reach != none` 時）：`consistent`「結果大致一致」／`mixed`「結果不一致」／`correlational`「只有相關，不能推論因果」／`single`「只有單一研究」

效果量、樣本、研究設計放在「看研究」展開區，不放徽章。

初值（**未經 §2.2 原文核對前一律不得高於此表**）：

| 卡 | reach | state | 依據摘要 |
|---|---|---|---|
| `bed_back` flag | related | mixed | 瞭望－庇護只測過座位與景觀偏好，統合分析不一致 |
| `bed_door_view` hidden | related | mixed | 同上（「看得到」這一半） |
| `bed_door_view` facing | none | — | 床尾對門無研究 |
| `bed_overhead` beam | related | single | 橫樑無研究；封閉空間壓力下皮質醇（單一 VR 研究） |
| `bed_overhead` heavy | none | — | |
| `bed_mirror` | none | — | |
| `bed_corner`、`seat_corner` | related | mixed | 曲線偏好以抽象圖形為主；真實桌椅 2025 未複製 |
| `bed_door_door` | none | — | |
| `bed_light` | direct | consistent | 夜間光照影響睡眠與作息（結果：睡眠） |
| `bed_clutter`、`seat_clutter` | direct | correlational | Saxbe & Repetti 2010 等（結果：壓力荷爾蒙型態、情緒） |
| `seat_back`、`seat_door_view` hidden | related | mixed | 座位偏好研究（結果：偏好，非壓力） |
| `seat_door_view` facing | none | — | |
| `seat_overhead` | related | single | 同 `bed_overhead` beam |
| `seat_front` | related | mixed | 注意力恢復與窗景（結果：注意力、情緒；效果小、異質大） |
| `seat_light` | related | single | 有窗與睡眠（Boubekri 2014，小樣本）；與「專心」無直接研究 |

ok 卡沿用同題 flag 卡的徽章（同一批研究）。

### 2.2 原文核對閘門（Kimi K03）

Batch 1 交付 `docs/superpowers/reports/fengshui-research/evidence-ledger.md`，每篇文獻記錄：研究設計、樣本、實際測量的結果、限制、**是否讀過全文**。規則：

- 未讀全文的文獻不得支撐 `consistent`；只讀摘要者，該卡徽章 `state` 降一級或維持保守文案。
- 大腦段每句話都要能對到 ledger 的一列。

### 2.3 說明的兩個位置（Kimi K02；調整原「已決 3」）

- **開場一句**（問答前）：「每一項會並列風水的說法和相關的環境研究。這些研究不是在檢驗風水，有些項目根本沒有人研究過。」
- **結果最後一張說明卡**：預測大腦是什麼、為什麼它只是解讀的框架不是證據、「你的預期本身就會影響你的感受」。

### 2.4 文案紅線（測試＋人工審稿，Kimi K11）

- `corpus_banned_words` 分段詞表（substring）：
  - 全段禁用──背書：「證實」「證明」「科學解釋」「科學根據」「研究印證」「腦科學支持」「所以風水」；恐嚇：「凶」「血光」「破財」「漏財」「離婚」「影響婚姻」「生病」「有損健康」「災厄」「意外」「運勢」「出殯」「相罵」；化煞商品：「八卦鏡」「山海鎮」「五帝錢」「葫蘆」「水晶」「貔貅」「風鈴」「化解」；理氣：「命卦」「吉方」「財位」「文昌」「桃花」「流年」「飛星」「羅盤」「五行」「缺角」。
  - 「煞」只能出現在「風水怎麼說」段，作為術語引用。
  - 研究段允許「睡眠」「焦慮」「壓力」等詞，但與風水段的組合不得構成因果或醫療宣稱──由人工審稿把關。
- **人工審稿閘門**：語料定稿前，Claude 逐卡審一次語意（含兩欄對讀），再給站長看；Batch 2 上線前看實際手機畫面，確認徽章不會被讀成「這條風水有研究支持」。
- 長度上限以 `chars().count()` 測試；每題每個非 unknown 狀態／變體都必須有卡。

### 2.5 版本規則（Kimi K09）

任何使用者看得到的卡片文字、徽章、文獻語意改動，都必須 bump `FENGSHUI_ROOM_CORPUS_VERSION`；題目、選項、判定改動 bump `FENGSHUI_ROOM_RULESET_VERSION`。

## 3. 先猜後試（登入）

### 3.1 流程

1. 在 flag／mild 卡按「試試看」。開始前一句揭露：「會上傳的只有：房型、這一項、你選的調整、以及你打的分數。」
2. **起點**（過去 7 天），1–5 分，附兩端錨點：
   - 安心感：「過去 7 天，待在這個位置你覺得多安心？」（1 很不安心 … 5 很安心）
   - 功能感：臥室「過去 7 天睡得好嗎？」（1 很差 … 5 很好）；書房「過去 7 天在這裡能專心嗎？」（1 很難 … 5 很容易）
3. **預測**：「調整後的 7 天，你猜這題會是幾分？」緊湊呈現「現在 3 → 一週後你猜 〔 〕」；可以維持或變差，不預設會改善（Kimi K08）。
4. 實際調整；lab 顯示開放倒數，可下載 `.ics` 行事曆提醒（純前端產生，Kimi K14）。
5. **回報**（滿 7 天開放、滿 30 天當下仍可、超過才過期）：
   - 先問「這一週你有做這個調整嗎？」（有／部分／沒有）
   - 再答過去 7 天的兩題分數。**回報時起點與預測都不顯示**，送出後一起揭曉（Kimi K08）。
6. **對答案**：顯示起點、預測、實際三點，附固定說明：
   > 「你可以把預測和實際的差距當作一個觀察線索。一次的結果分不出是調整、你的預期、生活上的其他變化，還是評分本身的起伏造成的，所以它不能說明這個調整有沒有用，也不能說明風水對不對。」
   UI 用語是「自我觀察」，不用「實驗驗證」（Kimi K04）。

### 3.2 為什麼回報時看不到預測

先看到自己的預測（或起點），回報時容易被拉過去。這是**降低錨定的設計，不是保密**：使用者本來就知道自己填過什麼。由 API 統一的 view 轉換強制（§4.3）。

### 3.3 限制

- **每個房型同時最多 1 個進行中的觀察**（全帳號最多 2 個），避免同房間多個調整互相混雜（Kimi K04）。
- 進行中 = `reported_at IS NULL AND created_at >= expiry_cutoff`；過期列不占名額（Kimi K05）。
- 過期後可以看到起點與預測，不能回報。
- 不收自由文字。

## 4. API

前綴 `/api/fengshui/experiments`；DTO 放 `ft_schema::fengshui::wire`（camelCase）。**所有回應（含錯誤）**都經 route 內 helper 加 `Cache-Control: private, no-store` 與 `Vary: Authorization`（現有 `error::error_code` 不帶 cache header，不能只照抄塔羅成功路徑，Kimi K10）。

### 4.1 端點

| 方法 | 路徑 | 成功 |
|---|---|---|
| POST | `/api/fengshui/experiments` | 201 `ExperimentView`（冪等重放 200） |
| GET | `/api/fengshui/experiments?limit&cursor` | 200 `{items: [ExperimentView], nextCursor}`，沿用 `page_query`（1–20、`(created_at,id)` cursor） |
| POST | `/api/fengshui/experiments/:id/report` | 200 `ExperimentView`（完整） |
| DELETE | `/api/fengshui/experiments/:id` | 200 `{deleted: 1}` |
| DELETE | `/api/fengshui/experiments` | 200 `{deleted: n}` |

建立 body：`{requestId, room, itemId, variant?, adjustmentId, baseline:{calm, func}, predicted:{calm, func}}`
回報 body：`{adherence: "yes"|"partly"|"no", actual:{calm, func}}`

### 4.2 `ExperimentView`

`{id, room, itemId, variant?, adjustmentId, itemLabel, adjustmentText, status, baseline?, predicted?, adherence?, actual?, opensAt, expiresAt, createdAt, reportedAt?, rulesetVersion, corpusVersion}`

- `status` ∈ `waiting`（未滿 7 天）／`open`（可回報）／`done`／`expired`，由伺服端用**同一次取得的 now** 衍生。
- `itemLabel`、`adjustmentText` 讀自建立時的快照欄位（§5）。

### 4.3 遮蔽（唯一 view 轉換，Kimi K06）

一個函式 `to_view(row, now)` 產生所有回應（建立、冪等重放、列表、回報）：
- `waiting`／`open`：省略 `baseline` 與 `predicted`（`skip_serializing_if`）。
- `done`／`expired`：完整回傳。
- 建立回應同樣遮蔽──前端在建立當下自己顯示剛填的值即可，不依賴伺服端回傳。

### 4.4 處理順序與錯誤

順序（塔羅先例）：IP 便宜限流 `fengshui:ip:{ip}` 20/min（fail-open）→ auth → `body_too_large`（64 KiB）→ parse → 驗證 → 冪等查詢 → uid 限流 `fengshui:create:{uid}` 10/24h → 寫入。無 AI，不用 strict。

| 情況 | 回應 |
|---|---|
| JSON 錯、`requestId` 非 UUID 形狀（比照 `valid_tarot_request_id`）、房型、題目、變體、調整歸屬、分數 1–5、`adherence` 值不合 | 400 `INVALID_EXPERIMENT` |
| 路徑 `:id` 非 UUID 形狀、不存在、他人、重刪 | 404 `NOT_FOUND` |
| 同房型已有進行中 | 409 `ROOM_ACTIVE` |
| 同 `requestId` 不同 body | 409 `IDEMPOTENCY_CONFLICT` |
| 回報太早 | 409 `REPORT_TOO_EARLY`（附 `opensAt`） |
| 已回報且 body 不同 | 409 `ALREADY_REPORTED` |
| 過期 | 409 `EXPIRED` |
| DB 錯誤 | 500 `"db error"`，不夾帶細節 |

### 4.5 時間

單次取得 `now = clock::now_iso()`，在 Rust 端算出 `open_cutoff = now − 7d`、`expiry_cutoff = now − 30d`（同格式 ISO 字串），傳入 SQL。邊界：`created_at <= open_cutoff` 開放；`created_at >= expiry_cutoff` 仍有效（滿 30 天當下可回報）。SQL 內不混用 `datetime('now')`。

## 5. DB（`schema.sql` ＋ `scripts/migrations/2026-11-XX-fengshui.sql`）

```sql
CREATE TABLE IF NOT EXISTS fengshui_experiments (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  request_id TEXT NOT NULL,
  room TEXT NOT NULL,
  item_id TEXT NOT NULL,
  variant TEXT,
  adjustment_id TEXT NOT NULL,
  item_label TEXT NOT NULL,        -- 建立時快照
  adjustment_text TEXT NOT NULL,   -- 建立時快照
  ruleset_version TEXT NOT NULL,
  corpus_version TEXT NOT NULL,
  baseline_calm INTEGER NOT NULL,
  baseline_func INTEGER NOT NULL,
  predicted_calm INTEGER NOT NULL,
  predicted_func INTEGER NOT NULL,
  adherence TEXT,
  actual_calm INTEGER,
  actual_func INTEGER,
  created_at TEXT NOT NULL,
  reported_at TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_fengshui_user_request ON fengshui_experiments(user_id, request_id);
CREATE INDEX IF NOT EXISTS idx_fengshui_user_created ON fengshui_experiments(user_id, created_at);
```

### 5.1 建立（條件式 INSERT，Kimi K07）

```sql
INSERT INTO fengshui_experiments (...)
SELECT ?…
WHERE NOT EXISTS (
  SELECT 1 FROM fengshui_experiments
  WHERE user_id = ?u AND room = ?room
    AND reported_at IS NULL AND created_at >= ?expiry_cutoff)
```

用 `db::exec_changes` 取 affected：
- 1 → 201。
- 0、UNIQUE 撞鍵、或傳輸錯誤（不確定是否寫入）→ **先以 `(user_id, request_id)` 回查**：存在且同 body → 200 原結果；存在但不同 body → 409 `IDEMPOTENCY_CONFLICT`；不存在 → affected=0 時回 409 `ROOM_ACTIVE`，其他錯誤回 500（塔羅 `tarot.rs:421-437` 先例）。

### 5.2 回報（條件式 UPDATE，Kimi K05）

```sql
UPDATE fengshui_experiments
SET adherence = ?, actual_calm = ?, actual_func = ?, reported_at = ?now
WHERE id = ? AND user_id = ?
  AND reported_at IS NULL
  AND created_at <= ?open_cutoff
  AND created_at >= ?expiry_cutoff
```

affected=1 → 回讀後 200 完整 view。affected=0 → 以 `id + user_id` 回查分類：不存在 → 404；已回報且 `adherence`／`actual` 與本次相同 → 200 完整 view（回應遺失後的重試，Kimi K07）；已回報但不同 → 409 `ALREADY_REPORTED`；太早 → 409 `REPORT_TOO_EARLY`；過期 → 409 `EXPIRED`。

### 5.3 其他

- 房間問答答案不落庫。
- 帳號刪除靠 `ON DELETE CASCADE`（站上無刪帳路由，各表皆此慣例）。
- **風水資料永不寫入** `predictions`／`prediction_feedback`／`f8_assignments`（route 檔頭註明）。

## 6. 前端

- `/fengshui`（公開）：開場一句揭露（§2.3）＋「答案只存在這個瀏覽器，不會上傳」→ 選房型 → 問答（俯視圖點選、進度條、可返回）→ 結果。
  - 答案存 `localStorage` `fengshui_room_v1_{room}`（key 帶 ruleset 版本），結果頁與開場都有顯眼的「清除我的房間答案」。不設 TTL（明文取捨：個人裝置為主，清除鈕足夠）（Kimi K12）。
- 結果頁（Kimi K14）：預設只展開 flag／mild 卡；ok 卡收在「已符合常見建議」折疊區、unknown 收在「還不確定的項目」；最後是說明卡。flag／mild 卡有「試試看」。
- 卡片：手機上「風水怎麼說」與「你的大腦怎麼看」上下排，桌機左右並列；徽章在大腦欄內；「看研究」展開文獻。
- `/fengshui/lab`（Protected）：我的觀察（等待中附倒數與 `.ics` 下載、可回報、已完成、過期）、回報流程、對答案畫面、刪除與全刪（二次確認、回報筆數）。
- 首頁 feature-grid 加入口；名詞解釋加「風水」分類（樑壓、明堂、門對門等），bump glossary 版本。
- 示意 SVG 以 Leptos `view!` 內嵌繪製，不引入外部 JS（CSP）。
- 色彩沿用現有 token；新色票照色彩紅線重採玻璃底對比。

## 7. 測試與驗收

**ft-schema（native）**
- T1 規則集完整：每題選項皆有狀態；題目 id 不重複；變體只出現在 flag。
- T2 語料完整：每題每個非 unknown 狀態／變體都有卡；flag／mild 都有調整段。
- T3 分段禁用詞與「煞」位置限制。
- T4 長度上限。
- T5 `assess` 分組（flag → mild → ok 折疊 → unknown 折疊）。
- T6 每張卡都有徽章；`reach=none` 時 `state` 必為空；徽章引用的文獻鍵都在 ledger 中存在。
- T7 wire 驗證函式：房型、題目、變體、調整歸屬、分數、`adherence`、`requestId` 形狀。

**ft-api（native，純函式）**
- T8 `to_view`：`waiting`／`open` 序列化後的 **JSON 實際不含** `baseline`、`predicted` 鍵；`done`／`expired` 含。
- T9 時窗：`status` 判定與 cutoff 計算，邊界（剛好 7 天、剛好 30 天）。

**併發與 DB 行為（驗收腳本，Kimi K07）**
`scripts/fengshui-e2e.sh`，比照 `f8-acceptance.sh`（DB 自造 login token、`--cleanup`），對正式站跑：
- 同房型並發建立 4 次 → 恰好 1 列。
- 同 `requestId` 並發 → 恰好 1 列，其餘 200 重放。
- 並發回報 → 不覆寫；相同 body 重試 → 200。
- 時窗邊界以 SQL 直接改 `created_at` 製造（6 天 23 時、7 天、30 天、30 天＋1 秒）。
- 未回報前所有回應（建立、重放、列表）都不含 `predicted`。
- 錯誤回應也帶 `no-store`。

**部署驗證**：`verify-deployment.sh` 加 `GET /api/fengshui/experiments` 未登入 401。

**站長驗收**：
1. 用自己的臥室或書房走完看房間，覺得好玩、不像問卷。
2. 看完卡片能用自己的話說出：「這些研究不是在證明風水」、「有些項目根本沒人研究過」。
3. （Batch 3 後）開一個觀察，七天後回報，看到對答案畫面，並能說出「這次結果不能說明調整有沒有用」。

## 8. 分批

1. **Batch 1（schema＋語料＋ledger）**：規則集、語料、徽章、`assess`、wire DTO、T1–T7；`evidence-ledger.md` 原文核對。語料經 Claude 語意審稿後給站長看。
2. **Batch 2（web 看房間）**：`/fengshui`、首頁入口、名詞解釋；手機實機審徽章呈現。純前端，可單獨部署。

**站長驗收閘門（2026-11-17 裁示）**：Batch 1＋2 上線後，站長先用自己的房間實際走過，再決定 Batch 3 做不做、怎麼做。理由：先猜後試是記錄型互動，和已下架的小練習同型，有「不好玩」的風險。

3. **Batch 3（API＋DB＋lab，待閘門）**：migration、routes、`/fengshui/lab`、T8–T9、`fengshui-e2e.sh`、verify。部署順序：migration → API → web。

## 9. 不做（v1）

照片與 AI 判讀（v2，需先 spike）、平面圖拖拉、全屋格局（穿堂、入門見灶等）、窗外形煞、任何理氣內容、統計彙總或跨使用者比較、自由文字。

## 10. 已決

2026-11-17 站長同意預設，並經 Kimi 審查後微調：

1. 回報滿 7 天開放、滿 30 天當下仍有效、超過才過期。這只是產品回訪時窗，不代表 7 天足以判斷效果。
2. 量尺：三次都用「過去 7 天」或「調整後的 7 天」同一參照期間，1–5 絕對分數加兩端錨點（§3.1）；不包裝成經驗證量表。
3. 說明分兩處：開場一句、結果最後一張完整說明卡（原「只放最後」依 Kimi K02 調整）。
4. 資料表依專案先例：FK `ON DELETE CASCADE`＋`UNIQUE(user_id, request_id)`。UNIQUE 是並發冪等的必要條件；值域檢查由 app 驗證、不加 CHECK。
5. 進行中上限由「全帳號 3 個」改為「每房型 1 個」（Kimi K04）。
