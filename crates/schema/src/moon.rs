//! 月相吉祥物（2026-11-23，「月亮貓・引路貓」設計稿 https://claude.ai/artifact/4j3inRg98QWKbvkq9NfR6X）。
//!
//! 純函式：依時間算當下月相（五種）、是否台北夜晚，以及月亮貓的固定台詞。
//! 月相用平均朔望月從已知新月推算（誤差約半天內），只拿來換首頁王座椅背的月亮與台詞（吉祥物的頭永遠是月牙），
//! **不做任何運勢判斷**——台詞受禁字測試守門（不說運勢、命、注定、改運）。
//! 依「離新月的天數」對稱分桶（虧月同一桶），wire 值 `key()` 是椅背月亮圖檔名的一部分，不可改名。

/// 平均朔望月（日）。
pub const SYNODIC_DAYS: f64 = 29.530_588_853;
/// 參考新月：2000-01-06 18:14 UTC（Unix 毫秒）。
const REF_NEW_MOON_MS: f64 = 947_182_440_000.0;
const DAY_MS: f64 = 86_400_000.0;

/// 月相（五桶）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonPhase {
    New,
    Crescent,
    Quarter,
    Gibbous,
    Full,
}

impl MoonPhase {
    /// 圖集檔名用的 wire 值（不可改名）。
    pub fn key(self) -> &'static str {
        match self {
            MoonPhase::New => "new",
            MoonPhase::Crescent => "crescent",
            MoonPhase::Quarter => "quarter",
            MoonPhase::Gibbous => "gibbous",
            MoonPhase::Full => "full",
        }
    }

    /// 顯示名稱。
    pub fn name_zh(self) -> &'static str {
        match self {
            MoonPhase::New => "新月",
            MoonPhase::Crescent => "眉月",
            MoonPhase::Quarter => "弦月",
            MoonPhase::Gibbous => "凸月",
            MoonPhase::Full => "滿月",
        }
    }
}

/// 月齡（日，0 ≤ age < 朔望月；0＝新月）。
pub fn moon_age_days(unix_ms: f64) -> f64 {
    ((unix_ms - REF_NEW_MOON_MS) / DAY_MS).rem_euclid(SYNODIC_DAYS)
}

/// 當下月相：以離最近新月的天數 d（0..≈14.77，盈虧對稱）分桶。
/// 新月 d<1.5、眉月 <5.5、弦月 <9.3、凸月 <13.3、其餘滿月（滿月前後約 1.5 天）。
pub fn phase_at(unix_ms: f64) -> MoonPhase {
    let a = moon_age_days(unix_ms);
    let d = a.min(SYNODIC_DAYS - a);
    if d < 1.5 {
        MoonPhase::New
    } else if d < 5.5 {
        MoonPhase::Crescent
    } else if d < 9.3 {
        MoonPhase::Quarter
    } else if d < 13.3 {
        MoonPhase::Gibbous
    } else {
        MoonPhase::Full
    }
}

/// 台北時間的小時（0..24）。
pub fn taipei_hour(unix_ms: f64) -> u32 {
    ((unix_ms / 3_600_000.0).floor() as i64 + 8).rem_euclid(24) as u32
}

/// 台北夜晚：18:00–翌日 05:59。月亮貓只在夜晚開口、新月夜才睡覺、滿月夜才變身。
pub fn is_night_taipei(unix_ms: f64) -> bool {
    let h = taipei_hour(unix_ms);
    !(6..18).contains(&h)
}

/// 「今晚」的台北日期鍵（凌晨 0–5 點算前一晚），滿月變身每晚只演一次用。
pub fn taipei_night_key(unix_ms: f64) -> String {
    let shifted = unix_ms + 8.0 * 3_600_000.0 - 6.0 * 3_600_000.0;
    let days = (shifted / DAY_MS).floor() as i64;
    let (y, m, d) = crate::cycle::civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// 首頁夜晚台詞（白天不開口）。
pub fn home_line(phase: MoonPhase) -> &'static str {
    match phase {
        MoonPhase::New => "今晚新月，月亮在休息，我陪你一下。",
        MoonPhase::Crescent => "今晚是眉月，想先看哪個？",
        MoonPhase::Quarter => "今晚是弦月，慢慢來就好。",
        MoonPhase::Gibbous => "月亮快圓了，今天想問什麼？",
        MoonPhase::Full => "今晚滿月！一起看看吧。",
    }
}

/// 新月夜睡著時的泡泡。
pub const SLEEP_LINE: &str = "呼……";

/// 叫醒按鈕的無障礙名稱。
pub const WAKE_LABEL: &str = "叫醒月亮貓";

