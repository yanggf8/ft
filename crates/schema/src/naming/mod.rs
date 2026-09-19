//! 姓名學 v1（五格剖象＋三才）— 純函式模組，無 IO，native 與 wasm32 皆可編。
//!
//! 設計與產品資料鎖定版：docs/superpowers/specs/2026-09-18-naming-v1-design.md。
//! web 直接引用本模組計算（零 API、零 DB、免登入）；表資料版本戳記見
//! `NAMING_DATA_VERSION` / `STROKE_TABLE_VERSION`（資訊性，非 api engine_version 體系）。

pub mod luck;
pub mod sancai;
pub mod strokes;

use serde::{Deserialize, Serialize};

/// 資料版本戳記（資訊性：無快取可比對，僅供 UI 顯示與審計）。
pub const NAMING_DATA_VERSION: &str = "naming-1";

/// 五格之一。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridKind {
    Heaven,
    Person,
    Earth,
    Outer,
    Total,
}

impl GridKind {
    pub fn label(&self) -> &'static str {
        match self {
            GridKind::Heaven => "天格",
            GridKind::Person => "人格",
            GridKind::Earth => "地格",
            GridKind::Outer => "外格",
            GridKind::Total => "總格",
        }
    }
}

/// 單一字與其康熙筆畫。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharStrokes {
    pub ch: char,
    pub strokes: u8,
}

/// 五格結果之一（吉凶短評由表端 `luck::luck_entry(luck_index)` 組裝，不進本結構）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Grid {
    pub kind: GridKind,
    pub strokes: u16,
    pub luck_index: u8,
    pub element: sancai::Element,
}

/// 三才配置 [天, 人, 地]。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sancai {
    pub elements: [sancai::Element; 3],
    /// 如「土金木」
    pub pattern: String,
    pub rating: luck::LuckClass,
    pub pairs: [sancai::PairRelation; 2],
}

/// 完整分析結果。**非 wire 契約**（v1 無 API）；serde 僅為鎖 roundtrip 防日後誤用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamingReport {
    pub surname: Vec<CharStrokes>,
    pub given: Vec<CharStrokes>,
    /// [天, 人, 地, 外, 總]
    pub grids: [Grid; 5],
    pub sancai: Sancai,
    pub data_version: String,
}

/// 輸入錯誤。不 serde：web 端就地映射為固定 zh-TW 訊息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingError {
    EmptySurname,
    EmptyGivenName,
    SurnameTooLong(usize),
    GivenTooLong(usize),
    NonCjk(Vec<char>),
    UnknownChars(Vec<char>),
}

/// 欄位驗證：trim 邊緣、姓/名各 1..=2 字（字數非位元組）、每字須在
/// CJK 擴A（0x3400..=0x4DBF）或基本區（0x4E00..=0x9FFF）。〇（U+3007）刻意排除。
fn validate(surname: &str, given: &str) -> Result<(), NamingError> {
    let s: Vec<char> = surname.trim().chars().collect();
    let g: Vec<char> = given.trim().chars().collect();
    if s.is_empty() {
        return Err(NamingError::EmptySurname);
    }
    if g.is_empty() {
        return Err(NamingError::EmptyGivenName);
    }
    if s.len() > 2 {
        return Err(NamingError::SurnameTooLong(s.len()));
    }
    if g.len() > 2 {
        return Err(NamingError::GivenTooLong(g.len()));
    }
    let mut non_cjk: Vec<char> = s
        .iter()
        .chain(&g)
        .copied()
        .filter(|c| !is_cjk(*c))
        .collect();
    // 同字重複（如「㐀㐀」）只報一次；排序讓訊息順序確定
    non_cjk.sort_unstable();
    non_cjk.dedup();
    if !non_cjk.is_empty() {
        return Err(NamingError::NonCjk(non_cjk));
    }
    Ok(())
}

fn is_cjk(c: char) -> bool {
    ('\u{3400}'..='\u{4DBF}').contains(&c) || ('\u{4E00}'..='\u{9FFF}').contains(&c)
}

