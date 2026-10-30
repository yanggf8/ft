//! 興趣探索 v1 — RIASEC 興趣剖面純函式（設計：
//! docs/superpowers/specs/2026-10-27-interest-explore-design.md）。
//! 量表：Armstrong, Allison & Rounds (2008) RIASEC Markers，Activities Set A，48 題。
//! 無 IO、native 與 wasm32 皆可編；API 端以此重算分數後才寫入（不信任前端分數）。
//!
//! 紅線：只呈現使用者自己六型之間的相對高低——不給常模、百分位或他人比較；
//! 興趣資料永不寫入 predictions／prediction_feedback／f8_assignments。

pub mod items;

use serde::{Deserialize, Serialize};

pub use items::{InterestItem, ANSWER_ANCHORS, FORM, INTEREST_INSTRUMENT_VERSION, ITEMS};

/// 題數（spec §2.2：48 題全保留，不自行刪題）。
pub const ITEM_COUNT: usize = 48;

/// 每型題數；answers 依附錄 A.4 題號排列，第 t 型佔索引 `t*8 .. t*8+8`。
pub const ITEMS_PER_TYPE: usize = 8;

/// 「較明顯的方向」候選：與最高分差距 ≤ 此值。產品門檻，非心理計量標準（spec §2.6）。
pub const DIRECTION_GAP: f64 = 0.5;

/// 最高分低於此值即「目前沒有特別突出的偏好」，不選方向。產品門檻，非心理計量標準。
pub const DIRECTION_MIN_TOP: f64 = 3.0;

/// 方向最多取幾型。
pub const MAX_DIRECTIONS: usize = 3;

/// 六型，宣告順序即結果頁固定的 R-I-A-S-E-C 順序（也是同分時的取序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiasecType {
    R,
    I,
    A,
    S,
    E,
    C,
}

impl RiasecType {
    pub const ALL: [RiasecType; 6] = [
        RiasecType::R,
        RiasecType::I,
        RiasecType::A,
        RiasecType::S,
        RiasecType::E,
        RiasecType::C,
    ];

    /// wire 值（scores JSON 的鍵，例如 `"R"`）。
    pub fn code(&self) -> &'static str {
        match self {
            RiasecType::R => "R",
            RiasecType::I => "I",
            RiasecType::A => "A",
            RiasecType::S => "S",
            RiasecType::E => "E",
            RiasecType::C => "C",
        }
    }
}

/// answers 形狀驗證：恰 48 個、每個 1..=5（spec §2.3：必須全答）。
pub fn valid_answers(answers: &[u8]) -> bool {
    answers.len() == ITEM_COUNT && answers.iter().all(|a| (1..=5).contains(a))
}

/// 每型取 8 題平均（1.0–5.0），依 `RiasecType::ALL` 順序；形狀不合回 `None`。
pub fn score(answers: &[u8]) -> Option<[f64; 6]> {
    if !valid_answers(answers) {
        return None;
    }
    let mut scores = [0.0; 6];
    for (t, chunk) in answers.chunks(ITEMS_PER_TYPE).enumerate() {
        let sum: u32 = chunk.iter().map(|&a| a as u32).sum();
        scores[t] = sum as f64 / ITEMS_PER_TYPE as f64;
    }
    Some(scores)
}

/// 較明顯的方向（spec §2.6）：最高分 < 3.0 回空；否則候選＝與最高分差距 ≤ 0.5 者，
/// 依分數降冪、同分依 R-I-A-S-E-C 序，取前 3。
///
/// 平均值皆為 1/8 的倍數（二進位可精確表示），故 `≤ 0.5` 的邊界比較無浮點誤差。
pub fn directions(scores: &[f64; 6]) -> Vec<RiasecType> {
    let top = scores.iter().cloned().fold(f64::MIN, f64::max);
    if top < DIRECTION_MIN_TOP {
        return Vec::new();
    }
    let mut candidates: Vec<usize> = (0..6)
        .filter(|&t| top - scores[t] <= DIRECTION_GAP)
        .collect();
    // sort_by 為穩定排序：同分保留 R-I-A-S-E-C 原序
    candidates.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
    candidates
        .into_iter()
        .take(MAX_DIRECTIONS)
        .map(|t| RiasecType::ALL[t])
        .collect()
}

/// 六型分數 wire 形狀：`{"R":3.25,"I":…}`（spec §4.1 `scores` 欄、§4.2 回應）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InterestScores {
    #[serde(rename = "R")]
    pub r: f64,
    #[serde(rename = "I")]
    pub i: f64,
    #[serde(rename = "A")]
    pub a: f64,
    #[serde(rename = "S")]
    pub s: f64,
    #[serde(rename = "E")]
    pub e: f64,
    #[serde(rename = "C")]
    pub c: f64,
}

impl InterestScores {
    /// 由 `score()` 的 R-I-A-S-E-C 陣列建立。
    pub fn from_array(scores: [f64; 6]) -> Self {
        let [r, i, a, s, e, c] = scores;
        InterestScores { r, i, a, s, e, c }
    }

    /// 轉回 R-I-A-S-E-C 陣列（供 `directions()`）。
    pub fn to_array(&self) -> [f64; 6] {
        [self.r, self.i, self.a, self.s, self.e, self.c]
    }
}

/// 一筆興趣結果（**不含 answers**——原始作答只存 DB 供改版重算，永不上 wire）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestResult {
    pub id: String,
    pub instrument_version: String,
    pub form: String,
    pub scores: InterestScores,
    pub created_at: String,
}

/// `GET /api/interest`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestOverview {
    pub interest_epoch: i64,
    pub total_results: u32,
    pub latest: Option<InterestResult>,
}

