//! Shared DTOs — single source of truth for Worker and Web.
//! Mirrors backend/src/shared/schemas/ziwei-v3.ts (Zod) but in Rust types.
//! Phase A: ZiWei V3 types. Western/Big5 types to follow.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZiWeiStarV3 {
    pub name: String,
    #[serde(rename = "type")]
    pub star_type: String,
    pub brightness: Option<String>,
    pub sihua: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZiWeiPalaceV3 {
    pub index: u8,
    pub name: String,
    pub branch: String,
    pub stem: String,
    pub stars: Vec<ZiWeiStarV3>,
    #[serde(rename = "isLifePalace")]
    pub is_life_palace: Option<bool>,
    #[serde(rename = "isBodyPalace")]
    pub is_body_palace: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZiWeiFourPillars {
    pub year: StemBranch,
    pub month: StemBranch,
    pub day: StemBranch,
    pub hour: StemBranch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StemBranch {
    pub stem: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZiWeiMeta {
    #[serde(rename = "dayDivide")]
    pub day_divide: String,
    #[serde(rename = "isLeap")]
    pub is_leap: bool,
    #[serde(rename = "fixLeap")]
    pub fix_leap: bool,
    #[serde(rename = "timeIndex")]
    pub time_index: u8,
    #[serde(rename = "hourShifted")]
    pub hour_shifted: Option<bool>,
    pub assumed: Option<bool>,
    #[serde(rename = "engineVersionZiwei")]
    pub engine_version_ziwei: String,
    #[serde(rename = "chartSchemaVersion")]
    pub chart_schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZiWeiChartV3 {
    #[serde(rename = "birthInfo")]
    pub birth_info: serde_json::Value,
    pub palaces: Vec<ZiWeiPalaceV3>,
    #[serde(rename = "fourPillars")]
    pub four_pillars: ZiWeiFourPillars,
    pub meta: ZiWeiMeta,
}