/// 由「已解析的筆畫陣列」算五格（[天, 人, 地, 外, 總]）。
/// 公式鎖定於 spec §1；恆等式 `天+地 == 人+外` 恆成立、**總格＝各字實畫總和（不含虛畫）**。
fn five_grids(surname: &[u8], given: &[u8]) -> [u16; 5] {
    let s: Vec<u16> = surname.iter().map(|&v| v as u16).collect();
    let g: Vec<u16> = given.iter().map(|&v| v as u16).collect();
    let total: u16 = s.iter().sum::<u16>() + g.iter().sum::<u16>();
    let (heaven, person, earth, outer) = match (s.len(), g.len()) {
        // 單姓雙名：外格＝名2+1
        (1, 2) => (s[0] + 1, s[0] + g[0], g[0] + g[1], g[1] + 1),
        // 單姓單名：地格假成一（名+1）、外格固定虛畫 2
        (1, 1) => (s[0] + 1, s[0] + g[0], g[0] + 1, 2),
        // 複姓雙名：外格＝姓1+名2
        (2, 2) => (s[0] + s[1], s[1] + g[0], g[0] + g[1], s[0] + g[1]),
        // 複姓單名：外格＝姓1+1
        (2, 1) => (s[0] + s[1], s[1] + g[0], g[0] + 1, s[0] + 1),
        _ => unreachable!("validate 已保證姓/名各 1..=2 字"),
    };
    [heaven, person, earth, outer, total]
}

/// 由「已解析的字畫配對」組裝完整報告（表無關；analyze 的純組裝段）。
fn build_report(surname: &[(char, u8)], given: &[(char, u8)]) -> NamingReport {
    let s: Vec<u8> = surname.iter().map(|&(_, v)| v).collect();
    let g: Vec<u8> = given.iter().map(|&(_, v)| v).collect();
    let raw = five_grids(&s, &g);
    let kinds = [
        GridKind::Heaven,
        GridKind::Person,
        GridKind::Earth,
        GridKind::Outer,
        GridKind::Total,
    ];
    let grids = std::array::from_fn(|i| Grid {
        kind: kinds[i],
        strokes: raw[i],
        luck_index: luck::luck_index(raw[i]),
        element: sancai::element_of(raw[i]),
    });
    let elements = [grids[0].element, grids[1].element, grids[2].element];
    let pairs = [
        sancai::pair_relation(elements[0], elements[1]),
        sancai::pair_relation(elements[1], elements[2]),
    ];
    let pattern: String = elements.iter().map(|e| e.label()).collect();
    NamingReport {
        surname: surname
            .iter()
            .map(|&(ch, strokes)| CharStrokes { ch, strokes })
            .collect(),
        given: given
            .iter()
            .map(|&(ch, strokes)| CharStrokes { ch, strokes })
            .collect(),
        grids,
        sancai: Sancai {
            elements,
            pattern,
            rating: sancai::sancai_rating(elements),
            pairs,
        },
        data_version: NAMING_DATA_VERSION.to_string(),
    }
}

