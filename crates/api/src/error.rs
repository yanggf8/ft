//! Error JSON helpers mirroring the TS routes' shape: `{ error, code? }` with a status.

use serde_json::json;
use worker::Response;

/// Fallback body for builders whose `Response::from_json` failed. Serialization of
/// these hand-built literals cannot fail in practice; when it somehow does, return a
/// minimal static JSON instead of panicking (P2-01: global `panic = "abort"` would
/// kill the isolate and leave the caller with a bare 500).
pub const FALLBACK_JSON: &[u8] = b"{\"error\":\"internal error\"}";

/// Infallible minimal JSON response — `Response::builder().fixed()` cannot fail
/// (no content-type header; only used on the unreachable fallback path).
pub fn raw_json(status: u16, body: &[u8]) -> Response {
    Response::builder().with_status(status).fixed(body.to_vec())
}

/// Build a JSON error response `{ "error": msg }` with the given status.
pub fn error(msg: impl Into<String>, status: u16) -> Response {
    match Response::from_json(&json!({ "error": msg.into() })) {
        Ok(res) => res.with_status(status),
        Err(_) => raw_json(status, FALLBACK_JSON),
    }
}

/// Build a JSON error response with an extra `code` field: `{ "error", "code" }`.
pub fn error_code(msg: impl Into<String>, code: impl Into<String>, status: u16) -> Response {
    match Response::from_json(&json!({ "error": msg.into(), "code": code.into() })) {
        Ok(res) => res.with_status(status),
        Err(_) => raw_json(status, FALLBACK_JSON),
    }
}

/// JSON success response (200 unless a status is passed).
pub fn ok(value: &serde_json::Value) -> Response {
    match Response::from_json(value) {
        Ok(res) => res,
        Err(_) => raw_json(500, FALLBACK_JSON),
    }
}

/// Build a response from a serde-serializable value (mirrors Hono's `c.json`).
pub fn json<T: serde::Serialize>(value: &T) -> Response {
    match Response::from_json(value) {
        Ok(res) => res,
        Err(_) => raw_json(500, FALLBACK_JSON),
    }
}
