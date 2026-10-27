//! 塔羅 v1 — 純函式模組（設計定稿：/tmp/ft-tarot-review/design-v1-final.md）。
//! 無 IO、native 與 wasm32 皆可編；web 直接引用（v1a 今日抽牌零 API、零 DB），
//! api 亦引用（v1b 伺服端抽牌＋冪等）。
//!
//! 紅線：隨機源一律由呼叫端注入（`entropy`）；伺服端必用 `secure_bytes`
//! （fail-closed），前端今日抽牌為決定性種子（體驗用途，非安全邊界）。
//! 塔羅資料永不寫入 predictions／prediction_feedback／f8_assignments。

pub mod catalog;

pub use catalog::{TarotCard, CATALOG};

use serde::{Deserialize, Serialize};

/// 語料版本（比照 `PRACTICE_CATALOG_VERSION`；改版 bump）。
pub const TAROT_CORPUS_VERSION: &str = "2";

/// 種子編碼＋PRNG 演算法版本——改演算法必 bump，否則「同日同裝置同牌」會靜默改變。
pub const TAROT_DRAW_ALGO_VERSION: &str = "1";

/// v2 AI 解牌 prompt 版本（spec §Tarot v2-7；改 prompt 模板必 bump）。
pub const TAROT_PROMPT_VERSION: &str = "1";

/// 洗牌熵下限（bytes）；不足即 `None`（fail-closed，不降級弱隨機）。
pub const MIN_ENTROPY_BYTES: usize = 32;

/// 三張牌陣的位置語意（固定順序：現況→阻礙→建議）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    Situation,
    Obstacle,
    Advice,
}

impl Position {
    /// wire 值（語義化小寫，見 DTO 慣例）。
    pub fn wire(&self) -> &'static str {
        match self {
            Position::Situation => "situation",
            Position::Obstacle => "obstacle",
            Position::Advice => "advice",
        }
    }
}

/// 抽出的一張牌（位置由呼叫端按序指定）。
#[derive(Debug, Clone, PartialEq)]
pub struct DrawnCard {
    pub card: &'static TarotCard,
    pub reversed: bool,
}

/// v1b wire DTO — 抽牌請求（camelCase，`serde(rename_all)` 慣例）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TarotDrawRequest {
    pub spread: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    pub request_id: String,
}

/// v1b wire DTO — 單張牌檢視。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TarotCardView {
    pub id: String,
    pub name: String,
    pub reversed: bool,
    pub position: String,
}

/// v1b wire DTO — 抽牌結果檢視（201／冪等 200 共用形狀）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TarotReadingView {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    pub spread: String,
    pub cards: Vec<TarotCardView>,
    pub interpretation: String,
    pub corpus_version: String,
    pub created_at: String,
    // ── v2 AI 解牌（None＝從未嘗試；failed＝AI 不可用，規則式解讀仍在）──
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_interpretation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_model: Option<String>,
}

/// v1b wire DTO — 歷史分頁（比照 practice 分頁形狀：items + nextCursor）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TarotHistoryPage {
    pub items: Vec<TarotReadingView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// UTC ISO 時刻 → Asia/Taipei（UTC+8，無 DST）當日 `YYYY-MM-DD`。
