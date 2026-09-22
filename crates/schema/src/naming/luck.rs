//! 81 數理吉凶 — 常數表與分類（v1）。
//!
//! 表的全文（81 條：吉凶四級＋數理名＋一句短評）鎖定於
//! docs/superpowers/specs/2026-09-18-naming-v1-design.md §3，本檔只謄錄。
//! 大於 81 的 wraparound（`while n > 81 { n -= 80 }`）在 mod.rs 管線套用後查表。

use serde::{Deserialize, Serialize};

/// 數理吉凶四級。來源寫「吉帶凶／凶帶吉／半吉半凶」一律歸半吉（spec §3 歸一化）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LuckClass {
    DaJi,
    Ji,
    BanJi,
    Xiong,
}

impl LuckClass {
    pub fn label(&self) -> &'static str {
        match self {
            LuckClass::DaJi => "大吉",
            LuckClass::Ji => "吉",
            LuckClass::BanJi => "半吉",
            LuckClass::Xiong => "凶",
        }
    }
}

/// 一條數理：吉凶級＋名稱＋一句短評（全文鎖定於 spec §3，本檔只謄錄）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LuckEntry {
    pub class: LuckClass,
    pub name: &'static str,
    pub text: &'static str,
}

/// 81 條數理，dense 1-based（index = 數理 − 1）。
/// 全文鎖定於 spec §3（Agy 四源交叉投票，2026-09-18）；分歧條目見 spec 分歧冊。
/// `pub(crate)`：表端一律走 `luck_entry`（契約編譯期強制，見 `luck_entry` doc）。
#[rustfmt::skip]
pub(crate) const LUCK: [LuckEntry; 81] = [
    LuckEntry { class: LuckClass::DaJi,  name: "天地開泰", text: "萬物起始之數，開創力強，宜靜中得機" },  // 1
    LuckEntry { class: LuckClass::Xiong, name: "混沌未定", text: "進退失據，親緣易離，謀事難聚" },        // 2
    LuckEntry { class: LuckClass::DaJi,  name: "進取如意", text: "天地人和，早發名利，才藝兼具" },        // 3
    LuckEntry { class: LuckClass::Xiong, name: "破敗凶變", text: "萬事休止，多變多難，宜守不宜進" },      // 4
    LuckEntry { class: LuckClass::DaJi,  name: "福祿長壽", text: "五行俱足，圓滿安定，福德集門" },        // 5
    LuckEntry { class: LuckClass::Ji,    name: "安穩餘慶", text: "天賦美德，安穩得助，女德之數" },        // 6 分歧：半吉/大吉
    LuckEntry { class: LuckClass::Ji,    name: "剛毅果斷", text: "勇往直前，剛強進取，忌過剛愎" },        // 7
    LuckEntry { class: LuckClass::Ji,    name: "意志剛健", text: "勤勉發展，貴在堅持，鐵鏡重磨" },        // 8
    LuckEntry { class: LuckClass::Xiong, name: "興盡凶始", text: "窮乏困苦，成敗兩極，破舟入海" },        // 9
    LuckEntry { class: LuckClass::Xiong, name: "萬事終局", text: "損耗飄零，諸事終結，宜守" },            // 10
    LuckEntry { class: LuckClass::DaJi,  name: "穩健吉祥", text: "枯木逢雨，富貴榮達，調順發展" },        // 11
    LuckEntry { class: LuckClass::Xiong, name: "意志薄弱", text: "家庭寂寞，掘井無泉，謀事難成" },        // 12
    LuckEntry { class: LuckClass::DaJi,  name: "智略超群", text: "博學多才，人緣早發，藝能兼備" },        // 13
    LuckEntry { class: LuckClass::Xiong, name: "淪落天涯", text: "失意煩悶，親緣薄弱，孤獨遭難" },       // 14
    LuckEntry { class: LuckClass::DaJi,  name: "福壽雙全", text: "雅量涵養，立身興家，富貴榮譽" },        // 15
    LuckEntry { class: LuckClass::DaJi,  name: "貴人相助", text: "德厚載物，興家興業，財官雙美" },        // 16
    LuckEntry { class: LuckClass::Ji,    name: "突破萬難", text: "剛柔兼備，突破困境，忌剛愎" },          // 17 分歧：半吉
    LuckEntry { class: LuckClass::Ji,    name: "有志竟成", text: "權威顯達，內含剛強，鐵鏡重磨" },        // 18
    LuckEntry { class: LuckClass::Xiong, name: "風雲蔽月", text: "辛苦重來，雖有智謀，災苦不斷" },        // 19
    LuckEntry { class: LuckClass::Xiong, name: "非業破運", text: "災禍不安，進退維谷，屋下藏金" },        // 20
    LuckEntry { class: LuckClass::DaJi,  name: "明月光照", text: "獨立權威，首領之數，官運亨通" },        // 21 傳統忌女
    LuckEntry { class: LuckClass::Xiong, name: "秋草逢霜", text: "志業難成，薄弱乏力，豪傑亦艱" },        // 22
    LuckEntry { class: LuckClass::DaJi,  name: "旭日東升", text: "質實剛堅，威勢旺盛，壯麗之數" },        // 23 傳統忌女
    LuckEntry { class: LuckClass::DaJi,  name: "家門餘慶", text: "金錢豐盈，白手成家，掘藏得金" },        // 24
    LuckEntry { class: LuckClass::Ji,    name: "英俊剛毅", text: "資性聰敏，才能奇特，忌傲慢" },          // 25
    LuckEntry { class: LuckClass::BanJi, name: "變怪奇異", text: "波瀾重疊，英雄豪俠，起伏極大" },        // 26 正規化；分歧：凶
    LuckEntry { class: LuckClass::BanJi, name: "足智多謀", text: "慾望強烈，多受毀謗，先苦後甘" },        // 27 正規化
    LuckEntry { class: LuckClass::Xiong, name: "離群獨處", text: "家親緣薄，闊水浮萍，漂泊無定" },        // 28
    LuckEntry { class: LuckClass::DaJi,  name: "智謀兼備", text: "財力歸集，智略超群，欲望難足" },        // 29 分歧：半吉；傳統忌女
    LuckEntry { class: LuckClass::BanJi, name: "一成一敗", text: "沉浮不定，吉凶難分，絕處逢生" },        // 30 正規化
    LuckEntry { class: LuckClass::DaJi,  name: "智勇得志", text: "心想事成，統領眾人，春日花開" },        // 31
    LuckEntry { class: LuckClass::Ji,    name: "權貴顯達", text: "意外惠澤，貴人得助，寶馬金鞍" },        // 32 分歧：大吉
    LuckEntry { class: LuckClass::DaJi,  name: "家門隆昌", text: "才德開展，旭日升天，名聞天下" },        // 33 傳統忌女
    LuckEntry { class: LuckClass::Xiong, name: "破家亡身", text: "凶變至極，財命危險，見識短淺" },        // 34
    LuckEntry { class: LuckClass::Ji,    name: "溫和平靜", text: "優雅發展，文昌技藝，智達通暢" },        // 35
    LuckEntry { class: LuckClass::Xiong, name: "風浪不息", text: "波瀾重疊，俠義薄運，沉浮萬狀" },        // 36
    LuckEntry { class: LuckClass::Ji,    name: "權威顯達", text: "熱誠忠信，吉人天相，猛虎出林" },        // 37 分歧：大吉
    LuckEntry { class: LuckClass::Ji,    name: "磨鐵成針", text: "刻意經營，才識不凡，終成大器" },        // 38 分歧：半吉
    LuckEntry { class: LuckClass::DaJi,  name: "富貴榮華", text: "變化無窮，暗藏險象，財帛豐盈" },        // 39 傳統忌女
    LuckEntry { class: LuckClass::BanJi, name: "謹慎保安", text: "冒險投機，沉浮不定，退安為宜" },        // 40 正規化；分歧：凶
    LuckEntry { class: LuckClass::DaJi,  name: "德高望重", text: "純陽獨秀，事事如意，和暢通達" },        // 41
    LuckEntry { class: LuckClass::BanJi, name: "寒蟬在柳", text: "博識多能，十藝九不成，宜專注" },        // 42 正規化；分歧：凶
    LuckEntry { class: LuckClass::Xiong, name: "邪途散財", text: "外祥內苦，諸事不遂，散財破產" },        // 43 分歧：半吉
    LuckEntry { class: LuckClass::Xiong, name: "須眉難展", text: "力量有限，事不如意，暗藏慘淡" },        // 44
    LuckEntry { class: LuckClass::Ji,    name: "順風揚帆", text: "新生泰和，萬事如意，智謀經緯" },        // 45 分歧：大吉
    LuckEntry { class: LuckClass::Xiong, name: "羅網繫身", text: "載寶沉舟，離祖成家，大難嘗盡" },        // 46
    LuckEntry { class: LuckClass::DaJi,  name: "點鐵成針", text: "開花結果，權威進取，點石成金" },        // 47
    LuckEntry { class: LuckClass::DaJi,  name: "青松立鶴", text: "德智兼備，出身清貴，德量榮達" },        // 48
    LuckEntry { class: LuckClass::BanJi, name: "吉凶難分", text: "轉變之數，不斷辛勞，吉凶隨環境" },      // 49 正規化
    LuckEntry { class: LuckClass::BanJi, name: "小舟入海", text: "吉凶參半，須防傾覆，先蔭後險" },        // 50 正規化；分歧：凶
    LuckEntry { class: LuckClass::BanJi, name: "盛衰交加", text: "竭力經營，慎始得安，波瀾重疊" },        // 51 正規化
    LuckEntry { class: LuckClass::DaJi,  name: "先見之明", text: "理想實現，智謀超群，達眼卓識" },        // 52
    LuckEntry { class: LuckClass::BanJi, name: "憂愁困苦", text: "外祥內患，先苦後甜，曲卷難伸" },        // 53 正規化；分歧：凶
    LuckEntry { class: LuckClass::Xiong, name: "多難悲運", text: "石上栽花，難望成功，憂悶頻來" },        // 54
    LuckEntry { class: LuckClass::BanJi, name: "外祥內苦", text: "善惡交集，盛極而衰，吉到極限" },        // 55 正規化；分歧：凶
    LuckEntry { class: LuckClass::Xiong, name: "浪裡行舟", text: "歷盡艱辛，四周障害，萬事齟齬" },        // 56
    LuckEntry { class: LuckClass::Ji,    name: "寒雪青松", text: "夜鶯吟春，最大榮運，必遭一過" },        // 57
    LuckEntry { class: LuckClass::Ji,    name: "晚行遇月", text: "先苦後甘，寬宏揚名，沉浮多端" },        // 58 分歧：半吉
    LuckEntry { class: LuckClass::Xiong, name: "寒蟬悲風", text: "時運不濟，意志衰退，缺乏忍耐" },        // 59
    LuckEntry { class: LuckClass::Xiong, name: "爭名奪利", text: "黑暗無光，無謀漂泊，晦暝難明" },        // 60
    LuckEntry { class: LuckClass::DaJi,  name: "名利雙收", text: "牡丹芙蓉，修德積福，花開富貴" },        // 61
    LuckEntry { class: LuckClass::Xiong, name: "基礎虛弱", text: "內外不和，艱難困苦，志望難達" },        // 62
    LuckEntry { class: LuckClass::DaJi,  name: "富貴榮華", text: "身心安泰，雨露惠澤，舟歸平浦" },        // 63
    LuckEntry { class: LuckClass::Xiong, name: "骨肉分離", text: "孤兒悲愁，難得心安，非命之數" },        // 64
    LuckEntry { class: LuckClass::Ji,    name: "富貴長壽", text: "光明正大，天長地久，巨流歸海" },        // 65 分歧：大吉
    LuckEntry { class: LuckClass::Xiong, name: "內外不和", text: "多欲失福，岩頭步馬，進退維谷" },        // 66
    LuckEntry { class: LuckClass::DaJi,  name: "財路亨通", text: "志氣堅強，四通八達，家道繁昌" },        // 67
    LuckEntry { class: LuckClass::DaJi,  name: "興家立業", text: "寬容好運，智慮周密，順風吹帆" },        // 68
    LuckEntry { class: LuckClass::Xiong, name: "坐立不安", text: "非業非力，災害交至，精神迫滯" },        // 69
    LuckEntry { class: LuckClass::Xiong, name: "家運衰退", text: "晚景淒涼，殘菊逢霜，寂寞愁慘" },        // 70
    LuckEntry { class: LuckClass::BanJi, name: "毫無實質", text: "耗神而勞，石上金花，貫徹始終" },        // 71 正規化
    LuckEntry { class: LuckClass::Xiong, name: "先甜後苦", text: "榮苦相伴，萬難艱辛，陰雲覆月" },        // 72 分歧：半吉
    LuckEntry { class: LuckClass::Ji,    name: "志高力微", text: "盛衰交加，徒有高志，天王福祉" },        // 73 分歧：半吉
    LuckEntry { class: LuckClass::Xiong, name: "沉淪逆境", text: "秋葉落寞，殘菊經霜，無能無智" },        // 74
    LuckEntry { class: LuckClass::BanJi, name: "守者可安", text: "發跡甚遲，退守保吉，雖有吉象" },        // 75 正規化
    LuckEntry { class: LuckClass::Xiong, name: "傾覆離散", text: "雖勞無功，內外不和，骨肉分離" },        // 76
    LuckEntry { class: LuckClass::BanJi, name: "家庭有悅", text: "半吉半凶，能獲援護，最需自律" },        // 77 正規化
    LuckEntry { class: LuckClass::BanJi, name: "晚境淒涼", text: "禍福參半，先盛後衰，晚苦之數" },        // 78 正規化；分歧：凶
    LuckEntry { class: LuckClass::Xiong, name: "挽回乏力", text: "身疲力盡，窮迫不伸，雲頭望月" },        // 79
    LuckEntry { class: LuckClass::Xiong, name: "凶星入度", text: "辛苦不絕，遁世安心，清本縮小" },        // 80
    LuckEntry { class: LuckClass::DaJi,  name: "萬物回春", text: "還本歸元，吉祥重疊，富貴尊榮" },        // 81 還本歸元本條
];

