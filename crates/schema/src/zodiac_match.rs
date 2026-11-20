//! 星座配對規則層（階段 A，plan 2026-11-19-zodiac-match §2.2）。純函式、前後端共用；
//! web 免登入路徑直接在瀏覽器呼叫，不打 API。
//!
//! - 以**星座**為單位的簡化看法：相位由兩星座在黃道上的距離 `d` 估算，不是行星度數相位。
//! - 不給分數：六類互斥質性標籤（D1 裁示）。全部「傾向」語氣，禁用字見測試。
//! - 三列框架＝「**你的**太陽／月亮／上升 × **對方的太陽星座**」（對方只點選太陽星座）。
//! - 文案改動必 bump `ZODIAC_MATCH_CORPUS_VERSION`（階段 B 的 AI 快取鍵含此版本）。

/// 配對語料版本；改任何文案或規則必 bump。
pub const ZODIAC_MATCH_CORPUS_VERSION: &str = "2";

/// 配對 AI prompt 版本（階段 B）；改 prompt 必 bump（AI 快取鍵含此版本）。
pub const ZODIAC_MATCH_PROMPT_VERSION: &str = "2";

/// AI 快取用的星座組合鍵：`"Taurus|Aquarius|Virgo"`；無上升（未填時辰）或
/// 無法辨識的星體以 `-` 表示。輸入先正規化。
pub fn sign_key(sun: &str, moon: Option<&str>, rising: Option<&str>) -> Option<String> {
    let sun = normalize_sign(sun)?;
    let part = |x: Option<&str>| x.and_then(normalize_sign).unwrap_or("-");
    Some(format!("{}|{}|{}", sun, part(moon), part(rising)))
}

/// 結果區底部固定聲明。
pub const MATCH_DISCLAIMER: &str = "以星座為單位的簡化看法，描述互動傾向，不是關係結局。";

/// 相位說明（避免被讀成行星度數相位）。
pub const ASPECT_NOTE: &str =
    "相位依兩個星座在黃道上相隔幾個星座來估算，是以星座為單位的簡化看法，不是行星度數相位。";

/// 未填出生時辰時的上升列說明。
pub const NO_RISING_NOTE: &str = "未填出生時辰，無法判讀上升星座。";

/// 未填出生時辰時的月亮列附註。
pub const MOON_HOUR_NOTE: &str = "時辰不詳時，月亮星座可能有誤差。";

/// 12 星座標準英文名（wire 值），黃道順序。
pub fn signs() -> [&'static str; 12] {
    crate::ZODIAC
}

/// 正規化成標準英文名（`" leo "` → `Some("Leo")`）；未知回 `None`。
pub fn normalize_sign(name: &str) -> Option<&'static str> {
    let n = name.trim();
    crate::ZODIAC
        .iter()
        .copied()
        .find(|s| s.eq_ignore_ascii_case(n))
}

fn sign_index(name: &str) -> Option<usize> {
    let n = name.trim();
    crate::ZODIAC.iter().position(|s| s.eq_ignore_ascii_case(n))
}

/// 兩星座距離 `d = min(|i−j|, 12−|i−j|)`（0–6）；任一未知回 `None`。
pub fn sign_distance(a: &str, b: &str) -> Option<u8> {
    let (i, j) = (sign_index(a)?, sign_index(b)?);
    let diff = i.abs_diff(j);
    Some(diff.min(12 - diff) as u8)
}

/// 以星座距離估算的相位（台灣繁中占星術語）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    Conjunction,
    SemiSextile,
    Sextile,
    Square,
    Trine,
    Quincunx,
    Opposition,
}

impl Aspect {
    pub fn from_distance(d: u8) -> Option<Self> {
        Some(match d {
            0 => Aspect::Conjunction,
            1 => Aspect::SemiSextile,
            2 => Aspect::Sextile,
            3 => Aspect::Square,
            4 => Aspect::Trine,
            5 => Aspect::Quincunx,
            6 => Aspect::Opposition,
            _ => return None,
        })
    }

