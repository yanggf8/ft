//! 三才五行 — 純邏輯模組（v1）。
//!
//! 數理五行採十天干序（個位 1,2木 3,4火 5,6土 7,8金 9,0水），非河圖數。
//! 三才評級為 v1 簡化規則：只看相鄰兩對（天↔人、人↔地）的比和/相生/相剋，
//! 兩和諧→吉、一和一衝→半吉、兩衝→凶。**非傳統 125 組配置表**——已知失真
//! （金金金、木木土、洩氣局）見 spec §2，UI 必須附「生剋簡表」聲明。

use serde::{Deserialize, Serialize};

use super::luck::LuckClass;

/// 五行元素。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Element {
    Wood,
    Fire,
    Earth,
    Metal,
    Water,
}

impl Element {
    pub fn label(&self) -> &'static str {
        match self {
            Element::Wood => "木",
            Element::Fire => "火",
            Element::Earth => "土",
            Element::Metal => "金",
            Element::Water => "水",
        }
    }

    /// 相生序的下一個（木→火→土→金→水→木）。
    fn next_in_cycle(self) -> Element {
        match self {
            Element::Wood => Element::Fire,
            Element::Fire => Element::Earth,
            Element::Earth => Element::Metal,
            Element::Metal => Element::Water,
            Element::Water => Element::Wood,
        }
    }
}

/// 個位數（index = strokes % 10）→ 五行，十天干序（0 與 9 同屬水：壬癸）。
pub const DIGIT_ELEMENTS: [Element; 10] = [
    Element::Water,
    Element::Wood,
    Element::Wood,
    Element::Fire,
    Element::Fire,
    Element::Earth,
    Element::Earth,
    Element::Metal,
    Element::Metal,
    Element::Water,
];

/// 數理五行：取筆畫個位查表（>81 wraparound 減 80 不改個位，故直接取模）。
pub fn element_of(strokes: u16) -> Element {
    DIGIT_ELEMENTS[(strokes % 10) as usize]
}

/// 相生：木生火、火生土、土生金、金生水、水生木（單向；「生我」由呼叫端反呼）。
pub fn generates(a: Element, b: Element) -> bool {
    a.next_in_cycle() == b
}

/// 相剋：木剋土、土剋水、水剋火、火剋金、金剋木（相生序跳兩步，單向）。
pub fn overcomes(a: Element, b: Element) -> bool {
    a.next_in_cycle().next_in_cycle() == b
}

/// 一對五行關係：比和 / 相生（任一方向）/ 相剋（任一方向）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PairRelation {
    Same,
    Harmonious,
    Conflicting,
}

impl PairRelation {
    pub fn label(&self) -> &'static str {
        match self {
            PairRelation::Same => "比和",
            PairRelation::Harmonious => "相生",
            PairRelation::Conflicting => "相剋",
        }
    }
}

/// 兩元素的一對關係：比和 / 相生（任一方向）/ 相剋（任一方向）。
pub fn pair_relation(a: Element, b: Element) -> PairRelation {
    if a == b {
        PairRelation::Same
    } else if generates(a, b) || generates(b, a) {
        PairRelation::Harmonious
    } else {
        PairRelation::Conflicting
    }
}