/// 大於 81 wraparound：反覆減 80 直到 ≤81（82→2、160→80、161→81）。
/// 81 查自身條目（還本歸元）；**禁用 n % 81**（82 會得 1，與共識矛盾）。
pub fn luck_index(n: u16) -> u8 {
    let mut m = n;
    while m > 81 {
        m -= 80;
    }
    m as u8
}

/// 1-based 查表：回傳數理 `n` 的條目。`n` 須為 `luck_index()` 的回傳值（1..=81）；
/// 表端一律走本函式（`LUCK` 為 `pub(crate)`，直接索引編譯不過）。反序列化路徑
/// 的越界值已由 `Grid.luckIndex` 的 Deserialize 邊界擋下，本 assert 是最後防線。
pub fn luck_entry(n: u8) -> &'static LuckEntry {
    assert!((1..=81).contains(&n), "luck_index out of range: {n}");
    &LUCK[usize::from(n) - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luck_labels_are_zh_tw() {
        assert_eq!(LuckClass::DaJi.label(), "大吉");
        assert_eq!(LuckClass::Ji.label(), "吉");
        assert_eq!(LuckClass::BanJi.label(), "半吉");
        assert_eq!(LuckClass::Xiong.label(), "凶");
    }

    #[test]
    fn luck_table_is_dense_and_fully_written() {
        assert_eq!(LUCK.len(), 81);
        for (i, e) in LUCK.iter().enumerate() {
            assert!(!e.name.is_empty(), "entry {} name empty", i + 1);
            assert!(!e.text.is_empty(), "entry {} text empty", i + 1);
        }
    }

    #[test]
    fn luck_table_spot_checks() {
        let at = |n: usize| LUCK[n - 1].class;
        // spec §3 抽查（含分歧條目：26 半吉、29 大吉、38 吉、40 半吉、51 半吉、77 半吉）
        assert_eq!(at(1), LuckClass::DaJi);
        assert_eq!(at(5), LuckClass::DaJi);
        assert_eq!(at(7), LuckClass::Ji);
        assert_eq!(at(9), LuckClass::Xiong);
        assert_eq!(at(11), LuckClass::DaJi);
        assert_eq!(at(12), LuckClass::Xiong);
        assert_eq!(at(16), LuckClass::DaJi);
        assert_eq!(at(23), LuckClass::DaJi);
        assert_eq!(at(26), LuckClass::BanJi);
        assert_eq!(at(29), LuckClass::DaJi);
        assert_eq!(at(38), LuckClass::Ji);
        assert_eq!(at(40), LuckClass::BanJi);
        assert_eq!(at(51), LuckClass::BanJi);
        assert_eq!(at(61), LuckClass::DaJi);
        assert_eq!(at(77), LuckClass::BanJi);
        assert_eq!(at(81), LuckClass::DaJi);
        // 名稱抽查（台灣系定名）
        assert_eq!(LUCK[0].name, "天地開泰");
        assert_eq!(LUCK[14].name, "福壽雙全");
        assert_eq!(LUCK[80].name, "萬物回春");
    }

    #[test]
    fn luck_classes_of_fixture_grids() {
        // 與 analyze fixture 連動：各格數理的吉凶級
        let seq = |nums: &[usize]| -> Vec<&'static str> {
            nums.iter().map(|&n| LUCK[n - 1].class.label()).collect()
        };
        // 王小明 [5,7,11,9,15]
        assert_eq!(
            seq(&[5, 7, 11, 9, 15]),
            ["大吉", "吉", "大吉", "凶", "大吉"]
        );
        // 李白 [8,12,6,2,12]
        assert_eq!(seq(&[8, 12, 6, 2, 12]), ["吉", "凶", "吉", "凶", "凶"]);
        // 歐陽鵬 [32,36,20,16,51]
        assert_eq!(
            seq(&[32, 36, 20, 16, 51]),
            ["吉", "凶", "凶", "大吉", "半吉"]
        );
        // 歐陽小明 [32,20,11,23,43]
        assert_eq!(
            seq(&[32, 20, 11, 23, 43]),
            ["吉", "凶", "大吉", "大吉", "凶"]
        );
    }

    #[test]
    fn wraparound_subtracts_80_above_81() {
        // 邊界：81 用本條、82 同 2、160→80、161→81（非 1）、162→2
        assert_eq!(luck_index(1), 1);
        assert_eq!(luck_index(80), 80);
        assert_eq!(luck_index(81), 81);
        assert_eq!(luck_index(82), 2);
        assert_eq!(luck_index(160), 80);
        assert_eq!(luck_index(161), 81);
        assert_eq!(luck_index(162), 2);
        // 241 → 161 → 81（減兩次；不是 1——「81 同 1」只是意象，不是查表規則）
        assert_eq!(luck_index(241), 81);
    }

    #[test]
    fn luck_entry_is_one_based_lookup() {
        assert_eq!(luck_entry(1).name, "天地開泰");
        assert_eq!(luck_entry(81).name, "萬物回春");
        assert_eq!(luck_entry(5).class, LuckClass::DaJi);
        // 邊界外一律 panic（0 與 82 都不是 luck_index 的合法輸出）
        assert!(std::panic::catch_unwind(|| luck_entry(0)).is_err());
        assert!(std::panic::catch_unwind(|| luck_entry(82)).is_err());
    }
}
