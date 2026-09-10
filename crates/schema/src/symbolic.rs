//! F2 命盤象徵傾向 — 純規則對照表與聚合(spec 2026-09-07-f2-f3-design.md §1)。
//! 紅線:本模組不得被 predictions.rs / services::ai import(spec §0.4)。

pub const SYMBOLIC_RULES_VERSION: &str = "symbolic-1";
pub const DIM_CODES: [&str; 5] = [
    "extraversion",
    "agreeableness",
    "conscientiousness",
    "stability",
    "openness",
];
pub const DIM_LABELS: [&str; 5] = ["外向", "友善", "嚴謹", "情緒穩定", "智性開放"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Low,
    MidLow,
    Mid,
    MidHigh,
    High,
}

pub fn band_of(score: f64) -> Band {
    if score < 35.0 {
        Band::Low
    } else if score < 45.0 {
        Band::MidLow
    } else if score <= 55.0 {
        Band::Mid
    } else if score <= 65.0 {
        Band::MidHigh
    } else {
        Band::High
    }
}

impl Band {
    pub fn code(self) -> &'static str {
        match self {
            Band::Low => "low",
            Band::MidLow => "mid_low",
            Band::Mid => "mid",
            Band::MidHigh => "mid_high",
            Band::High => "high",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Band::Low => "偏低",
            Band::MidLow => "略低",
            Band::Mid => "中等",
            Band::MidHigh => "略高",
            Band::High => "偏高",
        }
    }
}

pub fn band_label(code: &str) -> &'static str {
    match code {
        "low" => "偏低",
        "mid_low" => "略低",
        "mid" => "中等",
        "mid_high" => "略高",
        _ => "偏高",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorSource {
    Ziwei,
    Western,
}

/// 規則「依據」標註(spec §1.3 雙軸:依據 × 映射;映射一律 designer,不入表)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    Classical,
    Designer,
}

