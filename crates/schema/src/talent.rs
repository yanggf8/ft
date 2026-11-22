//! 上手小實驗（天賦線 v1，路徑 c；spec docs/superpowers/specs/2026-11-22-talent-tryout-design.md，§13 優先）。
//!
//! 使用者先猜「15 分鐘內會做到第幾步」並鎖定，實際做完回來勾實際步數，看差距。
//! 宣稱者＝使用者自己；本模組只描述「猜 vs 實際」，**永不產生能力宣稱**，也不做跨題比較。
//!
//! - `id`／title／domain／steps **永不改**：要改就新 id、舊 id 自 CATALOG 移除，並 bump
//!   [`TALENT_CATALOG_VERSION`]（舊紀錄的意義不能被改寫）。
//! - 所有使用者可見固定字串都在本檔，禁字由 `tests::no_banned_words` 守門（雙語清單）。
//! - [`draw_three`] 的輸入只有「試過哪些領域」，型別上拿不到任何結果（速度訊號禁轉推薦權重）。
//! - 紅線：天賦線資料永不寫入 predictions／prediction_feedback／f8_assignments；不 import `symbolic`。

/// 增刪題目必 bump；存入 `talent_trials.catalog_version`。
pub const TALENT_CATALOG_VERSION: &str = "1";

/// 每題的步數。
pub const STEPS: u8 = 3;
/// 猜完後回報的期限（秒）：7 天。
pub const WINDOW_SECS: i64 = 7 * 86_400;
/// 顯示「猜測傾向」需要的最少已回報筆數。
pub const TENDENCY_MIN: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Domain {
    Movement,
    Hands,
    Language,
    Memory,
    Music,
    Space,
    Words,
    Drawing,
    Logic,
    Speaking,
}

impl Domain {
    pub const ALL: [Domain; 10] = [
        Domain::Movement,
        Domain::Hands,
        Domain::Language,
        Domain::Memory,
        Domain::Music,
        Domain::Space,
        Domain::Words,
        Domain::Drawing,
        Domain::Logic,
        Domain::Speaking,
    ];

