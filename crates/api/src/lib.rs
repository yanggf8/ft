//! ft-api — the production API Worker (Stage B). Replaces the TS Hono Worker over
//! the same script name (`fortunet-api`), keeping the same class names and migration
//! tags so existing Durable Object storage (sessions, AI metrics) is bit-compatible.
//!
//! Layout mirrors backend/src:
//!   routes/          — auth, users, charts
//!   durable_objects/ — SessionDO, AIMutexDO (same storage keys as the JS versions)
//!   services/        — billing, birth_hash, engine (service-binding client), ai

mod durable_objects;
mod error;
mod routes;
mod services;

use worker::*;

pub use durable_objects::{AIMutexDO, SessionDO};

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // OPTIONS preflight — return 204 immediately with CORS + security headers.
    if req.method() == Method::Options {
        let mut res = Response::empty()?.with_status(204);
        decorate(&mut res, &req)?;
        res.headers_mut()
            .set("Access-Control-Allow-Methods", "GET,POST,PUT,DELETE,OPTIONS")?;
        res.headers_mut()
            .set("Access-Control-Allow-Headers", "authorization,content-type")?;
        res.headers_mut().set("Access-Control-Max-Age", "86400")?;
        return Ok(res);
    }

    let router = routes::router(env.clone());
    let outgoing = router.run(req.clone()?, env).await;

    let mut res = match outgoing {
        Ok(r) => r,
        Err(_) => error::error("Internal server error", 500),
    };
    // TS app.notFound returns { error: "Not found" } JSON. Worker Router's fallback
    // is plain-text 404 — normalize it to JSON so clients always get the same shape.
    if res.status_code() == 404 {
        let ct = res.headers().get("content-type").ok().flatten().unwrap_or_default();
        if !ct.contains("application/json") {
            res = error::error("Not found", 404);
        }
    }
    decorate(&mut res, &req)?;
    Ok(res)
}

/// Adds x-request-id, security headers, and dynamic CORS headers.
/// Mirrors backend/src/middleware/security.ts + backend/src/index.ts CORS config.
fn decorate(res: &mut Response, req: &Request) -> Result<()> {
    // x-request-id — mirrors TS crypto.randomUUID().slice(0,8)
    res.headers_mut().set("x-request-id", &gen_request_id())?;

    // Security headers — mirrors backend/src/middleware/security.ts
    res.headers_mut()
        .set("X-Content-Type-Options", "nosniff")?;
    res.headers_mut().set("X-Frame-Options", "DENY")?;
    res.headers_mut()
        .set("X-XSS-Protection", "1; mode=block")?;
    res.headers_mut().set(
        "Referrer-Policy",
        "strict-origin-when-cross-origin",
    )?;
    res.headers_mut().set(
        "Permissions-Policy",
        "geolocation=(), microphone=(), camera=()",
    )?;
    res.headers_mut().set(
        "Content-Security-Policy",
        "default-src 'none'; frame-ancestors 'none'",
    )?;

    // CORS — mirrors TS cors origin callback in backend/src/index.ts
    // Vary: Origin must always be present so caches differentiate by Origin.
    // Preserve any existing Vary (e.g. Authorization from cache headers) by merging.
    let existing_vary = res.headers().get("Vary").ok().flatten().unwrap_or_default();
    let merged_vary = if existing_vary.is_empty() {
        "Origin".to_string()
    } else if existing_vary.contains("Origin") {
        existing_vary
    } else {
        format!("{}, Origin", existing_vary)
    };
    res.headers_mut().set("Vary", &merged_vary)?;

    if let Some(allowed) = resolve_origin(req) {
        res.headers_mut()
            .set("Access-Control-Allow-Origin", &allowed)?;
        res.headers_mut()
            .set("Access-Control-Allow-Credentials", "true")?;
    }
    // When resolve_origin returns None, no Allow-Origin / Allow-Credentials header is set (null).

    Ok(())
}

/// Resolves the allowed origin per TS logic:
///   if !origin -> "*"
///   if origin contains localhost / 127.0.0.1 -> origin
///   if origin ends with .pages.dev / .workers.dev -> origin
///   else -> null (None)
fn resolve_origin(req: &Request) -> Option<String> {
    let origin = match req.headers().get("Origin").ok().flatten() {
        Some(v) if !v.is_empty() => v,
        _ => return Some("*".to_string()),
    };
    if origin.contains("localhost") || origin.contains("127.0.0.1") {
        return Some(origin);
    }
    if origin.ends_with(".pages.dev") || origin.ends_with(".workers.dev") {
        return Some(origin);
    }
    None
}

fn gen_request_id() -> String {
    use js_sys::Math;
    let a = (Math::random() * 0xffff as f64) as u32;
    let b = (Math::random() * 0xffff as f64) as u32;
    format!("{:04x}{:04x}", a, b)
}
