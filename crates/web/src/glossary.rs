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
    Tarot,
    Palm,
    Naming,
    SelfExplore,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Ziwei => "紫微斗數",
            Category::ZiweiPattern => "紫微格局",
            Category::Western => "西洋占星",
            Category::Tarot => "塔羅",
            Category::Palm => "手相",
            Category::Naming => "姓名學",
            Category::SelfExplore => "自我探索",
        }
    }
}

/// 一條名詞：詞＋一行定義。
pub struct Term {
    pub term: &'static str,
    pub category: Category,
    pub def: &'static str,
}

/// 詞條版本（2026-09-18 由初稿定案；後續 stakeholder 意見以修訂版處理並 bump。
/// 2026-09-22 glossary-1.1：「日月並明」雙棟之才 → 棟梁之才，無出處自造詞修正。
/// 2026-10-27 glossary-1.2：加塔羅 7 條——塔羅上線後名詞解釋未涵蓋（使用者指出）。
/// 2026-10-28 glossary-1.3：加手相 9 條（手相 v1 同日上線））。
/// 2026-10-31 glossary-1.4：新增「自我探索」分類＋興趣探索 2 條（興趣探索 spec §4.3）。
pub const GLOSSARY_VERSION: &str = "glossary-1.4";

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
        def: "太陽太陰皆處廟旺明亮的組合，主聲名與棟梁之才。",
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
    // 塔羅（glossary-1.2——站上實際使用的牌組與玩法）
    Term {
        term: "塔羅牌",
        category: Category::Tarot,
        def: "78 張牌組的占卜工具，抽牌對應當下處境，作為反思的引子。",
    },
    Term {
        term: "大阿爾克那",
        category: Category::Tarot,
        def: "22 張主牌，從愚者到世界，象徵人生旅程的重大主題。",
    },
    Term {
        term: "小阿爾克那",
        category: Category::Tarot,
        def: "權杖、聖杯、寶劍、錢幣四組花色各 14 張，對應日常生活的具體事件。",
    },
    Term {
        term: "正位",
        category: Category::Tarot,
        def: "牌面正立抽出，牌義依原本方向解讀。",
    },
    Term {
        term: "逆位",
        category: Category::Tarot,
        def: "牌面倒立抽出，常解讀為能量的受阻或內化，邀請你換個角度看。",
    },
    Term {
        term: "牌陣",
        category: Category::Tarot,
        def: "抽牌的位置安排，每個位置代表問題的一個切面。",
    },
    Term {
        term: "三張牌陣",
        category: Category::Tarot,
        def: "現況、阻礙、建議三張的入門牌陣，本站三張牌解牌採用的形式。",
    },
    // 手相（glossary-1.3——站上特徵區實際呈現的線名/掌丘/手型；措辭「對照」貼合非宣判式語料）
    Term {
        term: "手相",
        category: Category::Palm,
        def: "從掌紋與手型描繪傾向的傳統相術，本站以拍照抽取特徵、規則式組裝解讀。",
    },
    Term {
        term: "生命線",
        category: Category::Palm,
        def: "環繞拇指的弧線，傳統上對照生命力節奏與身心韌性。",
    },
    Term {
        term: "智慧線",
        category: Category::Palm,
        def: "橫越掌心的線，對照思考風格與做決定的方式。",
    },
    Term {
        term: "感情線",
        category: Category::Palm,
        def: "掌心上緣橫向的線，對照情感的表達與讓人靠近的方式。",
    },
    Term {
        term: "命運線",
        category: Category::Palm,
        def: "縱走掌心的線，對照方向感與階段耕耘；不少人天生不明顯，與命好不好無關。",
    },
    Term {
        term: "婚姻線",
        category: Category::Palm,
        def: "感情線上方的短橫線，對照對親密關係的態度，而非婚姻事件本身。",
    },
    Term {
        term: "太陽線",
        category: Category::Palm,
        def: "無名指下方縱走的線，對照自我實現與被什麼點亮。",
    },
    Term {
        term: "掌丘",
        category: Category::Palm,
        def: "掌心各處的隆起（金星丘、木星丘等），對照不同層面能量的飽滿程度。",
    },
    Term {
        term: "手型",
        category: Category::Palm,
        def: "火、土、風、水四型手掌分類，對照氣質傾向（如火型掌掌長指短、行動力強）。",
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
    // 自我探索（興趣探索 v1）
    Term {
        term: "RIASEC",
        category: Category::SelfExplore,
        def: "Holland 的六型興趣分類：實作、研究、藝術、社會、企業、事務，描述人比較喜歡哪類活動。",
    },
    Term {
        term: "興趣 vs 能力",
        category: Category::SelfExplore,
        def: "興趣是比較喜歡做什麼，能力是做得多好，兩者是不同的東西；興趣探索只看前者。",
    },
];
