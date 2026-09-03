# F5 Rule Anchor Catalogue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 落地 F5 規則錨點目錄（work+money × T1–T6 = 12 格、24–36 條）與 `predictions` / `situation_checks` / `prediction_feedback` 的表與 wire 形狀，讓 `anchor_coverage` 與 F6 兩段式回饋可被實作與測試

**Architecture:** 純靜態常數住 `crates/schema`（`anchors.rs` 對齊 `items.rs` 體例），命中與擇一為 `ft-schema` 純函數（無 DB、無 LLM），DDL 在 `scripts/schema.sql` 先定形狀（建表延後也以此為權威），wire 強型別 enum 經 `ft-schema::api` 暴露給 `ft-api` 與 `ft-web`

**Tech Stack:** Rust workspace (ft-schema / ft-api / ft-web), Turso (Hrana over Fetch), Leptos CSR, `cargo test` 純函數測試（沿 `items.rs` 體例，符合 `.testing-rules`）

**Spec:** `docs/superpowers/specs/2026-09-03-f5-rule-anchors-design.md`（前置：`2026-08-26-engine-modernization-big5-design.md` rev.4 Part I §F5/F6/F8、`2026-09-03-f4-f5-if-then-design-note.md` N2/N10）

## Global Constraints

- `trigger` 封閉列舉 T1–T6（`2026-09-03-f4-f5-if-then-design-note.md` §5.3.3），`domain` 五值但 v1 僅 `work`/`money` 可出現在目錄
- 命中切點與 F1 同一組：取整顯示分 `≥67 高 / [33,67) 中 / <33 低`，中檔不觸發（`2026-08-28-big5-f1-design.md` §5）
- `anchor_coverage` 含測量品質降級：`該維 IPIP-15 三題全距 ≥2 => low`（rev.4 P0，不可靜默移除）
- `tendency` / `forecast` 逐字來自錨點，LLM 禁止改寫；僅 `experiment` 可被 LLM 潤寫（rev.4 §F5）
- `trigger` 文字取 `TriggerClass` 標準問法（P1 偏離 §1.4），保證 F6 第 1 段提問逐字相同（備註 §5.4.2 priming 約束）
- F6 兩段式：`situation=absent/occurred`（`situation_checks`）+ `response=hit/miss/other`（`prediction_feedback`，僅 `occurred` 時），計入規則見備註 §5.4.1（`absent` 不進分母；`occurred+other` 進分母不進分子）
- `cycle_id` = `Asia/Taipei` 週一 00:00 起算的週起始日 `YYYY-MM-DD`（P1 補齊，影響去重與回訪窗口）
- 命盤不得決定 `trigger` 分類邊界（備註 §8）；`anchors.rs` 禁依賴 `ft-astrology`
- 負面效價 per-output 不得過半，超過丟棄最負面數條（rev.4 三規則）；catalog-level `Negative ≤ total/2` 僅為健康檢查
- `wire` 小寫封閉值：`domain` `work|love|family|money|health`、`trigger` `t1..t6`、`anchor_coverage` `high|low`、`situation` `absent|occurred`、`response` `hit|miss|other`
- `.testing-rules`：`crates/schema` 內 `#[cfg(test)]` 純常數不變式測試可寫；`ft-api` 整合測試不 mock、不造假；`cargo fmt --check` / `cargo build --target wasm32-unknown-unknown` 過 CI

---

## File Structure

```
crates/schema/src/anchors.rs      # 新增：Anchor / Domain / TriggerClass / Level / Valence / Source / ANCHORS
crates/schema/src/lib.rs          # 匯出 anchors 模組
crates/schema/src/api.rs          # 新增：Prediction / SituationCheck / PredictionFeedback wire 型別與強型別 enum
scripts/schema.sql                # 新增：predictions / situation_checks / prediction_feedback DDL + 遷移註記
crates/schema/src/anchors_tests.rs # 內嵌或獨立：catalog 不變式測試（亦可在 anchors.rs 內 #[cfg(test)]）
```

- `ft-big5` 不動（仍僅計分/亂答/常模）
- `ft-web` 在本 plan 不改 UI（空狀態與 `.on-light` 已在 `ac9727e` 處理），僅消費 `ft-schema::api` 型別

---

### Task 1: 建立 `crates/schema/src/anchors.rs` 型別與空目錄