/// 台北日界契約（v1a 種子用）；不可解析 → `None`（fail-closed，比照 cycle.rs）。
pub fn taipei_today(utc_iso: &str) -> Option<String> {
    let (days, secs) = crate::cycle::parse_iso_parts(utc_iso)?;
    let taipei_days = (days * 86_400 + secs + 8 * 3_600).div_euclid(86_400);
    let (y, m, d) = crate::cycle::civil_from_days(taipei_days);
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// FNV-1a 64（與 api `common.rs` 同型；schema 自持一份純實作，native+wasm 皆可編）。
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// 種子化 PRNG（SplitMix64）——演算法由 `TAROT_DRAW_ALGO_VERSION` 鎖定。
struct Prng {
    state: u64,
}

impl Prng {
    fn new(entropy: &[u8]) -> Self {
        Self {
            state: fnv1a_64(entropy),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 均勻取樣 `[0, n)`：拒絕取樣（`v < u64::MAX - u64::MAX % n` 才取，
    /// 杜絕取模偏差）。
    fn below(&mut self, n: u64) -> u64 {
        let limit = u64::MAX - u64::MAX % n;
        loop {
            let v = self.next_u64();
            if v < limit {
                return v % n;
            }
        }
    }
}

/// 三張牌陣抽牌（現況→阻礙→建議）：78 張不重複抽取、正逆位獨立 50%。
/// `entropy < MIN_ENTROPY_BYTES` → `None`（fail-closed）。
/// 決定性：同 entropy 必同結果（`TAROT_DRAW_ALGO_VERSION` 鎖定演算法）。
pub fn shuffle_three(entropy: &[u8]) -> Option<Vec<DrawnCard>> {
    if entropy.len() < MIN_ENTROPY_BYTES {
        return None;
    }
    let mut rng = Prng::new(entropy);
    let mut pool: Vec<&'static TarotCard> = CATALOG.iter().collect();
    let mut out = Vec::with_capacity(3);
    for _ in 0..3 {
        // 每位置：先抽牌（自剩餘池、不重複），後抽逆位（最低位元）
        let card = pool.swap_remove(rng.below(pool.len() as u64) as usize);
        let reversed = rng.next_u64() & 1 == 1;
        out.push(DrawnCard { card, reversed });
    }
    Some(out)
}

/// 今日抽牌（單張＋正逆位）——決定性種子（`"{YYYY-MM-DD}|{device_seed}"` UTF-8）。
pub fn daily_draw(entropy: &[u8]) -> Option<DrawnCard> {
    if entropy.len() < MIN_ENTROPY_BYTES {
        return None;
    }
    let mut rng = Prng::new(entropy);
    let card = &CATALOG[rng.below(CATALOG.len() as u64) as usize];
    let reversed = rng.next_u64() & 1 == 1;
    Some(DrawnCard { card, reversed })
}

/// 規則式解讀組裝：位置引言（3 位置×正逆，≤40 字）＋牌義（≤120 字）×3。
/// 輸出 ≤600 字；輸入須為 3 張（position 序 0→2）。
const POSITION_LABELS: [&str; 3] = ["現況", "阻礙", "建議"];

/// 位置引言（3 位置×正逆，各 ≤40 字）。
fn position_intro(pos: usize, reversed: bool) -> &'static str {
    match (pos, reversed) {
        (0, false) => "這是你當下的處境與基調。",
        (0, true) => "當下的能量有些受阻或內向。",
        (1, false) => "這是眼前主要的課題與阻礙。",
        (1, true) => "阻礙來自內在的抗拒或失衡。",
        (2, false) => "這是此刻可行動的方向。",
        _ => "建議先停下來，調整後再前進。",
    }
}

pub fn assemble_interpretation(cards: &[DrawnCard]) -> Option<String> {
    if cards.len() != 3 {
        return None;
    }
    let mut out = String::new();
    for (i, d) in cards.iter().enumerate() {
        let intro = position_intro(i, d.reversed);
        let (meaning, orientation) = if d.reversed {
            (d.card.reversed, "逆位")
        } else {
            (d.card.upright, "正位")
        };
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "【{}】{}\n{}（{}）：{}",
            POSITION_LABELS[i], intro, d.card.name, orientation, meaning
        ));
    }
    if out.chars().count() > 600 {
        return None;
    }
    Some(out)
}

/// 冪等比對用 question 正規化：trim；空 → `None`（存 NULL）；>200 字 → `None`。
pub fn normalize_question(q: &str) -> Option<String> {
    let t = q.trim();
    if t.is_empty() || t.chars().count() > 200 {
        return None;
    }
    Some(t.to_string())
}

/// requestId 形狀檢查（比照 practice `valid_request_id`：UUID 形狀——
/// 8/13/18/23 為 `-`、其餘為 hex）。
pub fn valid_tarot_request_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

/// question 超過 200 字元（`validate_question` 的錯誤型別）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestionTooLong;