/// 完整分析管線：validate → 逐字查康熙筆畫（**查無此字回報該字，絕不猜**）→
/// 五格 → 81 數理 wrap → 五行 → 三才 → 報告。
pub fn analyze(surname: &str, given: &str) -> Result<NamingReport, NamingError> {
    validate(surname, given)?;
    let resolve = |field: &str| -> Vec<(char, Option<u8>)> {
        field
            .trim()
            .chars()
            .map(|ch| (ch, strokes::kangxi_strokes(ch)))
            .collect()
    };
    let s = resolve(surname);
    let g = resolve(given);
    let mut unknown: Vec<char> = s
        .iter()
        .chain(&g)
        .filter(|(_, v)| v.is_none())
        .map(|(ch, _)| *ch)
        .collect();
    // 同字重複只報一次；排序讓訊息順序確定
    unknown.sort_unstable();
    unknown.dedup();
    if !unknown.is_empty() {
        return Err(NamingError::UnknownChars(unknown));
    }
    let unwrap = |v: Vec<(char, Option<u8>)>| -> Vec<(char, u8)> {
        v.into_iter()
            .map(|(ch, v)| (ch, v.expect("unknown 已在前面擋下")))
            .collect()
    };
    Ok(build_report(&unwrap(s), &unwrap(g)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_report_assembles_wang_xiao_ming() {
        let r = build_report(&[('王', 4)], &[('小', 3), ('明', 8)]);
        // [天, 人, 地, 外, 總]＝[5, 7, 11, 9, 15]
        let kinds: Vec<&str> = r.grids.iter().map(|g| g.kind.label()).collect();
        assert_eq!(kinds, ["天格", "人格", "地格", "外格", "總格"]);
        let strokes: Vec<u16> = r.grids.iter().map(|g| g.strokes).collect();
        assert_eq!(strokes, [5, 7, 11, 9, 15]);
        // luck_index 尚未 wrap 的案例：人格 7、總格 15
        assert_eq!(r.grids[1].luck_index, 7);
        assert_eq!(r.grids[4].luck_index, 15);
        // 五行：天5土 人7金 地11木 外9水 總15土
        use sancai::Element as E;
        assert_eq!(r.grids[0].element, E::Earth);
        assert_eq!(r.grids[1].element, E::Metal);
        assert_eq!(r.grids[2].element, E::Wood);
        assert_eq!(r.grids[3].element, E::Water);
        assert_eq!(r.grids[4].element, E::Earth);
        // 三才：土金木、兩對 [土生金=和, 金剋木=衝]、半吉
        assert_eq!(r.sancai.pattern, "土金木");
        assert_eq!(r.sancai.elements, [E::Earth, E::Metal, E::Wood]);
        assert_eq!(r.sancai.pairs[0], sancai::PairRelation::Harmonious);
        assert_eq!(r.sancai.pairs[1], sancai::PairRelation::Conflicting);
        assert_eq!(r.sancai.rating, luck::LuckClass::BanJi);
        assert_eq!(r.data_version, NAMING_DATA_VERSION);
    }

    #[test]
    fn build_report_wraps_luck_index_over_81() {
        // 歐陽鵬：總格 51 不過 81；改用總格 >81 的構造案例（例：四字全 25 畫）
        let r = build_report(&[('甲', 25), ('乙', 25)], &[('丙', 25), ('丁', 25)]);
        assert_eq!(r.grids[4].strokes, 100);
        assert_eq!(r.grids[4].luck_index, 20); // 100-80
    }

    #[test]
    fn naming_report_serde_roundtrip() {
        // 非 wire 契約；鎖 roundtrip 防日後被當 API 誤用時鍵名漂移
        let r = build_report(&[('李', 7)], &[('白', 5)]);
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"dataVersion\":\"naming-1\"")); // camelCase
        let back: NamingReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn analyze_full_fixtures_with_live_table() {
        let grids_of =
            |r: &NamingReport| -> Vec<u16> { r.grids.iter().map(|g| g.strokes).collect() };

        // 王小明：天5 人7 地11 外9 總15；三才土金木 半吉
        let r = analyze("王", "小明").unwrap();
        assert_eq!(
            r.surname,
            vec![CharStrokes {
                ch: '王',
                strokes: 4
            }]
        );
        assert_eq!(grids_of(&r), [5, 7, 11, 9, 15]);
        assert_eq!(r.sancai.pattern, "土金木");
        assert_eq!(r.sancai.rating, luck::LuckClass::BanJi);

        // 李白：天8 人12 地6 外2 總12（鎖外格=2 與總格=12——防四格相加雙計）
        let r = analyze("李", "白").unwrap();
        assert_eq!(grids_of(&r), [8, 12, 6, 2, 12]);

        // 歐陽鵬：天32 人36 地20 外16 總51
        let r = analyze("歐陽", "鵬").unwrap();
        assert_eq!(grids_of(&r), [32, 36, 20, 16, 51]);

        // 歐陽小明：天32 人20 地11 外23 總43
        let r = analyze("歐陽", "小明").unwrap();
        assert_eq!(grids_of(&r), [32, 20, 11, 23, 43]);
    }

    #[test]
    fn analyze_refuses_unknown_chars_without_guessing() {
        // validate 先跑：空欄位優先於查表
        assert_eq!(analyze("", "明"), Err(NamingError::EmptySurname));
        // 表時期收緊案例：明 已知、㐀（擴A 起點）未知 → 只列 㐀
        assert_eq!(
            analyze("㐀", "明"),
            Err(NamingError::UnknownChars(vec!['㐀']))
        );
        // 同字重複只報一次
        assert_eq!(
            analyze("㐀", "㐀"),
            Err(NamingError::UnknownChars(vec!['㐀']))
        );
    }

    #[test]
    fn validate_rejects_empty_and_whitespace() {
        assert_eq!(validate("", "小明"), Err(NamingError::EmptySurname));
        assert_eq!(validate("  ", "小明"), Err(NamingError::EmptySurname));
        assert_eq!(validate("王", "　"), Err(NamingError::EmptyGivenName)); // 全形空白
        assert_eq!(validate(" 王 ", " 小明 "), Ok(())); // 邊緣 trim 後合法
    }

    #[test]
    fn validate_rejects_over_length() {
        assert_eq!(
            validate("王李張", "明"),
            Err(NamingError::SurnameTooLong(3))
        );
        assert_eq!(
            validate("王", "小明明明"),
            Err(NamingError::GivenTooLong(4))
        );
    }

    #[test]
    fn validate_rejects_non_cjk() {
        assert_eq!(validate("王O", "明"), Err(NamingError::NonCjk(vec!['O'])));
        // 長度檢查先於字元集：非 CJK 案例須 ≤2 字才會走到 NonCjk
        assert_eq!(
            validate("王", "ab"),
            Err(NamingError::NonCjk(vec!['a', 'b']))
        );
        assert_eq!(validate("〇", "明"), Err(NamingError::NonCjk(vec!['〇'])));
        // 同字重複只報一次
        assert_eq!(validate("OO", "明"), Err(NamingError::NonCjk(vec!['O'])));
        assert_eq!(validate("王", "明"), Ok(()));
    }

    #[test]
    fn validate_accepts_ext_a_but_strokes_may_later_refuse() {
        // 㐀（U+3400，CJK 擴A 起點）通過字元集驗證；筆畫查無時由 analyze 回 UnknownChars
        assert_eq!(validate("㐀", "明"), Ok(()));
    }

    #[test]
    fn five_grids_single_surname_double_given() {
        // 王小明（王4 小3 明8）：天5 人7 地11 外9 總15
        assert_eq!(five_grids(&[4], &[3, 8]), [5, 7, 11, 9, 15]);
    }

    #[test]
    fn five_grids_single_surname_single_given() {
        // 李白（李7 白5）：天8 人12 地6 外2（固定虛畫）總12（實畫和，非四格和 28）
        assert_eq!(five_grids(&[7], &[5]), [8, 12, 6, 2, 12]);
    }

    #[test]
    fn five_grids_double_surname_single_given() {
        // 歐陽鵬（歐15 陽17 鵬19）：天32 人36 地20 外16 總51
        assert_eq!(five_grids(&[15, 17], &[19]), [32, 36, 20, 16, 51]);
    }

    #[test]
    fn five_grids_double_surname_double_given() {
        // 歐陽小明（歐15 陽17 小3 明8）：天32 人20 地11 外23 總43
        assert_eq!(five_grids(&[15, 17], &[3, 8]), [32, 20, 11, 23, 43]);
    }

    #[test]
    fn five_grids_identity_heaven_earth_eq_person_outer() {
        // 恆等式：天+地 == 人+外（所有形態）——防「總格＝四格相加」的雙計陷阱
        let cases: [(&[u8], &[u8]); 4] = [
            (&[4], &[3, 8]),
            (&[7], &[5]),
            (&[15, 17], &[19]),
            (&[15, 17], &[3, 8]),
        ];
        for (s, g) in cases {
            let [t, r, d, w, _z] = five_grids(s, g);
            assert_eq!(t + d, r + w, "surname={s:?} given={g:?}");
        }
    }
}
