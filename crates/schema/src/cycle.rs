//! `cycle_id` — Asia/Taipei 週一起算的週起始日（YYYY-MM-DD）。
//! Taipei 自 1979 起無 DST，固定 UTC+8，故不需 tz 資料庫、不需 `js_sys`；
//! 純字串/日期算術即可測試（native + wasm32 皆可編）。
//! Spec: docs/superpowers/specs/2026-09-04-f5-api-predictions-design.md §1.1

/// UTC ISO 時刻 → Asia/Taipei（UTC+8）當週**週一 00:00** 的 `YYYY-MM-DD`。
/// 輸入必須為 `YYYY-MM-DDTHH:MM:SS(.fff)?Z`（`clock::now_iso()` = JS
/// `toISOString()` → 含毫秒 `.000Z`）；不可解析 → `None`（fail-closed）。
pub fn week_start_asia_taipei(utc_iso: &str) -> Option<String> {
    let (days, secs) = parse_iso_parts(utc_iso)?;
    let total = days * 86_400 + secs;
    // UTC+8
    let taipei = total + 8 * 3_600;
    let taipei_days = taipei.div_euclid(86_400);
    // 1970-01-01 為週四 => weekday 0=Sun..6=Sat
    let weekday = (taipei_days + 4).rem_euclid(7);
    let back = (weekday + 6) % 7; // 回推至週一
    let (y, m, d) = civil_from_days(taipei_days - back);
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// cycleId 是否為合法週一（`YYYY-MM-DD` 且當日為週一）。
pub fn is_monday_cycle_id(cycle_id: &str) -> bool {
    parse_date_parts(cycle_id)
        .map(|(y, m, d)| {
            let days = days_from_civil(y, m, d);
            let weekday = (days + 4).rem_euclid(7);
            weekday == 1 // Monday
        })
        .unwrap_or(false)
}

/// 解析 `YYYY-MM-DDTHH:MM:SS(.fff)?Z` → (days_since_epoch, secs_in_day)。
/// 嚴格：要求尾端 `Z`（`toISOString` 恆為 UTC）、時分秒齊全；毫秒可缺。
fn parse_iso_parts(iso: &str) -> Option<(i64, i64)> {
    let iso = iso.strip_suffix(['Z', 'z'])?;
    let (date_part, time_part) = iso.split_once('T')?;
    let (y, m, d) = parse_date_parts(date_part)?;
    let mut tit = time_part.split(':');
    let h: i64 = tit.next()?.parse().ok()?;
    let mi: i64 = tit.next()?.parse().ok()?;
    let sec_part = tit.next()?;
    if tit.next().is_some() {
        return None;
    }
    let (sec, frac_ok) = match sec_part.split_once('.') {
        Some((s, f)) => (s, !f.is_empty() && f.chars().all(|c| c.is_ascii_digit())),
        None => (sec_part, true),
    };
    if !frac_ok {
        return None;
    }
    let s: i64 = sec.parse().ok()?;
    if !(0..=23).contains(&h) || !(0..=59).contains(&mi) || !(0..=59).contains(&s) {
        return None;
    }
    Some((days_from_civil(y, m, d), h * 3_600 + mi * 60 + s))
}

/// 解析 `YYYY-MM-DD`（含月/日範圍驗證）→ (y, m, d)。
fn parse_date_parts(s: &str) -> Option<(i64, i64, i64)> {
    let mut it = s.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let d: i64 = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&m) {
        return None;
    }
    if !(1..=days_in_month(y, m)).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Howard Hinnant `days_from_civil`（proleptic Gregorian，1970-01-01 = 0）。
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Hinnant `civil_from_days` — days_since_epoch → (y, m, d)。
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    ((if m <= 2 { y + 1 } else { y }), m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friday_taipei_belongs_to_current_monday() {
        // 台北週五 18:00 → 當週週一 = 2026-08-31（Grok P0-1 修正：非下週一）
        assert_eq!(
            week_start_asia_taipei("2026-09-04T10:00:00.000Z"),
            Some("2026-08-31".to_string())
        );
    }

    #[test]
    fn monday_0000_taipei_starts_new_cycle() {
        // 台北 09-07 週一 00:00 整（UTC 09-06 16:00:00.000Z）→ 新週
        assert_eq!(
            week_start_asia_taipei("2026-09-06T16:00:00.000Z"),
            Some("2026-09-07".to_string())
        );
        // 台北 09-07 週一 00:30 → 同週
        assert_eq!(
            week_start_asia_taipei("2026-09-06T16:30:00.000Z"),
            Some("2026-09-07".to_string())
        );
    }

    #[test]
    fn sunday_taipei_belongs_to_previous_cycle() {
        // 台北週日 18:00 → 該週所屬週一 2026-08-31
        assert_eq!(
            week_start_asia_taipei("2026-09-06T10:00:00.000Z"),
            Some("2026-08-31".to_string())
        );
    }

    #[test]
    fn fraction_optional() {
        assert_eq!(
            week_start_asia_taipei("2026-09-04T10:00:00Z"),
            Some("2026-08-31".to_string())
        );
    }

    #[test]
    fn bad_inputs_are_none() {
        for bad in [
            "",
            "2026-09-04",
            "2026-09-04T10:00:00", // 缺 Z
            "2026-09-04T10:00Z",   // 缺秒
            "2026-09-04X10:00:00Z",
            "2026-13-01T00:00:00Z", // 月 13
            "2026-02-30T00:00:00Z", // 2 月 30
            "2026-09-04T25:00:00Z", // 時 25
            "2026-09-04T10:60:00Z", // 分 60
            "2026-09-04T10:00:00.abcZ",
            "2026-09-04T10:00:00.000", // 缺 Z
        ] {
            assert_eq!(week_start_asia_taipei(bad), None, "input: {bad}");
        }
    }

    #[test]
    fn leap_year_feb29_accepted() {
        // 2024-02-29 為週四；台北 12:00 → 當週週一 2024-02-26
        assert_eq!(
            week_start_asia_taipei("2024-02-29T04:00:00.000Z"),
            Some("2024-02-26".to_string())
        );
        assert_eq!(week_start_asia_taipei("2025-02-29T04:00:00.000Z"), None);
    }

    #[test]
    fn monday_cycle_id_validation() {
        assert!(is_monday_cycle_id("2026-08-31"));
        assert!(is_monday_cycle_id("2026-09-07"));
        assert!(!is_monday_cycle_id("2026-09-04")); // 週五
        assert!(!is_monday_cycle_id("2026-13-01"));
        assert!(!is_monday_cycle_id("2026-09-04T10:00:00Z"));
        assert!(!is_monday_cycle_id(""));
    }
}
