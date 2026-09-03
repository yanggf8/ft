# 設計：F5 規則錨點目錄與 predictions 形狀（縱深 12 格）

日期：2026-09-03
狀態：**技術設計 — 待使用者核可**
前置：
- `2026-08-26-engine-modernization-big5-design.md`（rev.4 Part I §F5：四段結構 `tendency / trigger / forecast / experiment`、F6 兩段式、F8 對照、負面效價三規則、`anchor_coverage` 不看命盤）
- `2026-08-28-big5-f1-design.md`（F1 已落地：`ft-big5` 計分、切點 `≥67 高 / [33,67) 中 / <33 低`）
- `2026-09-03-f4-f5-if-then-design-note.md`（備註：T1–T6 封閉列舉 N2、F6 兩段式 N10 已定案、`situation_checks` 鍵為 `(user_id, cycle_id, trigger)`）

> **本文不重審** F1 量表、常模、切點、亂答偵測、T1–T6 定義、F6 兩段式與計入規則。僅補齊 rev.4 明言「規則目錄未定案前 `anchor_coverage` 門檻不可實作」的那塊拼圖：**錨點目錄的形狀、命中與擇一規則、以及 `predictions` / `situation_checks` 的表與 wire 形狀**。命盤象徵向量（F2）與落差疊圖（F3）不在本文範圍。

## 0. 範圍

**做**：
- 錨點目錄 `ANCHORS` 的型別與存放位置（`ft-schema` 靜態常數）
- v1 縱深：**`work` + `money` × T1–T6 = 12 格**，每格 ≥2 條（24–36 條），其餘三領域暫不產出（沿 rev.4「0 錨點 → 不產出」）
- 命中條件（單維、兩端）、格內優先序、跨格擇一與 `anchor_coverage` 判定
- `predictions` / `situation_checks` 的 DDL 與 wire 型別（形狀先定，建表延後，沿 F1 §K2 體例）
- 全中檔（零命中）的空狀態
- 明示偏離一項：`trigger` 文字來源

**不做**：
- 36 句文案本身（本文只定形狀與不變式；文字在 `ANCHORS` 常數中一次性填齊，由實作 PR 承載）
- F4 五領域強度 UI（0–3 級 > 待 F4 切片）
- F2/F3、付費計次、LLM prompt 細節
- 現在建表（DDL 為權威定義，施行待 F4/F5 開工）

## 1. 錨點資料結構

### 1.1 位置

`crates/schema/src/anchors.rs`，與 `items.rs`（`ITEMS`/`NORMS`）同體例：
- `ft-schema` 為 `crates/web` 可讀的唯一靜態真相（`crates/web` 禁依賴 `domain`，沿 F1 架構表）
- 對 `wasm32` 體積影響可忽略（純常數與 `&'static str`，無新依賴）

### 1.2 型別

```rust
/// 錨點觸發檔位 — 只有兩端，中檔不觸發（與 F1 側寫切點同一組）
pub enum Level { High, Low }

/// 效價 — 供 rev.4 §F5「負面不得過半」與 money/health 限制的編譯期檢查
pub enum Valence { Negative, Neutral, Positive }

/// 來源等級 — 沿 rev.3 §4.2 已立慣例
pub enum Source { Literature, DesignerJudgment }

pub struct Anchor {
  /// 目錄內唯一 id，供 predictions 列稽核與 F8 登記對帳
  /// 命名："{domain}-{trigger}-{dim}{level}-{seq}" 例 "work-t1-agr-lo-1"
  pub id: &'static str,

  // ── 格位 ──
  pub domain: Domain,            // Work | Love | Family | Money | Health
  pub trigger: TriggerClass,     // T1..T6（見 §1.3）

  // ── 觸發條件（單維、兩端）──
  pub dimension: usize,          // 0..4，索引同 DIMENSION_NAMES
  pub level: Level,

  /// 格內優先序，小者勝；同格不得重複（測試釘死 §4）
  pub priority: u8,

  // ── 逐字輸出文字（rev.4：LLM 禁止改寫此三欄）──
  pub tendency: &'static str,
  pub forecast: &'static str,
  pub experiment: Option<&'static str>,  // 選配，唯一可被 LLM 潤寫的欄位

  pub valence: Valence,
  pub source: Source,
}

pub const ANCHORS: &[Anchor] = &[
  // v1：work + money × T1..T6，每格 ≥2 條
];
```