#[derive(Clone, Debug)]
pub struct SymbolicVector {
    pub ruleset_version: &'static str,
    pub source: PriorSource,
    pub dims: [f64; 5],
    /// 每維貢獻規則的依據彙總:"classical" | "designer" | "mixed"
    pub dim_basis: [&'static str; 5],
}

/// (主星名, 維度 index 0..4 = E,A,C,S,O, 調整量, 依據)。缺席的維 = 無貢獻、
/// 不進分母(spec §1.3 聚合公約;Codex [6])。
pub const ZIWEI_RULES: &[(&str, usize, i32, Basis)] = &[
    ("紫微", 2, 20, Basis::Classical),
    ("紫微", 0, 10, Basis::Designer),
    ("紫微", 4, -10, Basis::Designer),
    ("天機", 4, 20, Basis::Classical),
    ("天機", 3, -10, Basis::Classical),
    ("天機", 2, -10, Basis::Classical),
    ("太陽", 0, 20, Basis::Classical),
    ("太陽", 1, 10, Basis::Classical),
    ("武曲", 2, 20, Basis::Classical),
    ("武曲", 1, -10, Basis::Classical),
    ("武曲", 0, -10, Basis::Designer),
    ("天同", 3, 20, Basis::Classical),
    ("天同", 2, -20, Basis::Classical),
    ("天同", 1, 10, Basis::Classical),
    ("廉貞", 2, 10, Basis::Classical),
    ("廉貞", 4, 10, Basis::Classical),
    ("廉貞", 3, -10, Basis::Designer),
    ("天府", 2, 20, Basis::Classical),
    ("天府", 3, 10, Basis::Classical),
    ("天府", 0, -10, Basis::Designer),
    ("太陰", 0, -20, Basis::Classical),
    ("太陰", 4, 10, Basis::Designer),
    ("太陰", 3, -10, Basis::Classical),
    ("貪狼", 0, 20, Basis::Classical),
    ("貪狼", 4, 10, Basis::Classical),
    ("貪狼", 2, -10, Basis::Classical),
    ("巨門", 4, 20, Basis::Classical),
    ("巨門", 1, -10, Basis::Classical),
    ("巨門", 3, -10, Basis::Classical),
    ("巨門", 0, -10, Basis::Designer),
    ("天相", 1, 20, Basis::Classical),
    ("天相", 2, 10, Basis::Classical),
    ("天梁", 3, 10, Basis::Classical),
    ("天梁", 1, 10, Basis::Classical),
    ("天梁", 2, 10, Basis::Classical),
    ("七殺", 0, 10, Basis::Classical),
    ("七殺", 1, -10, Basis::Classical),
    ("七殺", 3, -10, Basis::Designer),
    ("破軍", 4, 20, Basis::Classical),
    ("破軍", 2, -20, Basis::Classical),
    ("破軍", 0, 10, Basis::Classical),
];

use crate::{WesternSign, ZiWeiPalaceV3};

/// `WesternSign` 的星座名(element 對照用;單點映射,route 不碰欄位知識)。
pub fn western_sign_name(s: &WesternSign) -> &str {
    &s.name
}

const MAIN_STARS: [&str; 14] = [
    "紫微", "天機", "太陽", "武曲", "天同", "廉貞", "天府", "太陰", "貪狼", "巨門", "天相", "天梁",
    "七殺", "破軍",
];

/// 命宮主星(含借對宮,borrowed = true)。四化/亮度/身宮不計(spec §1.3)。
pub fn ziwei_life_stars(palaces: &[ZiWeiPalaceV3], life_palace_index: u8) -> Vec<(&str, bool)> {
    let at = |idx: u8| palaces.iter().find(|p| p.index == idx);
    let life = at(life_palace_index);
    let direct: Vec<(&str, bool)> = life
        .map(|p| {
            p.stars
                .iter()
                .filter(|s| MAIN_STARS.contains(&s.name.as_str()))
                .map(|s| (s.name.as_str(), false))
                .collect()
        })
        .unwrap_or_default();
    if !direct.is_empty() {
        return direct;
    }
    let opp = (life_palace_index + 6) % 12;
    at(opp)
        .map(|p| {
            p.stars
                .iter()
                .filter(|s| MAIN_STARS.contains(&s.name.as_str()))
                .map(|s| (s.name.as_str(), true))
                .collect()
        })
        .unwrap_or_default()
}

/// 每維命中調整的平均;無命中 = None。借星逐條 clamp [−10,10] 在平均前。
fn adjustments_for(stars: &[(&str, bool)]) -> [Option<f64>; 5] {
    let mut sums = [0f64; 5];
    let mut counts = [0usize; 5];
    for (star, borrowed) in stars {
        for (_s, dim, delta, _basis) in ZIWEI_RULES.iter().filter(|(s, _, _, _)| s == star) {
            let mut d = *delta as f64;
            if *borrowed {
                d = d.clamp(-10.0, 10.0);
            }
            sums[*dim] += d;
            counts[*dim] += 1;
        }
    }
    std::array::from_fn(|i| (counts[i] > 0).then(|| sums[i] / counts[i] as f64))
}

pub fn symbolic_vector(
    ziwei: Option<(&[ZiWeiPalaceV3], u8)>,
    western: Option<(&str, &str, &str)>,
) -> Option<SymbolicVector> {
    if let Some((palaces, idx)) = ziwei {
        let stars = ziwei_life_stars(palaces, idx);
        let adj = adjustments_for(&stars);
        let dims = std::array::from_fn(|i| adj[i].map_or(50.0, |a| (50.0 + a).clamp(0.0, 100.0)));
        return Some(SymbolicVector {
            ruleset_version: SYMBOLIC_RULES_VERSION,
            source: PriorSource::Ziwei,
            dims,
            dim_basis: basis_summary(&stars),
        });
    }
    if let Some((sun, moon, asc)) = western {
        let adj = western_adjustments(sun, moon, asc);
        let dims = std::array::from_fn(|i| adj[i].map_or(50.0, |a| (50.0 + a).clamp(0.0, 100.0)));
        return Some(SymbolicVector {
            ruleset_version: SYMBOLIC_RULES_VERSION,
            source: PriorSource::Western,
            dims,
            // 西洋元素→Big5 是純映射,無獨立「古典星性」依據軸 — 映射一律
            // designer(spec §1.3 雙軸;Kimi 終審 #2)
            dim_basis: ["designer"; 5],
        });
    }
    None
}

/// 每維貢獻規則的依據彙總(Codex [14]:UI 的來源行由此驅動,不寫死)。
fn basis_summary(stars: &[(&str, bool)]) -> [&'static str; 5] {
    let mut has_c = [false; 5];
    let mut has_d = [false; 5];
    for (star, _borrowed) in stars {
        for (_s, dim, _delta, basis) in ZIWEI_RULES.iter().filter(|(s, _, _, _)| s == star) {
            match basis {
                Basis::Classical => has_c[*dim] = true,
                Basis::Designer => has_d[*dim] = true,
            }
        }
    }
    std::array::from_fn(|i| match (has_c[i], has_d[i]) {
        (true, true) => "mixed",
        (false, true) => "designer",
        (true, false) => "classical",
        // 無任何規則命中的維(基線 50)不掛依據 — UI 顯示「—」(Kimi 終審 #5)
        (false, false) => "none",
    })
}

fn element(sign: &str) -> Option<usize> {
    let s = sign.trim().trim_end_matches('座');
    const FIRE: [&str; 6] = ["牡羊", "獅子", "射手", "Aries", "Leo", "Sagittarius"];
    const EARTH: [&str; 6] = ["金牛", "處女", "摩羯", "Taurus", "Virgo", "Capricorn"];
    const AIR: [&str; 6] = ["雙子", "天秤", "水瓶", "Gemini", "Libra", "Aquarius"];
    const WATER: [&str; 6] = ["巨蟹", "天蠍", "雙魚", "Cancer", "Scorpio", "Pisces"];
    let hit = |names: &[&str; 6]| names.iter().any(|n| s.eq_ignore_ascii_case(n));
    if hit(&FIRE) {
        Some(0)
    } else if hit(&EARTH) {
        Some(2)
    } else if hit(&AIR) {
        Some(4)
    } else if hit(&WATER) {
        Some(3)
    } else {
        None
    }
}

fn western_adjustments(sun: &str, moon: &str, asc: &str) -> [Option<f64>; 5] {
    let mut sums = [0f64; 5];
    let mut counts = [0usize; 5];
    for (sign, delta) in [(sun, 20.0), (moon, 10.0), (asc, 10.0)] {
        if let Some(dim) = element(sign) {
            // 水象是 S 的負向調整(spec §1.3:水 -> S−;Codex [P2])
            sums[dim] += if dim == 3 { -delta } else { delta };
            counts[dim] += 1;
        }
    }
    std::array::from_fn(|i| (counts[i] > 0).then(|| sums[i] / counts[i] as f64))
}

use serde::{Deserialize, Serialize};

pub const BANNED_WORDS: &[&str] = &[
    "神經質",
    "憂鬱",
    "焦慮症",
    "診斷",
    "病態",
    "缺陷",
    "低能",
    "人格障礙",
    "違背天性",
    "壓抑本性",
];

pub fn narrative_ok(prior: Option<f64>, measured: f64) -> bool {
    match prior {
        None => false,
        Some(p) => (measured - p).abs() >= 20.0 && band_of(measured) != Band::Low,
    }
}

pub fn narrative_text(dim_index: usize, prior_band: Band, measured_band: Band) -> String {
    format!(
        "在{}上,命盤象徵落在{},你的實測落在{}。象徵是傳統對照,實測是你作答的結果;這段落差只是描述,不含評價,也不暗示該怎麼改。",
        DIM_LABELS[dim_index], prior_band.label(), measured_band.label()
    )
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DimOutcome {
    pub dim: String,
    pub measured: f64,
    pub prior: Option<f64>,
    /// 檔帶 enum code:low / mid_low / mid / mid_high / high
    pub prior_band: Option<String>,
    pub measured_band: String,
    pub gap: Option<f64>,
    pub narrative_ok: bool,
    /// 該維象徵規則依據彙總:classical | designer | mixed(prior 缺席 = None;
    /// Codex [14]:UI 來源行由此驅動)
    pub basis: Option<String>,
    pub text: Option<String>,
}

/// overlay 回應外殼 — API 與 web 共用此型別(Codex [22]:不重複宣告)。
/// 欄位用 String 而非 &'static str:serde 零拷貝限制下,含 &'static str 的
/// 結構體無法從 owned String 反序列化(Deserialize derive 編譯不過)。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OverlayResponse {
    pub ruleset_version: String,
    pub prior_source: Option<String>,
    pub birth_known: bool,
    pub dims: Vec<DimOutcome>,
}

pub fn dim_outcomes(measured: [f64; 5], prior: Option<&SymbolicVector>) -> Vec<DimOutcome> {
    DIM_CODES
        .iter()
        .enumerate()
        .map(|(i, &code)| {
            let m_band = band_of(measured[i]);
            let p = prior.map(|v| v.dims[i]);
            let ok = narrative_ok(p, measured[i]);
            DimOutcome {
                dim: code.to_string(),
                measured: measured[i],
                prior: p,
                prior_band: prior.map(|v| band_of(v.dims[i]).code().to_string()),
                measured_band: m_band.code().to_string(),
                gap: p.map(|v| (measured[i] - v).abs()),
                narrative_ok: ok,
                basis: prior.map(|v| v.dim_basis[i].to_string()),
                text: ok.then(|| {
                    let pb = prior.map(|v| band_of(v.dims[i])).unwrap_or(Band::Mid);
                    narrative_text(i, pb, m_band)
                }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ZiWeiPalaceV3, ZiWeiStarV3};

    fn star(name: &str) -> ZiWeiStarV3 {
        ZiWeiStarV3 {
            name: name.to_string(),
            star_type: "major".into(),
            brightness: None,
            sihua: None,
        }
    }
    fn palace(index: u8, stars: Vec<ZiWeiStarV3>) -> ZiWeiPalaceV3 {
        ZiWeiPalaceV3 {
            index,
            name: String::new(),
            branch: String::new(),
            stem: String::new(),
            stars,
            is_life_palace: None,
            is_body_palace: None,
        }
    }

    #[test]
    fn band_boundaries_are_exact() {
        assert_eq!(band_of(34.9), Band::Low);
        assert_eq!(band_of(35.0), Band::MidLow);
        assert_eq!(band_of(44.9), Band::MidLow);
        assert_eq!(band_of(45.0), Band::Mid);
        assert_eq!(band_of(55.0), Band::Mid);
        assert_eq!(band_of(55.1), Band::MidHigh);
        assert_eq!(band_of(65.0), Band::MidHigh);
        assert_eq!(band_of(65.1), Band::High);
    }

    #[test]
    fn band_codes_and_labels_are_paired() {
        for (b, code, label) in [
            (Band::Low, "low", "偏低"),
            (Band::MidLow, "mid_low", "略低"),
            (Band::Mid, "mid", "中等"),
            (Band::MidHigh, "mid_high", "略高"),
            (Band::High, "high", "偏高"),
        ] {
            assert_eq!(b.code(), code);
            assert_eq!(b.label(), label);
            assert_eq!(band_label(code), label);
        }
    }

    #[test]
    fn rule_table_values_are_on_grid_and_cover_all_stars() {
        const MAIN: [&str; 14] = [
            "紫微", "天機", "太陽", "武曲", "天同", "廉貞", "天府", "太陰", "貪狼", "巨門", "天相",
            "天梁", "七殺", "破軍",
        ];
        for (star, _dim, delta, _basis) in ZIWEI_RULES {
            assert!(MAIN.contains(star), "unknown star {star}");
            assert!(
                matches!(*delta, -20 | -10 | 10 | 20),
                "off-grid delta {delta}"
            );
        }
        for star in MAIN {
            assert!(
                ZIWEI_RULES.iter().any(|(s, _, _, _)| s == &star),
                "{star} missing"
            );
        }
        // 依據標註(spec §1.3 雙軸):以下 8 條是設計裁量,其餘古典(Codex [14])
        for (s, d) in [
            ("紫微", 0usize),
            ("紫微", 4),
            ("武曲", 0),
            ("廉貞", 3),
            ("天府", 0),
            ("太陰", 4),
            ("巨門", 0),
            ("七殺", 3),
        ] {
            assert!(
                ZIWEI_RULES
                    .iter()
                    .any(|(st, dim, _, b)| *st == s && *dim == d && *b == Basis::Designer),
                "{s}/{d} 應標 designer"
            );
        }
        assert!(ZIWEI_RULES
            .iter()
            .any(|(st, dim, _, b)| *st == "太陽" && *dim == 0 && *b == Basis::Classical));
    }

    #[test]
    fn life_stars_taken_from_life_palace_only() {
        let palaces = vec![
            palace(0, vec![star("紫微"), star("天府")]),
            palace(1, vec![star("貪狼")]),
        ];
        let got = ziwei_life_stars(&palaces, 0);
        assert_eq!(got, vec![("紫微", false), ("天府", false)]);
    }

    #[test]
    fn empty_life_palace_borrows_opposite() {
        let palaces = vec![palace(0, vec![]), palace(6, vec![star("太陽")])];
        let got = ziwei_life_stars(&palaces, 0);
        assert_eq!(got, vec![("太陽", true)]);
    }

    #[test]
    fn omitted_dims_do_not_enter_denominator() {
        // 紫微+天府:O 僅紫微 -10 命中 -> 40(非 45)。Codex [6] fixture。
        let v = symbolic_vector(
            Some((&[palace(0, vec![star("紫微"), star("天府")])], 0)),
            None,
        )
        .unwrap();
        assert_eq!(v.source, PriorSource::Ziwei);
        assert_eq!(v.dims[4], 40.0);
        // C:紫微+20、天府+20 平均 -> 70;E:紫微+10、天府-10 平均 -> 50
        assert_eq!(v.dims[2], 70.0);
        assert_eq!(v.dims[0], 50.0);
        // A 無命中 -> 50;S 僅天府 +10 -> 60(Codex [P1] 修正:原 fixture 算錯)
        assert_eq!(v.dims[1], 50.0);
        assert_eq!(v.dims[3], 60.0);
    }

    #[test]
    fn borrowed_pojun_all_dims_clamped() {
        // 借破軍:O+20→+10、C-20→-10、E+10→+10(Codex [17] fixture)
        let v = symbolic_vector(
            Some((&[palace(0, vec![]), palace(6, vec![star("破軍")])], 0)),
            None,
        )
        .unwrap();
        assert_eq!(v.dims[4], 60.0);
        assert_eq!(v.dims[2], 40.0);
        assert_eq!(v.dims[0], 60.0);
    }

    #[test]
    fn borrowed_star_clamped_per_rule_before_averaging() {
        // 借破軍(O+20 被 clamp 成 +10)-> O = 60,不是 70。
        let v = symbolic_vector(
            Some((&[palace(0, vec![]), palace(6, vec![star("破軍")])], 0)),
            None,
        )
        .unwrap();
        assert_eq!(v.dims[4], 60.0);
    }

    #[test]
    fn no_charts_yields_none() {
        assert!(symbolic_vector(None, None).is_none());
    }

    #[test]
    fn western_element_rules_with_weights() {
        // sun=牡羊(火,E+20) moon=Cancer(水,S-10) asc=Leo(火,E+10)
        let v = symbolic_vector(None, Some(("牡羊座", "Cancer", "Leo"))).unwrap();
        assert_eq!(v.source, PriorSource::Western);
        assert_eq!(v.dims[0], 65.0); // E:(20+10)/2=15 -> 65
        assert_eq!(v.dims[3], 40.0); // S:-10 -> 40
        assert_eq!(v.dims[1], 50.0); // 無命中 -> 50
    }

    #[test]
    fn ziwei_wins_when_both_present() {
        let p = [palace(0, vec![star("紫微")])];
        let v = symbolic_vector(Some((&p, 0)), Some(("Aries", "Aries", "Aries"))).unwrap();
        assert_eq!(v.source, PriorSource::Ziwei);
        assert_eq!(v.dims[2], 70.0); // 紫微 C+20,不是西洋土象的值
    }

    #[test]
    fn english_sign_names_match_too() {
        let v = symbolic_vector(None, Some(("Capricorn", "Virgo", "Taurus"))).unwrap();
        assert!((v.dims[2] - (50.0 + 40.0 / 3.0)).abs() < 1e-9); // C 三條命中平均 13.33
    }

    #[test]
    fn all_12_signs_in_all_three_bodies_are_recognized() {
        const SIGNS: [&str; 12] = [
            "Aries",
            "Taurus",
            "Gemini",
            "Cancer",
            "Leo",
            "Virgo",
            "Libra",
            "Scorpio",
            "Sagittarius",
            "Capricorn",
            "Aquarius",
            "Pisces",
        ];
        for s in SIGNS {
            for (sun, moon, asc) in [
                (s, "Aries", "Aries"),
                ("Aries", s, "Aries"),
                ("Aries", "Aries", s),
            ] {
                let v = symbolic_vector(None, Some((sun, moon, asc))).unwrap();
                // 元素必命中 E/C/S/O 之一,故必有一維偏離 50(Codex [17] 覆蓋)
                assert!(
                    v.dims.iter().any(|&d| (d - 50.0).abs() > f64::EPSILON),
                    "{s} unmapped"
                );
            }
        }
    }

    #[test]
    fn dim_basis_mixed_and_designer() {
        // 紫微單星:E/O designer、C classical(Codex [14] fixture)
        let v = symbolic_vector(Some((&[palace(0, vec![star("紫微")])], 0)), None).unwrap();
        assert_eq!(v.dim_basis[0], "designer");
        assert_eq!(v.dim_basis[4], "designer");
        assert_eq!(v.dim_basis[2], "classical");
        // 紫微+太陽同宮:E 一 designer 一 classical -> mixed
        let v = symbolic_vector(
            Some((&[palace(0, vec![star("紫微"), star("太陽")])], 0)),
            None,
        )
        .unwrap();
        assert_eq!(v.dim_basis[0], "mixed");
    }

    #[test]
    fn narrative_gate_is_absolute_both_directions_and_exempt_low_band() {
        assert!(narrative_ok(Some(60.0), 40.0)); // |40-60|=20 過門檻
        assert!(narrative_ok(Some(40.0), 60.0)); // 反方向同樣過
        assert!(!narrative_ok(Some(50.0), 39.9)); // 10.1 < 20
        assert!(!narrative_ok(Some(69.9), 50.0)); // 19.9 < 20(邊界下;Codex [17])
        assert!(narrative_ok(Some(70.0), 50.0)); // 20.0 恰過(反方向)
        assert!(!narrative_ok(Some(50.0), 30.0)); // gap 20 但實測偏低檔 -> 豁免(Codex [5])
        assert!(!narrative_ok(None, 10.0)); // F2 缺席 -> 不敘事
    }

    #[test]
    fn dim_outcomes_wire_shape_and_texts() {
        let prior = SymbolicVector {
            ruleset_version: SYMBOLIC_RULES_VERSION,
            source: PriorSource::Ziwei,
            dim_basis: ["classical"; 5],
            dims: [60.0, 50.0, 50.0, 50.0, 50.0],
        };
        let out = dim_outcomes([40.0, 50.0, 50.0, 50.0, 50.0], Some(&prior));
        assert_eq!(out.len(), 5);
        assert_eq!(out[0].dim, "extraversion");
        assert_eq!(out[0].gap, Some(20.0));
        assert!(out[0].narrative_ok);
        assert!(out[0]
            .text
            .as_deref()
            .unwrap_or("")
            .contains("命盤象徵落在"));
        assert!(out[0].text.as_deref().unwrap_or("").contains("略高")); // 檔帶措詞,非二分高低
        assert!(!out[1].narrative_ok);
        assert!(out[1].text.is_none());
        assert!(out[1].gap.unwrap().abs() < f64::EPSILON);
        // F2 缺席:prior/gap/priorBand 全 null、全不敘事
        let none = dim_outcomes([40.0, 50.0, 50.0, 50.0, 50.0], None);
        assert!(none.iter().all(|d| d.text.is_none() && d.prior.is_none()));
    }

    #[test]
    fn dim_outcome_serializes_camelcase_with_band_codes() {
        let prior = SymbolicVector {
            ruleset_version: SYMBOLIC_RULES_VERSION,
            source: PriorSource::Ziwei,
            dim_basis: ["classical"; 5],
            dims: [40.0; 5],
        };
        let out = dim_outcomes([70.0, 50.0, 50.0, 50.0, 50.0], Some(&prior));
        let json: serde_json::Value = serde_json::to_value(&out).unwrap();
        assert_eq!(json[0]["dim"], "extraversion");
        assert_eq!(json[0]["priorBand"], "mid_low"); // prior 40
        assert_eq!(json[0]["measuredBand"], "high"); // measured 70
        assert_eq!(json[0]["gap"], 30.0); // |70-40|;計畫原 fixture 誤植 20(Codex 審後殘留)
        assert_eq!(json[0]["narrativeOk"], true);
        assert_eq!(json[0]["basis"], "classical");
        assert_eq!(json[1]["measuredBand"], "mid"); // measured 50
        assert_eq!(json[1]["narrativeOk"], false);
        assert!(json[1]["text"].is_null());
    }

    #[test]
    fn generated_text_never_contains_banned_words() {
        // 極端組合:每維都產生敘事,掃禁用詞(spec §2.4)
        let prior = SymbolicVector {
            ruleset_version: SYMBOLIC_RULES_VERSION,
            source: PriorSource::Ziwei,
            dim_basis: ["classical"; 5],
            dims: [30.0; 5],
        };
        let out = dim_outcomes([70.0, 70.0, 70.0, 70.0, 70.0], Some(&prior));
        for d in &out {
            if let Some(t) = &d.text {
                for w in BANNED_WORDS {
                    assert!(!t.contains(w), "{w} appeared in: {t}");
                }
            }
        }
    }
}
