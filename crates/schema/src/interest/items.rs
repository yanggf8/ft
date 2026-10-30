//! 興趣探索題庫 — Armstrong, Allison & Rounds (2008) RIASEC Markers，Activities Set A。
//! DOI 10.1016/j.jvb.2008.06.003；英文原文逐字（spec 附錄 A.4，題號為 spec 自訂：
//! R→I→A→S→E→C、型內依論文排列）。中文為本站翻譯，流程與逐題紀錄見 spec 附錄 B。
//! 授權：作者實驗室頁面為非商業使用＋註明出處（spec §2.2、附錄 A.2）。
//! **改任何一題（含中文措辭）都要 bump `INTEREST_INSTRUMENT_VERSION`。**

use super::RiasecType;

/// 量表版本；POST 帶來的版本不符即 409 `STALE_INSTRUMENT`（spec §4.2）。
pub const INTEREST_INSTRUMENT_VERSION: &str = "1";

/// 題組；v1 僅 Set A（DB `CHECK(form IN ('A'))`）。全部 48 題同屬此 form。
pub const FORM: &str = "A";

/// 作答錨點（spec §2.3）：1=非常不喜歡 … 5=非常喜歡。
pub const ANSWER_ANCHORS: [&str; 5] = ["非常不喜歡", "不喜歡", "不確定", "喜歡", "非常喜歡"];

pub struct InterestItem {
    /// 1-based 題號＝附錄 A.4 題號（answers 陣列索引 = no − 1）
    pub no: usize,
    pub kind: RiasecType,
    /// 英文原文（逐字）
    pub en: &'static str,
    pub zh: &'static str,
}

pub const ITEMS: [InterestItem; 48] = [
    InterestItem {
        no: 1,
        kind: RiasecType::R,
        en: "Test the quality of parts before shipment",
        zh: "在出貨前檢驗零件品質",
    },
    InterestItem {
        no: 2,
        kind: RiasecType::R,
        en: "Lay brick or tile",
        zh: "砌磚或鋪磁磚",
    },
    InterestItem {
        no: 3,
        kind: RiasecType::R,
        en: "Work on an offshore oil-drilling rig",
        zh: "在海上石油鑽井平台工作",
    },
    InterestItem {
        no: 4,
        kind: RiasecType::R,
        en: "Assemble electronic parts",
        zh: "組裝電子零件",
    },
    InterestItem {
        no: 5,
        kind: RiasecType::R,
        en: "Operate a grinding machine in a factory",
        zh: "在工廠操作研磨機",
    },
    InterestItem {
        no: 6,
        kind: RiasecType::R,
        en: "Fix a broken faucet",
        zh: "修理壞掉的水龍頭",
    },
    InterestItem {
        no: 7,
        kind: RiasecType::R,
        en: "Assemble products in a factory",
        zh: "在工廠組裝產品",
    },
    InterestItem {
        no: 8,
        kind: RiasecType::R,
        en: "Install flooring in houses",
        zh: "在住宅鋪設地板",
    },
    InterestItem {
        no: 9,
        kind: RiasecType::I,
        en: "Study the structure of the human body",
        zh: "研究人體構造",
    },
    InterestItem {
        no: 10,
        kind: RiasecType::I,
        en: "Study animal behavior",
        zh: "研究動物行為",
    },
    InterestItem {
        no: 11,
        kind: RiasecType::I,
        en: "Do research on plants or animals",
        zh: "研究植物或動物",
    },
    InterestItem {
        no: 12,
        kind: RiasecType::I,
        en: "Develop a new medical treatment or procedure",
        zh: "開發新的治療方法或醫療程序",
    },
    InterestItem {
        no: 13,
        kind: RiasecType::I,
        en: "Conduct biological research",
        zh: "從事生物學研究",
    },
    InterestItem {
        no: 14,
        kind: RiasecType::I,
        en: "Study whales and other types of marine life",
        zh: "研究鯨魚和其他海洋生物",
    },
    InterestItem {
        no: 15,
        kind: RiasecType::I,
        en: "Work in a biology lab",
        zh: "在生物實驗室工作",
    },
    InterestItem {
        no: 16,
        kind: RiasecType::I,
        en: "Make a map of the bottom of an ocean",
        zh: "繪製海底地圖",
    },
    InterestItem {
        no: 17,
        kind: RiasecType::A,
        en: "Conduct a musical choir",
        zh: "指揮合唱團",
    },
    InterestItem {
        no: 18,
        kind: RiasecType::A,
        en: "Direct a play",
        zh: "執導一齣舞台劇",
    },
    InterestItem {
        no: 19,
        kind: RiasecType::A,
        en: "Design artwork for magazines",
        zh: "為雜誌設計美術作品",
    },
    InterestItem {
        no: 20,
        kind: RiasecType::A,
        en: "Write a song",
        zh: "創作一首歌",
    },
    InterestItem {
        no: 21,
        kind: RiasecType::A,
        en: "Write books or plays",
        zh: "寫書或劇本",
    },
    InterestItem {
        no: 22,
        kind: RiasecType::A,
        en: "Play a musical instrument",
        zh: "演奏樂器",
    },
    InterestItem {
        no: 23,
        kind: RiasecType::A,
        en: "Perform stunts for a movie or television show",
        zh: "為電影或電視節目表演特技",
    },
    InterestItem {
        no: 24,
        kind: RiasecType::A,
        en: "Design sets for plays",
        zh: "為舞台劇設計布景",
    },
    InterestItem {
        no: 25,
        kind: RiasecType::S,
        en: "Give career guidance to people",
        zh: "為他人提供職涯輔導",
    },
    InterestItem {
        no: 26,
        kind: RiasecType::S,
        en: "Do volunteer work at a non-profit organization",
        zh: "在非營利組織當志工",
    },
    InterestItem {
        no: 27,
        kind: RiasecType::S,
        en: "Help people who have problems with drugs or alcohol",
        zh: "幫助有藥物或酒精問題的人",
    },
    InterestItem {
        no: 28,
        kind: RiasecType::S,
        en: "Teach an individual an exercise routine",
        zh: "一對一教人做一套運動動作",
    },
    InterestItem {
        no: 29,
        kind: RiasecType::S,
        en: "Help people with family-related problems",
        zh: "幫助有家庭問題的人",
    },
    InterestItem {
        no: 30,
        kind: RiasecType::S,
        en: "Supervise the activities of children at a camp",
        zh: "在營隊中照看孩子們的活動",
    },
    InterestItem {
        no: 31,
        kind: RiasecType::S,
        en: "Teach children how to read",
        zh: "教小孩閱讀",
    },
    InterestItem {
        no: 32,
        kind: RiasecType::S,
        en: "Help elderly people with their daily activities",
        zh: "協助長者處理日常生活",
    },
    InterestItem {
        no: 33,
        kind: RiasecType::E,
        en: "Sell restaurant franchises to individuals",
        zh: "向個人銷售餐廳加盟權",
    },
    InterestItem {
        no: 34,
        kind: RiasecType::E,
        en: "Sell merchandise at a department store",
        zh: "在百貨公司銷售商品",
    },
    InterestItem {
        no: 35,
        kind: RiasecType::E,
        en: "Manage the operations of a hotel",
        zh: "管理飯店的營運",
    },
    InterestItem {
        no: 36,
        kind: RiasecType::E,
        en: "Operate a beauty salon or barber shop",
        zh: "經營美容院或理髮店",
    },
    InterestItem {
        no: 37,
        kind: RiasecType::E,
        en: "Manage a department within a large company",
        zh: "管理大公司裡的一個部門",
    },
    InterestItem {
        no: 38,
        kind: RiasecType::E,
        en: "Manage a clothing store",
        zh: "管理服飾店",
    },
    InterestItem {
        no: 39,
        kind: RiasecType::E,
        en: "Sell houses",
        zh: "銷售房屋",
    },
    InterestItem {
        no: 40,
        kind: RiasecType::E,
        en: "Run a toy store",
        zh: "經營玩具店",
    },
    InterestItem {
        no: 41,
        kind: RiasecType::C,
        en: "Generate the monthly payroll checks for an office",
        zh: "為辦公室製作每月薪資單",
    },
    InterestItem {
        no: 42,
        kind: RiasecType::C,
        en: "Inventory supplies using a hand-held computer",
        zh: "用手持式盤點機盤點物資",
    },
    InterestItem {
        no: 43,
        kind: RiasecType::C,
        en: "Use a computer program to generate customer bills",
        zh: "用電腦程式製作客戶帳單",
    },
    InterestItem {
        no: 44,
        kind: RiasecType::C,
        en: "Maintain employee records",
        zh: "維護員工人事資料",
    },
    InterestItem {
        no: 45,
        kind: RiasecType::C,
        en: "Compute and record statistical and other numerical data",
        zh: "計算並記錄統計及其他數值資料",
    },
    InterestItem {
        no: 46,
        kind: RiasecType::C,
        en: "Operate a calculator",
        zh: "操作計算機",
    },
    InterestItem {
        no: 47,
        kind: RiasecType::C,
        en: "Handle customers' bank transactions",
        zh: "為客戶辦理銀行交易",
    },
    InterestItem {
        no: 48,
        kind: RiasecType::C,
        en: "Keep shipping and receiving records",
        zh: "記錄出貨與收貨資料",
    },
];