### 1.3 枚舉定義

`Domain` 列舉含全部五個值（`Work | Love | Family | Money | Health`），但 v1 目錄只填 `Work`/`Money` 兩格。其餘領域 `hits` 恆空，自然走「0 錨點 → 不產出」。

`TriggerClass` 即 N2 定案六類，標準問法即 §5.3.3（亦是 F6 第 1 段逐字問法）：

| # | 類別 | 標準問法（F6 第1段） | 主要維 | 效價 |
|---|---|---|---|---|
| T1 | 人際摩擦 | 這週有沒有跟人意見不合或起摩擦？ | 友善性 | 負 |
| T2 | 時限壓力 | 這週有沒有事情趕不完、被期限追著？ | 嚴謹性 | 負 |
| T3 | 生疏社交 | 這週有沒有需要跟不熟的人、或一群人相處？ | 外向性 | 中 |
| T4 | 被指出問題 | 這週有沒有被糾正、挑毛病或收到負面回饋？ | 情緒穩定 | 負 |
| T5 | 計畫被打亂 | 這週有沒有原本安排好的事突然變動？ | 嚴謹性 | 中 |
| T6 | 有選擇要做 | 這週有沒有需要在幾個選項之間做決定？ | 智性／想像 | 中/正 |

> **為何 v1 只做 work+money**：rev.4 §33/§68 已警告本量表最弱的兩維是友善性與開放性（α .67、AVE .41），而愛情/家庭最依賴那兩維；work/money 主要吃嚴謹性與情緒穩定，是量表測得較準且錨點文獻較足的兩維。先讓 `high/low` 能真正被驗證。

### 1.4 為何 Anchor 沒有 `trigger_text`

rev.4 寫「`trigger` 逐字來自錨點」，但 9/3 備註已把 `trigger` 改為封閉列舉。使用者看到的「什麼情況會放大這個傾向」直接取 `TriggerClass` 的標準問法/短句，而非每條錨點各寫一份。理由：

1. 少寫 24–36 份重複字串；
2. 保證 **F6 第1段提問與預測內 trigger 文字逐字相同**（備註 §5.4.2 的 priming 約束要求兩者一致；若每條錨點各寫各的則無機制保證）。

此為對 rev.4 的明示偏離，見 §5。

## 2. 命中、擇一與 `anchor_coverage`

### 2.1 命中條件（與 F1 同一切點）

- 取 **取整後顯示分** 比較（F1 §5 靜態側寫已用此切點；另立門檻會造成「側寫說偏高、預測說中等」的矛盾）。
- `High` 命中：該維顯示分 **≥ 67**；`Low` 命中：**< 33**；中檔 **[33,67)** 不觸發任何錨點。
- 領域強度（F4 的 0–3）**不參與命中**，僅為閘門（§3.1）。

### 2.2 擇一（一個領域一則預測）

沿 rev.4「每個強度 ≥1 的領域產出 1 條、最多 5 條」。對領域 D（強度 ≥1）：

```
hits(D) = { a in ANCHORS | a.domain == D && 命中(a.dimension, a.level) }
若 hits(D) 空 → 不產出（空狀態 §2.4）
否則按 trigger 分組：hits_T = hits(D) ∩ { a.trigger == T }
     選勝出 trigger T*：hits_T 數量大者勝；同數量比組內最小 priority 小者勝
     產出文字：取 T* 組內 priority 最小的那條錨點的 tendency / forecast / experiment
     參與計數的 anchor_ids = hits_T* 的全體 id（寫進 predictions 列，供稽核）
```

`trigger` 欄位寫 `T*`，`tendency/forecast/experiment` 取勝出錨點，`anchor_coverage` 按 §2.3 判定，其餘 `hits_T*` 僅計數不拼文字（避免多條錨點文字拼出不通順且不可歸因的句子；rev.4 禁止 LLM 修補此三欄）。

### 2.3 `anchor_coverage` 判定

沿 rev.4 表，判定對象為勝出組 `T*` 的 `hits_T*` 數量（而非全領域 hits）：

| 條件 | `anchor_coverage` |
|---|---|
| `hits_T*` 空 | 不產出（非 low） |
| 同維高/低同時命中（同 dimension 的 High 與 Low 同時在 hits_T*） | `low`，且禁止輸出對立因果 |
| `hits_T*` 數 == 1 | `low` |
| `hits_T*` 數 ≥ 2 且無同維矛盾 | `high` |

