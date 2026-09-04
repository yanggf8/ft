# 設計：F5 預測 UI 切片（我的命格 PredictionsCard — F6 兩段式動線）

日期：2026-09-04
狀態：**技術設計 — 已過 Grok 審查（2026-09-04，read-only）並完成 corroborate；本版已吸收審查**
前置：
- `2026-09-04-f5-api-predictions-design.md`（API 契約、forecast 遮罩、回報鎖定、cycle 凍結 — 已部署）
- `2026-09-03-f4-f5-if-then-design-note.md` §5.4（F6 兩段式、§5.4.2 順序約束）
- `2026-08-28-big5-f1-design.md`（人格測驗 / 我的命格）

> 本文只做 **web 消費端**。API 側保護（遮罩、occurred-only、一次性、凍結）已存在；**UI 不得繞過，且必須自行加「動線閘門」**——伺服器只保證「沒 occurred 不能交第 2 段、沒收齊不給看字」；「先收齊再問反應」（§5.4.2）是頁面順序，UI 自己關按鈕（Grok 二輪 P0-2）。

## 0. 範圍

**做**：`我的命格`（ProfilePage）`PredictionsCard`；F6 兩段式動線；尊重遮罩與閘門；`TriggerClass::question()/label()` 共用文字；web client 4 函數。
**不做**：歷史週、`is_control`/F8、experiment 潤寫、改 API 語意、繞過遮罩/鎖定。

## 1. 共用文字（ft-schema）

### 1.1 `TriggerClass::question()/label()`（anchors.rs）

```rust
impl TriggerClass {
    /// §5.3.3 標準問法（F6 第 1 段逐字；不得改寫）
    pub const fn question(&self) -> &'static str { /* 六句，見測試釘死 */ }
    /// §5.3.3「類別」短名（Stage 2 列標用）
    pub const fn label(&self) -> &'static str { /* 人際摩擦/時限壓力/生疏社交/被指出問題/計畫被打亂/有選擇要做 */ }
}
```

### 1.2 `TriggerWire ↔ TriggerClass`（api.rs / anchors.rs）

API 列上是 `TriggerWire`（`ft_schema::api`），`question()/label()` 住在 `TriggerClass`（anchors）。web 只 `use ft_schema::api::*`，需要雙向轉換：

```rust
impl From<crate::api::TriggerWire> for TriggerClass { /* t1..t6 → T1..T6 */ }
impl From<TriggerClass> for crate::api::TriggerWire { /* T1..T6 → t1..t6 */ }
```

測試：`question()`/`label()` 與 §5.3.3 表格**逐字**寫死斷言（非只含關鍵字）；雙向轉換 roundtrip。

## 2. web client（crates/web/src/api.rs）

```rust
pub async fn get_predictions(no_cache: bool) -> Result<ListPredictionsResponse, ApiErr>
    // GET /api/predictions，不帶 cycleId（U5）；進卡片/收齊/寫入後一律 no_cache: true
pub async fn generate_predictions() -> Result<GeneratePredictionsResponse, ApiErr>
    // 用既有的 post_empty（與 interpret/story 同）
pub async fn put_situation_check(b: &CheckSituationRequest) -> Result<SituationCheck, ApiErr>
    // PUT /api/predictions/checks；cycleId 一律 None（skip_serializing_if 省略）
pub async fn post_prediction_feedback(id: &str, b: &FeedbackRequest) -> Result<PredictionFeedback, ApiErr>
    // POST /api/predictions/{id}/feedback；body 僅 { "response": "hit"|"miss"|"other" }
```

沿用 `get_json`/`send_json`/`post_empty` 體例（auth 自動帶）。

## 3. PredictionsCard（crates/web/src/pages/profile.rs）

### 3.1 載入動線（Grok P1-1/P1-2/P1-3 修正）

卡片 mount → 狀態機（一律以 **GET 的 `ListPredictionsResponse`** 為 card state，Grok P1-6）：

```
init():
  GET /api/predictions (no_cache)
   ├─ Ok 且 predictions 非空            → Ready（依 checks/feedback 進入 Stage 1/2）
   ├─ Ok 且 predictions 空 且 !latch     → latch=true（記憶體 per-mount）
   │                                     → POST generate 一次（冪等）
   │                                        ├─ Ok → 再 GET（仍空 → honest empty）
   │                                        ├─ Err(PROFILE_INCOMPLETE) → NoProfile CTA（前往 /personality）
   │                                        └─ Err(其他) → Error（手動重試）
   └─ Err → Error（手動重試；RATE_LIMIT 禁止自動重試）
```

- **latch**：per-mount 記憶體旗標，POST 一次即停，**不寫 storage**（F7 刪檔後不得把「先完成測驗」顯示成「沒傾向」）。
- **空週判別**：POST generate 成功後 GET 仍空 → honest empty：「本週沒有明顯傾向可寫成可驗證的預測」。
- **換週（P1-3）**：任何 409 `STALE_CYCLE`（或頁面 focus 時比對 `cycleId` 變了）→ 清 latch 與 local checks/feedback → 重跑 init。
- 不做 profile 預檢（P1-2）：GET 空就 POST 一次；不平行打 `/personality/me` 解 status。