/// 塔羅翻牌前的台詞（翻牌後一律「抽到了！」）。
pub fn tarot_wait_line(phase: MoonPhase) -> &'static str {
    match phase {
        MoonPhase::New => "我陪你翻。",
        MoonPhase::Crescent | MoonPhase::Quarter => "翻開看看？",
        MoonPhase::Gibbous => "月亮快圓了，翻翻看？",
        MoonPhase::Full => "滿月下翻一張？",
    }
}

/// 塔羅翻牌後的台詞。
pub const TAROT_JOY_LINE: &str = "抽到了！";

#[cfg(test)]
mod tests {
    use super::*;

    /// 2024-01-11 11:57 UTC 新月、2024-01-18 03:52 上弦、2024-01-25 17:54 滿月（天文台資料）。
    fn ms(y: i64, mo: i64, d: i64, h: i64, mi: i64) -> f64 {
        let days = crate::cycle::days_from_civil(y, mo, d);
        ((days * 86_400 + h * 3_600 + mi * 60) * 1_000) as f64
    }

    #[test]
    fn known_dates_land_in_right_bucket() {
        assert_eq!(phase_at(ms(2024, 1, 11, 11, 57)), MoonPhase::New);
        assert_eq!(phase_at(ms(2024, 1, 18, 3, 52)), MoonPhase::Quarter);
        assert_eq!(phase_at(ms(2024, 1, 25, 17, 54)), MoonPhase::Full);
        assert_eq!(phase_at(ms(2024, 1, 14, 12, 0)), MoonPhase::Crescent);
        assert_eq!(phase_at(ms(2024, 1, 22, 12, 0)), MoonPhase::Gibbous);
        // 虧月對稱：滿月後一週＝弦月（下弦 2024-02-02 23:18）
        assert_eq!(phase_at(ms(2024, 2, 2, 23, 18)), MoonPhase::Quarter);
    }

    #[test]
    fn age_is_within_cycle() {
        for k in 0..400 {
            let a = moon_age_days(ms(2026, 1, 1, 0, 0) + k as f64 * 0.37 * DAY_MS);
            assert!((0.0..SYNODIC_DAYS).contains(&a));
        }
    }

    #[test]
    fn taipei_night_window() {
        // 10:00 UTC＝台北 18:00 → 夜晚；09:59 UTC＝17:59 → 白天；21:59 UTC＝05:59 → 夜晚；22:00 UTC＝06:00 → 白天
        assert!(is_night_taipei(ms(2026, 11, 23, 10, 0)));
        assert!(!is_night_taipei(ms(2026, 11, 23, 9, 59)));
        assert!(is_night_taipei(ms(2026, 11, 23, 21, 59)));
        assert!(!is_night_taipei(ms(2026, 11, 23, 22, 0)));
    }

    #[test]
    fn night_key_rolls_at_six_am_taipei() {
        // 台北 11/24 01:00（UTC 11/23 17:00）仍算 11/23 那一晚；台北 11/24 06:00 起算 11/24
        assert_eq!(taipei_night_key(ms(2026, 11, 23, 17, 0)), "2026-11-23");
        assert_eq!(taipei_night_key(ms(2026, 11, 23, 22, 0)), "2026-11-24");
        assert_eq!(taipei_night_key(ms(2026, 11, 23, 10, 0)), "2026-11-23");
    }

    #[test]
    fn lines_make_no_fortune_claims() {
        const BANNED: [&str; 6] = ["運", "命", "注定", "保證", "改運", "招財"];
        let phases = [
            MoonPhase::New,
            MoonPhase::Crescent,
            MoonPhase::Quarter,
            MoonPhase::Gibbous,
            MoonPhase::Full,
        ];
        let mut lines: Vec<&str> = vec![SLEEP_LINE, WAKE_LABEL, TAROT_JOY_LINE];
        for p in phases {
            lines.push(home_line(p));
            lines.push(tarot_wait_line(p));
            lines.push(p.name_zh());
        }
        for l in lines {
            for b in BANNED {
                assert!(!l.contains(b), "台詞「{l}」含禁字「{b}」");
            }
        }
    }

    #[test]
    fn keys_are_stable() {
        // wire 值是圖集檔名的一部分，改名＝舊圖集對不上
        assert_eq!(MoonPhase::New.key(), "new");
        assert_eq!(MoonPhase::Crescent.key(), "crescent");
        assert_eq!(MoonPhase::Quarter.key(), "quarter");
        assert_eq!(MoonPhase::Gibbous.key(), "gibbous");
        assert_eq!(MoonPhase::Full.key(), "full");
    }
}