    pub fn wire(self) -> &'static str {
        match self {
            Domain::Movement => "movement",
            Domain::Hands => "hands",
            Domain::Language => "language",
            Domain::Memory => "memory",
            Domain::Music => "music",
            Domain::Space => "space",
            Domain::Words => "words",
            Domain::Drawing => "drawing",
            Domain::Logic => "logic",
            Domain::Speaking => "speaking",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Domain::Movement => "身體動作",
            Domain::Hands => "手作",
            Domain::Language => "語言",
            Domain::Memory => "記憶",
            Domain::Music => "音樂",
            Domain::Space => "空間",
            Domain::Words => "文字",
            Domain::Drawing => "畫畫",
            Domain::Logic => "邏輯",
            Domain::Speaking => "表達",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrialItem {
    /// wire／儲存鍵，`[a-z0-9-]+`，永不改名
    pub id: &'static str,
    pub domain: Domain,
    pub title: &'static str,
    /// 要準備的東西（只限家中常見物品、紙筆、手機或電腦；不放外部連結）
    pub prep: &'static str,
    /// 由易到難的三步，每步可自己判定有沒有做到
    pub steps: [&'static str; 3],
}

pub const CATALOG: &[TrialItem] = &[
    TrialItem {
        id: "juggle",
        domain: Domain::Movement,
        title: "用捲好的襪子學拋接",
        prep: "三雙捲成球的襪子",
        steps: [
            "一顆單手連續拋接 10 下",
            "兩顆交叉拋接連續 10 下",
            "三顆連續拋接一輪（每顆各拋一次都接住）",
        ],
    },
    TrialItem {
        id: "dance-8",
        domain: Domain::Movement,
        title: "跟影片學一段 8 拍舞步",
        prep: "手機或電腦，自己找一支教學影片",
        steps: [
            "跟著影片做完一遍",
            "關掉影片做完一遍",
            "關掉影片連做兩遍不停",
        ],
    },
    TrialItem {
        id: "knots",
        domain: Domain::Hands,
        title: "用鞋帶學三個繩結",
        prep: "一條鞋帶或繩子，自己找教學",
        steps: [
            "不看說明打出平結",
            "不看說明打出八字結",
            "不看說明打出稱人結",
        ],
    },
    TrialItem {
        id: "crane",
        domain: Domain::Hands,
        title: "摺一隻紙鶴",
        prep: "一張正方形的紙，自己找教學",
        steps: [
            "摺到「正方基本形」",
            "摺到「鶴的基本形」（菱形、兩側已收窄）",
            "摺出能立起來的紙鶴",
        ],
    },
    TrialItem {
        id: "hangul",
        domain: Domain::Language,
        title: "認韓文的子音",
        prep: "手機或電腦，自己找一張韓文子音表",
        steps: [
            "蓋住讀音認出 5 個子音",
            "蓋住讀音認出 10 個子音",
            "蓋住讀音把 14 個子音全部認對",
        ],
    },
    TrialItem {
        id: "digits",
        domain: Domain::Memory,
        title: "背一串 30 位的數字",
        prep: "猜完之後，這頁會給你一串數字",
        steps: ["背出前 10 位", "背出前 20 位", "30 位全對"],
    },
    TrialItem {
        id: "song",
        domain: Domain::Music,
        title: "學一首沒聽過的歌的第一段",
        prep: "手機或電腦",
        steps: [
            "跟著原唱哼完旋律",
            "關掉原唱、看歌詞唱完",
            "關掉原唱、不看歌詞唱完",
        ],
    },
    TrialItem {
        id: "cube-draw",
        domain: Domain::Space,
        title: "畫立體方塊",
        prep: "紙和筆",
        steps: [
            "畫出一個能看到三個面的方塊",
            "畫出同一個方塊的另一個角度（看到不同的面）",
            "畫出三個方塊疊成階梯",
        ],
    },
    TrialItem {
        id: "haiku",
        domain: Domain::Words,
        title: "寫 5-7-5 的短詩",
        prep: "紙和筆",
        steps: [
            "寫出一首字數 5-7-5 的",
            "寫出三首",
            "把其中一首的最後一句改寫兩種不同的版本",
        ],
    },
    TrialItem {
        id: "cup-sketch",
        domain: Domain::Drawing,
        title: "畫眼前的一個杯子",
        prep: "一個杯子、紙和筆",
        steps: [
            "畫出輪廓",
            "加上明暗（亮的一面、暗的一面）",
            "把杯子收起來，憑記憶再畫一張",
        ],
    },
    TrialItem {
        id: "scratch",
        domain: Domain::Logic,
        title: "用 Scratch 讓角色動起來（需電腦）",
        prep: "電腦瀏覽器打開 Scratch 官網",
        steps: ["讓角色往前走", "碰到舞台邊緣會反彈", "按方向鍵可以控制角色"],
    },
    TrialItem {
        id: "one-minute",
        domain: Domain::Speaking,
        title: "用一分鐘介紹一個你喜歡的東西",
        prep: "手機錄音",
        steps: [
            "寫下三個重點",
            "不看稿錄一段 45–60 秒",
            "重聽一遍，沒有超過 3 秒的停頓",
        ],
    },
];

pub fn find(id: &str) -> Option<&'static TrialItem> {
    CATALOG.iter().find(|i| i.id == id)
}

// ═══ 卡點 ═══

/// 對答案時「最卡的是哪裡」（可選、單選）：(wire, 標籤)。
pub const STUCK_REASONS: &[(&str, &str)] = &[
    ("unclear", "看不懂說明"),
    ("body", "手或身體跟不上"),
    ("memory", "記不住"),
    ("time", "時間不夠"),
    ("motivation", "中途不想做了"),
    ("none", "沒有卡"),
];

pub fn stuck_label(wire: &str) -> Option<&'static str> {
    STUCK_REASONS
        .iter()
        .find(|(w, _)| *w == wire)
        .map(|(_, l)| *l)
}

// ═══ 固定文案 ═══

pub const PAGE_TITLE: &str = "上手小實驗";
pub const PAGE_INTRO: &str = "挑一個 15 分鐘的小挑戰，先猜自己會做到哪一步，做完再回來對答案。";
pub const SELF_NOTE: &str = "這是給你自己的小實驗：猜的人是你，做的人也是你。";
pub const LOGIN_PROMPT: &str = "登入後就能開始，紀錄只有你自己看得到。";
pub const HOME_CARD_BLURB: &str = "先猜自己 15 分鐘會做到哪，做完對答案";
pub const PREDICT_QUESTION: &str = "15 分鐘內，你猜你會做到第幾步？";
pub const PREDICT_LOCK_NOTE: &str = "送出後就不能改了。7 天內回來對答案。";
pub const REPORT_QUESTION: &str = "實際做到第幾步？";
pub const STUCK_QUESTION: &str = "最卡的是哪裡？（可不選）";
pub const SKIP_LABEL: &str = "這次沒做";
pub const TIMER_NOTE: &str = "計時 15 分鐘，到時間就停。";
pub const CONDITION_NOTE: &str = "這是你這一次的 15 分鐘。";
pub const SMOOTHER_TITLE: &str = "比你以為的順的那幾次";
pub const SMOOTHER_NOTE: &str =
    "只列出實際做到比你猜的多的那幾次；沒列出的，只代表那次沒超過你的猜測，或你還沒試過。";
pub const TENDENCY_TITLE: &str = "你的猜測傾向";
pub const TENDENCY_FOOTNOTE: &str = "這只是這幾次 15 分鐘的觀察，換一天、換題目都可能不同。";
pub const DELETE_NOTE: &str = "刪除後無法復原。";

/// 0 步顯示「還沒到第 1 步」，其餘「第 n 步」。
pub fn step_label(n: u8) -> String {
    if n == 0 {
        "還沒到第 1 步".to_string()
    } else {
        format!("第 {n} 步")
    }
}

/// 單筆差距句（只描述猜 vs 實際）。
pub fn gap_sentence(gap: i8) -> String {
    match gap {
        g if g > 0 => format!("比你猜的多走了 {g} 步。"),
        0 => "跟你猜的一樣。".to_string(),
        g => format!("比你猜的少走了 {} 步。", -g),
    }
}

/// 未達門檻提示。
pub fn tendency_need_sentence(need: u32) -> String {
    format!("再對 {need} 次答案就能看到你的猜測傾向。")
}

pub fn tendency_counts_sentence(under: u32, exact: u32, over: u32) -> String {
    format!("猜少了 {under} 次、剛好 {exact} 次、猜多了 {over} 次。")
}

// ═══ 規則 ═══

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TendencyKind {
    /// 猜得比做到的少
    Conservative,
    Close,
    /// 猜得比做到的多
    Optimistic,
}

impl TendencyKind {
    pub fn wire(self) -> &'static str {
        match self {
            TendencyKind::Conservative => "conservative",
            TendencyKind::Close => "close",
            TendencyKind::Optimistic => "optimistic",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "conservative" => Some(TendencyKind::Conservative),
            "close" => Some(TendencyKind::Close),
            "optimistic" => Some(TendencyKind::Optimistic),
            _ => None,
        }
    }

