//! 結果頁「你的房間」縮景：答案 → 零件（spec
//! docs/superpowers/specs/2026-11-22-fengshui-diorama-design.md，§7 優先）。
//!
//! 每個可畫的選項對應零件（`diorama_parts.rs`，由 `scripts/sprite-stack/fengshui_parts.py`
//! 產生）；前端把回傳的零件按層疊起來。縮景只重現使用者描述的東西：`unknown`、未作答、
//! 「沒有」類選項都不畫，不標示好壞。

use std::collections::BTreeMap;

use super::diorama_parts::{DioramaPart, DIORAMA_PARTS};
use super::ruleset::questions;
use super::Room;

/// 改零件或答案對照必 bump（目前僅記錄用）。
pub const DIORAMA_VERSION: &str = "1";

/// 縮景說明（spec §7.9）。
pub const DIORAMA_CAPTION: &str =
    "依你的答案組出的示意圖：只畫你描述到的東西，『不確定』和『沒有』不畫";

/// (題目 id, 選項 id, 零件 id)。畫不出東西的選項寫空陣列；`unknown` 不列。
pub const MAPPING: &[(&str, &str, &[&str])] = &[
    // ── 臥室 ──
    ("bed_back", "wall", &["bed-back-wall"]),
    ("bed_back", "window", &["bed-back-window"]),
    ("bed_back", "passage", &["bed-back-door"]),
    ("bed_back", "floating", &["bed-floating"]),
    ("bed_door_view", "visible", &["door-diag"]),
    ("bed_door_view", "facing", &["door-foot"]),
    ("bed_door_view", "hidden", &["door-behind"]),
    ("bed_overhead", "none", &[]),
    ("bed_overhead", "beam", &["beam"]),
    ("bed_overhead", "heavy", &["shelf-over"]),
    ("bed_mirror", "none", &[]),
    ("bed_mirror", "visible", &["mirror"]),
    ("bed_corner", "none", &[]),
    ("bed_corner", "present", &["corner"]),
    ("bed_door_door", "none", &[]),
    // bed_door_door=facing 的零件依房門位置而定，見 MAPPING_DEPENDENT。
    ("bed_door_door", "facing", &[]),
    ("bed_light", "dark", &[]),
    ("bed_light", "leak", &["window-glow"]),
    ("bed_clutter", "little", &[]),
    ("bed_clutter", "some", &["clutter-some"]),
    ("bed_clutter", "lots", &["clutter-lots"]),
    // ── 書房（在家）──
    ("seat_back", "wall", &["back-wall"]),
    ("seat_back", "window", &["back-window"]),
    ("seat_back", "passage", &["back-door"]),
    ("seat_door_view", "visible", &["door-diag"]),
    ("seat_door_view", "facing", &["door-front"]),
    ("seat_door_view", "hidden", &["door-behind"]),
    ("seat_overhead", "none", &[]),
    ("seat_overhead", "beam", &["beam"]),
    ("seat_front", "open", &["front-open"]),
    ("seat_front", "wall", &["front-wall"]),
    ("seat_corner", "none", &[]),
    ("seat_corner", "present", &["corner"]),
    ("seat_light", "enough", &["sun-patch"]),
    ("seat_light", "dim", &[]),
    ("seat_clutter", "little", &[]),
    ("seat_clutter", "some", &["clutter-some"]),
    ("seat_clutter", "lots", &["clutter-lots"]),
    // ── 辦公室座位 ──
    ("office_back", "wall", &["back-wall"]),
    ("office_back", "aisle", &["back-aisle"]),
    ("office_back", "low", &["back-low"]),
    ("office_door", "none", &[]),
    ("office_door", "entrance", &["door-front"]),
    ("office_door", "boss", &["boss-desk"]),
    ("office_front", "open", &["front-open"]),
    ("office_front", "close", &["front-partition"]),
    ("office_overhead", "none", &[]),
    ("office_overhead", "beam", &["beam"]),
    ("office_overhead", "lamp", &["lamp-tube"]),
    ("office_corner", "none", &[]),
    ("office_corner", "present", &["corner"]),
    ("office_light", "enough", &["window-side"]),
    ("office_light", "dim", &[]),
    ("office_clutter", "little", &[]),
    ("office_clutter", "some", &["clutter-some"]),
    ("office_clutter", "lots", &["clutter-lots"]),
];

/// 依另一題答案決定零件的選項（spec §7.5）。依賴題未作答、unknown 或不在清單內時不畫。
/// (題目, 選項, 依賴題目, [(依賴選項, 零件)])。
pub type DependentMapping = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

/// 依賴對照表（目前只有臥室「房門和其他房門正對」）。
pub const MAPPING_DEPENDENT: &[DependentMapping] = &[(
    "bed_door_door",
    "facing",
    "bed_door_view",
    &[
        ("visible", "door-opposite-diag"),
        ("facing", "door-opposite-foot"),
        ("hidden", "door-opposite-behind"),
    ],
)];