    /// 白話距離＋術語（`「隔 3 個星座（四分相）」`）。
    pub fn plain(self) -> &'static str {
        match self {
            Aspect::Conjunction => "同星座（合相）",
            Aspect::SemiSextile => "隔 1 個星座（半六分相）",
            Aspect::Sextile => "隔 2 個星座（六分相）",
            Aspect::Square => "隔 3 個星座（四分相）",
            Aspect::Trine => "隔 4 個星座（三分相）",
            Aspect::Quincunx => "隔 5 個星座（梅花相）",
            Aspect::Opposition => "正對面（對分相）",
        }
    }

    pub fn zh(self) -> &'static str {
        match self {
            Aspect::Conjunction => "合相",
            Aspect::SemiSextile => "半六分相",
            Aspect::Sextile => "六分相",
            Aspect::Square => "四分相",
            Aspect::Trine => "三分相",
            Aspect::Quincunx => "梅花相",
            Aspect::Opposition => "對分相",
        }
    }
}

/// 六類互斥質性標籤（不給分數）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchLabel {
    /// d=0
    Mirror,
    /// d=4
    Harmony,
    /// d=2
    Complement,
    /// d=6
    PolarPull,
    /// d=3
    Friction,
    /// d=1／5
    NeedsTuning,
}

impl MatchLabel {
    pub fn from_distance(d: u8) -> Option<Self> {
        Some(match d {
            0 => MatchLabel::Mirror,
            4 => MatchLabel::Harmony,
            2 => MatchLabel::Complement,
            6 => MatchLabel::PolarPull,
            3 => MatchLabel::Friction,
            1 | 5 => MatchLabel::NeedsTuning,
            _ => return None,
        })
    }

    /// 穩定 key（CSS class／日後 wire 用）。
    pub fn key(self) -> &'static str {
        match self {
            MatchLabel::Mirror => "mirror",
            MatchLabel::Harmony => "harmony",
            MatchLabel::Complement => "complement",
            MatchLabel::PolarPull => "polar",
            MatchLabel::Friction => "friction",
            MatchLabel::NeedsTuning => "tuning",
        }
    }

    pub fn zh(self) -> &'static str {
        match self {
            MatchLabel::Mirror => "熟悉的鏡子",
            MatchLabel::Harmony => "和諧",
            MatchLabel::Complement => "互補",
            MatchLabel::PolarPull => "互補也拉鋸",
            MatchLabel::Friction => "拉扯",
            MatchLabel::NeedsTuning => "需要磨合",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
      MatchLabel::Mirror => {
        "落在同一個星座，容易一眼看懂彼此的節奏，也可能一起放大同樣的盲點。"
      }
      MatchLabel::Harmony => {
        "同元素的星座，步調與價值觀傾向自然對上，相處時常有「不用多解釋」的輕鬆感。"
      }
      MatchLabel::Complement => {
        "元素彼此相生，一方的特質常能帶動另一方，適合一起嘗試新事物。"
      }
      MatchLabel::PolarPull => {
        "站在黃道的兩端，彼此身上常有自己較少展現的部分，互相欣賞與拉鋸往往同時出現。"
      }
      MatchLabel::Friction => {
        "做事的出發點不同，容易在節奏或優先順序上碰撞；說清楚各自需要什麼，摩擦也可能變成推進力。"
      }
      MatchLabel::NeedsTuning => {
        "元素與模式都不相同，彼此的語言需要時間翻譯；多一點好奇、少一點預設，比較容易找到相處方式。"
      }
    }
    }
}

/// 元素組合一句（無序 10 種）；未知元素回 `None`。
pub fn element_line(a: &str, b: &str) -> Option<&'static str> {
    let rank = |e: &str| match e {
        "Fire" => Some(0),
        "Earth" => Some(1),
        "Air" => Some(2),
        "Water" => Some(3),
        _ => None,
    };
    let (x, y) = (rank(a)?, rank(b)?);
    Some(match (x.min(y), x.max(y)) {
        (0, 0) => "兩個火象都重熱情與行動，容易一拍即合，也可能同時衝太快。",
        (1, 1) => "兩個土象都重實際與穩定，傾向一起慢慢累積，偶爾需要刻意加點新鮮感。",
        (2, 2) => "兩個風象都重交流與想法，話題不斷，落實到生活細節時可能需要多一點規劃。",
        (3, 3) => "兩個水象都重感受與連結，容易彼此體貼，也可能一起沉進情緒裡。",
        (0, 1) => "火象想先做、土象想先確認，一快一穩，節奏對齊後能互相補位。",
        (0, 2) => "火象的行動力遇上風象的點子，傾向互相助燃，聊著聊著就想出發。",
        (0, 3) => "火象直接、水象細膩，表達方式差異較大，放慢語氣比較容易聽見彼此。",
        (1, 2) => "土象重落地、風象重可能性，一個看眼前、一個看遠方，需要時間找到交集。",
        (1, 3) => "土象給安全感、水象給情感溫度，傾向形成彼此滋養的穩定組合。",
        (2, 3) => "風象講道理、水象講感受，溝通時把「我覺得」和「我認為」分開，比較不容易錯頻。",
        _ => return None,
    })
}

