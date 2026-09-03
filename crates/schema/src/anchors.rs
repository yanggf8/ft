//! 規則錨點目錄 — work+money × T1–T6（v1 縱深 12 格，每格 ≥2 條）
//! 與 `items.rs` 同體例：`ft-schema` 靜態真相，`crates/web` 可讀，`ft-big5` 不動
//! Trigger 封閉列舉 T1–T6 見 `2026-09-03-f4-f5-if-then-design-note.md` §5.3.3

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    Work,
    Love,
    Family,
    Money,
    Health,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerClass {
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    High,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Valence {
    Negative,
    Neutral,
    Positive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Literature,
    DesignerJudgment,
}

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
