//! Cryptographic randomness. The TS code used `crypto.randomUUID()`; this is
//! the same primitive via `web_sys` (`globalThis.crypto.randomUUID()`).
//!
//! Fail-closed policy (audit A-01): when `globalThis.crypto` is unavailable we
//! never degrade to a weak PRNG. Anything security-relevant (session ids,
//! magic-link tokens) must come from `getRandomValues`/`randomUUID` or not be
//! minted at all — the old `Math::random()` fallback was removed.

use wasm_bindgen::JsCast;

/// `globalThis.crypto`, or `None` when the runtime hides it. workerd always
/// exposes `crypto`; there is no `window`, so we read it off the global object
/// rather than `web_sys::window()`. A `None` here means "fail closed".
fn global_crypto() -> Option<web_sys::Crypto> {
    let global = js_sys::global();
    js_sys::Reflect::get(&global, &"crypto".into())
        .ok()
        .and_then(|v| v.dyn_into::<web_sys::Crypto>().ok())
}

/// Cryptographically strong random bytes via `crypto.getRandomValues`.
/// `None` = crypto unavailable (caller must abort the operation).
// TODO(P0-01 routes slice): drop this allow once routes/ calls secure_*.
#[allow(dead_code)]
pub fn secure_bytes(bytes: usize) -> Option<Vec<u8>> {
    let crypto = global_crypto()?;
    // getRandomValues caps the buffer at 65536 bytes; callers here stay far
    // below that, but guard anyway so an oversized request fails closed
    // instead of throwing a JS exception mid-request.
    if bytes == 0 || bytes > 65536 {
        return None;
    }
    let mut buf = vec![0u8; bytes];
    crypto.get_random_values_with_u8_array(&mut buf).ok()?;
    Some(buf)
}

/// Hex-encoded random token: `bytes` random bytes -> `2 * bytes` lowercase hex
/// chars. Use for anything security-relevant (magic-link tokens). `None` =
/// crypto unavailable; callers must fail closed (reject the login) rather than
/// fall back.
#[allow(dead_code)]
pub fn secure_token_hex(bytes: usize) -> Option<String> {
    let b = secure_bytes(bytes)?;
    Some(b.iter().map(|x| format!("{:02x}", x)).collect())
}

/// Generate a random UUIDv4 string (mirrors `crypto.randomUUID()`).
///
/// Signature is kept stable (`() -> String`) because callers in routes/
/// (personality.rs, auth.rs, charts.rs) still call it directly. Fail-closed:
/// **panics** if `globalThis.crypto` is unavailable. That is deliberate — a
/// weak id is an attacker-predictable session id, which is worse than a loud
/// 500. workerd always exposes `crypto`, so the panic is unreachable in
/// practice; the routes agent may migrate callers onto `secure_token_hex`
/// (`Option`-returning) later.
pub fn random_uuid() -> String {
    match global_crypto() {
        Some(c) => c.random_uuid(),
        None => panic!(
            "globalThis.crypto unavailable: refusing to mint a weak id (fail-closed, audit A-01)"
        ),
    }
}