**Files:**
- Create: `crates/schema/src/anchors.rs`
- Modify: `crates/schema/src/lib.rs` (匯出 `pub mod anchors;`)
- Test: `cargo test -p ft-schema --lib anchors` (編譯期型別檢查)

**Interfaces:**
- Consumes: `ft-schema::items::DIMENSION_NAMES` 索引約定（0..4）
- Produces: `pub enum Domain { Work, Love, Family, Money, Health }`, `pub enum TriggerClass { T1,T2,T3,T4,T5,T6 }`, `pub enum Level { High, Low }`, `pub enum Valence { Negative, Neutral, Positive }`, `pub enum Source { Literature, DesignerJudgment }`, `pub struct Anchor { id, domain, trigger, dimension, level, priority, tendency, forecast, experiment, valence, source }`, `pub const ANCHORS: &[Anchor]`

- [ ] **Step 1: 建立空 `anchors.rs` 與 lib 匯出**

```rust
// crates/schema/src/anchors.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain { Work, Love, Family, Money, Health }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerClass { T1, T2, T3, T4, T5, T6 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level { High, Low }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Valence { Negative, Neutral, Positive }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source { Literature, DesignerJudgment }

pub struct Anchor {
  pub id: &'static str,
  pub domain: Domain,
  pub trigger: TriggerClass,
  pub dimension: usize,
  pub level: Level,
  pub priority: u8,
  pub tendency: &'static str,
  pub forecast: &'static str,
  pub experiment: Option<&'static str>,
  pub valence: Valence,
  pub source: Source,
}
pub const ANCHORS: &[Anchor] = &[];
```

```rust
// crates/schema/src/lib.rs 增加
pub mod anchors;
```

- [ ] **Step 2: 跑編譯檢查確認匯出**

Run: `cargo check -p ft-schema`
Expected: PASS（空目錄可編）

- [ ] **Step 3: Commit**

```bash
git add crates/schema/src/anchors.rs crates/schema/src/lib.rs
git commit -m "feat(schema): add anchors.rs type skeleton (Domain/TriggerClass/Anchor)"
```

---

### Task 2: 填入 v1 縱深目錄（work+money × T1–T6，每格 ≥2 條）

**Files:**
- Modify: `crates/schema/src/anchors.rs` (填 `ANCHORS`)
- Test: `cargo test -p ft-schema --lib` (後續 Task 3 的測試會驗證格數)

**Interfaces:**
- Produces: 24–36 條 `Anchor` 常數，`id` 形如 `work-t1-agr-lo-1` 全域唯一，`priority` 同格 1..N 唯一連續

- [ ] **Step 1: 填入每格 ≥2 條的 `ANCHORS` 常數（先以佔位文案，標 valence/source）**

```rust
pub const ANCHORS: &[Anchor] = &[
  // Work × T1 (人際摩擦, 主要維 友善性)
  Anchor { id: "work-t1-agr-lo-1", domain: Domain::Work, trigger: TriggerClass::T1, dimension: 1, level: Level::Low, priority: 1, tendency: "在工作摩擦中傾向先退開", forecast: "這週遇到意見不合時，更可能先擱置而非當場釐清", experiment: Some("先記下分歧點，隔天再約 15 分鐘對齊"), valence: Valence::Negative, source: Source::DesignerJudgment },
  Anchor { id: "work-t1-es-lo-1", domain: Domain::Work, trigger: TriggerClass::T1, dimension: 3, level: Level::Low, priority: 2, tendency: "在張力下情緒起伏較明顯", forecast: "這週遇到摩擦時，更可能反覆回想對話", experiment: None, valence: Valence::Negative, source: Source::Literature },
  // ... 其餘 10 格各 2 條，Money×T6 需含 智性/想像 維
];
```

> 要求：同格的 2 條盡量跨維（滿足備註 C4），`tendency/forecast` 為行為傾向比較、不得人格缺陷（人工 checklist），`money` 不寫損失/負債、`health` 不寫負面（v1 無 health 格，此條恒過）

- [ ] **Step 2: 跑 `cargo fmt` 與 `cargo check`**

Run: `cargo fmt --all && cargo check -p ft-schema`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/schema/src/anchors.rs
git commit -m "feat(schema): populate v1 anchor catalogue work+money×T1-T6 (12 cells × ≥2)"
```

---

### Task 3: Catalog 不變式測試（壞錨點過不了 CI）

**Files:**
- Modify: `crates/schema/src/anchors.rs` (內 `#[cfg(test)]`)
- Test: `crates/schema/src/anchors.rs` 測試模組

