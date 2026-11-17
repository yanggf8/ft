//! 先猜後試（spec §4，Batch 3 純前端）的純邏輯：回報時窗、區間命中、校準比例。
//! 這套流程不保護任何測量，只是給自己的觀察工具；資料由 web 存在 localStorage。

use super::date::{iso_shift_days, parse_iso_ms, DAY_MS};

/// 滿 7 天開放回報。
pub const OPEN_AFTER_DAYS: i64 = 7;
/// 滿 30 天當下仍可回報，超過才過期。
pub const EXPIRE_AFTER_DAYS: i64 = 30;
/// 0–10 分的上限。
pub const SCORE_MAX: u8 = 10;

/// 回報時窗狀態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObsWindow {
    /// 未滿 7 天（含時鐘倒退）。
    NotYet,
    /// 滿 7 天到滿 30 天（兩端都含）。
    Open,
    /// 超過 30 天。
    Expired,
}

impl ObsWindow {
    pub fn wire(&self) -> &'static str {
        match self {
            ObsWindow::NotYet => "notYet",
            ObsWindow::Open => "open",
            ObsWindow::Expired => "expired",
        }
    }
}

/// 依開始時間（＝調整完成時間）與現在時間判定時窗；任一不可解析 → `None`。
pub fn obs_window(started_at: &str, now: &str) -> Option<ObsWindow> {
    let elapsed = parse_iso_ms(now)? - parse_iso_ms(started_at)?;
    Some(if elapsed < OPEN_AFTER_DAYS * DAY_MS {
        ObsWindow::NotYet
    } else if elapsed <= EXPIRE_AFTER_DAYS * DAY_MS {
        ObsWindow::Open
    } else {
        ObsWindow::Expired
    })
}

/// 開放回報的時刻（提醒只顯示日期，由 web 取前 10 字或轉台北日）。
pub fn opens_at(started_at: &str) -> Option<String> {
    iso_shift_days(started_at, OPEN_AFTER_DAYS)
}

/// 最後可回報的時刻（含）。
pub fn expires_at(started_at: &str) -> Option<String> {
    iso_shift_days(started_at, EXPIRE_AFTER_DAYS)
}

/// 距離開放還有幾天（無條件進位；已開放回 0）。
pub fn days_until_open(started_at: &str, now: &str) -> Option<i64> {
    let open = parse_iso_ms(started_at)? + OPEN_AFTER_DAYS * DAY_MS;
    let left = open - parse_iso_ms(now)?;
    Some(if left <= 0 {
        0
    } else {
        (left + DAY_MS - 1) / DAY_MS
    })
}

/// 預測區間（0–10，兩端都含）。輸入順序顛倒時自動對調。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreRange {
    pub low: u8,
    pub high: u8,
}

impl ScoreRange {
    /// 任一端超過 10 → `None`。
    pub fn new(a: u8, b: u8) -> Option<ScoreRange> {
        if a > SCORE_MAX || b > SCORE_MAX {
            return None;
        }
        Some(ScoreRange {
            low: a.min(b),
            high: a.max(b),
        })
    }

    /// 實際分數是否落在區間內；分數超過 10 → `None`。
    pub fn hit(&self, actual: u8) -> Option<bool> {
        (actual <= SCORE_MAX).then(|| (self.low..=self.high).contains(&actual))
    }
}

/// 回報時「這段時間你有維持這個調整嗎？」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adherence {
    Yes,
    Partial,
    No,
}

impl Adherence {
    pub fn wire(&self) -> &'static str {
        match self {
            Adherence::Yes => "yes",
            Adherence::Partial => "partial",
            Adherence::No => "no",
        }
    }

    pub fn from_wire(s: &str) -> Option<Adherence> {
        [Adherence::Yes, Adherence::Partial, Adherence::No]
            .into_iter()
            .find(|a| a.wire() == s)
    }
}

/// 校準紀錄：每一個區間預測算一次。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Calibration {
    pub hits: u32,
    pub total: u32,
}

impl Calibration {
    /// 0.0–1.0；沒有紀錄時 `None`。
    pub fn ratio(&self) -> Option<f64> {
        (self.total > 0).then(|| f64::from(self.hits) / f64::from(self.total))
    }
}

