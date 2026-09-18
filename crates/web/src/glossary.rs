//! 命理名詞解釋 — 靜態詞條（generation.rs 先例：小模組＋頁面渲染）。
//!
//! 全文鎖定於 docs/superpowers/specs/2026-09-18-naming-v1-design.md §5（22 條）。
//! 2026-09-18 定案（glossary-1）；後續 ping225710 意見以修訂版處理。ft-web 無測試
//! 慣例，詞條正確性由 spec 審訂流程把關。

/// 詞條分類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Ziwei,
    ZiweiPattern,
    Western,
    Naming,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Ziwei => "紫微斗數",
            Category::ZiweiPattern => "紫微格局",
            Category::Western => "西洋占星",
            Category::Naming => "姓名學",
        }
    }
}

/// 一條名詞：詞＋一行定義。
pub struct Term {
    pub term: &'static str,
    pub category: Category,
    pub def: &'static str,
}

/// 詞條版本（2026-09-18 由初稿定案；後續 stakeholder 意見以修訂版處理並 bump）。
pub const GLOSSARY_VERSION: &str = "glossary-1";

/// 詞條按分類排列。
pub static TERMS: &[Term] = &[
    // 紫微斗數（基礎）
    Term {
        term: "命宮",
        category: Category::Ziwei,
        def: "紫微命盤十二宮之首，代表整體格局與主要性格取向。",
    },
    Term {
        term: "身宮",
        category: Category::Ziwei,
        def: "與命宮互補的宮位，指向後天著力與身體力行的領域。",
    },
    Term {
        term: "四化",
        category: Category::Ziwei,
        def: "化祿、化權、化科、化忌四種星曜變化，標示能量的流向。",
    },
    Term {
        term: "五行局",
        category: Category::Ziwei,
        def: "命盤所屬的五行局數，決定大限起運歲數。",
    },
    Term {
        term: "大限",
        category: Category::Ziwei,
        def: "十年一階段的運勢區間，類似西洋占星的十年大運。",
    },
    Term {
        term: "借星",
        category: Category::Ziwei,
        def: "對宮無主星時借對宮星曜論斷的技法。",
    },
    // 紫微格局
    Term {
        term: "極居卯酉格",
        category: Category::ZiweiPattern,
        def: "紫微貪狼同坐卯酉宮的格局，才藝出眾、交際能力強。",
    },
    Term {
        term: "殺破狼",
        category: Category::ZiweiPattern,
        def: "七殺、破軍、貪狼三曜互相會照的組合，主開創與變動。",
    },
    Term {
        term: "機月同梁",
        category: Category::ZiweiPattern,
        def: "天機、太陰、天同、天梁組合，主企劃輔佐、穩中求進。",
    },
    Term {
        term: "君臣慶會",
        category: Category::ZiweiPattern,
        def: "帝星與輔弼諸吉同度會照，主得貴人助、團隊成就。",
    },
    Term {
        term: "日月並明",
        category: Category::ZiweiPattern,
        def: "太陽太陰皆處廟旺明亮的組合，主聲名與多元之才。",
    },
    Term {
        term: "火貪格",
        category: Category::ZiweiPattern,
        def: "火星與貪狼同宮會照的爆發格局，主橫發但也須防起伏。",
    },
    // 西洋占星
    Term {
        term: "上升星座",
        category: Category::Western,
        def: "出生時東方地平線升起的星座，影響外在形象與第一印象。",
    },
    Term {
        term: "太陽星座",
        category: Category::Western,
        def: "一般俗稱的星座，代表核心自我與人生主題。",
    },
    Term {
        term: "月亮星座",
        category: Category::Western,
        def: "出生時月亮所在星座，反映情緒反應與內在需求。",
    },
    Term {
        term: "相位",
        category: Category::Western,
        def: "兩顆行星之間的角度關係，如 0 度合相、90 度刑相位。",
    },
    Term {
        term: "合相",
        category: Category::Western,
        def: "兩星交會在同一位置（0 度），能量融合放大的相位。",
    },
    Term {
        term: "宮位",
        category: Category::Western,
        def: "星盤十二個人生領域區塊，如第一宮自我、第七宮伴侶。",
    },
    // 姓名學
    Term {
        term: "五格",
        category: Category::Naming,
        def: "天、人、地、外、總五個數理格局，五格剖象法的核心。",
    },
    Term {
        term: "三才",
        category: Category::Naming,
        def: "天格、人格、地格三格五行的配置關係。",
    },
    Term {
        term: "康熙筆畫",
        category: Category::Naming,
        def: "五格計算採用的《康熙字典》字畫標準，與日常筆畫不盡相同。",
    },
    Term {
        term: "81 數理",
        category: Category::Naming,
        def: "1 至 81 每個數字各有吉凶定評，超過 81 減 80 循環查表。",
    },
];
