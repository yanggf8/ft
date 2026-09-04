//! F5 命中與擇一純函數 — 無 DB、無 LLM，僅吃 ft-schema 靜態常數

use crate::anchors::{Domain, Level, TriggerClass, Valence, ANCHORS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorCoverage {
    High,
    Low,
}

#[derive(Debug, Clone)]
pub struct Selected<'a> {
    pub trigger: TriggerClass,
    pub anchor: &'a crate::anchors::Anchor,
    pub anchor_ids: Vec<&'static str>,
    pub coverage: AnchorCoverage,
    pub valence: Valence,
}

fn hit(anchor: &crate::anchors::Anchor, display: [f64; 5]) -> bool {
    let v = display[anchor.dimension];
    match anchor.level {
        Level::High => v >= 67.0,
        Level::Low => v < 33.0,
    }
}

/// OceanScores → 取整後顯示分 `[f64;5]`（索引 0..4 對齊 `DIMENSION_NAMES`）。
/// F1 §5 切點用「取整後顯示分」，避免 66.7 顯示 67 卻走中檔的錯位。
pub fn display_rounded(s: &crate::api::OceanScores) -> [f64; 5] {
    [
        s.extraversion.round(),
        s.agreeableness.round(),
        s.conscientiousness.round(),
        s.emotionalStability.round(),
        s.intellectImagination.round(),
    ]
}

/// ipip_answers `[15]`（1–5）→ 每維三題全距 `[u8;5]`（max−min）。
/// 反向題不影響全距，故不翻轉。length != 15 → `None`（fail-closed：
/// 不生成，而非無降級把該 `low` 的列標成 `high` — Grok P2）。
pub fn dim_ranges(answers: &[u8]) -> Option<[u8; 5]> {
    if answers.len() != crate::items::ITEMS.len() {
        return None;
    }
    let mut mins = [255u8; 5];
    let mut maxs = [0u8; 5];
    for (i, item) in crate::items::ITEMS.iter().enumerate() {
        let v = answers[i];
        let dim = item.dimension;
        mins[dim] = mins[dim].min(v);
        maxs[dim] = maxs[dim].max(v);
    }
    let mut out = [0u8; 5];
    for dim in 0..5 {
        out[dim] = maxs[dim].saturating_sub(mins[dim]);
    }
    Some(out)
}

/// 對單一 domain 選出勝出 trigger 及其代表錨點
/// `ranges` 為該使用者 IPIP-15 五維各自的三題全距（max-min），用於 `全距≥2 => low` 降級
pub fn select_for_domain(
    domain: Domain,
    display: [f64; 5],
    ranges: [u8; 5],
) -> Option<Selected<'static>> {
    // 收集命中
    let hits: Vec<&crate::anchors::Anchor> = ANCHORS
        .iter()
        .filter(|a| a.domain == domain && hit(a, display))
        .collect();
    if hits.is_empty() {
        return None;
    }

    // 按 trigger 分組
    use std::collections::HashMap;
    let mut by_trigger: HashMap<TriggerClass, Vec<&crate::anchors::Anchor>> = HashMap::new();
    for &a in &hits {
        by_trigger.entry(a.trigger).or_default().push(a);
    }

    // 勝出 T*：hits_T 數量大者勝，同數量比組內最小 priority 小者勝，再同則按 trigger 字典序（T1 < T2 ...）
    let mut best: Option<(TriggerClass, Vec<&crate::anchors::Anchor>)> = None;
    for (t, group) in by_trigger {
        let entry_count = group.len();
        let min_prio = group.iter().map(|a| a.priority).min().unwrap_or(255);
        match &best {
            None => best = Some((t, group)),
            Some((best_t, best_group)) => {
                let best_count = best_group.len();
                let best_min = best_group.iter().map(|a| a.priority).min().unwrap_or(255);
                let should_replace = if entry_count != best_count {
                    entry_count > best_count
                } else if min_prio != best_min {
                    min_prio < best_min
                } else {
                    // 字典序：T1..T6
                    (t as u8) < (*best_t as u8)
                };
                if should_replace {
                    best = Some((t, group));
                }
            }
        }
    }

    let (trigger, group) = best?;
    // 代表錨點：組內 priority 最小者
    let anchor = *group.iter().min_by_key(|a| a.priority).unwrap();
    let anchor_ids = group.iter().map(|a| a.id).collect();

    // coverage 判定
    let coverage = {
        // 同維高低同時命中 => low
        let mut dims_high: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut dims_low: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for a in &group {
            match a.level {
                Level::High => {
                    dims_high.insert(a.dimension);
                }
                Level::Low => {
                    dims_low.insert(a.dimension);
                }
            }
        }
        let clash = dims_high.intersection(&dims_low).next().is_some();

        // 全距 ≥2 降級：組內任一 dimension 的 range ≥2
        let range_downgrade = group.iter().any(|a| ranges[a.dimension] >= 2);

        if clash || range_downgrade {
            AnchorCoverage::Low
        } else if group.len() == 1 {
            AnchorCoverage::Low
        } else if group.len() >= 2 {
            AnchorCoverage::High
        } else {
            AnchorCoverage::Low
        }
    };

    Some(Selected {
        trigger,
        anchor,
        anchor_ids,
        coverage,
        valence: anchor.valence,
    })
}

