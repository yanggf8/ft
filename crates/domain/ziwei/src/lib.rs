//! ZiWei domain — wraps x-iztro, converts to ft-schema V3.
//! Mirrors backend/src/services/ziwei/iztro-adapter.ts (production logic).

use ft_schema::ziwei_luck::{
    BranchName, HoroscopeAge, HoroscopeDecadal, HoroscopeMutagen, HoroscopeYearly, ZiWeiHoroscopeV1,
};
use ft_schema::*;
use x_iztro::{
    by_solar, translate_earthly_branch, translate_heavenly_stem, translate_palace, translate_star,
    AgeDivide, Config, EarthlyBranch, Gender, HeavenlyStem, HoroscopeName, Language, StarKey,
};

const EARTHLY_BRANCHES: [&str; 12] = [
    "子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥",
];
const YANG_STEMS: [&str; 5] = ["甲", "丙", "戊", "庚", "壬"];

/// hour -> iztro time_index. Mirrors backend iztro-adapter.ts timeIndexFromHour:
/// hour 23 -> 12 (晚子); else branch = EARTHLY_BRANCHES[(hour+1)/2 % 12] index.
pub fn hour_to_time_index(hour: u8) -> u8 {
    if hour == 23 {
        return 12;
    }
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
            stem: arr
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            branch: arr
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        }
    };
    StemBranchX4 {
        year: sb("yearly"),
        month: sb("monthly"),
        day: sb("daily"),
        hour: sb("hourly"),
    }
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
                        stem: p["decadal"]["heavenlyStem"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        branch: p["decadal"]["earthlyBranch"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
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
        idx = if shun {
            (idx + 1) % 12
        } else {
            (idx + 11) % 12
        };
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
    let g = if is_male {
        Gender::Male
    } else {
        Gender::Female
    };
    let astrolabe = by_solar(
        solar_date,
        time_index,
        g,
        fix_leap,
        Language::ZhTW,
        chart_config(),
    )
    .map_err(|e| e.to_string())?;
    let dto = serde_json::to_value(astrolabe.to_dto()).map_err(|e| e.to_string())?;

    let four_pillars = parse_four_pillars(&dto);
    let soul_branch = dto["earthlyBranchOfSoulPalace"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let body_branch = dto["earthlyBranchOfBodyPalace"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let soul_idx = branch_index(&soul_branch);
    let body_idx = branch_index(&body_branch);
    let five_element = dto["fiveElementsClass"].as_str().unwrap_or("").to_string();
    let is_leap = dto["rawDates"]["lunarDate"]["isLeap"]
        .as_bool()
        .unwrap_or(false);
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
        .map(|i| {
            ground[i]
                .clone()
                .unwrap_or_else(|| ft_schema::ZiWeiPalaceV3 {
                    index: i as u8,
                    name: String::new(),
                    branch: EARTHLY_BRANCHES[i].to_string(),
                    stem: String::new(),
                    stars: Vec::new(),
                    is_life_palace: Some(i == soul_idx),
                    is_body_palace: Some(i == body_idx),
                })
        })
        .collect();

    let major_limits = reorder_major_limits(&raw_palaces, soul_idx, &year_stem, is_male);
    let birth_info = BirthInfoV3 {
        solar: SolarDate {
            year: solar_date
                .split('-')
                .nth(0)
                .and_then(|x| x.parse().ok())
                .unwrap_or(0),
            month: solar_date
                .split('-')
                .nth(1)
                .and_then(|x| x.parse().ok())
                .unwrap_or(0),
            day: solar_date
                .split('-')
                .nth(2)
                .and_then(|x| x.parse().ok())
                .unwrap_or(0),
        },
        lunar: parse_lunar(&dto),
        hour: time_index,
        hour_branch: EARTHLY_BRANCHES[if time_index == 12 {
            0
        } else {
            time_index as usize
        }]
        .to_string(),
        gender: if is_male {
            "男".to_string()
        } else {
            "女".to_string()
        },
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

/// 排盤設定：命盤與流年共用。改任何 divide 設定必 bump
/// `ENGINE_VERSION_ZIWEI_HOROSCOPE`（api `engine_version.rs`），測試
/// `config_default_divides_pinned` 守門。
fn chart_config() -> Config {
    Config::default()
}

fn zh_stem(s: HeavenlyStem) -> String {
    translate_heavenly_stem(s, Language::ZhTW).to_string()
}

fn zh_branch(b: EarthlyBranch) -> String {
    translate_earthly_branch(b, Language::ZhTW).to_string()
}

fn zh_star(k: Option<&StarKey>) -> String {
    k.map(|k| translate_star(*k, Language::ZhTW).to_string())
        .unwrap_or_default()
}

/// 流年大運（spec §A.4）。`target` 為陽曆 `YYYY-M-D`；流年以上層級只看農曆年，
/// 目標時辰固定午（6）。輸出索引一律 **子=0**（x-iztro 是 寅=0），並帶地支字串。
pub fn horoscope(
    solar_date: &str,
    time_index: u8,
    gender: &str,
    fix_leap: bool,
    target: &str,
) -> Result<ZiWeiHoroscopeV1, String> {
    let is_male = matches!(gender, "male" | "M" | "男");
    let g = if is_male {
        Gender::Male
    } else {
        Gender::Female
    };
    let config = chart_config();
    // luckYear 公式只在「跨農曆年即加一歲」成立（R05）。
    if config.age_divide != AgeDivide::Normal {
        return Err("unsupported age divide".to_string());
    }
    let astrolabe = by_solar(solar_date, time_index, g, fix_leap, Language::ZhTW, config)
        .map_err(|e| e.to_string())?;
    let h = astrolabe.horoscope(target, 6).map_err(|e| e.to_string())?;

    let decadal_branch = zh_branch(h.decadal.earthly_branch);
    let is_childhood = h.decadal.name_key == HoroscopeName::Childhood;
    let (start_age, end_age) = if is_childhood {
        (None, None)
    } else {
        let (s, e) = astrolabe.palaces[h.decadal.index].decadal.range;
        (Some(s.min(255) as u8), Some(e.min(255) as u8))
    };
    // 童限的干支取「本命宮位」干支；大限取大限干支。宮位一律以該宮所在地支定位。
    let decadal_palace_branch = zh_branch(astrolabe.palaces[h.decadal.index].earthly_branch);
    let age_branch = zh_branch(h.age.earthly_branch);
    let yearly_branch = zh_branch(h.yearly.earthly_branch);

    // 流年十二宮名：x-iztro 依宮位索引（寅=0）排列；以該宮地支為鍵，輸出依 子→亥。
    let mut names: Vec<Option<String>> = vec![None; 12];
    for (i, p) in h.yearly.palace_names.iter().enumerate() {
        let br = zh_branch(astrolabe.palaces[i].earthly_branch);
        names[branch_index(&br)] = Some(translate_palace(*p, Language::ZhTW).to_string());
    }
    let palace_names = names
        .into_iter()
        .enumerate()
        .map(|(i, n)| BranchName {
            branch: EARTHLY_BRANCHES[i].to_string(),
            name: n.unwrap_or_default(),
        })
        .collect();

    let m = &h.yearly.mutagen;
    let mutagen = HoroscopeMutagen {
        lu: zh_star(m.first()),
        quan: zh_star(m.get(1)),
        ke: zh_star(m.get(2)),
        ji: zh_star(m.get(3)),
    };
    let nominal_age = h.age.nominal_age;
    let luck_year = astrolabe.raw_dates.lunar_date.lunar_year as i32 + nominal_age as i32 - 1;

    Ok(ZiWeiHoroscopeV1 {
        target_date: target.to_string(),
        luck_year,
        nominal_age,
        decadal: HoroscopeDecadal {
            palace_index: branch_index(&decadal_palace_branch) as u8,
            branch: decadal_branch,
            stem: zh_stem(h.decadal.heavenly_stem),
            start_age,
            end_age,
            is_childhood,
        },
        age: HoroscopeAge {
            palace_index: branch_index(&age_branch) as u8,
            branch: age_branch,
            stem: zh_stem(h.age.heavenly_stem),
        },
        yearly: HoroscopeYearly {
            palace_index: branch_index(&yearly_branch) as u8,
            branch: yearly_branch,
            stem: zh_stem(h.yearly.heavenly_stem),
            palace_names,
            mutagen,
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
        assert_eq!(hour_to_time_index(0), 0); // 早子
        assert_eq!(hour_to_time_index(14), 7); // 未
        assert_eq!(hour_to_time_index(8), 4); // 辰
        assert_eq!(hour_to_time_index(20), 10); // 戌
    }
    #[test]
    fn leap_month_is_leap() {
        let c = calculate("2023-03-22", 4, "female", true).unwrap();
        assert!(c.meta.is_leap);
        assert_eq!(c.birth_info.lunar.year, 2023);
        assert_eq!(c.birth_info.lunar.day, 1);
    }

    /// 索引一致性：子=0 的 palaceIndex 指到的本命宮地支必須等於輸出的 branch。
    fn assert_index_consistent(c: &ZiWeiChartV3, h: &ZiWeiHoroscopeV1) {
        let at = |i: u8| c.palaces[i as usize].branch.clone();
        assert_eq!(h.yearly.branch, at(h.yearly.palace_index));
        assert_eq!(h.age.branch, at(h.age.palace_index));
        if !h.decadal.is_childhood {
            assert_eq!(h.decadal.branch, at(h.decadal.palace_index));
        }
        assert_eq!(h.yearly.palace_names.len(), 12);
        let life = h
            .yearly
            .palace_names
            .iter()
            .find(|b| b.name == "命宮")
            .unwrap();
        assert_eq!(life.branch, h.yearly.branch, "流年命宮名必落在流年地支");
        for (i, b) in h.yearly.palace_names.iter().enumerate() {
            assert_eq!(b.branch, EARTHLY_BRANCHES[i]);
        }
    }

    #[test]
    fn config_default_divides_pinned() {
        // 改任何一項 → bump api ENGINE_VERSION_ZIWEI_HOROSCOPE，且 luckYear 公式要重驗
        let c = chart_config();
        assert_eq!(c.year_divide, x_iztro::YearDivide::Normal);
        assert_eq!(c.horoscope_divide, x_iztro::HoroscopeDivide::Normal);
        assert_eq!(c.age_divide, AgeDivide::Normal);
    }

    #[test]
    fn golden_1990_male_wu_2026() {
        // spec §8 spike：1990-05-15 午時 男 → 2026 丙午
        let c = calculate("1990-05-15", 6, "male", true).unwrap();
        let h = horoscope("1990-05-15", 6, "男", true, "2026-7-1").unwrap();
        assert_eq!(h.luck_year, 2026);
        assert_eq!(h.nominal_age, 37);
        assert_eq!(
            (h.yearly.stem.as_str(), h.yearly.branch.as_str()),
            ("丙", "午")
        );
        assert_eq!(h.yearly.palace_index, 6);
        let m = &h.yearly.mutagen;
        assert_eq!(
            (m.lu.as_str(), m.quan.as_str(), m.ke.as_str(), m.ji.as_str()),
            ("天同", "天機", "文昌", "廉貞")
        );
        assert!(!h.decadal.is_childhood);
        assert_eq!(h.decadal.branch, "寅");
        assert_eq!(h.decadal.stem, "戊");
        assert_eq!(h.decadal.palace_index, 2);
        assert_eq!(
            (h.decadal.start_age, h.decadal.end_age),
            (Some(35), Some(44))
        );
        assert_eq!(h.age.branch, "辰");
        assert_eq!(h.age.palace_index, 4);
        // 大限與 chart.majorLimits 同一段
        assert!(c
            .major_limits
            .iter()
            .any(|ml| ml.branch == "寅" && ml.start_age == 35 && ml.end_age == 44));
        assert_index_consistent(&c, &h);
    }

    #[test]
    fn golden_1990_before_spring_festival_is_previous_year() {
        // 2026 春節＝2/17：2/1 仍是農曆 2025（乙巳）年
        let c = calculate("1990-05-15", 6, "male", true).unwrap();
        let h = horoscope("1990-05-15", 6, "male", true, "2026-2-1").unwrap();
        assert_eq!(h.luck_year, 2025);
        assert_eq!(h.nominal_age, 36);
        assert_eq!(
            (h.yearly.stem.as_str(), h.yearly.branch.as_str()),
            ("乙", "巳")
        );
        assert_eq!(h.age.branch, "卯");
        let m = &h.yearly.mutagen;
        assert_eq!(
            (m.lu.as_str(), m.quan.as_str(), m.ke.as_str(), m.ji.as_str()),
            ("天機", "天梁", "紫微", "太陰")
        );
        assert_index_consistent(&c, &h);
        // 7/1 必在農曆同年
        let h7 = horoscope("1990-05-15", 6, "male", true, "2025-7-1").unwrap();
        assert_eq!(h7.luck_year, 2025);
        assert_eq!(h7.yearly, h.yearly);
    }

    #[test]
    fn golden_born_before_spring_festival() {
        // 1985-01-20 生於春節前 → 農曆 1984 年；2026 虛歲 43，大限 申 42–51
        let c = calculate("1985-01-20", 0, "female", true).unwrap();
        assert_eq!(c.birth_info.lunar.year, 1984);
        let h = horoscope("1985-01-20", 0, "female", true, "2026-7-1").unwrap();
        assert_eq!(h.luck_year, 2026);
        assert_eq!(h.nominal_age, 43);
        assert_eq!(h.decadal.branch, "申");
        assert_eq!(
            (h.decadal.start_age, h.decadal.end_age),
            (Some(42), Some(51))
        );
        assert_eq!(h.age.branch, "辰");
        assert_index_consistent(&c, &h);
        // 2025-01-20 仍在農曆 2024 年
        let early = horoscope("1985-01-20", 0, "female", true, "2025-1-20").unwrap();
        assert_eq!(early.luck_year, 2024);
        assert_eq!(
            (early.yearly.stem.as_str(), early.yearly.branch.as_str()),
            ("甲", "辰")
        );
        assert_index_consistent(&c, &early);
    }

    #[test]
    fn golden_childhood_year() {
        // 2022-03-10 寅時 女：大限 3 歲起；虛歲 1 為童限（命宮）、虛歲 3 進第一大限
        let c = calculate("2022-03-10", 2, "female", true).unwrap();
        let h = horoscope("2022-03-10", 2, "female", true, "2022-7-1").unwrap();
        assert_eq!(h.nominal_age, 1);
        assert_eq!(h.luck_year, 2022);
        assert!(h.decadal.is_childhood);
        assert_eq!((h.decadal.start_age, h.decadal.end_age), (None, None));
        // 童限虛歲 1 → 本命命宮
        assert_eq!(h.decadal.palace_index, c.life_palace_index);
        assert_eq!(
            h.decadal.branch,
            c.palaces[c.life_palace_index as usize].branch
        );
        assert_index_consistent(&c, &h);
        let later = horoscope("2022-03-10", 2, "female", true, "2024-7-1").unwrap();
        assert_eq!(later.nominal_age, 3);
        assert!(!later.decadal.is_childhood);
        assert_eq!(
            (later.decadal.start_age, later.decadal.end_age),
            (Some(3), Some(12))
        );
        assert_index_consistent(&c, &later);
    }
}
