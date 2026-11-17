//! 先猜後試的日期算術（spec §6 T7、Fable F14）。
//! 重用 `cycle.rs` 的 `parse_iso_parts`／`civil_from_days`（Hinnant 演算法），
//! 不引入 chrono／js_sys；native 與 wasm32 皆可編。

use crate::cycle::{civil_from_days, parse_iso_parts};

/// 一天的毫秒數。
pub const DAY_MS: i64 = 86_400_000;

/// `YYYY-MM-DDTHH:MM:SS(.fff)?Z` → epoch 毫秒；不可解析 → `None`。
/// 小數秒取前三位（不足補零、超過截斷）。
pub fn parse_iso_ms(iso: &str) -> Option<i64> {
    let (days, secs) = parse_iso_parts(iso)?;
    let body = iso.strip_suffix(['Z', 'z'])?;
    let ms = match body.rsplit_once('.') {
        Some((_, frac)) => {
            let mut v: i64 = 0;
            for i in 0..3 {
                let digit = frac.as_bytes().get(i).map_or(0, |b| i64::from(b - b'0'));
                v = v * 10 + digit;
            }
            v
        }
        None => 0,
    };
    (days * 86_400 + secs).checked_mul(1_000)?.checked_add(ms)
}

/// epoch 毫秒 → 固定 24 字 `YYYY-MM-DDTHH:MM:SS.sssZ`；年份超出 0000–9999 → `None`。
pub fn format_iso_ms(ms: i64) -> Option<String> {
    let days = ms.div_euclid(DAY_MS);
    let rem = ms.rem_euclid(DAY_MS);
    let (y, m, d) = civil_from_days(days);
    if !(0..=9999).contains(&y) {
        return None;
    }
    let h = rem / 3_600_000;
    let mi = rem / 60_000 % 60;
    let s = rem / 1_000 % 60;
    let milli = rem % 1_000;
    Some(format!(
        "{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{milli:03}Z"
    ))
}

/// ISO 時刻平移 `days` 天（可為負）；輸出固定 24 字格式，解析失敗或溢位 → `None`。
pub fn iso_shift_days(iso: &str, days: i64) -> Option<String> {
    let base = parse_iso_ms(iso)?;
    format_iso_ms(base.checked_add(days.checked_mul(DAY_MS)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_keeps_millis_and_fixed_format() {
        assert_eq!(
            iso_shift_days("2026-11-17T08:30:15.123Z", 7).as_deref(),
            Some("2026-11-24T08:30:15.123Z")
        );
        let out = iso_shift_days("2026-11-17T08:30:15Z", 0).unwrap();
        assert_eq!(out, "2026-11-17T08:30:15.000Z");
        assert_eq!(out.chars().count(), 24);
    }

    #[test]
    fn shift_crosses_month_year_and_leap_day() {
        assert_eq!(
            iso_shift_days("2026-12-28T00:00:00.000Z", 30).as_deref(),
            Some("2027-01-27T00:00:00.000Z")
        );
        assert_eq!(
            iso_shift_days("2028-02-28T12:00:00.000Z", 1).as_deref(),
            Some("2028-02-29T12:00:00.000Z")
        );
        assert_eq!(
            iso_shift_days("2026-03-01T00:00:00.000Z", -1).as_deref(),
            Some("2026-02-28T00:00:00.000Z")
        );
    }

    #[test]
    fn fraction_is_truncated_or_padded_to_millis() {
        assert_eq!(
            iso_shift_days("2026-11-17T00:00:00.5Z", 0).as_deref(),
            Some("2026-11-17T00:00:00.500Z")
        );
        assert_eq!(
            iso_shift_days("2026-11-17T00:00:00.123456Z", 0).as_deref(),
            Some("2026-11-17T00:00:00.123Z")
        );
    }

    #[test]
    fn unparsable_input_is_none() {
        for bad in [
            "",
            "2026-11-17",
            "2026-11-17T00:00:00",
            "2026-11-17T00:00:00+08:00",
            "2026-13-01T00:00:00.000Z",
            "2026-02-30T00:00:00.000Z",
            "2026-11-17T24:00:00.000Z",
            "2026-11-17T00:00:00.Z",
            "2026-11-17T00:00:00.abcZ",
        ] {
            assert_eq!(iso_shift_days(bad, 1), None, "{bad}");
        }
    }

    #[test]
    fn overflow_or_out_of_range_year_is_none() {
        assert_eq!(iso_shift_days("2026-11-17T00:00:00.000Z", i64::MAX), None);
        assert_eq!(iso_shift_days("9999-12-31T00:00:00.000Z", 1), None);
    }
}
