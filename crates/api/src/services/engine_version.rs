//! Engine-version constants — mirrors backend/src/services/engine-version.ts.
//!
//! Kept at 3.0.0 intentionally: `x-iztro` matches iztro behavior (no bump needed
//! for ziwei); the western engine must bump to 4.0.0 only after the §8.2 event-table
//! validation. Do not change WESTERN until that gate passes.

pub const ENGINE_VERSION_ZIWEI: &str = "3.0.0";
// Western bumped to 4.0.0 when the engine gained real ephemeris + the top-level
// sunSign/moonSign contract (chart_data.sunSign). Old cached western charts (3.0.0,
// lacking sunSign) are no longer current and must be recalculated.
pub const ENGINE_VERSION_WESTERN: &str = "4.0.0";
pub const CHART_SCHEMA_VERSION: u32 = 3;
/// 紫微流年大運（`GET /api/ziwei/horoscope`、今年運勢快取鍵）的引擎版本（spec
/// 2026-11-21 §A.4）。改 ft-ziwei `horoscope()` 的輸出語意、或 `Config` 任一 divide
/// 設定（ft-ziwei 測試 `config_default_divides_pinned` 守門）必 bump——今年運勢
/// 快取以它為鍵之一，bump 後舊解讀自然不命中。
pub const ENGINE_VERSION_ZIWEI_HOROSCOPE: &str = "1";
