//! UUIDv4 generation. The TS code used `crypto.randomUUID()`; this is the same
//! primitive via `web_sys` (`globalThis.crypto.randomUUID()`).

use wasm_bindgen::JsCast;

/// Generate a random UUIDv4 string (mirrors `crypto.randomUUID()`).
/// workerd exposes `globalThis.crypto`; there is no `window`, so we read crypto
/// off the global object rather than `web_sys::window()`.
pub fn random_uuid() -> String {
    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &"crypto".into())
        .ok()
        .and_then(|v| v.dyn_into::<web_sys::Crypto>().ok());
    match crypto {
        Some(c) => c.random_uuid(),
        None => fallback_uuid(),
    }
}

/// Minimal fallback if `globalThis.crypto` is unavailable. Not cryptographically
/// strong, but only reached if the platform hides `crypto` (unusual in Workers).
fn fallback_uuid() -> String {
    use js_sys::Math;
    let bytes: Vec<u8> = (0..16)
        .map(|_| ((Math::random() * 256.0) as u64 % 256) as u8)
        .collect();
    let mut s = String::with_capacity(36);
    for (i, b) in bytes.iter().enumerate() {
        if i == 4 || i == 6 || i == 8 || i == 10 {
            s.push('-');
        }
        let b = match i {
            6 => (b & 0x0f) | 0x40,   // version 4
            8 => (b & 0x3f) | 0x80,   // variant 10
            _ => *b,
        };
        s.push_str(&format!("{:02x}", b));
    }
    s
}