### 3.2 兩段式狀態機（重點：動線閘門，Grok P0-1/P0-2/P2-1）

**單一謂詞（與伺服器 `redact_view` 同一條件）**：`stage1_complete = distinct(predictions.trigger) ⊆ checks.trigger`

**Stage 1（`!stage1_complete`）**：
- 未答 trigger：顯示 `TriggerClass::question()` + 「**沒有／有**」（不暴露 absent/occurred wire，nit）→ `PUT checks` → 成功後：若 `stage1_complete` 變成 true → **先 GET 全文（載入中）**；否則 patch local checks（並保留「改答」）。
- 已答 trigger（尚無 feedback，即 `!stage1_complete`）：顯示目前答案 + 「改答」→ 重 PUT（API 在無 feedback 前允許 absent↔occurred）。
- **範本不得綁** tendency/forecast/experiment（即使 payload 誤帶 `Some` 也不渲染）。

**refetch 窗**：`stage1_complete` 變 true → `GET (no_cache: true)` → 全文就位前 **禁止 Stage 2 按鈕**（載入中）。

**Stage 2（`stage1_complete` ∧ 已 refetch ∧ `forecast.is_some()`）**：
- 每條 prediction 依領域序（工作→金錢，API 已排）＋ `label()` 短名（問句作副標，分辨同 trigger 兩領域）。
- 該列 trigger `check.situation == occurred` 且無 feedback → 顯示 forecast 本文（tendency 可作摘要）＋ **§5.4.1 三句**（Grok P1-4，不得「命中/不準」打成績）：
  - 「接近預測的描述」→ hit
  - 「接近相反的那一邊」→ miss
  - 「兩者都不太像」→ other
- `absent` 底下列 → 標「情境未發生（不計入）」，不給選項。
- 已回饋列 → 「已回饋」。
- feedback 成功 → patch local feedback（append）；失敗 `FEEDBACK_EXISTS`/`SITUATION_LOCKED` → 重 GET。

**部分 checks 時（A occurred、B 未答）**：只繼續問 B；**A 不得出現 Stage 2**（API 單條會放行——那是給誠實 UI 用的，不是給半套畫面用的，Grok P0-2）。

### 3.3 錯誤分支（Grok P1-5 補全）

| code | 處理 |
|---|---|
| `PROFILE_INCOMPLETE` | CTA「先完成人格測驗」+ 前往 /personality |
| `RATE_LIMIT` | 「動作太頻繁」+ 手動重試（不自動重試） |
| `STALE_CYCLE` | 清狀態 → 重跑 init |
| `SITUATION_LOCKED` / `FEEDBACK_EXISTS` | 重 GET（視為已作答） |
| `SITUATION_REQUIRED` / `SITUATION_ABSENT` / `UNKNOWN_TRIGGER` / `NOT_FOUND` | 重 GET（他分頁已改/已刪） |
| 其他 / network | 「載入失敗」+ 重試 |

### 3.4 佈局與細節

- 沿用 `.card` / `.btn-link` 等既有樣式；標題「本週有 N 則可驗證預測」（N = `predictions.len()`）。
- 顯示當週 `cycleId` 一行（換週 409 不莫名）。
- 連點防護：PUT/POST 進行中停用該列按鈕。
- 中文領域名：work→工作、money→金錢。
- 不顯示 `isControl` / `rulesVersion` / `anchorIds` / `source`。
- `experiment`：與全文一起揭，但列為「回饋後再顯示」（v1 顯示亦可；非正式閘）。

## 4. 驗證策略

- `cargo test -p ft-schema`（question/label/From 轉換；**`check` 不跑測試**）。
- `cargo check -p ft-web --target wasm32-unknown-unknown`。
- `cd crates/web && ./scripts/build-web.sh`（release wasm + wasm-bindgen）；`wrangler pages deploy dist --project-name=fortunet`（OAuth）。
- 手動清單（明示動線閘門）：未收齊時不得見 Stage 2 按鈕；最後一題後先 GET 全文再出按鈕；空週不循環 429；換週重跑。

## 5. 決策（Grok 審查後定案）

| # | 決策 |
|---|---|
| U1 | 落點 ProfilePage 卡片，不新增 route |
| U2 | generate：GET 空 + per-mount latch → POST 一次；`RATE_LIMIT` 不自動重試 |
| U3 | **結構閘**：Stage 1 範本不綁 tendency/forecast/experiment；**資料閘**：Stage 2 只在 `Some` 時渲染，禁止 `unwrap_or_default()` |
| U4 | Stage 2 按鈕綁 `stage1_complete` ∧ 已 refetch ∧ `forecast.is_some()`（Grok P0-1/P0-2） |
| U5 | 不帶 `?cycleId=`；`CheckSituationRequest.cycleId` 恆 None |
| U6 | 第 2 段三句用 §5.4.1 原文，不寫「命中/不準」（Grok P1-4） |
| U7 | 已答 trigger 在 `!stage1_complete` 時可改答（P2-1） |

## 參考

- `2026-09-04-f5-api-predictions-design.md`（API 契約/遮罩/鎖定）
- `2026-09-03-f4-f5-if-then-design-note.md` §5.3.3 / §5.4
- `crates/web/src/pages/profile.rs`、`crates/web/src/api.rs`