/// 模式組合一句（無序 6 種）；未知模式回 `None`。
pub fn modality_line(a: &str, b: &str) -> Option<&'static str> {
    let rank = |m: &str| match m {
        "Cardinal" => Some(0),
        "Fixed" => Some(1),
        "Mutable" => Some(2),
        _ => None,
    };
    let (x, y) = (rank(a)?, rank(b)?);
    Some(match (x.min(y), x.max(y)) {
        (0, 0) => "兩個本位星座都習慣主動開局，容易一起起步，也可能搶著決定方向。",
        (1, 1) => "兩個固定星座都重堅持，方向一致時很穩，意見分歧時則需要更多耐心。",
        (2, 2) => "兩個變動星座都善於調整，相處彈性大，但重要的事可能需要有人先拍板。",
        (0, 1) => "本位擅長開始、固定擅長持續，分工得宜時傾向能把想法做成果。",
        (0, 2) => "本位帶方向、變動帶彈性，傾向互相補位，也要留意計畫一改再改。",
        (1, 2) => "固定重穩定、變動重變化，一個想守、一個想換，事先講好底線會比較順。",
        _ => return None,
    })
}

/// 使用者這一側的星體（對方一律是太陽星座）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchBody {
    Sun,
    Moon,
    Rising,
}

impl MatchBody {
    pub fn zh(self) -> &'static str {
        match self {
            MatchBody::Sun => "太陽",
            MatchBody::Moon => "月亮",
            MatchBody::Rising => "上升",
        }
    }

    /// 這一列在問什麼（核心價值／情感需求／第一印象）。
    pub fn theme(self) -> &'static str {
        match self {
            MatchBody::Sun => "核心價值",
            MatchBody::Moon => "情感需求",
            MatchBody::Rising => "第一印象",
        }
    }
}

/// 一列配對結果。使用者多個星體落在同一星座時合併成一列（`bodies` 依太陽→月亮→上升）。
#[derive(Debug, Clone, PartialEq)]
pub struct MatchRow {
    pub bodies: Vec<MatchBody>,
    /// 標準英文名
    pub my_sign: &'static str,
    pub their_sign: &'static str,
    pub distance: u8,
    pub aspect: Aspect,
    pub label: MatchLabel,
    pub element_line: &'static str,
    pub modality_line: &'static str,
}

impl MatchRow {
    /// 「你的太陽＋月亮（金牛座）」
    pub fn my_part(&self) -> String {
        let names: Vec<&str> = self.bodies.iter().map(|b| b.zh()).collect();
        format!(
            "你的{}（{}）",
            names.join("＋"),
            crate::sign_zh(self.my_sign)
        )
    }

    /// 「對方的太陽（天蠍座）」
    pub fn their_part(&self) -> String {
        format!("對方的太陽（{}）", crate::sign_zh(self.their_sign))
    }

    /// 「你的月亮（水瓶座）× 對方的太陽（獅子座）」
    pub fn frame(&self) -> String {
        format!("{}× {}", self.my_part(), self.their_part())
    }

    /// 「核心價值・情感需求」
    pub fn theme(&self) -> String {
        let t: Vec<&str> = self.bodies.iter().map(|b| b.theme()).collect();
        t.join("・")
    }

    pub fn has(&self, body: MatchBody) -> bool {
        self.bodies.contains(&body)
    }
}

/// 單列配對；任一星座未知回 `None`。
pub fn match_row(body: MatchBody, my_sign: &str, their_sun: &str) -> Option<MatchRow> {
    let my = normalize_sign(my_sign)?;
    let their = normalize_sign(their_sun)?;
    let d = sign_distance(my, their)?;
    let (me_e, me_m) = crate::sign_traits(my)?;
    let (th_e, th_m) = crate::sign_traits(their)?;
    Some(MatchRow {
        bodies: vec![body],
        my_sign: my,
        their_sign: their,
        distance: d,
        aspect: Aspect::from_distance(d)?,
        label: MatchLabel::from_distance(d)?,
        element_line: element_line(me_e, th_e)?,
        modality_line: modality_line(me_m, th_m)?,
    })
}