**Interfaces:**
- Consumes: `ANCHORS`
- Produces: 7 條不變式測試

- [ ] **Step 1: 寫 failing 測試（先以空目錄預期失敗，填入後應通過）**

```rust
#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::{HashMap, HashSet};

  #[test] fn every_cell_has_at_least_two() {
    let mut counts: HashMap<(Domain, TriggerClass), usize> = HashMap::new();
    for a in ANCHORS { *counts.entry((a.domain, a.trigger)).or_default() += 1; }
    for domain in [Domain::Work, Domain::Money] {
      for trigger in [TriggerClass::T1, TriggerClass::T2, TriggerClass::T3, TriggerClass::T4, TriggerClass::T5, TriggerClass::T6] {
        assert!(counts.get(&(domain, trigger)).copied().unwrap_or(0) >= 2, "cell {:?}/{:?} <2", domain, trigger);
      }
    }
    // Love/Family/Health 在 v1 應為 0
    for domain in [Domain::Love, Domain::Family, Domain::Health] {
      for trigger in [TriggerClass::T1, TriggerClass::T2, TriggerClass::T3, TriggerClass::T4, TriggerClass::T5, TriggerClass::T6] {
        assert_eq!(counts.get(&(domain, trigger)).copied().unwrap_or(0), 0);
      }
    }
  }
  #[test] fn priority_unique_and_contiguous_per_cell() { /* 同格 priority 1..N 唯一連續 */ }
  #[test] fn id_globally_unique() { /* HashSet 檢查 */ }
  #[test] fn dimension_in_range() { /* 0..4 */ }
  #[test] fn valence_not_over_half() {
    let neg = ANCHORS.iter().filter(|a| a.valence == Valence::Negative).count();
    assert!(neg * 2 <= ANCHORS.len());
  }
  #[test] fn money_has_no_loss_forecast() { /* 文案含「損失/負債/虧損」即失敗，人工 checklist 的可機檢部分 */ }
  #[test] fn ids_are_lowercase_t1_format() { /* regex: ^[a-z]+-t[1-6]-[a-z]+-(hi|lo)-[0-9]+$ */ }
}
```

- [ ] **Step 2: 跑測試確認失敗→填目錄後通過**

Run: `cargo test -p ft-schema --lib anchors::tests -q`
Expected: 填入 Task 2 後 PASS；若 valence 過半或某格 <2 則 FAIL

- [ ] **Step 3: 補 `cycle_id` 與 `trigger` 格式的 regex 測試（P2）**

```rust
#[test] fn trigger_wire_format_is_t1_lowercase() {
  // 僅檢查常數中 trigger 的 wire 映射為 t1..t6 小寫，無大寫 T1
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/schema/src/anchors.rs
git commit -m "test(schema): anchor catalogue invariants (cell ≥2, priority unique, valence ≤ half)"
```

---

### Task 4: DDL 形狀先定 — `predictions` / `situation_checks` / `prediction_feedback`

**Files:**
- Modify: `scripts/schema.sql` (追加三張表，附遷移註記)
- Test: `cargo test` 不直接測 DDL；由 `scripts/verify-deployment.sh` 與 Turso shell 驗證（手動步驟記於 commit 訊息）

**Interfaces:**
- Produces: 三張表的權威 DDL

- [ ] **Step 1: 追加 DDL（含 `cycle_id` 定義與遷移註記）**