/// `GET /api/interest/history`（照 practice history 契約）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterestHistoryPage {
    pub results: Vec<InterestResult>,
    pub next_cursor: Option<String>,
}

/// `POST /api/interest/results`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInterestResultRequest {
    pub instrument_version: String,
    pub form: String,
    pub answers: Vec<u8>,
    pub expected_epoch: i64,
    pub request_id: String,
}

#[cfg(test)]
mod tests {
    use super::RiasecType::*;
    use super::*;

    /// 每型 8 題同分的作答（依 R-I-A-S-E-C 順序給六個值）。
    fn answers_by_type(per_type: [u8; 6]) -> Vec<u8> {
        per_type
            .iter()
            .flat_map(|&v| std::iter::repeat_n(v, ITEMS_PER_TYPE))
            .collect()
    }

    #[test]
    fn codes_follow_riasec_order() {
        let codes: Vec<&str> = RiasecType::ALL.iter().map(|t| t.code()).collect();
        assert_eq!(codes, ["R", "I", "A", "S", "E", "C"]);
    }

    #[test]
    fn answers_must_be_exactly_48() {
        assert!(valid_answers(&[3; 48]));
        assert!(!valid_answers(&[3; 47]));
        assert!(!valid_answers(&[3; 49]));
        assert!(!valid_answers(&[]));
    }

    #[test]
    fn answers_must_be_within_1_to_5() {
        let mut a = vec![3u8; 48];
        a[47] = 0;
        assert!(!valid_answers(&a));
        a[47] = 6;
        assert!(!valid_answers(&a));
        a[47] = 1;
        assert!(valid_answers(&a));
        a[0] = 5;
        assert!(valid_answers(&a));
    }

    #[test]
    fn score_all_ones_and_all_fives() {
        assert_eq!(score(&[1; 48]), Some([1.0; 6]));
        assert_eq!(score(&[5; 48]), Some([5.0; 6]));
    }

    #[test]
    fn score_single_type_high() {
        let a = answers_by_type([1, 1, 1, 5, 1, 1]);
        assert_eq!(score(&a), Some([1.0, 1.0, 1.0, 5.0, 1.0, 1.0]));
    }

    #[test]
    fn score_is_per_type_mean() {
        // R 型八題 1..=5 混合：(1+2+3+4+5+5+4+2)/8 = 26/8 = 3.25
        let mut a = vec![1u8; 48];
        a[..8].copy_from_slice(&[1, 2, 3, 4, 5, 5, 4, 2]);
        assert_eq!(score(&a).unwrap()[0], 3.25);
    }

    #[test]
    fn score_rejects_invalid_answers() {
        assert_eq!(score(&[3; 47]), None);
        assert_eq!(score(&[0; 48]), None);
    }

    #[test]
    fn no_direction_when_top_below_3() {
        assert!(directions(&[2.875, 1.0, 1.0, 1.0, 1.0, 1.0]).is_empty());
    }

    #[test]
    fn top_exactly_3_still_selects() {
        assert_eq!(directions(&[3.0, 1.0, 1.0, 1.0, 1.0, 1.0]), vec![R]);
    }

    #[test]
    fn gap_exactly_half_is_included() {
        assert_eq!(directions(&[1.0, 4.0, 3.5, 3.375, 1.0, 1.0]), vec![I, A]);
    }

    #[test]
    fn candidates_sorted_by_score_desc() {
        assert_eq!(
            directions(&[4.0, 4.25, 1.0, 1.0, 4.125, 1.0]),
            vec![I, E, R]
        );
    }

    #[test]
    fn ties_break_by_riasec_order() {
        assert_eq!(directions(&[1.0, 1.0, 4.0, 1.0, 4.0, 4.0]), vec![A, E, C]);
    }

    #[test]
    fn top_3_with_fourth_candidate_at_exact_gap_takes_only_three() {
        // §5.3：最高分 3.0 且第四候選差距恰 0.5 → 仍只取前 3
        assert_eq!(directions(&[2.5, 3.0, 2.75, 2.5, 2.5, 1.0]), vec![I, A, R]);
    }

    #[test]
    fn scores_wire_keys_are_riasec_codes() {
        let s = InterestScores::from_array([1.0, 2.0, 3.0, 4.0, 5.0, 3.25]);
        let v = serde_json::to_value(s).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"R":1.0,"I":2.0,"A":3.0,"S":4.0,"E":5.0,"C":3.25})
        );
        assert_eq!(s.to_array(), [1.0, 2.0, 3.0, 4.0, 5.0, 3.25]);
    }

    #[test]
    fn result_wire_never_carries_answers() {
        let r = InterestResult {
            id: "x".into(),
            instrument_version: INTEREST_INSTRUMENT_VERSION.into(),
            form: FORM.into(),
            scores: InterestScores::from_array([1.0; 6]),
            created_at: "2026-10-30T00:00:00.000Z".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(|k| k.as_str()).collect();
        keys.sort();
        assert_eq!(
            keys,
            ["createdAt", "form", "id", "instrumentVersion", "scores"]
        );
    }

    #[test]
    fn overview_and_request_use_camel_case() {
        let o = InterestOverview {
            interest_epoch: 2,
            total_results: 0,
            latest: None,
        };
        assert_eq!(
            serde_json::to_value(&o).unwrap(),
            serde_json::json!({"interestEpoch":2,"totalResults":0,"latest":null})
        );
        let req: CreateInterestResultRequest = serde_json::from_value(serde_json::json!({
            "instrumentVersion":"1","form":"A","answers":[3,3],"expectedEpoch":0,
            "requestId":"123e4567-e89b-42d3-a456-426614174000"
        }))
        .unwrap();
        assert_eq!(req.answers, vec![3, 3]);
        assert_eq!(req.expected_epoch, 0);
    }
}
