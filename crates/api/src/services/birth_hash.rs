//! Birth-data hash — mirrors backend/src/services/birth-hash.ts EXACTLY.
//!
//! This is the cache-invalidation key stored in D1 (`users.birth_data_hash`,
//! `interpretations.birth_data_hash`). It must round-trip byte-for-byte with the
//! strings the TS worker produced, or every cached chart goes stale and the
//! DELETE-on-update invalidation breaks. Do not "improve" it.
//!
//! TS: `[y,m,d,h??12,min??0,gender??'',tz??'Asia/Taipei',lat??'',lon??''].join('-')`
//! then a signed 32-bit hash (`hash |= 0`, JS ToInt32) and `.toString(16)`.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BirthHashInput {
    pub birth_year: Option<i64>,
    pub birth_month: Option<i64>,
    pub birth_day: Option<i64>,
    pub birth_hour: Option<i64>,
    pub birth_minute: Option<i64>,
    pub gender: Option<String>,
    pub timezone: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Reproduce the TS `[ ... ].join('-')` with `??` defaults, then the djb2-ish
/// signed 32-bit hash. Number 0 must render as `"0"` (JS String(0)), never "".
fn join_parts(input: &BirthHashInput) -> String {
    let hour = input.birth_hour.unwrap_or(12);
    let minute = input.birth_minute.unwrap_or(0);
    let gender = input.gender.as_deref().unwrap_or("");
    let tz = input.timezone.as_deref().unwrap_or("Asia/Taipei");
    let lat = input.latitude.map(|f| render_f64(f)).unwrap_or_default();
    let lon = input.longitude.map(|f| render_f64(f)).unwrap_or_default();
    format!(
        "{}-{}-{}-{}-{}-{}-{}-{}-{}",
        num(input.birth_year),
        num(input.birth_month),
        num(input.birth_day),
        num(Some(hour)),
        num(Some(minute)),
        gender,
        tz,
        lat,
        lon
    )
}

fn num(v: Option<i64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => String::new(),
    }
}

/// JS `String(number)` for f64 — integral lat/lon render without a decimal point,
/// fractional render with minimal digits. We need this to match what TS produces
/// for the joined string (lat/lon are usually integers like 25 or 121.5).
fn render_f64(f: f64) -> String {
    if f.fract() == 0.0 && f.is_finite() {
        format!("{}", f as i64)
    } else {
        // Rust `{}` renders a float like JS `String()` for the values we care about
        // (e.g. 121.5 -> "121.5"). Integral lat/lon normally hit the branch above.
        format!("{}", f)
    }
}

/// Signed 32-bit hash reproducing the TS snippet.
fn hash_str(s: &str) -> String {
    let mut hash: i32 = 0;
    for ch in s.chars() {
        let code = ch as i32; // UTF-16 code unit; ASCII inputs only in practice
        hash = hash.wrapping_shl(5).wrapping_sub(hash).wrapping_add(code);
        // `hash |= 0` — ToInt32; i32 already sign-extends to the same value.
    }
    // JS `hash.toString(16)` for a negative number yields "-" + hex of magnitude.
    if hash < 0 {
        format!("-{:x}", (hash as i64).unsigned_abs())
    } else {
        format!("{:x}", hash)
    }
}

/// `computeBirthHash(data)` from the TS service.
pub fn compute_birth_hash(input: &BirthHashInput) -> String {
    hash_str(&join_parts(input))
}
