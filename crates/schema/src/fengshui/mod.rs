//! 空間風水 v1 — 房間格局 × 研究怎麼說（spec：
//! docs/superpowers/specs/2026-11-17-fengshui-room-v1-design.md）。
//!
//! 純函式模組：無 IO、native 與 wasm32 皆可編；web 直接呼叫，全程不經 API。
//! 紅線：沒有分數、總評、吉凶等級；理氣成分一律不收；兩欄並列、互不背書。

pub mod corpus;
pub mod date;
pub mod evidence;
pub mod observe;
pub mod ruleset;

pub use corpus::{card_text, Adjustment, Badge, CardText, CARDS};
pub use date::{iso_shift_days, parse_iso_ms};
pub use evidence::{evidence_by_key, Evidence, EVIDENCE};
pub use ruleset::{question, questions, QuestionDef, QuestionOption, QUESTIONS};

use std::collections::BTreeMap;

/// 題目、選項、判定改動 → bump（spec §2.7）。
pub const FENGSHUI_ROOM_RULESET_VERSION: &str = "1";

/// 使用者看得到的卡片文字、徽章、limitation、文獻語意改動 → bump（spec §2.7）。
pub const FENGSHUI_ROOM_CORPUS_VERSION: &str = "1";

/// 房型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Room {
    Bedroom,
    Study,
}

impl Room {
    pub const ALL: [Room; 2] = [Room::Bedroom, Room::Study];

    pub fn wire(&self) -> &'static str {
        match self {
            Room::Bedroom => "bedroom",
            Room::Study => "study",
        }
    }

    pub fn from_wire(s: &str) -> Option<Room> {
        Room::ALL.into_iter().find(|r| r.wire() == s)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Room::Bedroom => "臥室",
            Room::Study => "書房／工作位",
        }
    }
}

/// 單題判定狀態。順序即結果頁分組順序（flag → mild → ok → unknown）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Status {
    Flag,
    Mild,
    Ok,
    Unknown,
}

impl Status {
    pub fn wire(&self) -> &'static str {
        match self {
            Status::Flag => "flag",
            Status::Mild => "mild",
            Status::Ok => "ok",
            Status::Unknown => "unknown",
        }
    }

    /// 結果頁是否折疊（ok 收進「已符合常見建議」、unknown 收進「還不確定的項目」）。
    pub fn collapsed(&self) -> bool {
        matches!(self, Status::Ok | Status::Unknown)
    }

    /// 是否帶徽章與「可以試試看」（spec §2.1 ③④）。
    pub fn actionable(&self) -> bool {
        matches!(self, Status::Flag | Status::Mild)
    }
}

/// 研究徽章的單一軸（spec §2.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reach {
    Direct,
    Related,
    NoResearch,
}

impl Reach {
    pub const ALL: [Reach; 3] = [Reach::Direct, Reach::Related, Reach::NoResearch];

    pub fn wire(&self) -> &'static str {
        match self {
            Reach::Direct => "direct",
            Reach::Related => "related",
            Reach::NoResearch => "none",
        }
    }

    pub fn from_wire(s: &str) -> Option<Reach> {
        Reach::ALL.into_iter().find(|r| r.wire() == s)
    }

    /// 徽章文字（spec §2.2）。
    pub fn badge_label(&self) -> &'static str {
        match self {
            Reach::Direct => "有研究直接測過這個特徵",
            Reach::Related => "只有相關的空間研究",
            Reach::NoResearch => "目前沒有研究直接測過",
        }
    }

    /// 翻卡先猜的三選一選項（spec §3）。
    pub fn guess_label(&self) -> &'static str {
        match self {
            Reach::Direct => "有研究直接測過",
            Reach::Related => "只有相關研究",
            Reach::NoResearch => "沒有人研究過",
        }
    }
}

/// `assess` 的一列結果。`text` 只在 unknown 時為 `None`。
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub item_id: &'static str,
    pub prompt: &'static str,
    /// 使用者選的選項文字；未作答或選項不存在時為 `None`。
    pub answer_label: Option<&'static str>,
    pub status: Status,
    pub variant: Option<&'static str>,
    pub text: Option<&'static CardText>,
}

