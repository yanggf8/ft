# 設計：F5 預測 API 層（週期生成 + F6 兩段式回饋）

日期：2026-09-04
狀態：**技術設計 — 已過 Grok 審查（2026-09-04，read-only）並完成 corroborate；本版已吸收審查**
前置：
- `2026-09-03-f5-rule-anchors-design.md`（錨點目錄形狀、命中/擇一/coverage、三張表 DDL、wire 強型別 — 已落地）
- `2026-09-03-f4-f5-if-then-design-note.md` §5.4（F6 兩段式定案編碼、順序約束、鍵形狀）
- `2026-08-28-big5-f1-design.md`（計分、切點「取整後顯示分 ≥67 高 / [33,67) 中 / <33 低」、`personality_profiles`）

> **本文不重審** F5 目錄/命中/coverage/DDL/wire（已在 schema 層落地並測試）；僅補上 **API 層**：把純函數變成可被前端消費的端點，並把 F6 兩段式的測量保護落成 API 強制規則。

> **Grok 審查（2026-09-04, grok-4.6, read-only）**：4 個 P0、多個 P1/P2、D1–D7 逐項裁決。關鍵事實已 corroborate（2026-09-04 為週五 ⇒ 原 cycle 測試#1 自相矛盾；repo 已有 `interpretations UNIQUE(user_id, divination_type)` ⇒ 「無約束慣例」不成立；`filter_negative_half` 在 n=2 全負時丟至空；`DELETE /api/personality/me` 目前只刪側寫）。以下 §1–§8 為吸收審查後的修訂版；§9 保留審查意見摘要與逐項裁決。

## 0. 範圍

**做**：
- `cycle_id` 純函數（Asia/Taipei 週一 00:00 起算的週起始日 YYYY-MM-DD）
- 週期生成：最新 complete 側寫 → 取整顯示分 + 每維三題全距 → `select_for_domain`(work/money) → `filter_negative_half` → **cycle 級凍結**冪等落庫
- 四個端點：列週預測／生成／情境回報（第 1 段）／反應回報（第 2 段）
- F6 測量保護的 API 強制：**forecast 遮罩**、第 2 段僅在 `occurred` 後可交、**回報一次性鎖定**、寫入僅限當週
- F7 資料刪除延伸：`DELETE /api/personality/me` 連帶清 predictions / situation_checks / prediction_feedback / prediction_generations

**不做**：
- 前端 UI（本切片只保證 API 契約與測量保護；UI 動線見 §6）
- F4 五領域強度（0–3 級）的 UI 與閘門 — 見 §5 D1
- F8 對照組（`is_control` 恆 0）、`personal_record` source（恆 `rule_anchor`）
- 付費計次、LLM 潤寫 `experiment`（v1 逐字存錨點文字）

## 1. 純函數（`ft-schema`，全部 native 可測、wasm32 可編）

### 1.1 `cycle_id` — `crates/schema/src/cycle.rs`（新）

```rust
/// UTC ISO 時刻 → Asia/Taipei（UTC+8 固定，無 DST）當週**週一 00:00** 的 YYYY-MM-DD。
/// 輸入不可解析（含缺毫秒/時區）→ None（fail-closed，route 層回 500/400）。
pub fn week_start_asia_taipei(utc_iso: &str) -> Option<String>;
```

- Taipei 自 1979 起無 DST，固定 +8；不需 tz 資料庫、不需 `js_sys`。
- 解析器**必須吃毫秒**：`clock::now_iso()` = JS `toISOString()` → `2026-09-04T10:00:00.000Z`（含 `.000Z`）。
- 意涵：週一 00:00（台北）鎖上週；「當週」＝該時刻所在的週一。
- 釘死測試（Grok P0-1 修正）：
  - `2026-09-04T10:00:00.000Z`（台北週五 18:00）→ `2026-08-31`（當週週一）
  - `2026-09-06T16:30:00.000Z`（台北 09-07 週一 00:30）→ `2026-09-07`
  - `2026-09-06T10:00:00.000Z`（台北 09-06 週日 18:00）→ `2026-08-31`
  - `2026-09-06T16:00:00.000Z`（台北 09-07 週一 00:00 整）→ `2026-09-07`
  - 壞輸入（`""`、無 `Z`、亂字串）→ `None`

### 1.2 生成輸入轉換 — `crates/schema/src/predict.rs`（擴充）

