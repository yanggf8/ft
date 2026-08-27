//! ZiWei domain — pure logic, no IO. Wraps x-iztro and converts to ft-schema V3.
//! Compiles on both native and wasm32.

use ft_schema::{StemBranch, ZiWeiChartV3, ZiWeiFourPillars, ZiWeiMeta, ZiWeiPalaceV3, ZiWeiStarV3};
use x_iztro::{Config, Gender, Language};

pub fn calculate(
    solar_date: &str,
    time_index: u8,
    gender: &str,
    fix_leap: bool,
) -> Result<ZiWeiChartV3, String> {
    let g = match gender {
        "male" | "M" | "男" => Gender::Male,
        _ => Gender::Female,
    };
    let cfg = Config::default();
    let astrolabe =
        x_iztro::by_solar(solar_date, time_index, g, fix_leap, Language::ZhCN, cfg)
            .map_err(|e| e.to_string())?;
    let dto = astrolabe.to_dto();
    let v = serde_json::to_value(&dto).map_err(|e| e.to_string())?;
    // Convert DTO JSON to ft-schema types via serde_json
    // For Phase A we pass through the DTO shape; schema validation happens at the boundary
    let palaces: Vec<ZiWeiPalaceV3> = v["palaces"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .enumerate()
        .map(|(i, p)| ZiWeiPalaceV3 {
            index: i as u8,
            name: p["name"].as_str().unwrap_or("").to_string(),
            branch: p["earthlyBranch"].as_str().unwrap_or(p["branch"].as_str().unwrap_or("")).to_string(),
            stem: p["heavenlyStem"].as_str().unwrap_or(p["stem"].as_str().unwrap_or("")).to_string(),
            stars: p["majorStars"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|s| ZiWeiStarV3 {
                    name: s["name"].as_str().unwrap_or("").to_string(),
                    star_type: "main".to_string(),
                    brightness: s["brightness"].as_str().map(|b| b.to_string()),
                    sihua: {
                        let m = s["mutagen"].as_str().unwrap_or("");
                        if m.is_empty() { None } else { Some(m.to_string()) }
                    },
                })
                .collect(),
            is_life_palace: None,
            is_body_palace: None,
        })
        .collect();

    let meta = ZiWeiMeta {
        day_divide: "forward".to_string(),
        is_leap: v["isLeap"].as_bool().unwrap_or(false),
        fix_leap,
        time_index,
        hour_shifted: None,
        assumed: None,
        engine_version_ziwei: "4.0.0".to_string(),
        chart_schema_version: 3,
    };

    // fourPillars from DTO if available, else placeholder
    let four_pillars = ZiWeiFourPillars {
        year: StemBranch { stem: "".into(), branch: "".into() },
        month: StemBranch { stem: "".into(), branch: "".into() },
        day: StemBranch { stem: "".into(), branch: "".into() },
        hour: StemBranch { stem: "".into(), branch: "".into() },
    };

    Ok(ZiWeiChartV3 {
        birth_info: serde_json::json!({ "solar": solar_date, "timeIndex": time_index }),
        palaces,
        four_pillars,
        meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn palaces_twelve() {
        let c = calculate("1990-05-15", 6, "male", false).unwrap();
        assert_eq!(c.palaces.len(), 12);
    }
}