/// 依房型與答案（題目 id → 選項 id）產生結果卡，分組順序 flag → mild → ok → unknown，
/// 組內維持題目順序。未作答、或選項 id 不在規則集內的題目一律視為 unknown（fail-safe）。
pub fn assess(room: Room, answers: &BTreeMap<String, String>) -> Vec<Card> {
    let mut cards: Vec<Card> = questions(room)
        .map(|q| {
            let picked = answers
                .get(q.id)
                .and_then(|opt_id| q.options.iter().find(|o| o.id == opt_id));
            match picked {
                Some(o) if o.status != Status::Unknown => Card {
                    item_id: q.id,
                    prompt: q.prompt,
                    answer_label: Some(o.label),
                    status: o.status,
                    variant: o.variant,
                    text: card_text(q.id, o.status, o.variant),
                },
                _ => Card {
                    item_id: q.id,
                    prompt: q.prompt,
                    answer_label: picked.map(|o| o.label),
                    status: Status::Unknown,
                    variant: None,
                    text: None,
                },
            }
        })
        .collect();
    // stable sort：組內保留題目順序
    cards.sort_by_key(|c| c.status);
    cards
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // ── T5 assess 分組 ──

    #[test]
    fn t5_groups_flag_mild_ok_unknown_in_question_order() {
        let a = answers(&[
            ("bed_back", "wall"),
            ("bed_door_view", "hidden"),
            ("bed_overhead", "beam"),
            ("bed_mirror", "unknown"),
            ("bed_corner", "none"),
            // bed_door_door 未作答
            ("bed_light", "leak"),
            ("bed_clutter", "some"),
        ]);
        let cards = assess(Room::Bedroom, &a);
        let got: Vec<(&str, Status)> = cards.iter().map(|c| (c.item_id, c.status)).collect();
        assert_eq!(
            got,
            vec![
                ("bed_door_view", Status::Flag),
                ("bed_overhead", Status::Flag),
                ("bed_light", Status::Flag),
                ("bed_clutter", Status::Mild),
                ("bed_back", Status::Ok),
                ("bed_corner", Status::Ok),
                ("bed_mirror", Status::Unknown),
                ("bed_door_door", Status::Unknown),
            ]
        );
        // 分組序列不可回頭
        assert!(cards.windows(2).all(|w| w[0].status <= w[1].status));
        // 摺疊旗標
        assert!(cards
            .iter()
            .all(|c| c.status.collapsed() == matches!(c.status, Status::Ok | Status::Unknown)));
    }

    #[test]
    fn t5_every_question_yields_exactly_one_card() {
        for room in Room::ALL {
            let cards = assess(room, &BTreeMap::new());
            assert_eq!(cards.len(), questions(room).count());
            assert!(cards
                .iter()
                .all(|c| c.status == Status::Unknown && c.text.is_none()));
        }
    }

    #[test]
    fn t5_unknown_option_and_bogus_ids_fall_to_unknown() {
        let a = answers(&[
            ("seat_back", "lava"),
            ("seat_front", "unknown"),
            ("bed_back", "window"), // 別房型的題目被忽略
        ]);
        let cards = assess(Room::Study, &a);
        assert!(cards.iter().all(|c| c.item_id.starts_with("seat_")));
        let back = cards.iter().find(|c| c.item_id == "seat_back").unwrap();
        assert_eq!(back.status, Status::Unknown);
        assert_eq!(back.answer_label, None);
        let front = cards.iter().find(|c| c.item_id == "seat_front").unwrap();
        assert_eq!(front.status, Status::Unknown);
        assert_eq!(front.answer_label, Some("不確定"));
    }

    #[test]
    fn t5_variant_selects_its_own_card() {
        let a = answers(&[("bed_back", "passage")]);
        let cards = assess(Room::Bedroom, &a);
        let c = cards.iter().find(|c| c.item_id == "bed_back").unwrap();
        assert_eq!(c.status, Status::Flag);
        assert_eq!(c.variant, Some("passage"));
        let t = c.text.expect("flag card has text");
        assert_eq!(
            (t.item_id, t.status, t.variant),
            ("bed_back", Status::Flag, Some("passage"))
        );
    }

    #[test]
    fn wire_round_trips() {
        for r in Room::ALL {
            assert_eq!(Room::from_wire(r.wire()), Some(r));
        }
        for r in Reach::ALL {
            assert_eq!(Reach::from_wire(r.wire()), Some(r));
        }
        assert_eq!(Reach::NoResearch.wire(), "none");
    }
}
