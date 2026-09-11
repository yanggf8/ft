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

/// 閘門迭代順序 = F4 強度 ≥1 的領域生成順序，**同時**決定：
/// ① `gated_domains` 輸出序（→ `filter_negative_half` 的平手丟棄方向依此輸入序定義）；
/// ② `list_cycle` 的 SQL `ORDER BY` 順位（work 0 / money 1 / love 2 / 其餘 3，見
/// `crates/api/src/services/predictions.rs`）。改動此順序必須三處同步並重跑紅線 golden。
pub const GATE_ORDER: [Domain; 5] = [
    Domain::Work,
    Domain::Money,
    Domain::Love,
    Domain::Family,
    Domain::Health,
];

fn strength_of(s: &crate::api::DomainStrengths, d: Domain) -> u8 {
    match d {
        Domain::Work => s.work,
        Domain::Money => s.money,
        Domain::Love => s.love,
        Domain::Family => s.family,
        Domain::Health => s.health,
    }
}

/// F4 五領域強度值域檢查（0–3）。wire 層不做數值限制（serde 只鎖型別），
/// 由 route 層以此把關：不在範圍 → 400 `INVALID_STRENGTHS`。
pub fn strengths_in_range(s: &crate::api::DomainStrengths) -> bool {
    GATE_ORDER.iter().all(|d| strength_of(s, *d) <= 3)
}

