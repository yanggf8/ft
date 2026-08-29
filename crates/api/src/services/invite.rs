//! Invite codes: generation + usability predicate (spec 2026-08-30).
//! Registration requires a valid invite during beta; flip INVITE_REQUIRED to
//! open registration. Codes are crypto-random (fail-closed), drawn from a
//! 30-glyph alphabet without 0/O/1/I/L/U so a code read off a phone screen
//! cannot be misread. Modulo bias exists (256 % 30 != 0) and is acceptable:
//! an invite code is not a sole secret — it is rate-limited and quantity-bound.

use crate::services::uuid::secure_bytes;

/// Beta gate: true = register demands a valid invite code.
pub const INVITE_REQUIRED: bool = true;

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
        && row
            .expires_at
            .as_deref()
            .is_none_or(|e| e > now_iso)
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

    fn row(used: i64, max: i64, exp: Option<String>, rev: Option<String>) -> InviteRow {
        InviteRow {
            used_count: used,
            max_uses: max,
            expires_at: exp,
            revoked_at: rev,
        }
    }
}
