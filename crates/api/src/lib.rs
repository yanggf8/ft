//! ft-api — the production API Worker (Stage B). Replaces the TS Hono Worker over
//! the same script name (`fortunet-api`), keeping the same class names and migration
//! tags so existing Durable Object storage (sessions, AI metrics) is bit-compatible.
//!
//! Layout mirrors backend/src:
//!   routes/          — auth, users, charts
//!   durable_objects/ — SessionDO, AIMutexDO, RateLimitDO (first two reuse the
//!                      same storage keys as the JS versions)
//!   services/        — billing, birth_hash, engine (service-binding client), ai

mod durable_objects;
mod error;
mod routes;
mod services;

use worker::*;

pub use durable_objects::{AIMutexDO, RateLimitDO, SessionDO};

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // Extra exact origins (comma-separated) for pages.dev preview deployments.
    // A missing var yields an empty list — the built-in allowlist still applies.
    let extra_origins = env
        .var("ALLOWED_ORIGINS")
        .map(|v| v.to_string())
        .unwrap_or_default();

    // OPTIONS preflight — return 204 immediately with CORS + security headers.
    if req.method() == Method::Options {
        let mut res = Response::empty()?.with_status(204);
        decorate(&mut res, &req, &extra_origins)?;
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
    decorate(&mut res, &req, &extra_origins)?;
    Ok(res)
}

/// Adds x-request-id, security headers, and dynamic CORS headers.
/// Mirrors backend/src/middleware/security.ts + backend/src/index.ts CORS config.
/// `extra_origins` is the raw comma-separated ALLOWED_ORIGINS env value.
fn decorate(res: &mut Response, req: &Request, extra_origins: &str) -> Result<()> {
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

    if let Some(allowed) = resolve_origin(req, extra_origins) {
        res.headers_mut()
            .set("Access-Control-Allow-Origin", &allowed)?;
    }
    // When resolve_origin returns None, no Allow-Origin header is set (null).
    // No Access-Control-Allow-Credentials: sessions ride in localStorage + Bearer,
    // never cookies, so the header would be pure risk (findings P2-03 / A-04).

    Ok(())
}

/// Resolves the allowed origin against an explicit allowlist (finding P2-03):
///   - exactly `https://fortunet.pages.dev` (production), or
///   - localhost dev: scheme http or https, host localhost / 127.0.0.1 / [::1],
///     any port, or
///   - an exact origin listed in the ALLOWED_ORIGINS env var (comma-separated,
///     used for pages.dev preview deployments).
/// Matched on scheme + hostname + port via web_sys::Url — never substring.
/// No Origin header at all -> None so no `Access-Control-Allow-Origin` is
/// emitted. Browsers and caches still disambiguate via the Vary: Origin set above.
fn resolve_origin(req: &Request, extra_origins: &str) -> Option<String> {
    let origin = req
        .headers()
        .get("Origin")
        .ok()
        .flatten()
        .filter(|v| !v.is_empty())?;
    // Parse as a full URL and compare the hostname exactly, so
    // `https://evil.com/path?next=localhost` or `https://evil.fortunet.pages.dev`
    // are rejected even though they contain the literal "localhost"/"fortunet".
    let url = match web_sys::Url::new(&origin) {
        Ok(u) => u,
        Err(_) => return None,
    };
    if is_allowed_origin(&url, extra_origins) {
        Some(origin)
    } else {
        None
    }
}

/// Allowlist test over an already-parsed origin. Allocation-light: scans
/// `extra_origins` in place with `split(',')`, no intermediate Vec.
fn is_allowed_origin(url: &web_sys::Url, extra_origins: &str) -> bool {
    let raw_protocol = url.protocol();
    let scheme = raw_protocol.trim_end_matches(':');
    let host = url.hostname();
    let port = url.port(); // empty string when default / absent

    // localhost dev — any port, http or https.
    if (scheme == "http" || scheme == "https")
        && (host == "localhost" || host == "127.0.0.1" || host == "[::1]")
    {
        return true;
    }

    // Production — exact origin only (https, default port, exact host).
    if scheme == "https" && host == "fortunet.pages.dev" && port.is_empty() {
        return true;
    }

    // Extra exact origins from ALLOWED_ORIGINS, compared the same way.
    extra_origins.split(',').any(|entry| {
        let entry = entry.trim();
        if entry.is_empty() {
            return false;
        }
        match web_sys::Url::new(entry) {
            Ok(u) => u.protocol() == url.protocol() && u.hostname() == host && u.port() == port,
            Err(_) => false,
        }
    })
}

fn gen_request_id() -> String {
    use js_sys::Math;
    let a = (Math::random() * 0xffff as f64) as u32;
    let b = (Math::random() * 0xffff as f64) as u32;
    format!("{:04x}{:04x}", a, b)
}