> v1 縱深設計的不變式（§4）保證「≥2」可達（每格 ≥2 條）；「≥2」的分母為目錄總數，沿 rev.4。

### 2.4 全中檔（零命中）的空狀態

五維皆落中檔時，對應領域的 `hits(D)` 恆空，結果為零則預測。UI 顯示 honest empty：

> 「本週沒有明顯傾向可寫成可驗證的預測」

不硬湊。硬湊的「你可能這樣也可能那樣」正是 rev.4 §F8 要用對照組濾掉的 Barnum。

## 3. 表與 wire 形狀

> **形狀先定、建表延後**（沿 F1 §K2 與 9/3 備註 §9：`schema.sql` 為權威，施行待 F4/F5 開工；本文 DDL 即權威定義）。

### 3.1 `predictions`

```sql
CREATE TABLE IF NOT EXISTS predictions (
  id                TEXT PRIMARY KEY,           -- uuid v4
  user_id           TEXT NOT NULL,
  profile_id        TEXT NOT NULL,              -- 綁當時 personality_profiles.id 的快照
  cycle_id          TEXT NOT NULL,              -- 週起始日 ISO date (YYYY-MM-DD)，對齊 7 天視野
  domain            TEXT NOT NULL,              -- work | love | family | money | health
  trigger           TEXT NOT NULL,              -- t1..t6（封閉枚舉，小寫）
  tendency          TEXT NOT NULL,              -- 逐字來自勝出錨點
  forecast          TEXT NOT NULL,              -- 逐字來自勝出錨點，7 天後可判定真假
  experiment        TEXT,                       -- 選配，唯一可被 LLM 潤寫欄
  anchor_ids        TEXT NOT NULL,              -- JSON array of anchor id
  anchor_coverage   TEXT NOT NULL,              -- high | low
  source            TEXT NOT NULL DEFAULT 'rule_anchor',  -- rule_anchor | personal_record（後者先占位 §6）
  rules_version     TEXT NOT NULL,              -- 例 rules-1，語意遞增
  created_at        TEXT NOT NULL               -- ISO datetime，app 寫入
);
CREATE INDEX IF NOT EXISTS idx_predictions_user_cycle
  ON predictions(user_id, cycle_id);
CREATE INDEX IF NOT EXISTS idx_predictions_profile
  ON predictions(profile_id);
```

- `profile_id` 綁快照，不跟最新側寫漂移。
- `trigger` 文字不另存，前端由 `TriggerClass` 標準問法顯示，確保 F6 第1段逐字相同（§1.4）。
- `source` v1 恆為 `rule_anchor`，先占位給備註 §6 的 `personal_record`（每人每格累積足量 `occurred` 後的 `F8` 對照達標才切換）。
- `rules_version` 語意遞增（目錄實質變更才動），沿 rev.3 舊 Part II 慣例 `RULES_VERSION`。

#### 領域閘門

`domain` 是否產出一列，由 F4 強度決定：強度 ≥1 才走 §2.2；強度 0 不建列。此閘門與錨點命中無關。

### 3.2 `situation_checks`

鍵為 `(user_id, cycle_id, trigger)`，**不是** `(prediction_id)`（備註 §5.4.3 已定；同一週同 trigger 的多則預測只問一次，第1段去重）：

```sql
CREATE TABLE IF NOT EXISTS situation_checks (
  user_id     TEXT NOT NULL,
  cycle_id    TEXT NOT NULL,               -- 同 predictions.cycle_id
  trigger     TEXT NOT NULL,               -- t1..t6
  situation   TEXT NOT NULL,               -- absent | occurred
  created_at  TEXT NOT NULL,
  PRIMARY KEY (user_id, cycle_id, trigger)
);
```

- `(user_id, cycle_id, trigger)` 唯一，天然支撐去重與 `absent` 率的基率校準（備註 §5.4.4，準則 C2）。
- 值域校準的可行動作是**停用**某 trigger（不再產出、歷史保留），而非重定義（備註 §5.4.4 注意段）。

### 3.3 wire 型別（`crates/schema/src/api.rs`）

