//! 規則集（spec §1，`FENGSHUI_ROOM_RULESET_VERSION`）：兩房型 15 題。
//! 每題單選、必有「不確定」；變體只出現在 flag。改題目／選項／判定必 bump 規則集版本。

use super::{Room, Status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionOption {
    pub id: &'static str,
    pub label: &'static str,
    pub status: Status,
    pub variant: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionDef {
    pub id: &'static str,
    pub room: Room,
    pub prompt: &'static str,
    pub options: &'static [QuestionOption],
}

/// 所有題目共用的「不確定」選項 id。
pub const UNKNOWN_OPTION_ID: &str = "unknown";

const fn opt(
    id: &'static str,
    label: &'static str,
    status: Status,
    variant: Option<&'static str>,
) -> QuestionOption {
    QuestionOption {
        id,
        label,
        status,
        variant,
    }
}

const UNSURE: QuestionOption = opt(UNKNOWN_OPTION_ID, "不確定", Status::Unknown, None);

pub static QUESTIONS: &[QuestionDef] = &[
    // ── 臥室 ──
    QuestionDef {
        id: "bed_back",
        room: Room::Bedroom,
        prompt: "床頭後面是什麼？",
        options: &[
            opt("wall", "實牆", Status::Ok, None),
            opt("window", "窗", Status::Flag, Some("window")),
            opt("passage", "門或走道", Status::Flag, Some("passage")),
            opt("floating", "沒靠牆", Status::Flag, Some("floating")),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "bed_door_view",
        room: Room::Bedroom,
        prompt: "躺在床上，看得到房門嗎？",
        options: &[
            opt("visible", "看得到，門在斜前方", Status::Ok, None),
            opt("facing", "門正對床尾", Status::Flag, Some("facing")),
            opt("hidden", "看不到門", Status::Flag, Some("hidden")),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "bed_overhead",
        room: Room::Bedroom,
        prompt: "床的正上方有什麼？",
        options: &[
            opt("none", "沒有", Status::Ok, None),
            opt("beam", "外露的樑", Status::Flag, Some("beam")),
            opt("heavy", "吊櫃、層板或大型吊燈", Status::Flag, Some("heavy")),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "bed_mirror",
        room: Room::Bedroom,
        prompt: "躺著時，鏡子照得到你嗎？",
        options: &[
            opt("none", "照不到", Status::Ok, None),
            opt("visible", "照得到", Status::Flag, None),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "bed_corner",
        room: Room::Bedroom,
        prompt: "床旁邊有沒有柱角、牆角或櫃角，尖端朝著床？",
        options: &[
            opt("none", "沒有", Status::Ok, None),
            opt("present", "有", Status::Flag, None),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "bed_door_door",
        room: Room::Bedroom,
        prompt: "房門和其他房門正對嗎？",
        options: &[
            opt("none", "沒有", Status::Ok, None),
            opt("facing", "正對", Status::Flag, None),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "bed_light",
        room: Room::Bedroom,
        prompt: "晚上睡覺時，房間能暗下來嗎？",
        options: &[
            opt("dark", "能", Status::Ok, None),
            opt("leak", "外面的光會照進來", Status::Flag, None),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "bed_clutter",
        room: Room::Bedroom,
        prompt: "地面和床底下的雜物多嗎？",
        options: &[
            opt("little", "很少", Status::Ok, None),
            opt("some", "有一些", Status::Mild, None),
            opt("lots", "很多", Status::Flag, None),
            UNSURE,
        ],
    },
    // ── 書房／工作位 ──
    QuestionDef {
        id: "seat_back",
        room: Room::Study,
        prompt: "坐著時，椅背後面是什麼？",
        options: &[
            opt("wall", "實牆", Status::Ok, None),
            opt("window", "窗", Status::Flag, Some("window")),
            opt("passage", "門或走道", Status::Flag, Some("passage")),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "seat_door_view",
        room: Room::Study,
        prompt: "坐著時看得到房門嗎？",
        options: &[
            opt("visible", "在視線內，不正對", Status::Ok, None),
            opt("facing", "正對著門", Status::Flag, Some("facing")),
            opt("hidden", "背對門", Status::Flag, Some("hidden")),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "seat_overhead",
        room: Room::Study,
        prompt: "座位上方有外露的樑嗎？",
        options: &[
            opt("none", "沒有", Status::Ok, None),
            opt("beam", "有", Status::Flag, None),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "seat_front",
        room: Room::Study,
        prompt: "桌子前方是開闊的嗎？",
        options: &[
            opt("open", "開闊或有窗景", Status::Ok, None),
            opt("wall", "直接面對很近的牆", Status::Flag, None),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "seat_corner",
        room: Room::Study,
        prompt: "座位旁邊有尖角朝著你嗎？",
        options: &[
            opt("none", "沒有", Status::Ok, None),
            opt("present", "有", Status::Flag, None),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "seat_light",
        room: Room::Study,
        prompt: "白天座位的自然光夠嗎？",
        options: &[
            opt("enough", "夠", Status::Ok, None),
            opt("dim", "不太夠", Status::Flag, None),
            UNSURE,
        ],
    },
    QuestionDef {
        id: "seat_clutter",
        room: Room::Study,
        prompt: "桌面和周圍的雜物多嗎？",
        options: &[
            opt("little", "很少", Status::Ok, None),
            opt("some", "有一些", Status::Mild, None),
            opt("lots", "很多", Status::Flag, None),
            UNSURE,
        ],
    },
];

/// 某房型的題目（規則集順序）。
pub fn questions(room: Room) -> impl Iterator<Item = &'static QuestionDef> {
    QUESTIONS.iter().filter(move |q| q.room == room)
}

/// 依 id 查題。
pub fn question(id: &str) -> Option<&'static QuestionDef> {
    QUESTIONS.iter().find(|q| q.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // ── T1 規則集完整 ──

    #[test]
    fn t1_question_counts_match_spec() {
        assert_eq!(questions(Room::Bedroom).count(), 8);
        assert_eq!(questions(Room::Study).count(), 7);
        assert_eq!(QUESTIONS.len(), 15);
    }

    #[test]
    fn t1_question_ids_unique_and_prefixed_by_room() {
        let mut seen = HashSet::new();
        for q in QUESTIONS {
            assert!(seen.insert(q.id), "duplicate question id {}", q.id);
            let prefix = match q.room {
                Room::Bedroom => "bed_",
                Room::Study => "seat_",
            };
            assert!(q.id.starts_with(prefix), "{}", q.id);
        }
    }

    #[test]
    fn t1_options_well_formed() {
        for q in QUESTIONS {
            let mut ids = HashSet::new();
            let mut variants = HashSet::new();
            let unknowns = q
                .options
                .iter()
                .filter(|o| o.status == Status::Unknown)
                .count();
            assert_eq!(unknowns, 1, "{} must have exactly one 不確定", q.id);
            assert!(
                q.options.iter().any(|o| o.status == Status::Ok),
                "{} has no ok option",
                q.id
            );
            assert!(
                q.options.iter().any(|o| o.status == Status::Flag),
                "{} has no flag option",
                q.id
            );
            for o in q.options {
                assert!(ids.insert(o.id), "{}: duplicate option id {}", q.id, o.id);
                assert!(!o.label.is_empty());
                if o.status == Status::Unknown {
                    assert_eq!(o.id, UNKNOWN_OPTION_ID);
                    assert_eq!(o.label, "不確定");
                }
                // 變體只出現在 flag
                if o.variant.is_some() {
                    assert_eq!(
                        o.status,
                        Status::Flag,
                        "{}:{} variant on non-flag",
                        q.id,
                        o.id
                    );
                }
                if let Some(v) = o.variant {
                    assert!(variants.insert(v), "{}: duplicate variant {v}", q.id);
                }
            }
            // 同題的 flag 若有多個，必須全部以變體區分
            let flags: Vec<_> = q
                .options
                .iter()
                .filter(|o| o.status == Status::Flag)
                .collect();
            if flags.len() > 1 {
                assert!(flags.iter().all(|o| o.variant.is_some()), "{}", q.id);
            }
            // 同一 (狀態, 變體) 不得由兩個選項共用（否則卡片無法區分）
            let mut keys = HashSet::new();
            for o in q.options.iter().filter(|o| o.status != Status::Unknown) {
                assert!(keys.insert((o.status, o.variant)), "{}:{}", q.id, o.id);
            }
        }
    }

    #[test]
    fn t1_lookup() {
        assert_eq!(question("seat_light").map(|q| q.room), Some(Room::Study));
        assert!(question("nope").is_none());
    }
}