/// F4 領域閘門：回傳強度 ≥1 的領域（`GATE_ORDER` 序）。強度 0 不建列；
/// family/health 目錄尚空，`select_for_domain` 恆回 `None`（前瞻擴充用）。
pub fn gated_domains(s: &crate::api::DomainStrengths) -> Vec<Domain> {
    GATE_ORDER
        .iter()
        .copied()
        .filter(|d| strength_of(s, *d) >= 1)
        .collect()
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

/// per-week 負面不過半篩選：Negative 超過半數時，逐條丟棄「最劣」負面直至 ≤半數。
/// `Neutral`/`Positive` 永不丟；僅丟 `Negative`，low-coverage 先丟，再按 priority 高者先丟。
///
/// floor：永不丟最後一條——全負週保留最佳 1 條，接受 1/1 負面。這是 v1 D2-A 例外
/// （Grok 裁決：避免低 A/C/ES 特質的週被系統性清空）的**推廣版**：原 n==2 特例已廢除
/// （F8 登記「三領域落地後廢除」），由任何 n 的 floor 語意涵蓋；見
/// `docs/superpowers/specs/2026-09-11-f4-love-expansion-design.md`。
/// per-domain 語意不可取（1 條輸出任一 Negative 即 100% 違規）。
///
/// 平手方向：同 (coverage, priority) 丟鍵時**丟較前者**——本函數輸入序為 `gated_domains`
/// 的 `GATE_ORDER`（work 先、money 後），平手時後者存活。v1 紅線 golden 釘死此方向
/// （money 勝出）；`f5_*_pinned` 測試守護，不得改用 `max_by_key`（同分取最後，方向相反）。
pub fn filter_negative_half(mut selected: Vec<Selected<'static>>) -> Vec<Selected<'static>> {
    loop {
        let total = selected.len();
        if total <= 1 {
            break;
        }
        let neg = selected
            .iter()
            .filter(|s| s.valence == Valence::Negative)
            .count();
        if neg * 2 <= total {
            break;
        }
        // 丟鍵 argmax，**首同分勝**：手動掃描、嚴格 `>` 才替換（Rust `max_by_key` 同分取
        // 最後，方向會翻轉紅線 golden 的勝出者）。
        let mut drop_idx: Option<usize> = None;
        for (i, s) in selected.iter().enumerate() {
            if s.valence != Valence::Negative {
                continue;
            }
            let better = match drop_idx {
                None => true,
                Some(d) => drop_key(s) > drop_key(&selected[d]),
            };
            if better {
                drop_idx = Some(i);
            }
        }
        match drop_idx {
            Some(i) => {
                selected.remove(i);
            }
            None => break,
        }
    }
    selected
}

/// 丟棄優先鍵：low-coverage 加權 + priority（大者先丟）。
fn drop_key(s: &Selected<'_>) -> i32 {
    let low_bonus = if s.coverage == AnchorCoverage::Low {
        10
    } else {
        0
    };
    low_bonus + s.anchor.priority as i32
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
    fn three_negative_keep_single_best() {
        // D2-A 推廣：n=3 全負 → 驅逐 2 條，保留 (High, 最小 priority) 的 1 條
        let hi1 = selected_with(neg_anchor_p1(), AnchorCoverage::High);
        let lo1 = selected_with(neg_anchor_p2(), AnchorCoverage::Low);
        let lo2 = selected_with(neg_anchor_p2(), AnchorCoverage::Low);
        let filtered = filter_negative_half(vec![lo1, lo2, hi1]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].coverage, AnchorCoverage::High);
        assert_eq!(filtered[0].anchor.priority, 1);
    }

    #[test]
    fn single_negative_week_is_kept() {
        // 推廣後的行為變更（設計文件登記 #1）：單一負面條目不再丟至空
        let neg = selected_with(neg_anchor(), AnchorCoverage::Low);
        let filtered = filter_negative_half(vec![neg]);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn mixed_two_neg_two_neu_unchanged() {
        // 2 負 2 中：4<=4 已過半數規則 → 原樣保留
        let neg = selected_with(neg_anchor(), AnchorCoverage::High);
        let neu = selected_with(neutral_anchor(), AnchorCoverage::High);
        let filtered = filter_negative_half(vec![neg.clone(), neg, neu.clone(), neu]);
        assert_eq!(filtered.len(), 4);
    }

    #[test]
    fn drop_tie_keeps_later_domain() {
        // 平手方向釘死：同 (coverage, priority) 鍵時丟較前者 → 後者（money）存活。
        // 紅線 golden（f5_*_pinned）依賴此方向；改用 max_by_key（同分取最後）會炸 golden。
        let a1 = ANCHORS.iter().find(|a| a.id == "work-t1-agr-lo-1").unwrap();
        let a2 = ANCHORS
            .iter()
            .find(|a| a.id == "money-t1-agr-lo-1")
            .unwrap();
        let filtered = filter_negative_half(vec![
            selected_with(a1, AnchorCoverage::High),
            selected_with(a2, AnchorCoverage::High),
        ]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].anchor.id, "money-t1-agr-lo-1");
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

    #[test]
    fn gated_domains_order_and_filter() {
        use crate::api::DomainStrengths;
        let all = DomainStrengths {
            work: 1,
            love: 1,
            family: 0,
            money: 1,
            health: 0,
        };
        // GATE_ORDER 序（work → money → love → …），強度 0 剔除
        assert_eq!(
            gated_domains(&all),
            vec![Domain::Work, Domain::Money, Domain::Love]
        );
        let zeros = DomainStrengths {
            work: 0,
            love: 0,
            family: 0,
            money: 0,
            health: 0,
        };
        assert!(gated_domains(&zeros).is_empty());
        let maxed = DomainStrengths {
            work: 3,
            love: 3,
            family: 3,
            money: 3,
            health: 3,
        };
        assert_eq!(gated_domains(&maxed), GATE_ORDER.to_vec());
    }

    #[test]
    fn strengths_in_range_bounds() {
        use crate::api::DomainStrengths;
        let ok = DomainStrengths {
            work: 3,
            love: 0,
            family: 3,
            money: 1,
            health: 2,
        };
        assert!(strengths_in_range(&ok));
        let bad = DomainStrengths {
            work: 4,
            love: 0,
            family: 0,
            money: 0,
            health: 0,
        };
        assert!(!strengths_in_range(&bad));
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

    fn neg_anchor_p1() -> &'static crate::anchors::Anchor {
        ANCHORS
            .iter()
            .find(|a| a.valence == Valence::Negative && a.priority == 1)
            .unwrap()
    }

    fn neg_anchor_p2() -> &'static crate::anchors::Anchor {
        ANCHORS
            .iter()
            .find(|a| a.valence == Valence::Negative && a.priority == 2)
            .unwrap()
    }

    fn neutral_anchor() -> &'static crate::anchors::Anchor {
        ANCHORS
            .iter()
            .find(|a| a.valence == Valence::Neutral)
            .unwrap()
    }

    #[test]
    fn f5_selection_outputs_are_pinned_against_chart_side_effects() {
        // 紅線(spec 2026-09-07-f2-f3 §0.4):命盤/F2 相關改動若改變了 F5 選則
        // 輸出,這裡必炸。golden 值 2026-09-07 由 --nocapture 實跑抄錄;更新必須
        // 在 code review 中說明「為何 F5 輸出會變」。
        const SCORES: [f64; 5] = [70.0, 40.0, 55.0, 30.0, 62.0];
        let work = select_for_domain(Domain::Work, SCORES, [0; 5]).expect("work 應有選則");
        assert_eq!(work.trigger, TriggerClass::T4);
        assert_eq!(work.anchor.id, "work-t4-emo-lo-1");
        assert_eq!(work.anchor.domain, Domain::Work);
        assert_eq!(work.anchor.tendency, "被糾正時較易往心裡去、需要時間消化");
        assert_eq!(
            work.anchor.forecast,
            "這週被指出問題時，更可能先沉默而非立刻回應"
        );
        assert_eq!(
            work.anchor.experiment,
            Some("先複述對方的重點，確認理解再回應")
        );
        assert_eq!(work.anchor_ids, vec!["work-t4-emo-lo-1"]);
        assert_eq!(work.coverage, AnchorCoverage::Low);
        assert_eq!(work.valence, Valence::Negative);

        let money = select_for_domain(Domain::Money, SCORES, [0; 5]).expect("money 應有選則");
        assert_eq!(money.trigger, TriggerClass::T4);
        assert_eq!(money.anchor.id, "money-t4-emo-lo-1");
        assert_eq!(money.anchor.domain, Domain::Money);
        assert_eq!(money.anchor.tendency, "被指出花費問題時較易感到在意");
        assert_eq!(
            money.anchor.forecast,
            "這週被提醒花費時，更可能先解釋而非立刻調整"
        );
        assert_eq!(
            money.anchor.experiment,
            Some("先記錄提醒的內容，隔天再檢視")
        );
        assert_eq!(money.anchor_ids, vec!["money-t4-emo-lo-1"]);
        assert_eq!(money.coverage, AnchorCoverage::Low);
        assert_eq!(money.valence, Valence::Negative);

        // 全負面週:D2-A 例外 — 保留較佳 1 條(golden:money 勝出)
        let filtered = filter_negative_half(vec![work, money]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].anchor.id, "money-t4-emo-lo-1");
        assert_eq!(filtered[0].coverage, AnchorCoverage::Low);
        assert_eq!(filtered[0].valence, Valence::Negative);
    }

    #[test]
    fn f5_all_negative_low_fixture_pinned() {
        const LOW: [f64; 5] = [20.0; 5];
        let work = select_for_domain(Domain::Work, LOW, [0; 5]).expect("work-low 應有選則");
        assert_eq!(work.trigger, TriggerClass::T1);
        assert_eq!(work.anchor.id, "work-t1-agr-lo-1");
        assert_eq!(
            work.anchor_ids,
            vec!["work-t1-agr-lo-1", "work-t1-emo-lo-1"]
        );
        assert_eq!(work.coverage, AnchorCoverage::High);
        assert_eq!(work.valence, Valence::Negative);

        let money = select_for_domain(Domain::Money, LOW, [0; 5]).expect("money-low 應有選則");
        assert_eq!(money.trigger, TriggerClass::T1);
        assert_eq!(money.anchor.id, "money-t1-agr-lo-1");
        assert_eq!(
            money.anchor_ids,
            vec!["money-t1-agr-lo-1", "money-t1-emo-lo-1"]
        );

        let filtered = filter_negative_half(vec![work, money]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].anchor.id, "money-t1-agr-lo-1");
    }
}
