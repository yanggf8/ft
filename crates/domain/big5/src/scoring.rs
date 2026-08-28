//! IPIP-15 計分：反向題翻轉（5 點即 6−x）→ 每維 3 題加總（原始分 3–15）
//! → (raw − 3) × 25 / 3 → 0–100。情緒穩定性維以正向命名（不使用「神經質」）。

use ft_schema::api::OceanScores;
use ft_schema::items::ITEMS;

pub const NUM_ITEMS: usize = 15;
pub const SCALE_MIN: u8 = 1;
pub const SCALE_MAX: u8 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationErr {
    WrongLength { got: usize },
    OutOfRange { index: usize, value: u8 },
}

impl std::fmt::Display for ValidationErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationErr::WrongLength { got } => {
                write!(f, "expected {} answers, got {}", NUM_ITEMS, got)
            }
            ValidationErr::OutOfRange { index, value } => {
                write!(
                    f,
                    "answer at index {} is {}, must be {}–{}",
                    index, value, SCALE_MIN, SCALE_MAX
                )
            }
        }
    }
}

pub fn validate(answers: &[u8]) -> Result<(), ValidationErr> {
    if answers.len() != NUM_ITEMS {
        return Err(ValidationErr::WrongLength { got: answers.len() });
    }
    for (i, v) in answers.iter().enumerate() {
        if !(SCALE_MIN..=SCALE_MAX).contains(v) {
            return Err(ValidationErr::OutOfRange {
                index: i,
                value: *v,
            });
        }
    }
    Ok(())
}

/// 反向題翻轉（6−x）→ 每維 3 題加總 raw 3–15 → (raw − 3) × 25 / 3。
/// 題目順序即 answers 陣列順序（enumerate 索引；與 detect_careless 同一規則，
/// 並以 debug_assert 釘死 ITEMS[i].no == i+1，防 ITEMS 重排時兩處看不同格——Grok #8）。
/// 呼叫端必須先 `validate`（route 層已前置）；domain 層對短 slice 直接 panic（release
/// `panic=abort` 下 isolate 重啟＝顯式 500），不做靜默容錯——Grok 二審 R2-21 註記。
pub fn score(answers: &[u8]) -> OceanScores {
    debug_assert_eq!(answers.len(), NUM_ITEMS);
    debug_assert!(ITEMS.iter().enumerate().all(|(i, it)| it.no == i + 1));
    let mut sums = [0u32; 5];
    for (i, item) in ITEMS.iter().enumerate() {
        let v = answers[i];
        let v = if item.reverse { 6 - v } else { v };
        sums[item.dimension] += u32::from(v);
    }
    let to100 = |raw: u32| (f64::from(raw) - 3.0) * 25.0 / 3.0;
    OceanScores {
        extraversion: to100(sums[0]),
        agreeableness: to100(sums[1]),
        conscientiousness: to100(sums[2]),
        emotionalStability: to100(sums[3]),
        intellectImagination: to100(sums[4]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_wrong_length() {
        assert_eq!(
            validate(&[3; 14]),
            Err(ValidationErr::WrongLength { got: 14 })
        );
        assert!(validate(&[3; 15]).is_ok());
    }

    #[test]
    fn validate_rejects_out_of_range() {
        let mut a = [3u8; 15];
        a[4] = 0;
        assert_eq!(
            validate(&a),
            Err(ValidationErr::OutOfRange { index: 4, value: 0 })
        );
        a[4] = 6;
        assert_eq!(
            validate(&a),
            Err(ValidationErr::OutOfRange { index: 4, value: 6 })
        );
    }

    #[test]
    fn reverse_items_are_exactly_ten_to_twelve() {
        let reversed: Vec<usize> = ITEMS.iter().filter(|i| i.reverse).map(|i| i.no).collect();
        assert_eq!(reversed, vec![10, 11, 12]);
        assert!(ITEMS.iter().all(|i| i.dimension <= 4));
    }

    #[test]
    fn fixed_pattern_matches_hand_computed() {
        // [5,5,5 | 4,4,4 | 3,3,3 | 2,2,2 | 1,1,1]
        // E raw=15→100；A 12→75；C 9→50；ES 三題反向 2→4、raw=12→75；I 3→0
        let o = score(&[5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 1, 1, 1]);
        assert_eq!(o.extraversion, 100.0);
        assert_eq!(o.agreeableness, 75.0);
        assert_eq!(o.conscientiousness, 50.0);
        assert_eq!(o.emotionalStability, 75.0);
        assert_eq!(o.intellectImagination, 0.0);
    }

    #[test]
    fn score_range_is_exact_0_to_100() {
        let lo = score(&[1; 15]);
        let hi = score(&[5; 15]);
        assert_eq!(lo.extraversion, 0.0); // 正向維全 1
        assert_eq!(lo.emotionalStability, 100.0); // 反向維全 1 → 翻轉全 5
        assert_eq!(hi.extraversion, 100.0);
        assert_eq!(hi.emotionalStability, 0.0);
    }

    /// ITEMS 必須稠密 1-based——score/detect 的 enumerate 索引依賴此不變式
    /// （debug_assert 在 release 被剝，用測試鎖住；Grok 二審 R2-15）。
    #[test]
    fn items_are_dense_1_based() {
        assert_eq!(ITEMS.len(), NUM_ITEMS);
        for (i, it) in ITEMS.iter().enumerate() {
            assert_eq!(it.no, i + 1, "ITEMS[{}] no={} must be i+1", i, it.no);
        }
    }
}
