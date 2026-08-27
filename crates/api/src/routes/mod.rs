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
    .get_async("/debug/provider-models", |_, ctx: RouteContext<()>| async move {
        use crate::services::ai::providers::{CEREBRAS_BASE, GROQ_BASE, IFLOW_API_URL};
        async fn fetch_models(url: &str, key: &str) -> serde_json::Value {
            let mut headers = worker::Headers::new();
            let _ = headers.set("Authorization", &format!("Bearer {}", key));
            let mut init = worker::RequestInit::new();
            init.with_method(worker::Method::Get).with_headers(headers);
            let req = match worker::Request::new_with_init(url, &init) {
                Ok(r) => r,
                Err(e) => return serde_json::json!({ "error": e.to_string() }),
            };
            let res = match worker::Fetch::Request(req).send().await {
                Ok(r) => r,
                Err(e) => return serde_json::json!({ "error": e.to_string() }),
            };
            let status = res.status_code();
            let mut res = res;
            let body: serde_json::Value = res.json().await.unwrap_or(serde_json::json!({ "raw": "parse failed" }));
            serde_json::json!({ "status": status, "body": body })
        }
        let iflow_key = ctx.env.secret("IFLOW_API_KEY").map(|s| s.to_string()).unwrap_or_default();
        let groq_key = ctx.env.secret("GROQ_API_KEY").map(|s| s.to_string()).unwrap_or_default();
        let cerebras_key = ctx.env.secret("CEREBRAS_API_KEY").map(|s| s.to_string()).unwrap_or_default();
        let iflow = if !iflow_key.is_empty() {
            fetch_models(&format!("{}/models", IFLOW_API_URL.replace("/chat/completions", "")), &iflow_key).await
        } else { serde_json::json!({ "error": "no key" }) };
        let groq = if !groq_key.is_empty() {
            fetch_models(&format!("{}/models", GROQ_BASE), &groq_key).await
        } else { serde_json::json!({ "error": "no key" }) };
        let cerebras = if !cerebras_key.is_empty() {
            fetch_models(&format!("{}/models", CEREBRAS_BASE), &cerebras_key).await
        } else { serde_json::json!({ "error": "no key" }) };
        Ok(ok_json(&serde_json::json!({ "iflow": iflow, "groq": groq, "cerebras": cerebras })))
    })
    .get_async("/", |_, _: RouteContext<()>| async move {
        Ok(ok_json(&serde_json::json!({ "name": "FortuneT V2 API", "version": "1.0.0" })))
    })
}

fn ok_json(v: &serde_json::Value) -> Response {
    Response::from_json(v).expect("ok json")
}
