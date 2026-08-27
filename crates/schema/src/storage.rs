//! Durable Object storage compatibility contract (Stage B single-way gate).
//!
//! The Rust worker deploys OVER the existing `fortunet-api` Worker on the same
//! script name, so it must read the exact bytes the old JS Durable Objects wrote.
//! These are not `serde_json` shapes: `worker::Storage` (Stage B will use it) goes
//! through `serde_wasm_bindgen`, i.e. the JS value is unwrapped by workerd and then
//! deserialized by serde_wasm_bindgen. Field names and numeric widths must match
//! what JS produced, byte-for-byte at the JS-object level.
//!
//! The gate: `storage.get::<T>(key)` must deserialize the value that the previous
//! JS `storage.put(key, value)` stored. `serde_wasm_bindgen` maps JS objects to
//! structs by field name, and JS numbers to the Rust integer/float type. Because a
//! JS number is a float64, we use `f64` for every value written as a number from TS
//! (`Date.now()`, counts, timestamps): `i64`/`u64` can fail or silently saturate on
//! >2^53, and the intent is to round-trip the number exactly.

// The field names below are SEMANTIC: they are the exact JS object keys the old
// Durable Objects wrote (e.g. `expiresAt`, `latencySum`, `lastError`). Renaming to
// snake_case would silently break the Stage B single-way gate. Suppress the lint
// for this module rather than change them.
#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

/// `SESSION_DO` — JS stored the whole session under the single key `"session"`.
///
/// JS: `await this.state.storage.put('session', { userId, email, createdAt, expiresAt })`
/// Field order does not matter to serde_wasm_bindgen (it reads by name).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionDoRecord {
    pub userId: String,
    pub email: String,
    /// `Date.now()` — ms since epoch stored as a JS number (f64).
    pub createdAt: f64,
    /// `Date.now() + 7d` — ms since epoch stored as a JS number (f64).
    pub expiresAt: f64,
}

/// `AIMutexDO` — per-minute rate limit record, key `rpm:{provider}`.
///
/// JS: `{ count: number, reset: number }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinuteRecord {
    pub count: f64,
    /// `Date.now() + 60000` — epoch ms as a JS number (f64).
    pub reset: f64,
}

/// `AIMutexDO` — external-resource metric, key `exresource:{provider}:{date}`.
///
/// JS: `{ requests, tokens, errors, latencySum, failovers, lastError? }`
/// `lastError` is optional (absent until the first error). `f64` everywhere for
/// the same float round-trip reason as `SessionDoRecord`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExResource {
    pub requests: f64,
    pub tokens: f64,
    pub errors: f64,
    pub latencySum: f64,
    pub failovers: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lastError: Option<ExResourceError>,
}

/// Nested `lastError` object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExResourceError {
    pub time: String,
    pub code: String,
    pub message: String,
}

/// Provider names as used in `AIMutexDO` keys and the exresource `provider` field.
pub const AIMUTEX_PROVIDERS: [&str; 3] = ["iflow", "groq", "cerebras"];

/// Storage key under which `SessionDO` keeps its one session value.
pub const SESSION_KEY: &str = "session";

/// Build the `rpm:{provider}` key.
pub fn rpm_key(provider: &str) -> String {
    format!("rpm:{}", provider)
}

/// Build the `exresource:{provider}:{date}` key. `date` is `YYYY-MM-DD` (UTC).
pub fn exresource_key(provider: &str, date: &str) -> String {
    format!("exresource:{}:{}", provider, date)
}
