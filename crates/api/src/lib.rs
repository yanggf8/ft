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
        res.headers_mut().set(
            "Access-Control-Allow-Methods",
            "GET,POST,PUT,DELETE,OPTIONS",
        )?;
        res.headers_mut().set(
            "Access-Control-Allow-Headers",
            "authorization,content-type,cache-control",
        )?;
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
        let ct = res
            .headers()
            .get("content-type")
            .ok()
            .flatten()
            .unwrap_or_default();
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
    res.headers_mut().set("X-Content-Type-Options", "nosniff")?;
    res.headers_mut().set("X-Frame-Options", "DENY")?;
    res.headers_mut().set("X-XSS-Protection", "1; mode=block")?;
    res.headers_mut()
        .set("Referrer-Policy", "strict-origin-when-cross-origin")?;
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

/// Resolves the allowed origin. Mirrors the TS `cors()` origin callback: only
/// localhost / 127.0.0.1 and `*.pages.dev` / `*.workers.dev` are allowed, matched
/// on the exact hostname (NOT substring), and no Origin header at all -> None so
/// no `Access-Control-Allow-*` is emitted (never `*` alongside credentials).
fn resolve_origin(req: &Request) -> Option<String> {
    let origin = req
        .headers()
        .get("Origin")
        .ok()
        .flatten()
        .filter(|v| !v.is_empty())?;
    // Parse as a full URL and compare the hostname exactly, so
    // `https://evil.com/path?next=localhost` or `https://notlocalhost.attacker.com`
    // are rejected even though they contain the literal "localhost"/"127.0.0.1".
    let url = match web_sys::Url::new(&origin) {
        Ok(u) => u,
        Err(_) => return None,
    };
    let host = url.hostname();
    let allowed = host == "localhost"
        || host == "127.0.0.1"
        || host == "[::1]"
        || host.ends_with(".pages.dev")
        || host.ends_with(".workers.dev");
    if allowed {
        Some(origin)
    } else {
        None
    }
}

fn gen_request_id() -> String {
    use js_sys::Math;
    let a = (Math::random() * 0xffff as f64) as u32;
    let b = (Math::random() * 0xffff as f64) as u32;
    format!("{:04x}{:04x}", a, b)
}