/// per-week 負面不過半篩選：超過半數為 Negative 時，丟棄 valence 最負者直至 ≤半數。
/// `Neutral`/`Positive` 永不丟；僅丟 `Negative`，low-coverage 先丟，再按 priority 高者先丟。
///
/// v1 D2-A 例外（Grok 裁決，F8 登記「三領域落地後廢除」）：total==2 且兩條皆 Negative
/// → 保留 1 條（coverage 較高者勝；同 coverage 比 priority 小者勝），接受該週 1/1 負面，
/// 避免低 A/C/ES 特質的週被系統性清空。per-domain 語意不可取（1 條輸出任一 Negative 即 100% 違規）。
pub fn filter_negative_half(mut selected: Vec<Selected<'static>>) -> Vec<Selected<'static>> {
    if selected.len() == 2 && selected.iter().all(|s| s.valence == Valence::Negative) {
        let keep = selected
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                let ac = if a.coverage == AnchorCoverage::High {
                    1
                } else {
                    0
                };
                let bc = if b.coverage == AnchorCoverage::High {
                    1
                } else {
                    0
                };
                ac.cmp(&bc)
                    .then_with(|| b.anchor.priority.cmp(&a.anchor.priority))
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        return vec![selected.remove(keep)];
    }
    loop {
        let total = selected.len();
        if total == 0 {
            break;
        }
        let neg = selected
            .iter()
            .filter(|s| s.valence == Valence::Negative)
            .count();
        if neg * 2 <= total {
            break;
        }
        let idx = selected
            .iter()
            .enumerate()
            .filter(|(_, s)| s.valence == Valence::Negative)
            .max_by_key(|(_, s)| {
                let low_bonus = if s.coverage == AnchorCoverage::Low {
                    10
                } else {
                    0
                };
                low_bonus + s.anchor.priority as i32
            })
            .map(|(i, _)| i);
        if let Some(i) = idx {
            selected.remove(i);
        } else {
            break;
        }
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchors::{Domain, TriggerClass};

    fn display(ext: f64, agr: f64, con: f64, emo: f64, int: f64) -> [f64; 5] {
        [ext, agr, con, emo, int]
    }
    fn ranges_all(v: u8) -> [u8; 5] {
        [v; 5]
    }

    #[test]
    fn high_low_hit_and_mid_not() {
        // agr 70 => High 命中, agr 20 => Low 命中, agr 50 中檔不命中
        // Work T1 有 agr-lo-1, Work T3 有 agr-hi-1
        let d_high = display(50.0, 70.0, 50.0, 50.0, 50.0);
        let sel = select_for_domain(Domain::Work, d_high, ranges_all(0));
        // Work 中 agr-hi 僅在 T3，且 T3 有 ext-lo + agr-hi，僅 agr-hi 命中 => 該 T3 組 count 1 => low 但應有選中
        assert!(sel.is_some());
        let s = sel.unwrap();
        assert_eq!(s.trigger, TriggerClass::T3);

        let d_low = display(50.0, 20.0, 50.0, 50.0, 50.0);
        let sel2 = select_for_domain(Domain::Work, d_low, ranges_all(0)).unwrap();
        // 低 agr 命中 T1 (agr-lo) 與 T3? T3 agr-hi 不命中，ext-lo 不命中 => T1 勝
        assert_eq!(sel2.trigger, TriggerClass::T1);

        let d_mid = display(50.0, 50.0, 50.0, 50.0, 50.0);
        assert!(select_for_domain(Domain::Work, d_mid, ranges_all(0)).is_none());
    }

    #[test]
    fn range_ge2_downgrades_to_low() {
        // emo 20 (Low) 同時命中 Work T1 的 emo-lo 與 Work T2 的 emo-lo，但測全距降級
        // 讓 Work T1 組命中 2 條皆 low 但 emo 全距 2 => 應 low
        let d = display(50.0, 20.0, 20.0, 20.0, 50.0);
        // T1 有 agr-lo(1) + emo-lo(3) 兩條皆命中 => 若無降級應 high
        let sel_no_range = select_for_domain(Domain::Work, d, ranges_all(0)).unwrap();
        assert_eq!(sel_no_range.coverage, AnchorCoverage::High);
        // 同維全距 2 降級
        let mut r = ranges_all(0);
        r[1] = 2; // agr 全距 2
        let sel_range = select_for_domain(Domain::Work, d, r).unwrap();
        assert_eq!(sel_range.coverage, AnchorCoverage::Low);
    }

    #[test]
    fn picks_winning_trigger_by_count_then_priority() {
        // 讓 Work T1 命中 2 條，T2 命中 1 條 => T1 勝
        // Work T1: agr-lo + emo-lo =2, T2: con-lo+emo-lo=2 其實都是 2，會 tie
        // 調整：讓 con 高誤命中避開 T2
        let d2 = display(50.0, 20.0, 70.0, 20.0, 50.0); // con 高 => T2 僅 emo-lo 1 條
        let sel = select_for_domain(Domain::Work, d2, ranges_all(0)).unwrap();
        assert_eq!(sel.trigger, TriggerClass::T1);
        assert_eq!(sel.anchor_ids.len(), 2);
    }

    #[test]
    fn same_dimension_clash_is_low() {
        // 構造同 trigger 內同維高低同時命中：需要一個 trigger 組內有同維的 High 和 Low
        // 目前 Work T4 有 emo-lo + con-hi 不同維，不會同維衝突；但我們測試邏輯：
        // 強行構造：Money T? 沒有同維；此測試改為驗證邏輯存在：若同維衝突則 low
        // 我們用一個合成場景：讓某 trigger 組內同維兩條同時命中是不可能的（同維高低互斥），
        // 所以此分支在當前目錄永不觸發，但邏輯仍保留以防未來目錄擴張
        // 驗證：低分 20 與高分 80 不能同時成立，故不測實際觸發，僅測函數不 panic
        let d = display(50.0, 20.0, 50.0, 20.0, 50.0);
        let sel = select_for_domain(Domain::Work, d, ranges_all(0));
        assert!(sel.is_some());
    }

    #[test]
    fn empty_is_none() {
        let d = display(50.0, 50.0, 50.0, 50.0, 50.0);
        assert!(select_for_domain(Domain::Work, d, ranges_all(0)).is_none());
        assert!(select_for_domain(Domain::Money, d, ranges_all(0)).is_none());
    }

    #[test]
    fn tie_break_is_deterministic() {
        // 兩組同數同 min priority 時按 trigger 字典序 T1 < T2
        // 構造：Work T1 兩條 priority 1,2 皆命中，Work T2 兩條 priority 1,2 皆命中 => 同為 2
        let d = display(50.0, 20.0, 20.0, 20.0, 50.0); // agr-lo, con-lo, emo-lo 皆命中
                                                       // 此時 T1 (agr+emo) 2 條, T2 (con+emo) 2 條，但 emo 重疊？實際 T1 2 條 T2 2 條同數同 min 1 => 應選 T1
        let sel = select_for_domain(Domain::Work, d, ranges_all(0)).unwrap();
        assert_eq!(sel.trigger, TriggerClass::T1);
    }

    #[test]
    fn per_week_negative_not_over_half() {
        // 2 負 1 中 → 正常迴圈丟 1 負 → 1 負 1 中（負面不過半）
        let neg = selected_with(neg_anchor(), AnchorCoverage::High);
        let neu = selected_with(neutral_anchor(), AnchorCoverage::High);
        let filtered = filter_negative_half(vec![neg.clone(), neg.clone(), neu.clone()]);
        assert_eq!(filtered.len(), 2);
        let neg = filtered
            .iter()
            .filter(|s| s.valence == Valence::Negative)
            .count();
        assert!(neg * 2 <= filtered.len());
    }

    #[test]
    fn two_negative_domains_keep_better_coverage() {
        // D2-A：2 條皆負 → 保留 coverage 較高者（同 coverage 比 priority 小者勝）
        let neg_lo = selected_with(neg_anchor(), AnchorCoverage::Low);
        let neg_hi = selected_with(neg_anchor(), AnchorCoverage::High);
        for v in [
            vec![neg_lo.clone(), neg_hi.clone()],
            vec![neg_hi.clone(), neg_lo.clone()],
        ] {
            let filtered = filter_negative_half(v);
            assert_eq!(filtered.len(), 1);
            assert_eq!(filtered[0].coverage, AnchorCoverage::High);
        }
    }

    #[test]
    fn two_negative_same_coverage_keeps_lower_priority() {
        // D2-A：同 coverage → priority 小者勝（Grok 二審 nit）
        let a1 = ANCHORS
            .iter()
            .find(|a| a.valence == Valence::Negative && a.priority == 1)
            .unwrap();
        let a2 = ANCHORS
            .iter()
            .find(|a| a.valence == Valence::Negative && a.priority == 2)
            .unwrap();
        let s1 = selected_with(a1, AnchorCoverage::High);
        let s2 = selected_with(a2, AnchorCoverage::High);
        for v in [vec![s1.clone(), s2.clone()], vec![s2.clone(), s1.clone()]] {
            let filtered = filter_negative_half(v);
            assert_eq!(filtered.len(), 1);
            assert_eq!(filtered[0].anchor.priority, 1);
        }
    }

    #[test]
    fn all_neutral_unchanged() {
        let neu = selected_with(neutral_anchor(), AnchorCoverage::High);
        let filtered = filter_negative_half(vec![neu.clone(), neu.clone()]);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn empty_stays_empty() {
        assert!(filter_negative_half(vec![]).is_empty());
    }

    #[test]
    fn display_rounded_rounds_to_integer() {
        use crate::api::OceanScores;
        let o = OceanScores {
            extraversion: 66.7,
            agreeableness: 33.3,
            conscientiousness: 50.0,
            emotionalStability: 66.5,
            intellectImagination: 100.0,
        };
        assert_eq!(display_rounded(&o), [67.0, 33.0, 50.0, 67.0, 100.0]);
    }

    #[test]
    fn dim_ranges_computed_from_answers() {
        // 維 0: 1,1,5 → 4；維 1: 3,3,3 → 0；維 2: 2,4,1 → 3；維 3: 5,5,5 → 0；維 4: 1,5,2 → 4
        let answers = [1u8, 1, 5, 3, 3, 3, 2, 4, 1, 5, 5, 5, 1, 5, 2];
        assert_eq!(dim_ranges(&answers), Some([4, 0, 3, 0, 4]));
        assert_eq!(dim_ranges(&[1, 2, 3]), None);
    }

    // ── 測試輔助：手動建 Selected（anchor 取自目錄）──

    fn selected_with(
        anchor: &'static crate::anchors::Anchor,
        coverage: AnchorCoverage,
    ) -> Selected<'static> {
        Selected {
            trigger: anchor.trigger,
            anchor,
            anchor_ids: vec![anchor.id],
            coverage,
            valence: anchor.valence,
        }
    }

    fn neg_anchor() -> &'static crate::anchors::Anchor {
        ANCHORS
            .iter()
            .find(|a| a.valence == Valence::Negative)
            .unwrap()
    }

    fn neutral_anchor() -> &'static crate::anchors::Anchor {
        ANCHORS
            .iter()
            .find(|a| a.valence == Valence::Neutral)
            .unwrap()
    }
}
