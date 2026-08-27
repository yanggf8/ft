//! Clock helpers. The TS code used `new Date().toISOString()` (ISO strings) and
//! `Date.now()` (epoch ms). Both are needed across routes/DOs; keep them here so
//! every caller rounds to the same semantics.

use js_sys::Date as JsDate;

/// Epoch ms since 1970-01-01T00:00:00Z (mirrors `Date.now()`).
pub fn now_ms() -> f64 {
    JsDate::now()
}

/// Current time as an ISO-8601 UTC string (mirrors `new Date().toISOString()`).
pub fn now_iso() -> String {
    JsDate::new_0().to_iso_string().as_string().unwrap_or_default()
}

/// Current date as a `YYYY-MM-DD` string (mirrors `new Date().toISOString().split('T')[0]`).
pub fn today_utc() -> String {
    let iso = now_iso();
    iso.split('T').next().unwrap_or_default().to_string()
}

/// `now_ms() + N days` as an ISO string (mirrors the 30-day trial / 7-day session math).
pub fn now_plus_ms(ms: f64) -> String {
    let ms = now_ms() + ms;
    JsDate::new(&wasm_bindgen::JsValue::from_f64(ms))
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}