```rust
/// OceanScores → 取整後顯示分 [f64;5]（F1 §5 切點用取整顯示分，避免 66.7 顯示 67 卻走中檔）。
/// 索引 0..4 對齊 DIMENSION_NAMES。
pub fn display_rounded(s: &OceanScores) -> [f64; 5];

/// ipip_answers [15]（1–5）→ 每維三題全距 [u8;5]（max−min）。反向題不影響全距，故不翻轉。
/// length != 15 → None（fail-closed：不生成，而非無降級；Grok P2）。
pub fn dim_ranges(answers: &[u8]) -> Option<[u8; 5]>;
```

### 1.3 `RULES_VERSION` — `crates/schema/src/anchors.rs`（擴充）

```rust
/// 語意遞增：目錄實質變更（增/改錨點、改切點）才 bump。
pub const RULES_VERSION: &str = "rules-1";
```

### 1.4 `filter_negative_half` 例外（D2-A，Grok 裁決）

v1 只有 2 領域，全負面週若依嚴格的 `neg*2<=total` 會丟至空 → 系統性清空低 A/C/ES 特質樣本（F8 undersample）。修訂（`predict.rs`）：

```rust
/// 例外：total==2 且兩條皆 Negative → 保留 1 條（coverage 較高者勝；同 coverage 比 priority 小者勝），
/// 接受該週 1/1 負面，並在 F8 登記「三領域落地後廢除此例外」。
/// 其餘情況維持嚴格 `neg * 2 <= total`（Neutral 永不丟）。
pub fn filter_negative_half(mut selected: Vec<Selected<'static>>) -> Vec<Selected<'static>>;
```

> per-domain 語意**不可取**（1 條輸出任一 Negative 即 100% 違規，空週更常見）— Grok P1 裁決。

## 2. DB 形狀（`scripts/schema.sql` 修訂）

既有三張表不動，新增**兩項**（沿用 repo 既有 UNIQUE 前例：`interpretations`、`usage_tracking`）：

```sql
-- F5：cycle 級生成快照（Grok P0-4 凍結語意）：一週一 profile 一快照；空週也寫（凍結）
CREATE TABLE IF NOT EXISTS prediction_generations (
  user_id      TEXT NOT NULL,
  cycle_id     TEXT NOT NULL,
  profile_id   TEXT NOT NULL,
  generated_at TEXT NOT NULL,
  PRIMARY KEY (user_id, cycle_id)
);

-- 防呆：一週一領域一列（既有 index 之外額外加 UNIQUE；冪等）
CREATE UNIQUE INDEX IF NOT EXISTS idx_predictions_user_cycle_domain
  ON predictions(user_id, cycle_id, domain);
```

- `prediction_generations` 是 cycle 級**凍結閘**：已有列 → 整次 generate 只回現況，**絕不補 domain**（防週中重測混 profile：一週兩套 OCEAN 會毀掉洗牌對照可解釋性）。
- `prediction_feedback` 無 FK 依賴也無妨（本切片不改 predictions 列）；SQLite FK/CASCADE 未開，F7 用顯式 DELETE。

## 3. 生成流程（`crates/api/src/services/predictions.rs`，新）

### 3.1 `generate(db, user_id, cycle_id) -> GenOutcome`

```
1. SELECT profile_id FROM prediction_generations WHERE user_id AND cycle_id
   → 有 → 回現況列表，generated=false（cycle 凍結，重測不重算）
2. SELECT id, ipip_answers, ocean_measured FROM personality_profiles
   WHERE user_id=? AND measurement_status='complete'
   ORDER BY created_at DESC, rowid DESC LIMIT 1
   → 無 → Err(PROFILE_INCOMPLETE 409)
3. ocean_measured → OceanScores（反序列化失敗 → Err(DB_ERROR 500)，不當空週）
   ipip_answers → [u8;15]（dim_ranges 回 None → Err(DB_ERROR 500)，fail-closed）
4. display = display_rounded(&ocean)；ranges = dim_ranges(&answers).unwrap()
5. sel = [select_for_domain(Work, display, ranges), select_for_domain(Money, display, ranges)]
        .into_iter().flatten() → Vec<Selected>
   sel = filter_negative_half(sel)                 // D2-A 例外已含
6. 先插 prediction_generations 快照列（PRIMARY KEY 擋重複；affected==0 表示併發/已凍結 → 回現況）
7. 對每個 Selected：INSERT INTO predictions (…15 欄) SELECT ?1..?15
   WHERE NOT EXISTS (SELECT 1 FROM predictions WHERE user_id AND cycle_id AND domain)
   （UNIQUE 當防呆；WHERE NOT EXISTS 原子；SQLite 單寫者）
8. 回 GenOutcome { created, predictions: 當週全列表 }（已含遮罩，見 §4）
```