```rust
pub enum Domain { Work, Love, Family, Money, Health }
pub enum TriggerClass { T1, T2, T3, T4, T5, T6 }
pub enum AnchorCoverage { High, Low }
pub enum PredictionSource { RuleAnchor } // v1 單值；PersonalRecord 待 F8 後

pub struct Prediction {
  pub id: String,
  pub profileId: String,
  pub cycleId: String,           // YYYY-MM-DD
  pub domain: String,            // wire 小寫
  pub trigger: String,           // t1..t6
  pub tendency: String,
  pub forecast: String,
  pub experiment: Option<String>,
  pub anchorIds: Vec<String>,
  pub anchorCoverage: String,    // high | low
  pub source: String,            // rule_anchor
  pub rulesVersion: String,
  pub createdAt: String,
}
pub struct SituationCheck {
  pub cycleId: String,
  pub trigger: String,
  pub situation: String,         // absent | occurred
  pub createdAt: String,
}
```

亦可改為 `#[derive(Serialize, Deserialize)]` 的強型別枚舉（`serde(rename)` 小寫），實作時擇一；本文以字串標出封閉值域。

### 3.4 `ft-big5` 是否需要動

不需要。錨點邏輯住 `ft-schema` 純常數，`ft-big5` 仍只含計分/亂答/常模（沿 F1 職責表）。不新增依賴，`wasm32` 預算不變。

## 4. 驗證策略

沿 `items.rs` 已有前例（`items_are_dense_1_based`、`norms_match_paper_values`）與 `.testing-rules`：

- `crates/schema` 內 `#[cfg(test)]` 釘編譯期/測試期不變式（壞錨點過不了 `cargo test`/`cargo build`）：
  - 每格（`Work`/`Money` × T1–T6）錨點數 ≥ 2
  - 同格 `priority` 唯一且 1..N 連續
  - 全域 `id` 唯一
  - `dimension` ∈ 0..4、`level` ∈ {High,Low}、`valence` 分布不過半（`Negative` ≤ 總數/2，檢 rev.4 負面效價規則）
  - `money` 領域無 `Negative` forecast 涉及損失/負債、`health` 領域無 `Negative` forecast（rev.4 三規則；但 v1 不含 health 領域，此條恒過，仍作不變式以防後續擴領域時遺漏）
  - `domain` 僅 `work`/`money` 出現在 `ANCHORS`（v1 縱深約束；擴領域時放寬）
- `crates/api` / `crates/schema` 皆無 mock、無 fake data，符合 `.testing-rules`。
- 命中與擇一邏輯的測試置於 `ft-schema` 的純函數測試（輸入為 `OceanScores` 取整顯示分與 `Domain`，輸出為選中 `Anchor` 與 `anchor_coverage`），不依賴 D1/Turso。

## 5. 明示偏離

| 偏離 | rev.4 原文 | 本文 | 理由 |
|---|---|---|---|
| P1 | §F5「`trigger` 逐字來自錨點」 | `trigger` 文字來自 `TriggerClass` 標準問法（§1.4） | 保證 F6 第1段提問與預測內 trigger 逐字相同（備註 §5.4.2 priming 約束）；若每條錨點各寫各的則無機制保證。錨點的 `tendency/forecast/experiment` 仍逐字來自錨點，偏離僅限 `trigger` 一欄 |

## 6. 待後續裁決（不屬本文）

| 項目 | 說明 | 時機 |
|---|---|---|
| N6 部分池化 + `source` | 備註 §6：個人估計向規則先驗收縮，`source` 切 `personal_record` | F8 對照達標後 |
| N7 顯示門檻（建議 ≥8 筆 `occurred`） | designer 判斷，上線後校準 | F6 上線後 |
| N8 簽名層級置換對照 | 備註 §7：打亂 `trigger` 配對的對照 | F8 實作時 |
| 領域擴張（love/family/health） | 每增一領域 +6 格、≥12 條錨點，含文獻較弱維的風險（rev.4 §33/§68） | 本縱深驗證 `high` 可達且 F8 樣本充足後 |

## 參考

- `2026-08-26-engine-modernization-big5-design.md` rev.4 Part I §F5/F6/F8
- `2026-09-03-f4-f5-if-then-design-note.md` §5.3–§5.4（T1–T6、F6 兩段式、鍵形狀）
- `crates/schema/src/items.rs`（`ITEMS`/`NORMS` 存放體例與測試體例）
- `.testing-rules`（整合測試、無 mock、線上真環境）
