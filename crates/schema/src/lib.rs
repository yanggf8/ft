//! Shared DTOs — single source of truth for Worker and Web.
//! Mirrors backend/src/shared/schemas/ziwei-v3.ts (Zod) but in Rust types.
//! Phase A: ZiWei V3 + Western types. Big5 to follow.

use serde::{Deserialize, Serialize};

// ── ZiWei V3 ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StemBranch {
    pub stem: String,
    pub branch: String,
}

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
pub struct ZiWeiMeta {
    #[serde(rename = "dayDivide")]
    pub day_divide: String,
    #[serde(rename = "isLeap")]
    pub is_leap: bool,
    #[serde(rename = "fixLeap")]
    pub fix_leap: bool,
    #[serde(rename = "timeIndex")]
    pub time_index: u8,
    #[serde(rename = "engineVersionZiwei")]
    pub engine_version_ziwei: String,
    #[serde(rename = "chartSchemaVersion")]
    pub chart_schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BirthInfoV3 {
    pub solar: SolarDate,
    pub lunar: LunarDate,
    pub hour: u8,
    #[serde(rename = "hourBranch")]
    pub hour_branch: String,
    pub gender: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SolarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LunarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    #[serde(rename = "isLeap")]
    pub is_leap: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MajorLimit {
    #[serde(rename = "startAge")]
    pub start_age: u8,
    #[serde(rename = "endAge")]
    pub end_age: u8,
    pub stem: String,
    pub branch: String,
    #[serde(rename = "palaceIndex")]
    pub palace_index: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ZiWeiChartV3 {
    #[serde(rename = "birthInfo")]
    pub birth_info: BirthInfoV3,
    #[serde(rename = "fourPillars")]
    pub four_pillars: StemBranchX4,
    #[serde(rename = "fiveElement")]
    pub five_element: String,
    #[serde(rename = "lifePalaceIndex")]
    pub life_palace_index: u8,
    #[serde(rename = "bodyPalaceIndex")]
    pub body_palace_index: u8,
    pub palaces: Vec<ZiWeiPalaceV3>,
    #[serde(rename = "majorLimits")]
    pub major_limits: Vec<MajorLimit>,
    pub meta: ZiWeiMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StemBranchX4 {
    pub year: StemBranch,
    pub month: StemBranch,
    pub day: StemBranch,
    pub hour: StemBranch,
}

// ── Western chart types ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WesternPlanet {
    pub name: String,
    pub longitude: f64,
    pub sign: String,
    pub degree: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WesternChartV3 {
    pub planets: Vec<WesternPlanet>,
    pub ascendant: WesternAscendant,
    pub houses: Vec<WesternHouse>,
    pub jd_utc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WesternAscendant {
    pub longitude: f64,
    pub sign: String,
    pub degree: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WesternHouse {
    pub index: u8,
    pub sign: String,
    pub cusp: f64,
}

impl WesternChartV3 {
    pub fn from_longitudes(planets_raw: Vec<(&str, f64)>, asc_lon: f64, jd_utc: f64) -> Self {
        let planets = planets_raw
            .into_iter()
            .map(|(name, lon)| {
                let (sign, degree) = sign_degree(lon);
                WesternPlanet { name: name.to_string(), longitude: lon, sign, degree }
            })
            .collect();
        let (asc_sign, asc_deg) = sign_degree(asc_lon);
        let ascendant = WesternAscendant { longitude: asc_lon, sign: asc_sign.clone(), degree: asc_deg };
        let asc_sign_idx = sign_index(&asc_sign);
        let houses = (0..12)
            .map(|i| {
                let sign_idx = (asc_sign_idx + i) % 12;
                let sign = ZODIAC[sign_idx].to_string();
                let cusp = (asc_sign_idx as f64 * 30.0 + i as f64 * 30.0).rem_euclid(360.0);
                WesternHouse { index: i as u8 + 1, sign, cusp }
            })
            .collect();
        Self { planets, ascendant, houses, jd_utc }
    }
}

const ZODIAC: [&str; 12] = ["Aries","Taurus","Gemini","Cancer","Leo","Virgo","Libra","Scorpio","Sagittarius","Capricorn","Aquarius","Pisces"];

fn sign_degree(lon: f64) -> (String, f64) {
    let lon = lon.rem_euclid(360.0);
    let idx = (lon / 30.0).floor() as usize % 12;
    (ZODIAC[idx].to_string(), lon - idx as f64 * 30.0)
}

fn sign_index(sign: &str) -> usize {
    ZODIAC.iter().position(|s| *s == sign).unwrap_or(0)
}
