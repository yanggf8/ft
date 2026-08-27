//! Error JSON helpers mirroring the TS routes' shape: `{ error, code? }` with a status.

use serde_json::json;
use worker::Response;

/// Build a JSON error response `{ "error": msg }` with the given status.
pub fn error(msg: impl Into<String>, status: u16) -> Response {
    Response::from_json(&json!({ "error": msg.into() }))
        .expect("build error response")
        .with_status(status)
}

/// Build a JSON error response with an extra `code` field: `{ "error", "code" }`.
pub fn error_code(msg: impl Into<String>, code: impl Into<String>, status: u16) -> Response {
    Response::from_json(&json!({ "error": msg.into(), "code": code.into() }))
        .expect("build error_code response")
        .with_status(status)
}

/// JSON success response (200 unless a status is passed).
pub fn ok(value: &serde_json::Value) -> Response {
    Response::from_json(value).expect("build ok response")
}

/// Build a response from a serde-serializable value (mirrors Hono's `c.json`).
pub fn json<T: serde::Serialize>(value: &T) -> Response {
    Response::from_json(value).expect("build json response")
}
