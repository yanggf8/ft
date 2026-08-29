//! Route composition. Mirrors backend/src/index.ts + routes/.
//!
//! `Router`'s `data` is `()` — every handler reads Cloudflare bindings off
//! `ctx.env` (`.d1`, `.durable_object`, `.service`, `.secret`, `.var`), which the
//! router injects on `run`. Carrying the `Env` struct inside `data` as well only
//! duplicates what `ctx.env` already provides, so we keep `D = ()`.

use worker::*;

use super::services::{clock, db};

mod auth;
mod charts;
mod common;
mod personality;
mod users;

type R<'a> = Router<'a, ()>;

pub fn router(_env: Env) -> R<'static> {
    let a: R<'static> = auth::register(R::new());
    let u: R<'static> = users::register(a);
    let c: R<'static> = charts::register(u);
    let p: R<'static> = personality::register(c);

    p.get_async("/health", |_, ctx: RouteContext<()>| async move {
        let env_label = ctx
            .env
            .var("ENVIRONMENT")
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "development".to_string());
        ok_json(&serde_json::json!({
            "status": "ok",
            "timestamp": clock::now_iso(),
            "environment": env_label,
        }))
    })
    .get_async("/health/db", |_, ctx: RouteContext<()>| async move {
        let db = match ctx.env.d1("DB") {
            Ok(db) => db,
            Err(_) => {
                return ok_json(&serde_json::json!({ "status": "error", "error": "db not ready" }))
                    .map(|r| r.with_status(500))
            }
        };
        match db::first::<serde_json::Value>(&db, "SELECT 1 as ok", &[]).await {
            Ok(v) => ok_json(&serde_json::json!({ "status": "ok", "db": v })),
            Err(e) => ok_json(&serde_json::json!({ "status": "error", "error": e.to_string() }))
                .map(|r| r.with_status(500)),
        }
    })
    .get_async("/", |_, _: RouteContext<()>| async move {
        ok_json(&serde_json::json!({ "name": "FortuneT V2 API", "version": "1.0.0" }))
    })
}

fn ok_json(v: &serde_json::Value) -> worker::Result<Response> {
    Response::from_json(v)
}