/// 從每次預測的命中結果累計校準。
pub fn calibration<I: IntoIterator<Item = bool>>(results: I) -> Calibration {
    results
        .into_iter()
        .fold(Calibration::default(), |mut c, hit| {
            c.total += 1;
            if hit {
                c.hits += 1;
            }
            c
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: &str = "2026-11-17T09:30:00.000Z";

    // ── T7 時窗邊界 ──

    #[test]
    fn t7_window_boundaries() {
        let w = |now: &str| obs_window(START, now).unwrap();
        assert_eq!(w(START), ObsWindow::NotYet);
        assert_eq!(w("2026-11-24T09:29:59.999Z"), ObsWindow::NotYet);
        // 剛好 7 天 → 開放
        assert_eq!(w("2026-11-24T09:30:00.000Z"), ObsWindow::Open);
        assert_eq!(w("2026-12-01T00:00:00.000Z"), ObsWindow::Open);
        // 剛好 30 天 → 仍可
        assert_eq!(w("2026-12-17T09:30:00.000Z"), ObsWindow::Open);
        // 超過 30 天 1 毫秒 → 過期
        assert_eq!(w("2026-12-17T09:30:00.001Z"), ObsWindow::Expired);
    }

    #[test]
    fn t7_clock_going_backwards_is_not_yet() {
        assert_eq!(
            obs_window(START, "2026-11-16T00:00:00.000Z"),
            Some(ObsWindow::NotYet)
        );
    }

    #[test]
    fn t7_unparsable_is_none() {
        assert_eq!(obs_window("bad", START), None);
        assert_eq!(obs_window(START, "2026-11-24"), None);
        assert_eq!(days_until_open(START, "x"), None);
    }

    #[test]
    fn t7_open_and_expire_timestamps() {
        assert_eq!(opens_at(START).as_deref(), Some("2026-11-24T09:30:00.000Z"));
        assert_eq!(
            expires_at(START).as_deref(),
            Some("2026-12-17T09:30:00.000Z")
        );
        // 用 opens_at／expires_at 算出的時刻本身就落在邊界上
        assert_eq!(
            obs_window(START, &opens_at(START).unwrap()),
            Some(ObsWindow::Open)
        );
        assert_eq!(
            obs_window(START, &expires_at(START).unwrap()),
            Some(ObsWindow::Open)
        );
    }

    #[test]
    fn t7_days_until_open_rounds_up() {
        assert_eq!(days_until_open(START, START), Some(7));
        assert_eq!(days_until_open(START, "2026-11-17T09:30:00.001Z"), Some(7));
        assert_eq!(days_until_open(START, "2026-11-23T09:30:00.000Z"), Some(1));
        assert_eq!(days_until_open(START, "2026-11-24T09:30:00.000Z"), Some(0));
        assert_eq!(days_until_open(START, "2026-12-30T00:00:00.000Z"), Some(0));
    }

    // ── T7 區間命中 ──

    #[test]
    fn t7_range_hit_inclusive_both_ends() {
        let r = ScoreRange::new(4, 6).unwrap();
        assert_eq!(r.hit(3), Some(false));
        assert_eq!(r.hit(4), Some(true));
        assert_eq!(r.hit(5), Some(true));
        assert_eq!(r.hit(6), Some(true));
        assert_eq!(r.hit(7), Some(false));
        assert_eq!(r.hit(11), None);
    }

    #[test]
    fn t7_range_normalizes_and_validates() {
        assert_eq!(ScoreRange::new(7, 2), Some(ScoreRange { low: 2, high: 7 }));
        assert_eq!(ScoreRange::new(5, 5).unwrap().hit(5), Some(true));
        assert_eq!(ScoreRange::new(0, 10).unwrap().hit(0), Some(true));
        assert_eq!(ScoreRange::new(0, 11), None);
        assert_eq!(ScoreRange::new(11, 3), None);
    }

    // ── T7 校準比例 ──

    #[test]
    fn t7_calibration_ratio() {
        let c = calibration([true, false, true, true, false]);
        assert_eq!(c, Calibration { hits: 3, total: 5 });
        assert_eq!(c.ratio(), Some(0.6));
        let empty = calibration(std::iter::empty());
        assert_eq!(empty.ratio(), None);
        assert_eq!(calibration([false]).ratio(), Some(0.0));
    }

    #[test]
    fn adherence_wire_round_trips() {
        for a in [Adherence::Yes, Adherence::Partial, Adherence::No] {
            assert_eq!(Adherence::from_wire(a.wire()), Some(a));
        }
    }
}
