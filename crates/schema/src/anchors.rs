//! 規則錨點目錄 — work+money × T1–T6（v1 縱深 12 格，每格 ≥2 條）
//! 與 `items.rs` 同體例：`ft-schema` 靜態真相，`crates/web` 可讀，`ft-big5` 不動
//! Trigger 封閉列舉 T1–T6 見 `2026-09-03-f4-f5-if-then-design-note.md` §5.3.3

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    Work,
    Love,
    Family,
    Money,
    Health,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerClass {
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    High,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Valence {
    Negative,
    Neutral,
    Positive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Literature,
    DesignerJudgment,
}

#[derive(Debug)]
pub struct Anchor {
    pub id: &'static str,
    pub domain: Domain,
    pub trigger: TriggerClass,
    pub dimension: usize,
    pub level: Level,
    pub priority: u8,
    pub tendency: &'static str,
    pub forecast: &'static str,
    pub experiment: Option<&'static str>,
    pub valence: Valence,
    pub source: Source,
}

pub const ANCHORS: &[Anchor] = &[
    // ── Work × T1 人際摩擦（負）主維 友善性 / 情緒穩定 ──
    Anchor {
        id: "work-t1-agr-lo-1",
        domain: Domain::Work,
        trigger: TriggerClass::T1,
        dimension: 1,
        level: Level::Low,
        priority: 1,
        tendency: "在工作摩擦中傾向先退開、避免當場對峙",
        forecast: "這週遇到意見不合時，更可能先擱置而非當場釐清",
        experiment: Some("先記下分歧點，隔天再約 15 分鐘對齊"),
        valence: Valence::Negative,
        source: Source::DesignerJudgment,
    },
    Anchor {
        id: "work-t1-emo-lo-1",
        domain: Domain::Work,
        trigger: TriggerClass::T1,
        dimension: 3,
        level: Level::Low,
        priority: 2,
        tendency: "在張力情境下情緒起伏較明顯、需要時間平復",
        forecast: "這週遇到摩擦後，更可能反覆回想對話內容",
        experiment: None,
        valence: Valence::Negative,
        source: Source::Literature,
    },
    // ── Work × T2 時限壓力（負）主維 嚴謹性 / 情緒穩定 ──
    Anchor {
        id: "work-t2-con-lo-1",
        domain: Domain::Work,
        trigger: TriggerClass::T2,
        dimension: 2,
        level: Level::Low,
        priority: 1,
        tendency: "在期限壓力下較易感到手忙腳亂、節奏被打亂",
        forecast: "這週事情趕不完時，更可能先做最急的而非先排優先序",
        experiment: Some("每天先列三件最重要的事，再動手"),
        valence: Valence::Negative,
        source: Source::DesignerJudgment,
    },
    Anchor {
        id: "work-t2-emo-lo-1",
        domain: Domain::Work,
        trigger: TriggerClass::T2,
        dimension: 3,
        level: Level::Low,
        priority: 2,
        tendency: "在時間壓力下較易感到焦慮、擔心做不完",
        forecast: "這週被期限追著時，更可能反覆檢查進度",
        experiment: None,
        valence: Valence::Negative,
        source: Source::Literature,
    },
    // ── Work × T3 生疏社交（中）主維 外向性 / 友善性 ──
    Anchor {
        id: "work-t3-ext-lo-1",
        domain: Domain::Work,
        trigger: TriggerClass::T3,
        dimension: 0,
        level: Level::Low,
        priority: 1,
        tendency: "在生疏或人多的場合傾向先觀察、少主動開口",
        forecast: "這週需要跟不熟的人相處時，更可能先聽而非先講",
        experiment: Some("先準備一句開場白，降低臨場壓力"),
        valence: Valence::Neutral,
        source: Source::DesignerJudgment,
    },
    Anchor {
        id: "work-t3-agr-hi-1",
        domain: Domain::Work,
        trigger: TriggerClass::T3,
        dimension: 1,
        level: Level::High,
        priority: 2,
        tendency: "在團體中傾向主動照顧氣氛、讓他人放鬆",
        forecast: "這週在群體場合中，更可能主動招呼或串場",
        experiment: None,
        valence: Valence::Neutral,
        source: Source::Literature,
    },
    // ── Work × T4 被指出問題（負）主維 情緒穩定 / 嚴謹性 ──
    Anchor {
        id: "work-t4-emo-lo-1",
        domain: Domain::Work,
        trigger: TriggerClass::T4,
        dimension: 3,
        level: Level::Low,
        priority: 1,
        tendency: "被糾正時較易往心裡去、需要時間消化",
        forecast: "這週被指出問題時，更可能先沉默而非立刻回應",
        experiment: Some("先複述對方的重點，確認理解再回應"),
        valence: Valence::Negative,
        source: Source::Literature,
    },
    Anchor {
        id: "work-t4-con-hi-1",
        domain: Domain::Work,
        trigger: TriggerClass::T4,
        dimension: 2,
        level: Level::High,
        priority: 2,
        tendency: "被挑毛病時傾向立刻修正、把細節補齊",
        forecast: "這週收到負面回饋後，更可能當天就調整做法",
        experiment: None,
        valence: Valence::Negative,
        source: Source::DesignerJudgment,
    },
    // ── Work × T5 計畫被打亂（中）主維 嚴謹性 / 智性 ──
    Anchor {
        id: "work-t5-con-hi-1",
        domain: Domain::Work,
        trigger: TriggerClass::T5,
        dimension: 2,
        level: Level::High,
        priority: 1,
        tendency: "安排被打亂時傾向先重排計畫、找回秩序",
        forecast: "這週原定安排變動時，更可能先列出替代方案",
        experiment: Some("先寫下變動的影響範圍，再重排時程"),
        valence: Valence::Neutral,
        source: Source::DesignerJudgment,
    },
    Anchor {
        id: "work-t5-int-hi-1",
        domain: Domain::Work,
        trigger: TriggerClass::T5,
        dimension: 4,
        level: Level::High,
        priority: 2,
        tendency: "面對變動時較易聯想多種可能性",
        forecast: "這週計畫變動時，更可能同時想到兩三種做法",
        experiment: None,
        valence: Valence::Neutral,
        source: Source::Literature,
    },
    // ── Work × T6 有選擇要做（中/正）主維 智性 / 嚴謹性 ──
    Anchor {
        id: "work-t6-int-hi-1",
        domain: Domain::Work,
        trigger: TriggerClass::T6,
        dimension: 4,
        level: Level::High,
        priority: 1,
        tendency: "面對選項時傾向多方比較、尋找更好做法",
        forecast: "這週需要在幾個選項間決定時，更可能先做小範圍試驗",
        experiment: Some("先為每個選項寫下一項優點與一項風險"),
        valence: Valence::Positive,
        source: Source::Literature,
    },
    Anchor {
        id: "work-t6-con-hi-1",
        domain: Domain::Work,
        trigger: TriggerClass::T6,
        dimension: 2,
        level: Level::High,
        priority: 2,
        tendency: "做決定時傾向按步驟、有條理地評估",
        forecast: "這週做選擇時，更可能先列出判斷標準再決定",
        experiment: None,
        valence: Valence::Neutral,
        source: Source::DesignerJudgment,
    },
    // ── Money × T1 人際摩擦（負） ──
    Anchor {
        id: "money-t1-agr-lo-1",
        domain: Domain::Money,
        trigger: TriggerClass::T1,
        dimension: 1,
        level: Level::Low,
        priority: 1,
        tendency: "在金錢相關的討論中遇分歧時傾向先迴避爭執",
        forecast: "這週談到花費分攤有不同意見時，更可能先擱置而非當場決定",
        experiment: Some("先各自寫下期待的數字，再找交集"),
        valence: Valence::Negative,
        source: Source::DesignerJudgment,
    },
    Anchor {
        id: "money-t1-emo-lo-1",
        domain: Domain::Money,
        trigger: TriggerClass::T1,
        dimension: 3,
        level: Level::Low,
        priority: 2,
        tendency: "在金錢摩擦中較易感到壓力、反覆思量",
        forecast: "這週因花費起摩擦後，更可能反覆回想對話",
        experiment: None,
        valence: Valence::Negative,
        source: Source::Literature,
    },
    // ── Money × T2 時限壓力（負） ──
    Anchor {
        id: "money-t2-con-lo-1",
        domain: Domain::Money,
        trigger: TriggerClass::T2,
        dimension: 2,
        level: Level::Low,
        priority: 1,
        tendency: "在繳費或預算期限壓力下較易拖延整理",
        forecast: "這週面對繳費期限時，更可能先處理最急的一筆",
        experiment: Some("先把所有待付項目列清單再排順序"),
        valence: Valence::Negative,
        source: Source::DesignerJudgment,
    },
    Anchor {
        id: "money-t2-emo-lo-1",
        domain: Domain::Money,
        trigger: TriggerClass::T2,
        dimension: 3,
        level: Level::Low,
        priority: 2,
        tendency: "在預算追趕下較易感到緊繃",
        forecast: "這週被預算期限追著時，更可能頻繁查看餘額",
        experiment: None,
        valence: Valence::Negative,
        source: Source::Literature,
    },
    // ── Money × T3 生疏社交（中） ──
    Anchor {
        id: "money-t3-ext-lo-1",
        domain: Domain::Money,
        trigger: TriggerClass::T3,
        dimension: 0,
        level: Level::Low,
        priority: 1,
        tendency: "在不熟的人面前談錢時傾向較為保留",
        forecast: "這週需要跟不熟的人談費用時，更可能先聽對方開價",
        experiment: Some("先準備一個可接受的區間再進場"),
        valence: Valence::Neutral,
        source: Source::DesignerJudgment,
    },
    Anchor {
        id: "money-t3-agr-hi-1",
        domain: Domain::Money,
        trigger: TriggerClass::T3,
        dimension: 1,
        level: Level::High,
        priority: 2,
        tendency: "在群體消費場合傾向顧及他人感受",
        forecast: "這週與他人共同消費時，更可能先詢問大家的偏好",
        experiment: None,
        valence: Valence::Neutral,
        source: Source::Literature,
    },
    // ── Money × T4 被指出問題（負） ──
    Anchor {
        id: "money-t4-emo-lo-1",
        domain: Domain::Money,
        trigger: TriggerClass::T4,
        dimension: 3,
        level: Level::Low,
        priority: 1,
        tendency: "被指出花費問題時較易感到在意",
        forecast: "這週被提醒花費時，更可能先解釋而非立刻調整",
        experiment: Some("先記錄提醒的內容，隔天再檢視"),
        valence: Valence::Negative,
        source: Source::Literature,
    },
    Anchor {
        id: "money-t4-con-hi-1",
        domain: Domain::Money,
        trigger: TriggerClass::T4,
        dimension: 2,
        level: Level::High,
        priority: 2,
        tendency: "被提醒預算時傾向立刻核對數字",
        forecast: "這週收到花費回饋後，更可能當天就整理收支",
        experiment: None,
        valence: Valence::Negative,
        source: Source::DesignerJudgment,
    },
    // ── Money × T5 計畫被打亂（中） ──
    Anchor {
        id: "money-t5-con-hi-1",
        domain: Domain::Money,
        trigger: TriggerClass::T5,
        dimension: 2,
        level: Level::High,
        priority: 1,
        tendency: "預算安排被打亂時傾向重排計畫",
        forecast: "這週預算變動時，更可能先重算本週可支配金額",
        experiment: Some("先標出變動的金額與影響的項目"),
        valence: Valence::Neutral,
        source: Source::DesignerJudgment,
    },
    Anchor {
        id: "money-t5-int-hi-1",
        domain: Domain::Money,
        trigger: TriggerClass::T5,
        dimension: 4,
        level: Level::High,
        priority: 2,
        tendency: "面對變動時較易想到替代做法",
        forecast: "這週花費計畫變動時，更可能同時考慮兩種調整方式",
        experiment: None,
        valence: Valence::Neutral,
        source: Source::Literature,
    },
    // ── Money × T6 有選擇要做（中/正） ──
    Anchor {
        id: "money-t6-int-hi-1",
        domain: Domain::Money,
        trigger: TriggerClass::T6,
        dimension: 4,
        level: Level::High,
        priority: 1,
        tendency: "面對消費選項時傾向比較多種可能性",
        forecast: "這週需要在幾個花費選項間決定時，更可能先比較性價比",
        experiment: Some("先為每個選項寫下一項優點與一項顧慮"),
        valence: Valence::Positive,
        source: Source::Literature,
    },
    Anchor {
        id: "money-t6-con-hi-1",
        domain: Domain::Money,
        trigger: TriggerClass::T6,
        dimension: 2,
        level: Level::High,
        priority: 2,
        tendency: "做花費決定時傾向有條理地評估",
        forecast: "這週做消費選擇時，更可能先訂出判斷標準再決定",
        experiment: None,
        valence: Valence::Neutral,
        source: Source::DesignerJudgment,
    },
];

/// 規則版本（語意遞增）：目錄實質變更（增/改錨點、改切點）才 bump。
/// 每列 `predictions.rules_version` 寫此值；F8 分析須按此分層。
pub const RULES_VERSION: &str = "rules-1";

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn every_v1_cell_has_at_least_two() {
        let mut counts: HashMap<(Domain, TriggerClass), usize> = HashMap::new();
        for a in ANCHORS {
            *counts.entry((a.domain, a.trigger)).or_default() += 1;
        }
        for domain in [Domain::Work, Domain::Money] {
            for trigger in [
                TriggerClass::T1,
                TriggerClass::T2,
                TriggerClass::T3,
                TriggerClass::T4,
                TriggerClass::T5,
                TriggerClass::T6,
            ] {
                let c = counts.get(&(domain, trigger)).copied().unwrap_or(0);
                assert!(c >= 2, "cell {:?}/{:?} has {} <2", domain, trigger, c);
            }
        }
        for domain in [Domain::Love, Domain::Family, Domain::Health] {
            for trigger in [
                TriggerClass::T1,
                TriggerClass::T2,
                TriggerClass::T3,
                TriggerClass::T4,
                TriggerClass::T5,
                TriggerClass::T6,
            ] {
                let c = counts.get(&(domain, trigger)).copied().unwrap_or(0);
                assert_eq!(c, 0, "v1 should have 0 for {:?}/{:?}", domain, trigger);
            }
        }
    }

    #[test]
    fn priority_unique_and_contiguous_per_cell() {
        let mut by_cell: HashMap<(Domain, TriggerClass), Vec<u8>> = HashMap::new();
        for a in ANCHORS {
            by_cell
                .entry((a.domain, a.trigger))
                .or_default()
                .push(a.priority);
        }
        for ((d, t), mut ps) in by_cell {
            ps.sort_unstable();
            let mut seen = HashSet::new();
            for &p in &ps {
                assert!(
                    seen.insert(p),
                    "duplicate priority {} in {:?}/{:?}",
                    p,
                    d,
                    t
                );
            }
            for (i, &p) in ps.iter().enumerate() {
                assert_eq!(
                    p,
                    (i as u8) + 1,
                    "priority not 1..N contiguous in {:?}/{:?}: got {:?}",
                    d,
                    t,
                    ps
                );
            }
        }
    }

    #[test]
    fn id_globally_unique() {
        let mut seen = HashSet::new();
        for a in ANCHORS {
            assert!(seen.insert(a.id), "duplicate id {}", a.id);
        }
    }

    #[test]
    fn dimension_in_range() {
        for a in ANCHORS {
            assert!(
                a.dimension <= 4,
                "dimension {} out of range for {}",
                a.dimension,
                a.id
            );
        }
    }

    #[test]
    fn valence_not_over_half() {
        let neg = ANCHORS
            .iter()
            .filter(|a| a.valence == Valence::Negative)
            .count();
        assert!(
            neg * 2 <= ANCHORS.len(),
            "Negative {} > half of {}",
            neg,
            ANCHORS.len()
        );
    }

    #[test]
    fn money_has_no_loss_forecast() {
        for a in ANCHORS.iter().filter(|a| a.domain == Domain::Money) {
            let f = a.forecast;
            assert!(
                !f.contains("損失") && !f.contains("負債") && !f.contains("虧損"),
                "money forecast contains loss word: {} => {}",
                a.id,
                f
            );
        }
    }

    #[test]
    fn ids_are_lowercase_t_format() {
        for a in ANCHORS {
            // 例: work-t1-agr-lo-1
            let parts: Vec<&str> = a.id.split('-').collect();
            assert!(parts.len() >= 4, "id format unexpected: {}", a.id);
            assert!(
                parts[1].starts_with('t'),
                "id trigger part should be t1..t6: {}",
                a.id
            );
            // 小寫檢查
            assert_eq!(
                a.id.to_ascii_lowercase(),
                a.id,
                "id should be lowercase: {}",
                a.id
            );
        }
    }
}
