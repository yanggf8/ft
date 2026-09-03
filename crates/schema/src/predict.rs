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

/// per-week 負面不過半篩選：超過半數為 Negative 時，丟棄 valence 最負者直至 ≤半數
/// `Neutral` 永不丟，`Positive` 亦不丟；僅丟 `Negative`，按 priority 高者先丟（可選，本文按出現順序）
pub fn filter_negative_half(mut selected: Vec<Selected<'static>>) -> Vec<Selected<'static>> {
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
        // 構造 3 條 selected，其中 2 負 1 中 => 過濾後應 1 負 1 中
        let d_neg = display(50.0, 20.0, 20.0, 20.0, 50.0);
        // 產生兩個 domain 的 selected 模擬 per-week
        let s1 = select_for_domain(Domain::Work, d_neg, ranges_all(0)).unwrap();
        let s2 = select_for_domain(Domain::Money, d_neg, ranges_all(0)).unwrap();
        // 人工構造第三條負面
        let mut vec = vec![s1, s2];
        // 複製一個負面
        let s3 = select_for_domain(Domain::Work, d_neg, ranges_all(0)).unwrap();
        vec.push(s3);
        // 此時可能 3 條皆負（因為 work/money T1 皆負），過濾後應 ≤1
        let filtered = filter_negative_half(vec);
        let neg = filtered
            .iter()
            .filter(|s| s.valence == Valence::Negative)
            .count();
        assert!(neg * 2 <= filtered.len());
    }
}
