//! ZiWei domain — wraps x-iztro, converts to ft-schema V3.
//! Mirrors backend/src/services/ziwei/iztro-adapter.ts (production logic).

use ft_schema::*;
use x_iztro::{by_solar, Config, Gender, Language};

const EARTHLY_BRANCHES: [&str; 12] =
    ["子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥"];
const YANG_STEMS: [&str; 5] = ["甲", "丙", "戊", "庚", "壬"];

/// hour -> iztro time_index. Mirrors backend iztro-adapter.ts timeIndexFromHour:
/// hour 23 -> 12 (晚子); else branch = EARTHLY_BRANCHES[(hour+1)/2 % 12] index.
pub fn hour_to_time_index(hour: u8) -> u8 {
    if hour == 23 { return 12; }
    let branch_idx = ((hour as usize + 1) / 2) % 12;
    branch_idx as u8
}

fn branch_index(b: &str) -> usize {
    EARTHLY_BRANCHES.iter().position(|x| *x == b).unwrap_or(0)
}

fn sihua_code(mutagen: &str) -> Option<&'static str> {
    match mutagen {
        "祿" | "禄" => Some("lu"),
        "權" | "权" => Some("quan"),
        "科" => Some("ke"),
        "忌" => Some("ji"),
        _ => None,
    }
}

fn map_star(name: &str, star_type: &str, brightness: Option<&str>, mutagen: &str) -> ZiWeiStarV3 {
    let sihua = sihua_code(mutagen);
    let mapped_type = if star_type == "major" {
        "main"
    } else if sihua.is_some() {
        "transformation"
    } else {
        "auxiliary"
    };
    ZiWeiStarV3 {
        name: name.to_string(),
        star_type: mapped_type.to_string(),
        brightness: brightness.filter(|b| !b.is_empty()).map(|b| b.to_string()),
        sihua: sihua.map(|s| s.to_string()),
    }
}

fn parse_lunar(dto: &serde_json::Value) -> LunarDate {
    let rd = &dto["rawDates"]["lunarDate"];
    LunarDate {
        year: rd["lunarYear"].as_u64().unwrap_or(0) as u16,
        month: rd["lunarMonth"].as_u64().unwrap_or(0) as u8,
        day: rd["lunarDay"].as_u64().unwrap_or(0) as u8,
        is_leap: rd["isLeap"].as_bool(),
    }
}

fn parse_four_pillars(dto: &serde_json::Value) -> StemBranchX4 {
    let cd = &dto["rawDates"]["chineseDate"];
    let sb = |key: &str| -> StemBranch {
        let arr = cd[key].as_array().cloned().unwrap_or_default();
        StemBranch {
            stem: arr.first().and_then(|v| v.as_str()).unwrap_or("").to_string(),
            branch: arr.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string(),
        }
    };
    StemBranchX4 { year: sb("yearly"), month: sb("monthly"), day: sb("daily"), hour: sb("hourly") }
}

/// Reorder x-iztro decadal (branch-ordered, 寅-first) into production order:
/// life-palace first, then follow 陽男陰女順 / 陰男陽女逆 through the 12 branches.
fn reorder_major_limits(
    raw_palaces: &[serde_json::Value],
    soul_idx: usize,
    year_stem: &str,
    is_male: bool,
) -> Vec<MajorLimit> {
    // Collect limits keyed by palace_index (branch-ordered, same as was wasm).
    let mut by_palace: Vec<Option<MajorLimit>> = vec![None; 12];
    for p in raw_palaces {
        let range = p["decadal"]["range"].as_array();
        if let Some(range) = range {
            if let Some(start) = range.first().and_then(|v| v.as_u64()) {
                if start > 0 {
                    let br = p["earthlyBranch"].as_str().unwrap_or("");
                    let pi = branch_index(br);
                    by_palace[pi] = Some(MajorLimit {
                        start_age: start as u8,
                        end_age: range.get(1).and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                        stem: p["decadal"]["heavenlyStem"].as_str().unwrap_or("").to_string(),
                        branch: p["decadal"]["earthlyBranch"].as_str().unwrap_or("").to_string(),
                        palace_index: pi as u8,
                    });
                }
            }
        }
    }
    // Direction: 陽男/陰女 = 順 (increasing branch index); 陰男/陽女 = 逆 (decreasing).
    let yang = YANG_STEMS.contains(&year_stem);
    let shun = (yang && is_male) || (!yang && !is_male);
    let mut result = Vec::with_capacity(12);
    let mut idx = soul_idx;
    for _ in 0..12 {
        if let Some(Some(m)) = by_palace.get(idx) {
            result.push(m.clone());
        }
        idx = if shun { (idx + 1) % 12 } else { (idx + 11) % 12 };
    }
    result
}