/// 三才配置 [天, 人, 地] 的評級（v1 簡化規則，見模組註解）。
/// 兩對和諧→吉、一和一衝→半吉、兩衝→凶；永不回傳 DaJi（簡表無大吉檔）。
pub fn sancai_rating(elements: [Element; 3]) -> LuckClass {
    let conflicts = [
        pair_relation(elements[0], elements[1]),
        pair_relation(elements[1], elements[2]),
    ]
    .iter()
    .filter(|r| **r == PairRelation::Conflicting)
    .count();
    match conflicts {
        0 => LuckClass::Ji,
        1 => LuckClass::BanJi,
        _ => LuckClass::Xiong,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digit_map_follows_ten_stems_order() {
        // 1,2木 3,4火 5,6土 7,8金 9,0水（十天干序，非河圖數）
        let expected = [
            Element::Water, // 0
            Element::Wood,
            Element::Wood,
            Element::Fire,
            Element::Fire,
            Element::Earth,
            Element::Earth,
            Element::Metal,
            Element::Metal,
            Element::Water, // 9 與 0 同屬水（壬癸）
        ];
        for (d, e) in expected.iter().enumerate() {
            assert_eq!(element_of(d as u16), *e, "digit {d}");
        }
    }

    #[test]
    fn element_invariant_under_wraparound() {
        // 減 80 不改個位數：同一數理的五行在 wrap 前後一致
        for n in [7u16, 87, 167, 247] {
            assert_eq!(element_of(n), Element::Metal, "n={n}");
        }
        for n in [10u16, 90, 170] {
            assert_eq!(element_of(n), Element::Water, "n={n}");
        }
    }

    #[test]
    fn generates_follows_cycle() {
        // 木生火 火生土 土生金 金生水 水生木；其餘不算相生
        assert!(generates(Element::Wood, Element::Fire));
        assert!(generates(Element::Fire, Element::Earth));
        assert!(generates(Element::Earth, Element::Metal));
        assert!(generates(Element::Metal, Element::Water));
        assert!(generates(Element::Water, Element::Wood));
        // 反方向不算（我生 vs 生我由 pair_relation 統一處理）
        assert!(!generates(Element::Fire, Element::Wood));
        assert!(!generates(Element::Wood, Element::Metal));
        assert!(!generates(Element::Wood, Element::Earth));
        assert!(!generates(Element::Wood, Element::Water));
    }

    #[test]
    fn overcomes_follows_cycle() {
        // 木剋土 土剋水 水剋火 火剋金 金剋木
        assert!(overcomes(Element::Wood, Element::Earth));
        assert!(overcomes(Element::Earth, Element::Water));
        assert!(overcomes(Element::Water, Element::Fire));
        assert!(overcomes(Element::Fire, Element::Metal));
        assert!(overcomes(Element::Metal, Element::Wood));
        assert!(!overcomes(Element::Earth, Element::Wood));
        assert!(!overcomes(Element::Wood, Element::Fire));
    }

    #[test]
    fn pair_relation_classifies_three_kinds() {
        assert_eq!(
            pair_relation(Element::Wood, Element::Wood),
            PairRelation::Same
        );
        // 相生任一方向皆和諧
        assert_eq!(
            pair_relation(Element::Wood, Element::Fire),
            PairRelation::Harmonious
        );
        assert_eq!(
            pair_relation(Element::Fire, Element::Wood),
            PairRelation::Harmonious
        );
        // 相剋任一方向皆衝突
        assert_eq!(
            pair_relation(Element::Metal, Element::Wood),
            PairRelation::Conflicting
        );
        assert_eq!(
            pair_relation(Element::Wood, Element::Metal),
            PairRelation::Conflicting
        );
    }

    #[test]
    fn sancai_rating_snapshots() {
        use Element as E;
        // 手算 fixture（與五格 fixture 連動）
        let t = |a: E, b: E, c: E| sancai_rating([a, b, c]);
        // 王小明 土金木：土生金=和、金剋木=衝 → 半吉
        assert_eq!(t(E::Earth, E::Metal, E::Wood), LuckClass::BanJi);
        // 李白 金木土：金剋木、木剋土 → 凶
        assert_eq!(t(E::Metal, E::Wood, E::Earth), LuckClass::Xiong);
        // 歐陽鵬 木土水：木剋土、土剋水 → 凶
        assert_eq!(t(E::Wood, E::Earth, E::Water), LuckClass::Xiong);
        // 歐陽小明 木水木：水生木、水生木 → 吉
        assert_eq!(t(E::Wood, E::Water, E::Wood), LuckClass::Ji);
        // 失真代表例（古表 vs 簡表，spec §2）
        // 金金金：古表多凶，簡表=吉（兩對比和）
        assert_eq!(t(E::Metal, E::Metal, E::Metal), LuckClass::Ji);
        // 木木土：古表常大吉，簡表=半吉（木木=和、木剋土=衝）
        assert_eq!(t(E::Wood, E::Wood, E::Earth), LuckClass::BanJi);
        // 兩和諧：木生火+火生土 → 吉
        assert_eq!(t(E::Wood, E::Fire, E::Earth), LuckClass::Ji);
        // 一和一衝的另一半：金水木（金生水=和、水生木=和）→ 吉（洩氣局失真例）
        assert_eq!(t(E::Metal, E::Water, E::Wood), LuckClass::Ji);
        // 全衝的另一形：水火金（水剋火、火剋金）→ 凶
        assert_eq!(t(E::Water, E::Fire, E::Metal), LuckClass::Xiong);
    }

    #[test]
    fn all_125_configs_are_rated_without_daji() {
        let all = [
            Element::Wood,
            Element::Fire,
            Element::Earth,
            Element::Metal,
            Element::Water,
        ];
        for a in all {
            for b in all {
                for c in all {
                    let r = sancai_rating([a, b, c]);
                    assert!(
                        r == LuckClass::Ji || r == LuckClass::BanJi || r == LuckClass::Xiong,
                        "{a:?}{b:?}{c:?} got {r:?}"
                    );
                }
            }
        }
    }
}