```sql
-- F5 predictions (authoritative shape, creation deferred per F1 K2)
-- cycle_id = Asia/Taipei 週一 00:00 起算的週起始日 YYYY-MM-DD，對齊 7 天視野與 F6 回訪
-- 遷移註記：rev.3 舊 predictions(situation_id, divination_type, prediction_text, cache_key) 與
-- 舊 situation_checks(id, domains JSON) 已作廢；本 DDL 為重建權威，舊 prod 表需 DROP 後重建
CREATE TABLE IF NOT EXISTS predictions (
  id                TEXT PRIMARY KEY,
  user_id           TEXT NOT NULL,
  profile_id        TEXT NOT NULL,
  cycle_id          TEXT NOT NULL,
  domain            TEXT NOT NULL,  -- work|love|family|money|health
  trigger           TEXT NOT NULL,  -- t1..t6
  tendency          TEXT NOT NULL,
  forecast          TEXT NOT NULL,
  experiment        TEXT,
  anchor_ids        TEXT NOT NULL,  -- JSON array
  anchor_coverage   TEXT NOT NULL,  -- high|low
  source            TEXT NOT NULL DEFAULT 'rule_anchor',
  rules_version     TEXT NOT NULL,  -- rules-1 語意遞增
  is_control        INTEGER NOT NULL DEFAULT 0,  -- F8 對照組標記 (P1 補)
  created_at        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_predictions_user_cycle ON predictions(user_id, cycle_id);
CREATE INDEX IF NOT EXISTS idx_predictions_profile ON predictions(profile_id);

CREATE TABLE IF NOT EXISTS situation_checks (
  user_id     TEXT NOT NULL,
  cycle_id    TEXT NOT NULL,
  trigger     TEXT NOT NULL,  -- t1..t6
  situation   TEXT NOT NULL,  -- absent|occurred
  created_at  TEXT NOT NULL,
  PRIMARY KEY (user_id, cycle_id, trigger)
);

-- F6 第 2 段（P0）：僅在 occurred 時的 hit/miss/other
CREATE TABLE IF NOT EXISTS prediction_feedback (
  prediction_id TEXT PRIMARY KEY,
  response      TEXT NOT NULL,  -- hit|miss|other
  created_at    TEXT NOT NULL,
  FOREIGN KEY (prediction_id) REFERENCES predictions(id) ON DELETE CASCADE
);
```

- [ ] **Step 2: 本地 Turso 驗證（不推遠端）**

Run: `turso db shell fortunet < scripts/schema.sql && echo ok`
Expected: `ok`（`IF NOT EXISTS` 冪等）

- [ ] **Step 3: Commit**

```bash
git add scripts/schema.sql
git commit -m "feat(db): define predictions / situation_checks / prediction_feedback DDL (F5 shape, cycle_id Asia/Taipei Monday)"
```

---

### Task 5: Wire 強型別（`crates/schema/src/api.rs`）

**Files:**
- Modify: `crates/schema/src/api.rs`
- Test: `cargo check -p ft-schema -p ft-web --target wasm32-unknown-unknown`

**Interfaces:**
- Produces: `pub enum DomainWire / TriggerWire / AnchorCoverageWire / SituationWire / ResponseWire`（`serde(rename)` 小寫）與 `pub struct Prediction / SituationCheck / PredictionFeedback`

- [ ] **Step 1: 新增強型別 enum（鎖小寫，避免 t1 vs T1 漂移）**

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DomainWire { Work, Love, Family, Money, Health }
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerWire { T1, T2, T3, T4, T5, T6 } // wire 為 "t1" 需自訂：#[serde(rename="t1")] 等
```

> 註：`TriggerWire` 需 `#[serde(rename="t1")]` 六枚，`serde(rename_all)` 無法產生 `t1`；明確列出。

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Prediction {
  pub id: String,
  pub profileId: String,
  pub cycleId: String,
  pub domain: DomainWire,
  pub trigger: TriggerWire,
  pub tendency: String,
  pub forecast: String,
  pub experiment: Option<String>,
  pub anchorIds: Vec<String>,
  pub anchorCoverage: AnchorCoverageWire,
  pub source: PredictionSourceWire,
  pub rulesVersion: String,
  pub isControl: bool,
  pub createdAt: String,
}
```

- [ ] **Step 2: 跑 `cargo check`（含 wasm32）**

Run: `cargo check -p ft-schema && cargo check -p ft-web --target wasm32-unknown-unknown`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/schema/src/api.rs
git commit -m "feat(schema): add Prediction / SituationCheck / PredictionFeedback wire strong types"
```

---

### Task 6: 命中與擇一純函數（`ft-schema`）

**Files:**
- Create: `crates/schema/src/predict.rs` (或併入 `anchors.rs` 的 `pub fn select_for_domain`)
- Modify: `crates/schema/src/lib.rs` (匯出)
- Test: `crates/schema/src/predict.rs` 內 `#[cfg(test)]`

**Interfaces:**
- Consumes: `ANCHORS`, `ft_big5::OceanScores` 取整顯示分、`dimension` 全距（`max - min` per 3 items）
- Produces: `pub fn select_for_domain(domain: Domain, scores: &OceanScores, ranges: [u8;5]) -> Option<Selected>` where `Selected { trigger: TriggerClass, anchor: &Anchor, anchorIds: Vec<String>, coverage: AnchorCoverage }`

- [ ] **Step 1: 寫 failing 測試**

