//! Invite codes: generation + usability predicate (spec 2026-08-30).
//! Registration requires a valid invite during beta; flip INVITE_REQUIRED to
//! open registration. Codes are crypto-random (fail-closed), drawn from a
//! 30-glyph alphabet without 0/O/1/I/L/U so a code read off a phone screen
//! cannot be misread. Modulo bias exists (256 % 30 != 0) and is acceptable:
//! an invite code is not a sole secret — it is rate-limited and quantity-bound.

use crate::services::uuid::secure_bytes;

/// Registration gate: true = signup (magic-link AND Google OAuth) demands a
/// valid invite code. Flipped to false on 2026-09-06 — the business model is
/// not settled, so gate management is deferred; when it is, flip this back to
/// re-arm BOTH doors at once (the OAuth consume lives in
/// routes/oauth.rs::resolve_google_user, keyed off this constant). A supplied
/// code is still validated + consumed as channel attribution either way.
pub const INVITE_REQUIRED: bool = false;

pub const CODE_LEN: usize = 10;

const CHARSET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ"; // 30 glyphs, no 0O1ILU

/// One `invites` row, as needed for the usability check.
#[derive(Debug, serde::Deserialize)]
pub struct InviteRow {
    pub used_count: i64,
    pub max_uses: i64,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

/// Mint a code, or `None` when crypto is unavailable (fail closed).
pub fn new_code() -> Option<String> {
    let bytes = secure_bytes(CODE_LEN)?;
    Some(bytes_to_code(&bytes))
}

/// Pure mapping bytes -> code (unit-testable off-wasm; js_sys is not).
fn bytes_to_code(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| CHARSET[(*b as usize) % CHARSET.len()] as char)
        .collect()
}

/// ISO-vs-ISO comparison only (see scripts/schema.sql note). The exact expiry
/// instant counts as expired, mirroring login_token semantics.
pub fn is_usable(row: &InviteRow, now_iso: &str) -> bool {
    row.revoked_at.is_none()
        && row.used_count < row.max_uses
        && row.expires_at.as_deref().is_none_or(|e| e > now_iso)
}

/// Strict shape check for an admin-supplied `expires_at` before it is stored:
/// exactly `YYYY-MM-DDTHH:MM:SS.mmmZ` — the shape `clock::now_iso()` emits — so
/// `is_usable`'s plain string comparison stays chronologically correct (uneven
/// fraction lengths break lexicographic ordering). Calendar-aware: month
/// lengths and leap years are checked.
pub fn valid_expires_iso(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 24
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'.'
        || b[23] != b'Z'
    {
        return false;
    }
    let f = |r: std::ops::Range<usize>| s.get(r).and_then(|p| p.parse::<u32>().ok());
    let (Some(y), Some(mo), Some(d), Some(h), Some(mi), Some(se), Some(_ms)) = (
        f(0..4),
        f(5..7),
        f(8..10),
        f(11..13),
        f(14..16),
        f(17..19),
        f(20..23),
    ) else {
        return false;
    };
    if !(1..=12).contains(&mo) || d == 0 || d > days_in_month(y, mo) {
        return false;
    }
    h < 24 && mi < 60 && se < 60
}

/// Days in a month, Gregorian leap rules.
fn days_in_month(y: u32, m: u32) -> u32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_uses_restricted_alphabet_and_length() {
        // bytes_to_code is the pure half of new_code; new_code itself needs
        // workerd's crypto and is covered by production E2E.
        let bytes: Vec<u8> = (0..=255u8).collect();
        let c = bytes_to_code(&bytes);
        assert_eq!(c.len(), 256);
        assert!(c.chars().all(|ch| CHARSET.contains(&(ch as u8))));
        assert!(!c.contains('0') && !c.contains('O') && !c.contains('1'));
    }

    #[test]
    fn code_len_matches_constant() {
        let c = bytes_to_code(&vec![7u8; CODE_LEN]);
        assert_eq!(c.len(), CODE_LEN);
    }

    #[test]
    fn usable_fresh_invite() {
        let row = row(0, 20, None, None);
        assert!(is_usable(&row, "2026-08-30T00:00:00.000Z"));
    }

    #[test]
    fn unusable_when_revoked() {
        let row = row(0, 20, None, Some("2026-08-29T00:00:00.000Z".into()));
        assert!(!is_usable(&row, "2026-08-30T00:00:00.000Z"));
    }

    #[test]
    fn unusable_when_expired_boundary_counts_as_expired() {
        let row = row(0, 20, Some("2026-08-30T00:00:00.000Z".into()), None);
        assert!(!is_usable(&row, "2026-08-30T00:00:00.000Z"));
        assert!(is_usable(&row, "2026-08-29T23:59:59.999Z"));
    }

    #[test]
    fn unusable_at_capacity() {
        let row = row(20, 20, None, None);
        assert!(!is_usable(&row, "2026-08-30T00:00:00.000Z"));
    }

    #[test]
    fn null_expiry_never_expires() {
        let row = row(0, 1, None, None);
        assert!(is_usable(&row, "2099-01-01T00:00:00.000Z"));
    }

    #[test]
    fn accepts_canonical_iso_with_millis() {
        assert!(valid_expires_iso("2026-09-30T23:59:59.999Z"));
        assert!(valid_expires_iso("2026-01-01T00:00:00.000Z"));
        assert!(valid_expires_iso("2024-02-29T12:00:00.000Z")); // leap year
    }

    #[test]
    fn rejects_wrong_shapes() {
        assert!(!valid_expires_iso(""));
        assert!(!valid_expires_iso("2026-09-30")); // date only
        assert!(!valid_expires_iso("2026-09-30T23:59")); // no seconds
        assert!(!valid_expires_iso("2026-09-30T23:59:59")); // no fraction, no Z
        assert!(!valid_expires_iso("2026-09-30T23:59:59Z")); // missing .mmm
        assert!(!valid_expires_iso("2026-09-30 23:59:59.000Z")); // space, not T
        assert!(!valid_expires_iso("2026-9-3T0:0:0.000Z")); // unpadded
        assert!(!valid_expires_iso("2026-09-30T23:59:59.999+08:00")); // not Z
    }

    #[test]
    fn rejects_impossible_calendar_dates() {
        assert!(!valid_expires_iso("2026-13-01T00:00:00.000Z")); // month 13
        assert!(!valid_expires_iso("2026-00-10T00:00:00.000Z")); // month 0
        assert!(!valid_expires_iso("2026-02-30T00:00:00.000Z")); // Feb 30
        assert!(!valid_expires_iso("2025-02-29T00:00:00.000Z")); // non-leap
        assert!(!valid_expires_iso("2026-09-31T00:00:00.000Z")); // Sep 31
        assert!(!valid_expires_iso("2026-09-30T24:00:00.000Z")); // hour 24
        assert!(!valid_expires_iso("2026-09-30T23:60:00.000Z")); // minute 60
        assert!(!valid_expires_iso("2026-09-30T23:59:60.000Z")); // second 60
    }

    fn row(used: i64, max: i64, exp: Option<String>, rev: Option<String>) -> InviteRow {
        InviteRow {
            used_count: used,
            max_uses: max,
            expires_at: exp,
            revoked_at: rev,
        }
    }
}