- **快照完整性**：`prediction_generations.profile_id` 與每列 `predictions.profile_id` 皆=步驟 2 的 id。
- **空週也凍結**：全部命中為空或全被濾掉 → 仍寫 generations 列（防重測後「從空變有列」）。
- **domain 順序**：work 先、money 後（決定性）。

### 3.2 列表 `list_cycle(db, user_id, cycle_id) -> CycleView`

```sql
-- predictions：固定領域序（UI 穩定）
SELECT * FROM predictions WHERE user_id=? AND cycle_id=? 
  ORDER BY CASE domain WHEN 'work' THEN 0 WHEN 'money' THEN 1 ELSE 2 END;
-- checks
SELECT * FROM situation_checks WHERE user_id=? AND cycle_id=? ORDER BY trigger;
-- feedback（Grok P1：回訪中斷後 UI 需知第 2 段是否已交）
SELECT * FROM prediction_feedback pf JOIN predictions p ON p.id=pf.prediction_id
  WHERE p.user_id=? AND p.cycle_id=?;
```

`CycleView { predictions, checks, feedback }`。

### 3.3 情境回報 `upsert_check(db, user_id, cycle_id, trigger, situation)`

```
1. cycle_id != 當週 → Err(STALE_CYCLE 409)                 // D4 統一 409
2. SELECT 1 FROM predictions WHERE user_id AND cycle_id AND trigger LIMIT 1
   → 無 → Err(UNKNOWN_TRIGGER 400)                        // 只能回報當週有預測的 trigger（D7）
3. 鎖定檢查（Grok P0-3）：該 (user,cycle,trigger) 下任一 prediction 已有 feedback →
   Err(SITUATION_LOCKED 409)                              // 一旦進第 2 段，第 1 段不得再改
4. INSERT INTO situation_checks (user_id, cycle_id, trigger, situation, created_at)
   VALUES (?1..?5)
   ON CONFLICT(user_id, cycle_id, trigger) DO UPDATE
     SET situation=excluded.situation, created_at=excluded.created_at
   （僅在無 feedback 時允許覆寫：absent↔occurred 皆可，未進第 2 段前容錯）
```

### 3.4 反應回報 `record_feedback(db, user_id, prediction_id, response)`

```
1. SELECT user_id, cycle_id, trigger FROM predictions WHERE id=?1
   → 無 OR user_id 不符 → Err(NOT_FOUND 404)              // 不洩漏存在性
2. cycle_id != 當週 → Err(STALE_CYCLE 409)
3. SELECT situation FROM situation_checks WHERE user_id AND cycle_id AND trigger
   → 無 → Err(SITUATION_REQUIRED 409)                     // 第 1 段未答
   → absent → Err(SITUATION_ABSENT 409)                   // 第 1 段=absent 不得進第 2 段
4. SELECT 1 FROM prediction_feedback WHERE prediction_id=?1
   → 有 → Err(FEEDBACK_EXISTS 409)                        // 一次性（D5：否決 latest-wins）
5. INSERT INTO prediction_feedback (prediction_id, response, created_at) VALUES (?1,?2,?3)
```

**F8 編碼（寫死）**：`occurred` 且未交第 2 段 = **缺測**（不進分母、不當 miss）— 掉答不得向下偏命中率；`absent` 不進分母；`occurred+hit` 分子分母；`occurred+miss/other` 僅分母。

## 4. API 契約（`crates/api/src/routes/predictions.rs`，新；註冊進 `routes/mod.rs`）

全部 route：`auth_user` 護欄。**GET 不限流** + `apply_cache_headers(0, true)`（對齊 `/personality/me`）；寫入端點（generate/checks/feedback）per-IP 10/min（`predictions:ip:` 命名空間）。

| 端點 | 請求 | 回應 | 錯誤 |
|---|---|---|---|
| `GET /api/predictions?cycleId=` | —（省略＝當週） | `ListPredictionsResponse { cycleId, checks[], predictions[], feedback[] }` | 400 `INVALID_CYCLE` |
| `POST /api/predictions/generate` | — | `GeneratePredictionsResponse { cycleId, generated: bool, predictions[] }` | 409 `PROFILE_INCOMPLETE` |
| `PUT /api/predictions/checks` | `CheckSituationRequest { cycleId?, trigger, situation }` | `SituationCheck` | 409 `STALE_CYCLE`、409 `SITUATION_LOCKED`、400 `UNKNOWN_TRIGGER` |
| `POST /api/predictions/:id/feedback` | `FeedbackRequest { response }` | `PredictionFeedback` | 404 `NOT_FOUND`、409 `STALE_CYCLE`、409 `SITUATION_REQUIRED`、409 `SITUATION_ABSENT`、409 `FEEDBACK_EXISTS` |

