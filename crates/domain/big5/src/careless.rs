//! 亂答偵測三訊號（spec §7；designer 判斷，門檻集中此處，上線後依 1%–15% 雙側觸發率校準）：
//! 1. 總作答時長過短（< 20 秒 ≈ 1.3 秒/題）
//! 2. 全部同一選項（任何值，rev.4 原文語意；偶然機率 (1/5)^14）
//! 3. 維內極端不一致（同維三題全距 ≥ 4 量表點）——原 rev.4「正反題矛盾」在 IPIP-15
//!    每維 3 題同向的題目結構下無正反配對可檢（spec K5 修正）。性質是**端點衝突**
//!    （同維 1 與 5 並存），非語意矛盾的等價物——循環／中間偏作答抓不到；「低變異
//!    訊號」（近全同但非整條 straight-line）列為上線後依 1%–15% 觸發率校準的候選。
//!    與 F5 anchor_coverage「全距 ≥2 → low」層級分明（≥2 僅降級覆蓋，≥4 才算亂答級）。

use ft_schema::items::ITEMS;

use crate::scoring::{NUM_ITEMS, SCALE_MAX, SCALE_MIN};

/// 總作答時長門檻（ms）。
pub const MIN_TOTAL_MS: u64 = 20_000;
/// 同維三題全距觸發值（量表點）。
pub const MAX_DIMENSION_RANGE: u8 = 4;

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

pub fn detect_careless(duration_ms: u64, answers: &[u8]) -> CarelessFlags {
    let too_fast = duration_ms < MIN_TOTAL_MS;
    let straight_lining = answers.len() == NUM_ITEMS && answers.windows(2).all(|w| w[0] == w[1]);
    // 索引規則與 scoring::score 一致：enumerate 序 + debug_assert 釘死（Grok #8）。
    debug_assert!(ITEMS.iter().enumerate().all(|(i, it)| it.no == i + 1));
    let inconsistent = answers.len() == NUM_ITEMS
        && (0..5).any(|dim| {
            let vals: Vec<u8> = ITEMS
                .iter()
                .enumerate()
                .filter(|(_, i)| i.dimension == dim)
                .map(|(idx, _)| answers[idx])
                .collect();
            let max = vals.iter().copied().max().unwrap_or(SCALE_MIN);
            let min = vals.iter().copied().min().unwrap_or(SCALE_MAX);
            max.saturating_sub(min) >= MAX_DIMENSION_RANGE
        });
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
    fn dimension_range_threshold() {
        // 外向性三題 [1,5,3] 全距 4 → 觸發
        let mut a = [3u8; 15];
        a[0] = 1;
        a[1] = 5;
        a[2] = 3;
        assert!(detect_careless(60_000, &a).inconsistent);
        // 全距 3 → 不觸發
        let mut b = [3u8; 15];
        b[0] = 1;
        b[2] = 4;
        assert!(!detect_careless(60_000, &b).inconsistent);
    }

    #[test]
    fn dimension_range_detects_on_es_and_intellect() {
        // 情緒穩定性（dim 3，三題皆反向）：全距 4 → 觸發
        let mut es = [3u8; 15];
        es[9] = 1;
        es[11] = 5;
        assert!(detect_careless(60_000, &es).inconsistent);
        // 智性／想像（dim 4，題目異質、誠實作答也可能 1+5——Grok #7：誤傷集中此維，
        // 上線後觸發率指標需分維記錄）
        let mut im = [3u8; 15];
        im[12] = 1;
        im[14] = 5;
        assert!(detect_careless(60_000, &im).inconsistent);
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