/// 每個房型一律畫的零件。
pub const FIXED_PARTS: &[(Room, &[&str])] = &[
    (Room::Bedroom, &["base", "bed"]),
    (Room::Study, &["base", "desk", "chair"]),
    (Room::Office, &["base", "desk", "chair"]),
];

/// (房型, 零件, 被取代的固定零件)：床沒靠牆時 `bed-floating` 取代 `bed`。
pub const REPLACES: &[(Room, &str, &str)] = &[(Room::Bedroom, "bed-floating", "bed")];

/// 依房型與答案組出要畫的零件，順序＝`DIORAMA_PARTS` 順序（base 先）。
/// 只看該房型的題目；選項不在對照表（含 unknown、不存在的 id）一律不畫。
pub fn diorama_parts(room: Room, answers: &BTreeMap<String, String>) -> Vec<&'static DioramaPart> {
    let mut ids: Vec<&str> = FIXED_PARTS
        .iter()
        .filter(|(r, _)| *r == room)
        .flat_map(|(_, ps)| ps.iter().copied())
        .collect();
    for q in questions(room) {
        let Some(oid) = answers.get(q.id) else {
            continue;
        };
        if let Some((_, _, ps)) = MAPPING.iter().find(|(qid, o, _)| *qid == q.id && *o == oid) {
            ids.extend(ps.iter().copied());
        }
        for (_, _, dep_q, table) in MAPPING_DEPENDENT
            .iter()
            .filter(|(qid, o, _, _)| *qid == q.id && *o == oid)
        {
            let dep = answers.get(*dep_q).map(String::as_str);
            if let Some((_, part)) = table.iter().find(|(d, _)| Some(*d) == dep) {
                ids.push(part);
            }
        }
    }
    for (r, new, old) in REPLACES {
        if *r == room && ids.contains(new) {
            ids.retain(|id| id != old);
        }
    }
    DIORAMA_PARTS
        .iter()
        .filter(|p| p.room == room && ids.contains(&p.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::diorama_parts::DIORAMA_GRID;
    use super::super::ruleset::{question, UNKNOWN_OPTION_ID};
    use super::super::QUESTIONS;
    use super::*;
    use std::collections::HashSet;

    fn answers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(q, o)| (q.to_string(), o.to_string()))
            .collect()
    }

    fn ids(room: Room, a: &BTreeMap<String, String>) -> Vec<&'static str> {
        diorama_parts(room, a).iter().map(|p| p.id).collect()
    }

    fn part_exists(room: Room, id: &str) -> bool {
        DIORAMA_PARTS.iter().any(|p| p.room == room && p.id == id)
    }

    fn fixed(room: Room) -> &'static [&'static str] {
        FIXED_PARTS.iter().find(|(r, _)| *r == room).unwrap().1
    }

    #[test]
    fn mapping_pairs_exist_in_questions_and_are_not_unknown() {
        for (qid, oid, _) in MAPPING {
            let q = question(qid).unwrap_or_else(|| panic!("unknown question {qid}"));
            assert!(q.options.iter().any(|o| o.id == *oid), "{qid}/{oid}");
            assert_ne!(*oid, UNKNOWN_OPTION_ID, "{qid}: unknown must not be mapped");
        }
        for (qid, oid, dep_q, table) in MAPPING_DEPENDENT {
            let q = question(qid).unwrap();
            assert!(q.options.iter().any(|o| o.id == *oid), "{qid}/{oid}");
            let d = question(dep_q).unwrap();
            assert_eq!(d.room, q.room, "{qid} depends on another room's question");
            for (dep_o, _) in *table {
                assert!(d.options.iter().any(|o| o.id == *dep_o), "{dep_q}/{dep_o}");
            }
        }
    }

    #[test]
    fn every_question_option_is_mapped() {
        // 新增題目或選項沒寫對照會紅燈
        for q in QUESTIONS {
            for o in q.options.iter().filter(|o| o.id != UNKNOWN_OPTION_ID) {
                let n = MAPPING
                    .iter()
                    .filter(|(qid, oid, _)| *qid == q.id && *oid == o.id)
                    .count();
                assert_eq!(
                    n, 1,
                    "{}/{} must appear exactly once in MAPPING",
                    q.id, o.id
                );
            }
        }
    }

    #[test]
    fn mapped_parts_exist() {
        for (qid, oid, ps) in MAPPING {
            let room = question(qid).unwrap().room;
            for p in *ps {
                assert!(part_exists(room, p), "{qid}/{oid} → missing part {p}");
            }
        }
        for (qid, _, _, table) in MAPPING_DEPENDENT {
            let room = question(qid).unwrap().room;
            for (_, p) in *table {
                assert!(part_exists(room, p), "{qid} → missing part {p}");
            }
        }
        for (room, ps) in FIXED_PARTS {
            for p in *ps {
                assert!(part_exists(*room, p), "fixed part {p} missing");
            }
        }
        for (room, new, old) in REPLACES {
            assert!(part_exists(*room, new) && fixed(*room).contains(old));
        }
    }

    #[test]
    fn every_part_is_reachable() {
        for p in DIORAMA_PARTS {
            let used = fixed(p.room).contains(&p.id)
                || MAPPING.iter().any(|(qid, _, ps)| {
                    question(qid).unwrap().room == p.room && ps.contains(&p.id)
                })
                || MAPPING_DEPENDENT.iter().any(|(qid, _, _, t)| {
                    question(qid).unwrap().room == p.room && t.iter().any(|(_, x)| *x == p.id)
                });
            assert!(used, "{:?}/{} is never drawn", p.room, p.id);
        }
    }

    #[test]
    fn every_non_unknown_option_yields_existing_parts() {
        for q in QUESTIONS {
            for o in q.options.iter().filter(|o| o.id != UNKNOWN_OPTION_ID) {
                for p in diorama_parts(q.room, &answers(&[(q.id, o.id)])) {
                    assert_eq!(p.room, q.room);
                    assert!(DIORAMA_PARTS.contains(p));
                }
            }
        }
    }

    #[test]
    fn unknown_or_missing_draws_only_fixed_parts() {
        for room in Room::ALL {
            assert_eq!(ids(room, &BTreeMap::new()), fixed(room));
            let all_unknown: BTreeMap<String, String> = questions(room)
                .map(|q| (q.id.to_string(), UNKNOWN_OPTION_ID.to_string()))
                .collect();
            assert_eq!(ids(room, &all_unknown), fixed(room));
            let bogus: BTreeMap<String, String> = questions(room)
                .map(|q| (q.id.to_string(), "no-such-option".to_string()))
                .collect();
            assert_eq!(ids(room, &bogus), fixed(room));
        }
    }

    #[test]
    fn floating_bed_replaces_bed() {
        let got = ids(Room::Bedroom, &answers(&[("bed_back", "floating")]));
        assert!(got.contains(&"bed-floating"));
        assert!(!got.contains(&"bed"));
        let got = ids(Room::Bedroom, &answers(&[("bed_back", "wall")]));
        assert!(got.contains(&"bed") && !got.contains(&"bed-floating"));
    }

    #[test]
    fn opposite_door_follows_room_door() {
        for (view, part) in [
            ("visible", "door-opposite-diag"),
            ("facing", "door-opposite-foot"),
            ("hidden", "door-opposite-behind"),
        ] {
            let got = ids(
                Room::Bedroom,
                &answers(&[("bed_door_door", "facing"), ("bed_door_view", view)]),
            );
            let opposite: Vec<_> = got
                .iter()
                .filter(|p| p.starts_with("door-opposite"))
                .collect();
            assert_eq!(opposite, vec![&part], "{view}");
        }
        for view in [None, Some(UNKNOWN_OPTION_ID)] {
            let mut a = answers(&[("bed_door_door", "facing")]);
            if let Some(v) = view {
                a.insert("bed_door_view".into(), v.into());
            }
            assert!(!ids(Room::Bedroom, &a)
                .iter()
                .any(|p| p.starts_with("door-opposite")));
        }
    }

    #[test]
    fn other_rooms_answers_are_ignored_and_order_is_fixed() {
        let a = answers(&[
            ("bed_clutter", "lots"),
            ("seat_corner", "present"),
            ("seat_back", "wall"),
            ("office_overhead", "lamp"),
        ]);
        let study = ids(Room::Study, &a);
        assert_eq!(study.first(), Some(&"base"));
        assert!(study.contains(&"corner") && study.contains(&"back-wall"));
        assert!(!study.contains(&"clutter-lots") && !study.contains(&"lamp-tube"));
        // 順序＝DIORAMA_PARTS 順序
        let order: Vec<&str> = DIORAMA_PARTS
            .iter()
            .filter(|p| p.room == Room::Study && study.contains(&p.id))
            .map(|p| p.id)
            .collect();
        assert_eq!(study, order);
    }

    #[test]
    fn generated_parts_are_well_formed() {
        assert_eq!(DIORAMA_GRID, 40);
        let mut seen = HashSet::new();
        for p in DIORAMA_PARTS {
            assert!(
                seen.insert((p.room, p.id)),
                "duplicate part {:?}/{}",
                p.room,
                p.id
            );
            assert!(!p.layers.is_empty(), "{} has no layers", p.id);
            assert!(
                p.layers.windows(2).all(|w| w[0] < w[1]),
                "{} layers unsorted",
                p.id
            );
        }
        for room in Room::ALL {
            let first = DIORAMA_PARTS.iter().find(|p| p.room == room).unwrap();
            assert_eq!(first.id, "base", "{room:?}: base must be drawn first");
        }
    }
}