### 4.1 forecast 遮罩（Grok P0-2：§5.4.2 的 API 強制，非 UI 細節）

- **規則**：當週 `distinct(predictions.trigger)` 尚未全部有 `situation_checks` 列 → 回應中 `tendency`/`forecast`/`experiment` **缺省（skip 序列化，語意等同 null）**；收齊後才吐全文。前端一律視為 optional（`== null` 語義），不得假設 key 存在（Grok 二審 P2 #8）。
- 遮罩同時套用於 **GET** 與 **generate** 回應（兩者皆走同一序列化函數）。
- **週中揭示政策（寫死）**：週內 UI 只可揭露 trigger 標準問法與「本週有 N 則預測」；第 1 段收齊後才顯示 tendency/forecast/experiment。不可「週一給全文、週日才問第 1 段」——那等於整週促發。
- wire 調整：`Prediction.tendency` / `forecast` 改 `Option<String>`（`#[serde(default, skip_serializing_if="Option::is_none")]`）；`experiment` 維持 Option。`ft-schema` F5 wire 測試同步更新。

### 4.2 新 DTO（`ft-schema::api`）

```rust
pub struct ListPredictionsResponse {
  pub cycleId: String,
  pub checks: Vec<SituationCheck>,
  pub predictions: Vec<Prediction>,
  pub feedback: Vec<PredictionFeedback>,
}
pub struct GeneratePredictionsResponse { pub cycleId: String, pub generated: bool, pub predictions: Vec<Prediction> }
pub struct CheckSituationRequest { pub cycleId: Option<String>, pub trigger: TriggerWire, pub situation: SituationWire }
pub struct FeedbackRequest { pub response: ResponseWire }
```

### 4.3 輸入驗證

- `cycleId`（若有）：必須 `YYYY-MM-DD` **且為週一**；否則 400 `INVALID_CYCLE`（非週一日期會像「無預測」空陣列，易誤導）。
- `trigger` / `situation` / `response`：serde 強型別 enum 即鎖死小寫封閉值，壞值 400 `INVALID_JSON` 路徑。

## 5. 明示決策（Grok 裁決後定案）

| # | 決策 | 內容 |
|---|---|---|
| D1 | F4 閘門佔位 | v1 對有目錄領域（work/money）一律視為強度 ≥1；F8 登記「無強度閘門」。F4 落地後接閘 |
| D2 | 全負面週例外 | n==2 皆負 → 保留較佳 1 條（coverage 高者勝、再比 priority 小者）；F8 登記「三領域落地後廢除此例外」。**不改 per-domain** |
| D3 | cycle 級凍結 | `prediction_generations` PK 凍結整週；UNIQUE(user_id, cycle_id, domain) 防呆。**不用 per-domain 冪等**（防混 profile） |
| D4 | 寫入僅限當週 | checks/feedback 對非當週 → 409 `STALE_CYCLE`（統一）；cycleId 必須週一 |
| D5 | 回報一次性 | checks 在該 trigger 已有 feedback 後鎖定（409 `SITUATION_LOCKED`）；feedback 重送 → 409 `FEEDBACK_EXISTS` |
| D6 | forecast 遮罩 | API 強制：第 1 段未收齊 → tendency/forecast/experiment 為 null；UI 動線依 §4.1 |
| D7 | trigger 限當週有預測者 | `UNKNOWN_TRIGGER` 400；**absent 率是條件機率 P(absent\|被選中)，不得當 §5.4.4 C2 無條件基率**（文件寫死） |

## 6. F6 兩段式的 API 側強制

- 硬規則：① forecast 未收齊第 1 段不揭露（§4.1）；② feedback 僅在 `occurred` 後可交（409 `SITUATION_ABSENT`）；③ 一旦進第 2 段，第 1 段鎖定（409 `SITUATION_LOCKED`）；④ feedback 一次性（409 `FEEDBACK_EXISTS`）。
- 去重（§5.4.3）：`situation_checks` 鍵 `(user_id, cycle_id, trigger)`；同 trigger 多條預測只問一次。
- UI 動線（本切片不實作）：`checks[]` 未收齊 → 只顯示 trigger 問法；收齊 → 顯示全文並逐條收第 2 段。