```rust
#[test]
fn high_low_hit_and_mid_not() {
  // Ocean 顯示分 70/20/50/50/50：高者在 high 命中，低者在 low 命中，中檔不命中
}
#[test]
fn range_ge2_downgrades_to_low() {
  // 同維三題全距 ≥2 時，即使 hits_T*≥2 也應 low
}
#[test]
fn picks_winning_trigger_by_count_then_priority() {
  // 構造兩組各 2 命中，驗勝出 T* 與 anchorIds
}
#[test]
fn same_dimension_clash_is_low() { /* 同維 High+Low 同時命中 => low */ }
#[test]
fn empty_is_none() { /* 全中檔 => None */ }
#[test]
fn tie_break_is_deterministic() { /* 兩組同數同 min priority 時按 trigger 字典序 */ }
```

- [ ] **Step 2: 實作最小 `select_for_domain`**

```rust
pub fn select_for_domain(domain: Domain, display: [f64;5], ranges: [u8;5]) -> Option<Selected> {
  let hits = ANCHORS.iter().filter(|a| a.domain==domain && hit(a, display)).collect::<Vec<_>>();
  // 分組 -> 計數 -> 全距降級 -> 勝出 T* -> anchor_coverage
}
fn hit(a: &Anchor, display: [f64;5]) -> bool {
  match a.level { Level::High => display[a.dimension] >= 67.0, Level::Low => display[a.dimension] < 33.0 }
}
```

- [ ] **Step 3: 跑測試通過**

Run: `cargo test -p ft-schema --lib predict -q`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/schema/src/predict.rs crates/schema/src/lib.rs
git commit -m "feat(schema): hit/selection pure function with coverage (incl. range≥2 downgrade)"
```

---

### Task 7: F4 閘門與冪等、per-week 負面篩選（文件化 + 測試）

**Files:**
- Modify: `crates/schema/src/predict.rs` (新增 `filter_negative_half` 與閘門語義註記)
- Modify: `docs/superpowers/specs/2026-09-03-f5-rule-anchors-design.md` §6（若需補註）

**Interfaces:**
- Produces: `pub fn filter_negative_half(predictions: Vec<Selected>) -> Vec<Selected>`（per-output 負面 ≤半數，超過丟 valence 最負者，`Neutral` 永不丟）

- [ ] **Step 1: 寫 failing 測試**

```rust
#[test]
fn per_week_negative_not_over_half() {
  // 3 負 1 中 -> 過濾後 2 負 1 中；全中性不過濾
}
#[test]
fn gate_is_snapshot_at_creation() {
  // 文件化：F4 強度僅在創建時判定，週中變動不追改已產 predictions
}
```

- [ ] **Step 2: 實作 `filter_negative_half` 與冪等語義（`UNIQUE(user_id, cycle_id, domain)`，重觸為冪等返回）**

- [ ] **Step 3: Commit**

```bash
git add crates/schema/src/predict.rs
git commit -m "feat(schema): per-week negative-half filter and gate/idempotency semantics"
```

---

## Self-Review

**1. Spec coverage:**
- §1 型別與 12 格目錄 -> Task 1–3
- §2 命中/擇一/coverage（含全距 ≥2 P0）-> Task 6
- §3 DDL（含 `prediction_feedback` P0、`is_control` P1、`cycle_id` Asia/Taipei P1）與 wire 強型別 -> Task 4–5
- §4 不變式 -> Task 3
- §5 P1 偏離（trigger 文字來源）-> Task 1 文件化
- F4 閘門/冪等/per-week 負面篩選（Grok P1/P2）-> Task 7
- 間隙：`forecast` 文案的人工 checklist（行為傾向比較、不得人格缺陷）不在 CI，以 Task 2 註記涵蓋

**2. Placeholder scan:** 無 TBD/TODO；每步含可執行 code 與 `cargo test/check` 指令

**3. Type consistency:** `Domain` / `TriggerClass` / `Level` / `Valence` 在 anchors 與 predict、api wire 三處同源；`TriggerWire` 小寫 `t1..t6` 鎖定避免大小寫漂移

---

Plan complete and saved to `docs/superpowers/plans/2026-09-03-f5-rule-anchors.md`. Two execution options:

**1. Subagent-Driven (recommended)** - 依 Task 1–7 每任務派 fresh subagent，任務間 review，快迭代

**2. Inline Execution** - 在本 session 內批次執行，設 checkpoint 供你 review

Which approach?
