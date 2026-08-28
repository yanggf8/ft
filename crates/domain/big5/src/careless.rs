//! 亂答偵測三訊號（spec §7；designer 判斷，門檻集中此處，上線後依 1%–15% 雙側觸發率校準）：
//! 1. 總作答時長過短（< 20 秒 ≈ 1.3 秒/題）
//! 2. 全部同一選項（任何值，rev.4 原文語意；偶然機率 (1/5)^14）
//! 3. 同維端點衝突（同維三題全距 ≥ 4 ＝該維同時出現 1 與 5）。原 rev.4「正反題矛盾」在
//!    IPIP-15 每維 3 題同向的結構下無正反配對可檢（spec K5 修正）；且誠實作答在**單一**維
//!    偶發 1+5（智性/想像面向分裂、情緒穩定性次構面異質）並非亂答——Grok 對抗審 #1：
//!    僅當 **≥2 維**同時端點衝突才算亂答（均勻亂點 P(≥2)≈25%；誠實多維 1+5 極罕）。
//!    循環／中間偏作答抓不到；「低變異訊號」（近全同但非整條 straight-line）列為上線後
//!    依 1%–15% 觸發率校準的候選。與 F5 anchor_coverage「全距 ≥2 → low」層級分明（≥2 僅
//!    降級覆蓋，≥4 才算亂答級）。

use ft_schema::items::ITEMS;

use crate::scoring::{NUM_ITEMS, SCALE_MAX, SCALE_MIN};

/// 總作答時長門檻（ms）。
pub const MIN_TOTAL_MS: u64 = 20_000;
/// 同維三題全距觸發值（量表點）。
pub const MAX_DIMENSION_RANGE: u8 = 4;
/// 認定亂答所需的最低「端點衝突維數」（Grok 對抗審 #1：單維端點衝突＝面向分裂，非亂答）。
pub const MIN_INCONSISTENT_DIMS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CarelessFlags {
    pub too_fast: bool,
    pub straight_lining: bool,
    pub inconsistent: bool,
}

/// 任一訊號觸發即亂答嫌疑（spec：觸發 → 提示重測一次）。
pub fn any_triggered(f: &CarelessFlags) -> bool {
    f.too_fast || f.straight_lining || f.inconsistent
}

/// 回傳「同維三題全距 ≥4（端點衝突）」的所有維度（0-based dim），供 per-dim 校準日誌。
/// 索引規則與 scoring::score 一致：enumerate 序 + debug_assert 釘死（Grok #8）。
pub fn inconsistent_dims(answers: &[u8]) -> Vec<usize> {
    if answers.len() != NUM_ITEMS {
        return Vec::new();
    }
    debug_assert!(ITEMS.iter().enumerate().all(|(i, it)| it.no == i + 1));
    (0..5)
        .filter(|&dim| {
            let vals: Vec<u8> = ITEMS
                .iter()
                .enumerate()
                .filter(|(_, i)| i.dimension == dim)
                .map(|(idx, _)| answers[idx])
                .collect();
            let max = vals.iter().copied().max().unwrap_or(SCALE_MIN);
            let min = vals.iter().copied().min().unwrap_or(SCALE_MAX);
            max.saturating_sub(min) >= MAX_DIMENSION_RANGE
        })
        .collect()
}

pub fn detect_careless(duration_ms: u64, answers: &[u8]) -> CarelessFlags {
    let too_fast = duration_ms < MIN_TOTAL_MS;
    let straight_lining = answers.len() == NUM_ITEMS && answers.windows(2).all(|w| w[0] == w[1]);
    // 單維端點衝突（面向分裂）不列亂答；≥2 維同時衝突才計（Grok 對抗審 #1）。
    let inconsistent = inconsistent_dims(answers).len() >= MIN_INCONSISTENT_DIMS;
    CarelessFlags {
        too_fast,
        straight_lining,
        inconsistent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_threshold_boundary() {
        assert!(detect_careless(19_999, &[3; 15]).too_fast);
        assert!(!detect_careless(20_000, &[3; 15]).too_fast);
    }

    #[test]
    fn straight_lining_catches_any_value() {
        for v in 1..=5u8 {
            assert!(detect_careless(60_000, &[v; 15]).straight_lining, "v={v}");
        }
        assert!(
            !detect_careless(60_000, &[1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5])
                .straight_lining
        );
    }

    #[test]
    fn dimension_range_requires_two_dims() {
        // 單一維端點衝突（面向分裂，誠實作答即有）→ 不算亂答
        let mut single = [3u8; 15];
        single[0] = 1; // dim0（外向性）{1,5,3} 全距 4，但只有單維
        single[1] = 5;
        assert!(!detect_careless(60_000, &single).inconsistent);
        // ≥2 維同時端點衝突 → 才算亂答
        let mut two = [3u8; 15];
        two[0] = 1; // dim0 {1,5,3} 全距 4
        two[1] = 5;
        two[3] = 1; // dim1（友善性，items 4-6）{1,5,3} 全距 4
        two[4] = 5;
        assert!(detect_careless(60_000, &two).inconsistent);
    }

    #[test]
    fn inconsistent_dims_lists_offending_dims() {
        let mut a = [3u8; 15];
        a[0] = 1; // dim0 全距 4
        a[1] = 5;
        a[9] = 1; // dim3（情緒穩定，items 10-12 反向）{1,3,5} 全距 4
        a[11] = 5;
        assert_eq!(inconsistent_dims(&a), vec![0, 3]);
        assert_eq!(inconsistent_dims(&[3; 15]), Vec::<usize>::new());
    }

    #[test]
    fn single_dim_endpoint_is_not_careless_even_on_heterogeneous() {
        // 智性／想像（dim 4，題目異質、誠實作答也可能 1+5——Grok #7/#1：單維豁免）
        let mut im = [3u8; 15];
        im[12] = 1;
        im[14] = 5;
        assert!(!detect_careless(60_000, &im).inconsistent);
        assert_eq!(inconsistent_dims(&im), vec![4]);
    }

    #[test]
    fn any_triggered_is_union() {
        assert!(any_triggered(&CarelessFlags {
            too_fast: true,
            ..Default::default()
        }));
        assert!(any_triggered(&CarelessFlags {
            straight_lining: true,
            ..Default::default()
        }));
        assert!(any_triggered(&CarelessFlags {
            inconsistent: true,
            ..Default::default()
        }));
        assert!(!any_triggered(&CarelessFlags::default()));
    }
}