/// Route 端 question 驗證（spec 定案：trim 後空＝未填、存 NULL；
/// 僅 >200 字元回 `Err`）。
pub fn validate_question(q: Option<&str>) -> Result<Option<String>, QuestionTooLong> {
    let Some(t) = q.map(str::trim) else {
        return Ok(None);
    };
    if t.is_empty() {
        return Ok(None);
    }
    if t.chars().count() > 200 {
        return Err(QuestionTooLong);
    }
    Ok(Some(t.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 版本常數 ──
    #[test]
    fn version_constants_are_pinned() {
        assert_eq!(TAROT_CORPUS_VERSION, "2");
        assert_eq!(TAROT_DRAW_ALGO_VERSION, "1");
    }

    // ── taipei_today：台北 00:00 邊界（前後一秒）與 fail-closed ──
    #[test]
    fn taipei_today_before_midnight_is_same_day() {
        // 台北 2026-10-26 23:59:59（UTC 15:59:59）
        assert_eq!(
            taipei_today("2026-10-26T15:59:59.000Z"),
            Some("2026-10-26".to_string())
        );
    }

    #[test]
    fn taipei_today_at_midnight_rolls_over() {
        // 台北 2026-10-27 00:00:00（UTC 16:00:00）——換日那一秒
        assert_eq!(
            taipei_today("2026-10-26T16:00:00.000Z"),
            Some("2026-10-27".to_string())
        );
    }

    #[test]
    fn taipei_today_rejects_invalid_iso() {
        assert_eq!(taipei_today("not-a-date"), None);
        assert_eq!(taipei_today("2026-10-26"), None); // 缺時間與 Z
    }

    // ── catalog 完整性 ──
    #[test]
    fn catalog_has_78_unique_semantic_ids() {
        assert_eq!(CATALOG.len(), 78, "78 張：22 大阿爾克那＋4 花色×14");
        let mut ids: Vec<&str> = CATALOG.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "id 不得重複");
        for c in CATALOG {
            let (suit, num) = c.id.split_once('_').expect("id 形如 suit_NN");
            assert!(
                (suit == "major" && num.len() == 2 && num.parse::<u8>().is_ok_and(|v| v < 22))
                    || (matches!(suit, "wands" | "cups" | "swords" | "pentacles")
                        && num.len() == 2
                        && num.parse::<u8>().is_ok_and(|v| (1..=14).contains(&v))),
                "id 語義格式錯誤：{}",
                c.id
            );
        }
    }

    #[test]
    fn catalog_meanings_within_120_chars_and_named() {
        for c in CATALOG {
            assert!(!c.name.trim().is_empty(), "{} 缺牌名", c.id);
            assert!(!c.keywords.trim().is_empty(), "{} 缺關鍵詞", c.id);
            assert!(
                c.upright.chars().count() <= 120,
                "{} 正位 >120 字（{}）",
                c.id,
                c.upright.chars().count()
            );
            assert!(
                c.reversed.chars().count() <= 120,
                "{} 逆位 >120 字（{}）",
                c.id,
                c.reversed.chars().count()
            );
            // 逆位不得僅是正位加「不」（防語料草率；比對去掉「不」後是否相同過於
            // 嚴格，此處釘最低限度：兩者不得完全相同）
            assert_ne!(c.upright, c.reversed, "{} 正逆位文案相同", c.id);
        }
    }

    // ── 洗牌核心 ──
    #[test]
    fn shuffle_rejects_entropy_below_floor() {
        let short = [0u8; 31];
        assert!(
            shuffle_three(&short).is_none(),
            "31 bytes < 32 須 fail-closed"
        );
        assert!(daily_draw(&short).is_none());
    }

    #[test]
    fn shuffle_three_is_deterministic_and_distinct() {
        let e = [7u8; 32];
        let a = shuffle_three(&e).expect("32 bytes 合法");
        let b = shuffle_three(&e).expect("同 entropy 應成功");
        assert_eq!(a.len(), 3);
        // 決定性：同 entropy → 逐張相同（含正逆位）
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.card.id, y.card.id);
            assert_eq!(x.reversed, y.reversed);
        }
        // 78 張不重複抽取
        let ids: Vec<&str> = a.iter().map(|d| d.card.id).collect();
        assert_ne!(ids[0], ids[1]);
        assert_ne!(ids[1], ids[2]);
        assert_ne!(ids[0], ids[2]);
    }

    #[test]
    fn daily_draw_is_deterministic() {
        let e = [9u8; 43]; // 43 bytes：v1a 種子契約（10+1+32）
        let a = daily_draw(&e).expect("≥32 bytes 合法");
        let b = daily_draw(&e).expect("同 entropy 應成功");
        assert_eq!(a.card.id, b.card.id);
        assert_eq!(a.reversed, b.reversed);
    }

    // ── 分佈 sanity（防「順手優化」改掉分佈）──
    /// 測試專用熵源：counter → splitmix64 填 32 bytes（與實作內部 PRNG 無關）。
    fn test_entropy(i: u64) -> [u8; 32] {
        let mut s = i.wrapping_add(0x9E3779B97F4A7C15);
        let mut out = [0u8; 32];
        for chunk in out.chunks_mut(8) {
            s = s.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^= z >> 31;
            chunk.copy_from_slice(&z.to_le_bytes());
        }
        out
    }

    #[test]
    fn distribution_is_roughly_uniform() {
        const DRAWS: u64 = 800; // 2400 個牌位，期望每張 ≈30 次
        let mut freq = std::collections::HashMap::<&str, u32>::new();
        let mut reversed = 0u32;
        for i in 0..DRAWS {
            let cards = shuffle_three(&test_entropy(i)).expect("entropy 合法");
            for c in &cards {
                *freq.entry(c.card.id).or_default() += 1;
                reversed += u32::from(c.reversed);
            }
        }
        assert_eq!(freq.len(), 78, "800 抽後 78 張全數應出現過");
        for (id, n) in &freq {
            assert!(*n >= 5, "{} 出現 {n} 次（期望 ≈30），疑似分佈偏斜", id);
        }
        let ratio = f64::from(reversed) / (DRAWS as f64 * 3.0);
        assert!(
            (0.45..=0.55).contains(&ratio),
            "逆位比例 {ratio:.3} 超出 45–55%"
        );
    }

    // ── 規則式解讀組裝 ──
    #[test]
    fn interpretation_assembles_within_600_chars() {
        let cards = shuffle_three(&[3u8; 32]).expect("entropy 合法");
        let text = assemble_interpretation(&cards).expect("3 張合法輸入");
        assert!(text.chars().count() <= 600, "解讀 >600 字");
        // 每張牌義須出現在組裝結果（快照可追溯）
        for c in &cards {
            let meaning = if c.reversed {
                c.card.reversed
            } else {
                c.card.upright
            };
            assert!(text.contains(meaning), "{} 的牌義未進入解讀", c.card.id);
        }
        assert!(
            assemble_interpretation(&cards[..2]).is_none(),
            "非 3 張 → None"
        );
    }

    // ── 冪等輔助 ──
    #[test]
    fn question_normalization_trim_null_and_limit() {
        assert_eq!(
            normalize_question("  我想問事業  "),
            Some("我想問事業".to_string())
        );
        assert_eq!(normalize_question("   "), None, "trim 後空 → NULL");
        let two_hundred = "字".repeat(200);
        assert_eq!(normalize_question(&two_hundred), Some(two_hundred.clone()));
        let two_oh_one = "字".repeat(201);
        assert_eq!(normalize_question(&two_oh_one), None, ">200 字 → None");
    }

    #[test]
    fn request_id_shape_matches_practice() {
        assert!(valid_tarot_request_id(
            "01234567-89ab-cdef-0123-456789abcdef"
        ));
        // 無連字號的 36 字 hex 不合法（practice 為完整 UUID 形狀檢查）
        assert!(!valid_tarot_request_id(
            "0123456789abcdef0123456789abcdef0123"
        ));
        assert!(!valid_tarot_request_id("short"));
    }
}

#[cfg(test)]
mod golden {
    use super::*;

    /// Golden pin（演算法 `TAROT_DRAW_ALGO_VERSION = "1"`）：
    /// 固定種子的整組輸出逐項釘死——任何「順手優化」改掉洗牌/PRNG 都會在此爆紅。
    #[test]
    fn shuffle_three_golden_pinned() {
        let cards = shuffle_three(&[7u8; 32]).expect("entropy 合法");
        let got: Vec<(&str, bool)> = cards.iter().map(|d| (d.card.id, d.reversed)).collect();
        assert_eq!(
            got,
            vec![("swords_08", true), ("swords_01", true), ("cups_04", false),]
        );
    }

    #[test]
    fn daily_draw_golden_pinned() {
        let d = daily_draw(&[9u8; 43]).expect("entropy 合法");
        assert_eq!((d.card.id, d.reversed), ("major_15", false));
    }
}
