//! 開運小物 v1 商品目錄（spec docs/superpowers/specs/2026-11-22-shop-interest-design.md §3）。
//!
//! 只展示＋「我有興趣」登記：沒有價格、沒有金流。小物定位是「給自己的提醒物」，
//! 文字**不得宣稱任何改運效果**（禁字由 `tests::no_banned_words` 守門）。
//! `id` 是 wire／儲存鍵（`shop_interest.item_id`），**不可改名**；下架＝自 CATALOG 移除＋bump。

/// 改任何品項文字或增刪品項必 bump；存入 `shop_interest.catalog_version`。
pub const SHOP_CATALOG_VERSION: &str = "1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopItem {
    /// wire／儲存鍵，`[a-z0-9-]+`，不可改名
    pub id: &'static str,
    pub name: &'static str,
    /// 卡片副標，一句
    pub tagline: &'static str,
    /// 卡片內文
    pub body: &'static str,
    /// 連回的站內單元路徑
    pub unit_href: &'static str,
    pub unit_label: &'static str,
    /// 圖片路徑（web 靜態資產）
    pub image: &'static str,
}

pub const CATALOG: &[ShopItem] = &[
  ShopItem {
    id: "tarot-postcards",
    name: "塔羅牌卡明信片（自選三張）",
    tagline: "把想記住的那張牌留在桌上",
    body: "用站上的塔羅卡面印成明信片，從七十八張裡自己挑三張。立在桌上或寄給朋友，提醒自己當時想通的事。",
    unit_href: "/tarot",
    unit_label: "塔羅",
    image: "/assets/shop/tarot-postcards.webp",
  },
  ShopItem {
    id: "zodiac-bookmark",
    name: "十二星座色票書籤",
    tagline: "每個星座一種顏色",
    body: "十二張書籤，用站上每個星座的配色。夾在正在讀的書裡，或送給同星座的朋友。",
    unit_href: "/zodiac",
    unit_label: "星座配對",
    image: "/assets/shop/zodiac-bookmark.webp",
  },
  ShopItem {
    id: "ziwei-stickers",
    name: "紫微主星手帳貼紙",
    tagline: "十四主星的小插圖",
    body: "紫微斗數十四顆主星各一張小貼紙，附一句性格傾向的描述。可以貼在手帳裡，標記想多練習的那一面。",
    unit_href: "/ziwei",
    unit_label: "紫微斗數",
    image: "/assets/shop/ziwei-stickers.webp",
  },
  ShopItem {
    id: "year-journal",
    name: "今年主題反思手帳",
    tagline: "一年寫四次的小冊子",
    body: "每季一頁：寫下這一季在意的事、做了什麼、下一季想調整什麼。可以搭配「今年運勢」的年度主題一起看，也可以完全只寫自己的。",
    unit_href: "/ziwei/year",
    unit_label: "今年運勢",
    image: "/assets/shop/year-journal.webp",
  },
  ShopItem {
    id: "desk-plant",
    name: "桌面小盆栽",
    tagline: "好照顧、放桌角的小植物",
    body: "一盆好照顧的小植物，放在書桌或辦公桌一角。照顧它的那幾分鐘，也是離開螢幕喘口氣的時間。",
    unit_href: "/fengshui",
    unit_label: "空間風水",
    image: "/assets/shop/desk-plant.webp",
  },
  ShopItem {
    id: "warm-lamp",
    name: "暖光小檯燈",
    tagline: "晚上看書用的柔和燈光",
    body: "一盞可調亮度的暖色小檯燈。空間風水單元也談到光線；這盞燈只是讓晚上的桌面亮一點。",
    unit_href: "/fengshui",
    unit_label: "空間風水",
    image: "/assets/shop/warm-lamp.webp",
  },
];

/// 頁首說明（spec §2）。
pub const PAGE_INTRO: &str = "還沒開賣。這些是我們考慮做的小物，按「我有興趣」讓我們知道你想要哪一個。小物是給自己的提醒，不會改變運勢。";
/// 資料說明（spec §2）。
pub const PRIVACY_NOTE: &str = "只記錄你點了哪一件，以及這個瀏覽器的一組隨機代號（用來避免重複計算、讓你可以取消）。登入時會連到你的帳號；不收姓名、信箱或付款資料。";
/// 首頁卡內文（spec §5）；標題「開運小物」由 web 寫死，是唯一允許出現「開運」的地方。
pub const HOME_CARD_BLURB: &str = "還沒開賣：看看候選小物，按「我有興趣」告訴我們（免登入）";

/// 依 id 找品項；不在目錄內（含已下架）回 None。
pub fn find(id: &str) -> Option<&'static ShopItem> {
    CATALOG.iter().find(|it| it.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec §2 禁字：文字不得宣稱改運效果、不得借研究背書、不得出現價格或促銷。
    const BANNED: &[&str] = &[
        "開運", "招財", "化煞", "擋煞", "避邪", "驅邪", "轉運", "改運", "旺", "靈驗", "加持",
        "保證", "一定", "必定", "能量", "磁場", "功效", "療效", "治療", "神奇", "好運", "幸運",
        "預購", "價格", "桃花", "貴人", "財運", "保佑", "護身", "守護", "祈福", "開光", "消災",
        "效果", "有效", "提升", "吸引", "限量", "優惠", "研究", "NT", "$", "＄",
    ];

    fn all_texts() -> Vec<(&'static str, &'static str)> {
        let mut v = Vec::new();
        for it in CATALOG {
            for t in [it.name, it.tagline, it.body, it.unit_label] {
                v.push((it.id, t));
            }
        }
        v.push(("PAGE_INTRO", PAGE_INTRO));
        v.push(("PRIVACY_NOTE", PRIVACY_NOTE));
        v.push(("HOME_CARD_BLURB", HOME_CARD_BLURB));
        v
    }

    /// 「數字＋元」金額：半形、全形、中文數字都算（單字「元」不禁，避免誤傷「單元」）。
    fn has_price(t: &str) -> bool {
        let is_num = |c: char| {
            c.is_ascii_digit()
                || ('０'..='９').contains(&c)
                || "零一二三四五六七八九十百千萬".contains(c)
        };
        let cs: Vec<char> = t.chars().collect();
        cs.windows(2).any(|w| is_num(w[0]) && w[1] == '元')
    }

    #[test]
    fn ids_unique_and_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for it in CATALOG {
            assert!(seen.insert(it.id), "duplicate id {}", it.id);
            assert!(!it.id.is_empty());
            assert!(
                it.id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "bad id {}",
                it.id
            );
        }
    }

    #[test]
    fn no_banned_words() {
        for (id, t) in all_texts() {
            for w in BANNED {
                assert!(!t.contains(w), "{id} contains banned word {w}: {t}");
            }
            assert!(!has_price(t), "{id} contains a price: {t}");
        }
    }

    #[test]
    fn price_detector_catches_variants() {
        for t in ["300元", "３００元", "三百元"] {
            assert!(has_price(t), "{t}");
        }
        assert!(!has_price("相關單元"));
    }

    #[test]
    fn paths_well_formed() {
        for it in CATALOG {
            assert!(it.unit_href.starts_with('/'), "{}", it.id);
            assert_eq!(it.image, format!("/assets/shop/{}.webp", it.id));
        }
    }

    #[test]
    fn find_known_and_unknown() {
        assert_eq!(find("desk-plant").map(|i| i.name), Some("桌面小盆栽"));
        assert!(find("nope").is_none());
        assert_eq!(CATALOG.len(), 6);
    }
}
