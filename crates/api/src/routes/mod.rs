//! Route composition. Mirrors backend/src/index.ts + routes/.
//!
//! `Router`'s `data` is `()` — every handler reads Cloudflare bindings off
//! `ctx.env` (`.d1`, `.durable_object`, `.service`, `.secret`, `.var`), which the
//! router injects on `run`. Carrying the `Env` struct inside `data` as well only
//! duplicates what `ctx.env` already provides, so we keep `D = ()`.

use worker::*;

use super::services::{clock, db};
use crate::error;

mod auth;
mod charts;
mod common;
mod users;

type R<'a> = Router<'a, ()>;

pub fn router(_env: Env) -> R<'static> {
    let a: R<'static> = auth::register(R::new());
    let u: R<'static> = users::register(a);
    let c: R<'static> = charts::register(u);

    c.get_async("/health", |_, ctx: RouteContext<()>| async move {
        let env_label = ctx
            .env
            .var("ENVIRONMENT")
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "development".to_string());
        Ok(ok_json(&serde_json::json!({
            "status": "ok",
            "timestamp": clock::now_iso(),
            "environment": env_label,
        })))
    })
    .get_async("/health/db", |_, ctx: RouteContext<()>| async move {
        let db = match ctx.env.d1("DB") {
            Ok(db) => db,
            Err(_) => return Ok(Response::from_json(&serde_json::json!({ "status": "error", "error": "db not ready" })).expect("ok json").with_status(500)),
        };
        match db::first::<serde_json::Value>(&db, "SELECT 1 as ok", &[]).await {
            Ok(v) => Ok(ok_json(&serde_json::json!({ "status": "ok", "db": v }))),
            Err(e) => Ok(Response::from_json(&serde_json::json!({ "status": "error", "error": e.to_string() })).expect("ok json").with_status(500)),
        }
    })
    .get_async("/", |_, _: RouteContext<()>| async move {
        Ok(ok_json(&serde_json::json!({ "name": "FortuneT V2 API", "version": "1.0.0" })))
    })
}

fn ok_json(v: &serde_json::Value) -> Response {
    Response::from_json(v).expect("ok json")
}