## 7. F7 資料刪除（延伸現有 route）

`DELETE /api/personality/me`（`routes/personality.rs`）在刪 `personality_profiles` 後，連帶顯式刪除該使用者的：
`prediction_generations`、`predictions`、`situation_checks`、`prediction_feedback`（feedback 先於 predictions 刪——無 FK 依賴）。幽靈列不得進 F8。

## 8. 驗證策略

- `ft-schema`：`cycle.rs` / `predict.rs` 純函數測試（釘死日期、毫秒 ISO、round 邊界、全距手算、壞輸入 None、filter 例外）。`cargo test -p ft-schema`。
- `cargo check -p ft-api && cargo check -p ft-api --target wasm32-unknown-unknown`；`cargo fmt --all --check`。
- `ft-api` 不 mock、不造假（`.testing-rules`）：routes 語意 code review + 部署後手動 API 驗證（§10）；P0 狀態機（凍結/鎖定/遮罩）以 ft-schema 純函數測試覆蓋可測部分。
- **建表**：DDL＝`scripts/schema.sql`（含新增 generations 表 + UNIQUE index），施行需使用者同意後 `turso db shell fortunet < scripts/schema.sql`。

## 9. Grok 審查摘要與裁決（2026-09-04，read-only）

| 嚴重度 | 意見 | 裁決 |
|---|---|---|
| P0-1 | cycle 測試 #1 自相矛盾（週五→下週一）；解析須吃毫秒 | ✅ 修正 §1.1 釘死測試 + `.000Z` 邊界 |
| P0-2 | forecast 未遮罩＝§5.4.2 於 API 層空洞；需寫死週中揭示政策 | ✅ §4.1 遮罩（缺省=null 語義）+ 週中政策寫死 |
| 二審 P0 | freeze 寫在 predictions 之後，重試/並發會重開混 profile 窗 | ✅ §3.1 已是 freeze 先寫；實作對齊 |
| 二審 P1 | lock 檢查非原子；feedback 對非 occurred 放行；filter_map 靜默丟列 | ✅ 單句原子鎖/一次性；僅 occurred 放行；壞列 fail-closed 500 |
| P0-3 | latest-wins × 可改 situation → 計入規則無法強制；occurred 無第 2 段無編碼 | ✅ §3.3–3.4 鎖定/一次性；缺測不當 miss |
| P0-4 | 冪等粒度 (user,cycle,domain) 可混 profile；generated 誤報 | ✅ §2 generations 表 cycle 凍結 |
| P1-D2 | per-domain 不可取；A/B 擇一 | ✅ 採 A（n==2 留 1），F8 登記 |
| P1-D3 | 「無約束慣例」不成立（repo 已有 UNIQUE）；UNIQUE 可加 | ✅ UNIQUE index + cycle 凍結 |
| P1-D7 | absent 率是條件機率，不能當 C2 基率 | ✅ §5 D7 註記 |
| P1-D4 | upsert_check 漏 STALE_CYCLE；400/409 不一 | ✅ 統一 409；§4.3 週一格式 |
| P1-F7 | 刪側寫留幽靈預測 | ✅ §7 連帶刪除 |
| P1 | GET 不含 feedback；週中重測語意 | ✅ §3.2 回 feedback；§3.1 凍結不重算 |
| P2 | GET 限流撞 429；dim_ranges fail-closed；ocean parse 500；ORDER BY 固定 | ✅ §4 不限 GET+快取；§1.2 None；§3.1 500；§3.2 CASE |
| nit | is_control F8 打標；feedback FK/CASCADE | 🔜 F8 切片；本切片不改 predictions 列 |

## 10. 部署 / 驗證手順（需使用者同意）

```
1. cargo test -p ft-schema
2. cargo check -p ft-api --target wasm32-unknown-unknown
3. cargo fmt --all --check
4. turso db shell fortunet < scripts/schema.sql   # 建 3 表 + generations + UNIQUE index（冪等；生產變更需同意）
5. wrangler deploy（OAuth）→ verify-deployment.sh
6. 手動 API：register → quiz complete → POST generate（遮罩）→ GET（遮罩）→ PUT checks 全答 → GET（全文）→ POST feedback → 重送 409
```

## 參考

- `docs/superpowers/specs/2026-09-03-f5-rule-anchors-design.md`
- `docs/superpowers/specs/2026-09-03-f4-f5-if-then-design-note.md` §5.4
- `crates/schema/src/predict.rs`（select_for_domain / filter_negative_half）
- `crates/api/src/routes/personality.rs`（route 體例）