/// 多列配對：同星座的星體合併成一列（保持首次出現順序）；未知星座的列略過。
pub fn match_rows(mine: &[(MatchBody, &str)], their_sun: &str) -> Vec<MatchRow> {
    let mut out: Vec<MatchRow> = Vec::new();
    for (body, sign) in mine {
        let Some(row) = match_row(*body, sign, their_sun) else {
            continue;
        };
        if let Some(existing) = out.iter_mut().find(|r| r.my_sign == row.my_sign) {
            existing.bodies.push(*body);
        } else {
            out.push(row);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BANNED: [&str; 5] = ["注定", "不合", "速配", "分手", "%"];

    fn all_text() -> Vec<String> {
        let mut v: Vec<String> = vec![
            MATCH_DISCLAIMER.into(),
            ASPECT_NOTE.into(),
            NO_RISING_NOTE.into(),
            MOON_HOUR_NOTE.into(),
        ];
        for a in signs() {
            for b in signs() {
                for body in [MatchBody::Sun, MatchBody::Moon, MatchBody::Rising] {
                    let r = match_row(body, a, b).expect("all known pairs resolve");
                    v.extend([
                        r.frame(),
                        r.aspect.zh().into(),
                        r.aspect.plain().into(),
                        r.theme(),
                        r.label.zh().into(),
                        r.label.line().into(),
                        r.element_line.into(),
                        r.modality_line.into(),
                    ]);
                }
            }
        }
        v
    }

    #[test]
    fn sign_key_normalizes_and_marks_missing() {
        assert_eq!(
            sign_key(" taurus", Some("AQUARIUS"), Some("Virgo")).as_deref(),
            Some("Taurus|Aquarius|Virgo")
        );
        assert_eq!(
            sign_key("Taurus", Some("Aquarius"), None).as_deref(),
            Some("Taurus|Aquarius|-")
        );
        assert_eq!(
            sign_key("Taurus", Some("bogus"), None).as_deref(),
            Some("Taurus|-|-")
        );
        assert_eq!(sign_key("bogus", None, None), None);
    }

    #[test]
    fn all_144_pairs_have_output() {
        for a in signs() {
            for b in signs() {
                let r = match_row(MatchBody::Sun, a, b).unwrap();
                assert!(!r.element_line.is_empty() && !r.modality_line.is_empty());
                assert!(r.frame().contains("你的太陽") && r.frame().contains("對方的太陽"));
            }
        }
    }

    #[test]
    fn symmetric() {
        for a in signs() {
            for b in signs() {
                let x = match_row(MatchBody::Sun, a, b).unwrap();
                let y = match_row(MatchBody::Sun, b, a).unwrap();
                assert_eq!(x.distance, y.distance);
                assert_eq!(x.aspect, y.aspect);
                assert_eq!(x.label, y.label);
                assert_eq!(x.element_line, y.element_line);
                assert_eq!(x.modality_line, y.modality_line);
            }
        }
    }

    #[test]
    fn distances_and_labels() {
        assert_eq!(sign_distance("Aries", "Aries"), Some(0));
        assert_eq!(sign_distance("Aries", "Pisces"), Some(1));
        assert_eq!(sign_distance("Aries", "Gemini"), Some(2));
        assert_eq!(sign_distance("Aries", "Cancer"), Some(3));
        assert_eq!(sign_distance("Aries", "Leo"), Some(4));
        assert_eq!(sign_distance("Aries", "Virgo"), Some(5));
        assert_eq!(sign_distance("Aries", "Libra"), Some(6));
        assert_eq!(sign_distance("Taurus", "Aquarius"), Some(3));
        let lab = |a, b| match_row(MatchBody::Sun, a, b).unwrap();
        assert_eq!(lab("Leo", "Aquarius").aspect, Aspect::Opposition);
        assert_eq!(lab("Leo", "Aquarius").label, MatchLabel::PolarPull);
        assert_eq!(lab("Leo", "Sagittarius").label, MatchLabel::Harmony);
        assert_eq!(lab("Leo", "Libra").label, MatchLabel::Complement);
        assert_eq!(lab("Leo", "Scorpio").label, MatchLabel::Friction);
        assert_eq!(lab("Leo", "Virgo").label, MatchLabel::NeedsTuning);
        assert_eq!(lab("Leo", "Capricorn").label, MatchLabel::NeedsTuning);
        assert_eq!(lab("Leo", "Capricorn").aspect, Aspect::Quincunx);
        assert_eq!(lab("Leo", "Leo").label, MatchLabel::Mirror);
        // 三分相＝同元素
        assert_eq!(
            lab("Leo", "Sagittarius").element_line,
            element_line("Fire", "Fire").unwrap()
        );
    }

    #[test]
    fn unknown_sign_is_none() {
        assert_eq!(normalize_sign("Ophiuchus"), None);
        assert_eq!(normalize_sign(""), None);
        assert_eq!(normalize_sign(" leo "), Some("Leo"));
        assert!(sign_distance("Leo", "Unknown").is_none());
        assert!(match_row(MatchBody::Moon, "Leo", "x").is_none());
        assert!(match_row(MatchBody::Moon, "", "Leo").is_none());
    }

    #[test]
    fn frame_names_both_sides() {
        let r = match_row(MatchBody::Moon, "aquarius", "LEO").unwrap();
        assert_eq!(r.frame(), "你的月亮（水瓶座）× 對方的太陽（獅子座）");
        assert_eq!(r.my_sign, "Aquarius");
        assert_eq!(r.their_sign, "Leo");
    }

    #[test]
    fn ten_element_and_six_modality_lines_distinct() {
        let els = ["Fire", "Earth", "Air", "Water"];
        let mut e = std::collections::HashSet::new();
        for a in els {
            for b in els {
                e.insert(element_line(a, b).unwrap());
            }
        }
        assert_eq!(e.len(), 10);
        let ms = ["Cardinal", "Fixed", "Mutable"];
        let mut m = std::collections::HashSet::new();
        for a in ms {
            for b in ms {
                m.insert(modality_line(a, b).unwrap());
            }
        }
        assert_eq!(m.len(), 6);
        assert!(element_line("Fire", "Metal").is_none());
    }

    #[test]
    fn distance_table() {
        use Aspect::*;
        use MatchLabel::*;
        let table = [
            (0, Conjunction, "合相", Mirror),
            (1, SemiSextile, "半六分相", NeedsTuning),
            (2, Sextile, "六分相", Complement),
            (3, Square, "四分相", Friction),
            (4, Trine, "三分相", Harmony),
            (5, Quincunx, "梅花相", NeedsTuning),
            (6, Opposition, "對分相", PolarPull),
        ];
        for (d, asp, zh, lab) in table {
            assert_eq!(Aspect::from_distance(d), Some(asp), "d={d}");
            assert_eq!(asp.zh(), zh, "d={d}");
            assert!(asp.plain().contains(zh), "d={d}");
            assert_eq!(MatchLabel::from_distance(d), Some(lab), "d={d}");
        }
        assert_eq!(Aspect::from_distance(7), None);
        assert_eq!(MatchLabel::from_distance(7), None);
        assert_eq!(Aspect::Conjunction.plain(), "同星座（合相）");
        assert_eq!(Aspect::Opposition.plain(), "正對面（對分相）");
        assert_eq!(Aspect::Quincunx.plain(), "隔 5 個星座（梅花相）");
    }

    #[test]
    fn same_sign_bodies_merge_into_one_row() {
        let mine = [
            (MatchBody::Sun, "Taurus"),
            (MatchBody::Moon, "taurus"),
            (MatchBody::Rising, "Virgo"),
        ];
        let rows = match_rows(&mine, "Scorpio");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].bodies, vec![MatchBody::Sun, MatchBody::Moon]);
        assert_eq!(rows[0].my_part(), "你的太陽＋月亮（金牛座）");
        assert_eq!(rows[0].theme(), "核心價值・情感需求");
        assert!(rows[0].has(MatchBody::Moon));
        assert_eq!(rows[1].bodies, vec![MatchBody::Rising]);
        // 未知星座的列略過
        assert_eq!(match_rows(&[(MatchBody::Sun, "x")], "Leo").len(), 0);
    }

    /// 頁面字面文案（雙引號內字串）也掃禁用字：zodiac.rs／home.rs。
    #[test]
    fn page_literals_have_no_banned_words() {
        for (name, src) in [
            ("zodiac.rs", include_str!("../../web/src/pages/zodiac.rs")),
            ("home.rs", include_str!("../../web/src/pages/home.rs")),
        ] {
            for lit in src.split('"').skip(1).step_by(2) {
                for w in BANNED {
                    assert!(!lit.contains(w), "{name} 字面文案含禁用字「{w}」：{lit}");
                }
            }
        }
    }

    #[test]
    fn no_banned_words() {
        for t in all_text() {
            for w in BANNED {
                assert!(!t.contains(w), "禁用字「{w}」出現在：{t}");
            }
        }
    }
}