#[cfg(test)]
mod tests {
    use super::super::{RiasecType, ITEMS_PER_TYPE, ITEM_COUNT};
    use super::*;

    #[test]
    fn items_numbered_1_to_48() {
        assert_eq!(ITEMS.len(), ITEM_COUNT);
        for (i, item) in ITEMS.iter().enumerate() {
            assert_eq!(item.no, i + 1);
        }
    }

    #[test]
    fn each_type_has_8_items_in_scoring_block_order() {
        // score() 依索引區塊計分：第 t 型必須恰佔 t*8 .. t*8+8
        for (i, item) in ITEMS.iter().enumerate() {
            assert_eq!(
                item.kind,
                RiasecType::ALL[i / ITEMS_PER_TYPE],
                "item {}",
                item.no
            );
        }
    }

    #[test]
    fn every_item_has_both_texts() {
        for item in ITEMS.iter() {
            assert!(!item.en.trim().is_empty(), "item {} en", item.no);
            assert!(!item.zh.trim().is_empty(), "item {} zh", item.no);
        }
    }

    #[test]
    fn zh_texts_avoid_banned_words() {
        for item in ITEMS.iter() {
            for w in super::super::BANNED_WORDS {
                assert!(!item.zh.contains(w), "item {} contains {w}", item.no);
            }
        }
    }

    #[test]
    fn anchors_are_five_point_like_scale() {
        assert_eq!(
            ANSWER_ANCHORS,
            ["非常不喜歡", "不喜歡", "不確定", "喜歡", "非常喜歡"]
        );
    }

    #[test]
    fn instrument_is_form_a_with_version() {
        assert_eq!(FORM, "A");
        assert!(!INTEREST_INSTRUMENT_VERSION.is_empty());
    }
}
