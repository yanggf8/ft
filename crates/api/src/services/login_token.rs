//! Magic-link login tokens (audit P0-01 foundation + A-01 fail-closed
//! randomness).
//!
//! Lifecycle: `new_token()` mints a 256-bit random token and returns
//! `(plain, hash)`. Only the SHA-256 hash is persisted
//! (`login_tokens.token_hash`); the plain value exists solely in the emailed
//! link. Verification is a D1 lookup on `token_hash` — constant-time compare
//! is not required (equality on the hash of a 256-bit random value leaks
//! nothing an attacker can grind on). The token hash is unit-tested; expiry
//! itself is enforced in SQL by routes/auth.rs (ISO-vs-ISO comparison — see
//! the note in scripts/schema.sql). F1 fix: this constant is the SINGLE
//! source of truth for the TTL; routes and the email copy both read it.

use crate::services::uuid::secure_token_hex;
use sha2::{Digest, Sha256};

/// Randomness per token: 32 bytes = 256 bits -> 64 lowercase hex chars.
pub const TOKEN_BYTES: usize = 32;

/// Link validity window: 10 minutes, in epoch ms.
pub const TOKEN_TTL_MS: u64 = 10 * 60 * 1000;

/// Mint a new login token. Returns `(token_plain, token_hash)` where the hash
/// is the only form ever stored. `None` = `globalThis.crypto` unavailable —
/// fail closed (reject the login attempt), never fall back (audit A-01).
pub fn new_token() -> Option<(String, String)> {
    let plain = secure_token_hex(TOKEN_BYTES)?;
    let hash = hash_token(&plain);
    Some((plain, hash))
}

/// SHA-256 of the plain token, lowercase hex — the persisted form.
pub fn hash_token(plain: &str) -> String {
    hex(&Sha256::digest(plain.as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const SHA256_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn hash_is_stable_and_matches_known_vectors() {
        assert_eq!(hash_token("abc"), SHA256_ABC);
        assert_eq!(hash_token(""), SHA256_EMPTY);
        assert_eq!(hash_token("abc"), hash_token("abc"));
    }

    #[test]
    fn hash_is_hex_64_and_lowercase() {
        let h = hash_token("some-token-value");
        assert_eq!(h.len(), 64, "sha-256 hex must be 64 chars");
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn different_inputs_hash_differently() {
        assert_ne!(hash_token("token-a"), hash_token("token-b"));
    }
}