pub fn calculate(
    solar_date: &str,
    time_index: u8,
    gender: &str,
    fix_leap: bool,
) -> Result<ZiWeiChartV3, String> {
    let is_male = matches!(gender, "male" | "M" | "男");
    let g = if is_male { Gender::Male } else { Gender::Female };
    let astrolabe =
        by_solar(solar_date, time_index, g, fix_leap, Language::ZhTW, Config::default())
            .map_err(|e| e.to_string())?;
    let dto = serde_json::to_value(astrolabe.to_dto()).map_err(|e| e.to_string())?;

    let four_pillars = parse_four_pillars(&dto);
    let soul_branch = dto["earthlyBranchOfSoulPalace"].as_str().unwrap_or("").to_string();
    let body_branch = dto["earthlyBranchOfBodyPalace"].as_str().unwrap_or("").to_string();
    let soul_idx = branch_index(&soul_branch);
    let body_idx = branch_index(&body_branch);
    let five_element = dto["fiveElementsClass"].as_str().unwrap_or("").to_string();
    let is_leap = dto["rawDates"]["lunarDate"]["isLeap"].as_bool().unwrap_or(false);
    let year_stem = four_pillars.year.stem.clone();

    let raw_palaces = dto["palaces"].as_array().cloned().unwrap_or_default();
    let mut ground: Vec<Option<ft_schema::ZiWeiPalaceV3>> = vec![None; 12];
    for p in &raw_palaces {
        let br = p["earthlyBranch"].as_str().unwrap_or("");
        let gi = branch_index(br);
        let mut stars = Vec::new();
        for group_key in ["majorStars", "minorStars", "adjectiveStars"] {
            for s in p[group_key].as_array().cloned().unwrap_or_default() {
                stars.push(map_star(
                    s["name"].as_str().unwrap_or(""),
                    s["type"].as_str().unwrap_or(""),
                    s["brightness"].as_str(),
                    s["mutagen"].as_str().unwrap_or(""),
                ));
            }
        }
        ground[gi] = Some(ft_schema::ZiWeiPalaceV3 {
            index: gi as u8,
            name: p["name"].as_str().unwrap_or("").to_string(),
            branch: br.to_string(),
            stem: p["heavenlyStem"].as_str().unwrap_or("").to_string(),
            stars,
            is_life_palace: Some(br == soul_branch),
            is_body_palace: Some(br == body_branch),
        });
    }

    let palaces: Vec<ft_schema::ZiWeiPalaceV3> = (0..12)
        .map(|i| ground[i].clone().unwrap_or_else(|| ft_schema::ZiWeiPalaceV3 {
            index: i as u8,
            name: String::new(),
            branch: EARTHLY_BRANCHES[i].to_string(),
            stem: String::new(),
            stars: Vec::new(),
            is_life_palace: Some(i == soul_idx),
            is_body_palace: Some(i == body_idx),
        }))
        .collect();

    let major_limits = reorder_major_limits(&raw_palaces, soul_idx, &year_stem, is_male);
    let birth_info = BirthInfoV3 {
        solar: SolarDate {
            year: solar_date.split('-').nth(0).and_then(|x| x.parse().ok()).unwrap_or(0),
            month: solar_date.split('-').nth(1).and_then(|x| x.parse().ok()).unwrap_or(0),
            day: solar_date.split('-').nth(2).and_then(|x| x.parse().ok()).unwrap_or(0),
        },
        lunar: parse_lunar(&dto),
        hour: time_index,
        hour_branch: EARTHLY_BRANCHES[if time_index == 12 { 0 } else { time_index as usize }].to_string(),
        gender: if is_male { "男".to_string() } else { "女".to_string() },
    };

    Ok(ZiWeiChartV3 {
        birth_info,
        four_pillars,
        five_element,
        life_palace_index: soul_idx as u8,
        body_palace_index: body_idx as u8,
        palaces,
        major_limits,
        meta: ZiWeiMeta {
            day_divide: "forward".to_string(),
            is_leap,
            fix_leap,
            time_index,
            engine_version_ziwei: "4.0.0".to_string(),
            chart_schema_version: 3,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn palaces_twelve_and_stars() {
        let c = calculate("1990-05-15", 6, "male", true).unwrap();
        assert_eq!(c.palaces.len(), 12);
        let yin = &c.palaces[branch_index("寅")];
        assert_eq!(yin.name, "田宅");
        assert!(yin.stars.iter().any(|s| s.name == "廉貞"));
    }
    #[test]
    fn hour_to_time_index_matches_prod() {
        // prod timeIndexFromHour: 23->12; else branch[(hour+1)/2%12]
        assert_eq!(hour_to_time_index(23), 12); // 晚子
        assert_eq!(hour_to_time_index(0), 0);   // 早子
        assert_eq!(hour_to_time_index(14), 7);  // 未
        assert_eq!(hour_to_time_index(8), 4);   // 辰
        assert_eq!(hour_to_time_index(20), 10); // 戌
    }
    #[test]
    fn leap_month_is_leap() {
        let c = calculate("2023-03-22", 4, "female", true).unwrap();
        assert!(c.meta.is_leap);
        assert_eq!(c.birth_info.lunar.year, 2023);
        assert_eq!(c.birth_info.lunar.day, 1);
    }
}