    pub fn sentence(self) -> &'static str {
        match self {
            TendencyKind::Conservative => "你目前常常把自己猜得保守一點。",
            TendencyKind::Close => "你目前猜得跟實際差不多。",
            TendencyKind::Optimistic => "你目前常常把自己猜得樂觀一點。",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tendency {
    pub reported: u32,
    /// 還差幾筆才達門檻（0＝已達）
    pub need: u32,
    pub under: u32,
    pub exact: u32,
    pub over: u32,
    pub kind: Option<TendencyKind>,
}

/// 差距＝實際−猜（gap>0：做到的比猜的多＝猜少了）。
pub fn gap(predicted: u8, reached: u8) -> i8 {
    reached as i8 - predicted as i8
}

/// 只吃已回報（非沒做、非過期）的差距。平均 ≥ 0.5 → 保守；≤ −0.5 → 樂觀；其餘差不多。
pub fn tendency(gaps: &[i8]) -> Tendency {
    let n = gaps.len() as u32;
    let under = gaps.iter().filter(|g| **g > 0).count() as u32;
    let exact = gaps.iter().filter(|g| **g == 0).count() as u32;
    let over = gaps.iter().filter(|g| **g < 0).count() as u32;
    let need = TENDENCY_MIN.saturating_sub(n);
    let kind = if need > 0 {
        None
    } else {
        let sum: i32 = gaps.iter().map(|g| *g as i32).sum();
        // mean ≥ 0.5 ⇔ 2·sum ≥ n（整數比較，避免浮點）
        Some(if 2 * sum >= n as i32 {
            TendencyKind::Conservative
        } else if 2 * sum <= -(n as i32) {
            TendencyKind::Optimistic
        } else {
            TendencyKind::Close
        })
    };
    Tendency {
        reported: n,
        need,
        under,
        exact,
        over,
        kind,
    }
}

/// RFC3339 UTC（`YYYY-MM-DDTHH:MM:SS[.fff]Z`）→ Unix 秒；格式不合 → None。
pub fn epoch_secs(iso: &str) -> Option<i64> {
    let b = iso.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    if !iso.ends_with('Z') {
        return None;
    }
    let num = |r: std::ops::Range<usize>| iso.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    // days_from_civil（Howard Hinnant）
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + h * 3600 + mi * 60 + s)
}

/// 猜完 ≥ 7×24 小時未回報＝過期。GET／POST／report 共用的唯一判斷。
/// 任一時間格式不合 → 視為過期（不讓壞資料保持進行中而擋住新實驗）。
pub fn is_expired(created_at: &str, now: &str) -> bool {
    match (epoch_secs(created_at), epoch_secs(now)) {
        (Some(c), Some(n)) => n - c >= WINDOW_SECS,
        _ => true,
    }
}

/// `requestId`：8–64 字 `[A-Za-z0-9_-]`。
pub fn valid_request_id(s: &str) -> bool {
    (8..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// 抽三題：有未試領域時第一題來自未試領域；其餘兩題不重複、不同領域優先。
/// `pick(n)` 回傳 `0..n` 的亂數。**輸入只有試過的領域，沒有任何結果。**
pub fn draw_three(
    tried_domains: &[Domain],
    mut pick: impl FnMut(usize) -> usize,
) -> Vec<&'static TrialItem> {
    let mut out: Vec<&'static TrialItem> = Vec::with_capacity(3);
    let untried: Vec<&'static TrialItem> = CATALOG
        .iter()
        .filter(|i| !tried_domains.contains(&i.domain))
        .collect();
    if !untried.is_empty() {
        out.push(untried[pick(untried.len()) % untried.len()]);
    }
    while out.len() < 3 && out.len() < CATALOG.len() {
        let fresh: Vec<&'static TrialItem> = CATALOG
            .iter()
            .filter(|i| !out.iter().any(|o| o.id == i.id))
            .filter(|i| !out.iter().any(|o| o.domain == i.domain))
            .collect();
        let pool: Vec<&'static TrialItem> = if fresh.is_empty() {
            CATALOG
                .iter()
                .filter(|i| !out.iter().any(|o| o.id == i.id))
                .collect()
        } else {
            fresh
        };
        out.push(pool[pick(pool.len()) % pool.len()]);
    }
    out
}

/// `digits` 題的 30 位數字：由試驗 id 確定性產生（同一筆永遠同一串，不另存）。FNV-1a＋xorshift。
pub fn digits_for(trial_id: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in trial_id.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut out = String::with_capacity(30);
    for _ in 0..30 {
        h ^= h << 13;
        h ^= h >> 7;
        h ^= h << 17;
        out.push(char::from(b'0' + (h % 10) as u8));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BANNED: &[&str] = &[
        "天賦",
        "天分",
        "天生",
        "資質",
        "潛能",
        "擅長",
        "適合",
        "能力",
        "優勢",
        "強項",
        "弱項",
        "研究",
        "科學",
        "理論",
        "證實",
        "證明",
        "準確",
        "預測誤差",
        "大腦",
        "測出",
        "診斷",
        "命中",
        "注定",
        "IQ",
        "智商",
        "talent",
        "gifted",
        "aptitude",
        "brain",
        "theory",
        "science",
        "research",
        "proven",
        "accurate",
        "potential",
        "destiny",
        "fate",
    ];

    fn all_texts() -> Vec<String> {
        let mut v: Vec<String> = vec![
            PAGE_TITLE,
            PAGE_INTRO,
            SELF_NOTE,
            LOGIN_PROMPT,
            HOME_CARD_BLURB,
            PREDICT_QUESTION,
            PREDICT_LOCK_NOTE,
            REPORT_QUESTION,
            STUCK_QUESTION,
            SKIP_LABEL,
            TIMER_NOTE,
            CONDITION_NOTE,
            SMOOTHER_TITLE,
            SMOOTHER_NOTE,
            TENDENCY_TITLE,
            TENDENCY_FOOTNOTE,
            DELETE_NOTE,
        ]
        .into_iter()
        .map(String::from)
        .collect();
        for i in CATALOG {
            v.push(i.title.into());
            v.push(i.prep.into());
            v.extend(i.steps.iter().map(|s| s.to_string()));
        }
        for d in Domain::ALL {
            v.push(d.label().into());
        }
        for (_, l) in STUCK_REASONS {
            v.push((*l).into());
        }
        for k in [
            TendencyKind::Conservative,
            TendencyKind::Close,
            TendencyKind::Optimistic,
        ] {
            v.push(k.sentence().into());
        }
        for g in -3..=3 {
            v.push(gap_sentence(g));
        }
        for n in 0..=3 {
            v.push(step_label(n));
        }
        v.push(tendency_need_sentence(2));
        v.push(tendency_counts_sentence(1, 2, 3));
        v
    }

    #[test]
    fn no_banned_words() {
        for t in all_texts() {
            let lower = t.to_lowercase();
            for w in BANNED {
                assert!(!lower.contains(&w.to_lowercase()), "banned {w:?} in {t:?}");
            }
        }
    }

    #[test]
    fn no_external_links() {
        for t in all_texts() {
            assert!(!t.contains("http") && !t.contains("www."), "{t}");
        }
    }

    #[test]
    fn catalog_well_formed() {
        let mut ids = std::collections::BTreeSet::new();
        for i in CATALOG {
            assert!(ids.insert(i.id), "dup {}", i.id);
            assert!(
                !i.id.is_empty()
                    && i.id
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "{}",
                i.id
            );
            assert!(i.steps.iter().all(|s| !s.is_empty()));
            assert!(!i.title.is_empty() && !i.prep.is_empty());
        }
        for d in Domain::ALL {
            assert!(CATALOG.iter().any(|i| i.domain == d), "{d:?} empty");
        }
        assert_eq!(STEPS as usize, CATALOG[0].steps.len());
    }

    #[test]
    fn find_and_stuck() {
        assert_eq!(find("juggle").unwrap().domain, Domain::Movement);
        assert!(find("nope").is_none());
        assert_eq!(stuck_label("time"), Some("時間不夠"));
        assert!(stuck_label("x").is_none());
    }

    #[test]
    fn gap_and_labels() {
        assert_eq!(gap(1, 3), 2);
        assert_eq!(gap(3, 0), -3);
        assert_eq!(gap_sentence(1), "比你猜的多走了 1 步。");
        assert_eq!(gap_sentence(-2), "比你猜的少走了 2 步。");
        assert_eq!(step_label(0), "還沒到第 1 步");
        assert_eq!(step_label(2), "第 2 步");
    }

    #[test]
    fn tendency_threshold_and_kinds() {
        let t = tendency(&[1, 2]);
        assert_eq!((t.need, t.kind), (1, None));
        assert_eq!(tendency(&[]).need, 3);
        assert_eq!(tendency(&[1, 0, 1]).kind, Some(TendencyKind::Conservative)); // mean .67
        assert_eq!(tendency(&[1, 0, 0]).kind, Some(TendencyKind::Close)); // .33
        assert_eq!(tendency(&[-1, -1, 1]).kind, Some(TendencyKind::Close)); // −.33
        assert_eq!(tendency(&[-1, 0, -1]).kind, Some(TendencyKind::Optimistic)); // −.67
        assert_eq!(
            tendency(&[1, 0, 0, 1]).kind,
            Some(TendencyKind::Conservative)
        ); // .5 邊界
        let t = tendency(&[2, 0, -1, -3]);
        assert_eq!((t.under, t.exact, t.over, t.reported), (1, 1, 2, 4));
    }

    #[test]
    fn tendency_kind_wire_roundtrip() {
        for k in [
            TendencyKind::Conservative,
            TendencyKind::Close,
            TendencyKind::Optimistic,
        ] {
            assert_eq!(TendencyKind::from_wire(k.wire()), Some(k));
        }
    }

    #[test]
    fn epoch_and_expiry() {
        assert_eq!(epoch_secs("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(epoch_secs("2000-03-01T00:00:00Z"), Some(951_868_800));
        assert_eq!(epoch_secs("2026-11-22T13:11:33.800Z"), Some(1_795_353_093));
        assert!(epoch_secs("2026-11-22 13:11:33").is_none());
        assert!(epoch_secs("").is_none());
        let c = "2026-11-22T00:00:00.000Z";
        assert!(!is_expired(c, "2026-11-28T23:59:59.999Z"));
        assert!(is_expired(c, "2026-11-29T00:00:00.000Z"));
        assert!(is_expired("garbage", "2026-11-29T00:00:00.000Z"));
    }

    #[test]
    fn request_id_rules() {
        assert!(valid_request_id("abcd-1234_ef"));
        assert!(!valid_request_id("short"));
        assert!(!valid_request_id(""));
        assert!(!valid_request_id("has space 123"));
        assert!(!valid_request_id(&"a".repeat(65)));
    }

    #[test]
    fn draw_includes_untried_and_ignores_results() {
        // 試過 9 個領域 → 第一題一定是唯一沒試的 Speaking
        let tried: Vec<Domain> = Domain::ALL
            .into_iter()
            .filter(|d| *d != Domain::Speaking)
            .collect();
        for seed in 0..20usize {
            let mut s = seed;
            let three = draw_three(&tried, |n| {
                s = s.wrapping_mul(31).wrapping_add(7);
                s % n
            });
            assert_eq!(three.len(), 3);
            assert_eq!(three[0].domain, Domain::Speaking);
            let ids: std::collections::BTreeSet<_> = three.iter().map(|i| i.id).collect();
            assert_eq!(ids.len(), 3);
            let doms: std::collections::BTreeSet<_> = three.iter().map(|i| i.domain).collect();
            assert_eq!(doms.len(), 3, "不同領域優先");
        }
        // 全部試過也能抽
        assert_eq!(draw_three(&Domain::ALL, |_| 0).len(), 3);
    }

    #[test]
    fn digits_deterministic() {
        let a = digits_for("trial-abc");
        assert_eq!(a.len(), 30);
        assert!(a.bytes().all(|b| b.is_ascii_digit()));
        assert_eq!(a, digits_for("trial-abc"));
        assert_ne!(a, digits_for("trial-abd"));
    }
}
