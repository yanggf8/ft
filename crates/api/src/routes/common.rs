//! Shared route helpers — response builders, auth guard, ETag, chart metadata.

use worker::*;

use super::super::error;
use super::super::services::engine_version::{
    CHART_SCHEMA_VERSION, ENGINE_VERSION_WESTERN, ENGINE_VERSION_ZIWEI,
};

/// P2-01: serialization of a `serde_json::Value` cannot fail, so `from_json` only
/// errors in impossible states — fall back to a minimal static JSON rather than
/// panicking under `panic = "abort"`.
pub fn ok_json(v: &serde_json::Value, status: u16) -> Response {
    match Response::from_json(v) {
        Ok(res) => res.with_status(status),
        Err(_) => error::raw_json(status, error::FALLBACK_JSON),
    }
}

pub fn client_ip(req: &Request) -> String {
    req.headers()
        .get("cf-connecting-ip")
        .ok()
        .flatten()
        .or_else(|| req.headers().get("x-forwarded-for").ok().flatten())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Auth middleware — validates Bearer token against SESSION_DO, returns userId.
pub async fn auth_user(req: &Request, ctx: &RouteContext<()>) -> Result<String, Response> {
    let auth = req
        .headers()
        .get("authorization")
        .ok()
        .flatten()
        .unwrap_or_default();
    let sid = match auth.strip_prefix("Bearer ") {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => return Err(error::error("Missing authorization header", 401)),
    };
    let ns = match ctx.env.durable_object("SESSION_DO") {
        Ok(n) => n,
        Err(_) => return Err(error::error("Authentication failed", 401)),
    };
    let stub = match ns.id_from_name(&sid).and_then(|id| id.get_stub()) {
        Ok(s) => s,
        Err(_) => return Err(error::error("Authentication failed", 401)),
    };
    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    let req = match Request::new_with_init("http://do/get", &init) {
        Ok(r) => r,
        Err(_) => return Err(error::error("Authentication failed", 401)),
    };
    let mut res = match stub.fetch_with_request(req).await {
        Ok(r) => r,
        Err(_) => return Err(error::error("Authentication failed", 401)),
    };
    if res.status_code() != 200 {
        return Err(error::error("Invalid or expired session", 401));
    }
    let session: SessionInfo = match res.json().await {
        Ok(s) => s,
        Err(_) => return Err(error::error("Authentication failed", 401)),
    };
    // A-02 revocation epoch: sessions created before the magic-link deploy have
    // no ISO `createdAt` stamp -> treat as invalid (normal 401, no distinction).
    match session.createdAt.as_deref() {
        Some(c) if !c.is_empty() => {}
        _ => return Err(error::error("Invalid or expired session", 401)),
    }
    Ok(session.userId)
}

#[derive(Debug, serde::Deserialize)]
#[allow(non_snake_case)]
struct SessionInfo {
    userId: String,
    /// ISO creation stamp written by SessionDO since the magic-link deploy.
    #[serde(default)]
    createdAt: Option<String>,
}

/// Weak ETag string (mirrors createETag in cache.ts).
pub fn create_etag(hash: String, timestamp: Option<&str>) -> String {
    match timestamp {
        Some(t) if !t.is_empty() => format!("\"{}-{}\"", hash, t),
        _ => format!("\"{}\"", hash),
    }
}

/// Parse `chart_data` JSON string to a serde_json::Value; on failure return Null.
pub fn parse_chart(raw: Option<&str>) -> serde_json::Value {
    match raw {
        Some(s) if !s.is_empty() => serde_json::from_str(s).unwrap_or(serde_json::Value::Null),
        _ => serde_json::Value::Null,
    }
}

/// Extract the stored engine version from a parsed chart (meta.engineVersionZiwei, or
/// top-level engineVersion). Mirrors the TS `storedVersion` read in charts.ts.
pub fn extracted_version(parsed: &serde_json::Value) -> String {
    // Only `meta.engineVersion*` counts as "current". Do NOT fall back to the
    // top-level `engineVersion` written by the Phase A/B worker: a western cache
    // created before `embed_meta` started writing `meta` has no `meta.engineVersionWestern`,
    // and the old top-level `engineVersion` would have matched the expected version and
    // left a STALE (pre-fix) chart cached forever. A meta-less chart is now treated as
    // stale → recalculated on next GET. (§8.2 revealed the cached western asc=173°.)
    if let Some(v) = parsed
        .pointer("/meta/engineVersionZiwei")
        .and_then(|x| x.as_str())
    {
        return v.to_string();
    }
    if let Some(v) = parsed
        .pointer("/meta/engineVersionWestern")
        .and_then(|x| x.as_str())
    {
        return v.to_string();
    }
    String::new()
}

/// Embed per-type engine version + schema version into stored chart data (mirrors the
/// TS `chartDataWithVersion` builder). Always writes `meta` even when absent.
pub fn embed_meta(
    mut chart: serde_json::Value,
    div_type: &str,
    expected_version: &str,
) -> serde_json::Value {
    // Ensure `meta` exists — TS always spreads `...(chart.meta ?? {})` so we must create it.
    let has_meta = chart.get("meta").and_then(|m| m.as_object()).is_some();
    if has_meta {
        if let Some(meta) = chart.get_mut("meta").and_then(|m| m.as_object_mut()) {
            if div_type == "ziwei" {
                meta.insert(
                    "engineVersionZiwei".into(),
                    serde_json::json!(expected_version),
                );
            } else {
                meta.insert(
                    "engineVersionWestern".into(),
                    serde_json::json!(expected_version),
                );
            }
            meta.insert(
                "chartSchemaVersion".into(),
                serde_json::json!(CHART_SCHEMA_VERSION),
            );
        }
    } else if let Some(obj) = chart.as_object_mut() {
        let mut new_meta = serde_json::Map::new();
        if div_type == "ziwei" {
            new_meta.insert(
                "engineVersionZiwei".into(),
                serde_json::json!(expected_version),
            );
        } else {
            new_meta.insert(
                "engineVersionWestern".into(),
                serde_json::json!(expected_version),
            );
        }
        new_meta.insert(
            "chartSchemaVersion".into(),
            serde_json::json!(CHART_SCHEMA_VERSION),
        );
        obj.insert("meta".into(), serde_json::Value::Object(new_meta));
    }
    if let Some(obj) = chart.as_object_mut() {
        obj.insert("engineVersion".into(), serde_json::json!(expected_version));
        obj.insert(
            "chartSchemaVersion".into(),
            serde_json::json!(CHART_SCHEMA_VERSION),
        );
    }
    chart
}

/// Apply Cache-Control + Vary: Authorization headers. Mirrors setCacheHeaders in cache.ts
/// but always sets Vary even when max_age <= 0 (no-store still varies on Authorization).
pub fn apply_cache_headers(res: &mut Response, max_age: i32, must_revalidate: bool) {
    let cc = if max_age <= 0 {
        "no-store, no-cache, must-revalidate".to_string()
    } else {
        let mut directives = vec!["private".to_string(), format!("max-age={}", max_age)];
        if must_revalidate {
            directives.push("must-revalidate".to_string());
        }
        directives.join(", ")
    };
    let _ = res.headers_mut().set("Cache-Control", &cc);
    let _ = res.headers_mut().set("Vary", "Authorization");
}

/// `isStoryChartCurrent` — a story is current only when both engine versions and the
/// schema still match. Mirrors isStoryChartCurrent in charts.ts.
pub fn is_story_chart_current(raw_chart: Option<&str>) -> bool {
    let parsed = parse_chart(raw_chart);
    if parsed.is_null() {
        return false;
    }
    parsed
        .pointer("/meta/engineVersionZiwei")
        .and_then(|x| x.as_str())
        == Some(ENGINE_VERSION_ZIWEI)
        && parsed
            .pointer("/meta/engineVersionWestern")
            .and_then(|x| x.as_str())
            == Some(ENGINE_VERSION_WESTERN)
        && parsed
            .pointer("/meta/chartSchemaVersion")
            .and_then(|x| x.as_number())
            .map(|n| n.as_u64() == Some(CHART_SCHEMA_VERSION as u64))
            .unwrap_or(false)
}

/// RateLimitDO instance count — shards the keyspace so limiter traffic does not
/// funnel through a single global DO (P2-02).
const RATE_LIMIT_SHARDS: u32 = 8;

/// P2-02: rate limiting lives in `RateLimitDO` (cross-isolate, durable), replacing
/// the old isolate-local `OnceLock` limiter whose counters reset on every cold
/// start and whose budget was per-isolate, so "N per window per IP" never held
/// globally. `key` is caller namespaced (`login:ip:…`, `login:email:…`,
/// `verify:ip:…`, `personality:ip:…`, `ai:…`) so routes get independent buckets.
/// The DO instance is `rl:{fnv1a(key) % RATE_LIMIT_SHARDS}` — deterministic, so a
/// given key always lands on the same shard and its counter is consistent.
///
/// Fail-open: any DO roundtrip failure returns `true` (allowed). Availability
/// first — a transient limiter outage only widens the window for the duration,
/// it never takes login/AI endpoints down with it.
pub async fn rate_limit(ctx: &RouteContext<()>, key: &str, limit: u32, window_ms: f64) -> bool {
    let shard = fnv1a_64(key.as_bytes()) % u64::from(RATE_LIMIT_SHARDS);
    let do_name = format!("rl:{}", shard);
    let ns = match ctx.env.durable_object("RATE_LIMIT") {
        Ok(n) => n,
        Err(_) => return true,
    };
    let stub = match ns.id_from_name(&do_name).and_then(|id| id.get_stub()) {
        Ok(s) => s,
        Err(_) => return true,
    };
    let body = serde_json::json!({ "key": key, "limit": limit, "windowMs": window_ms });
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(body.to_string().into()));
    let req = match Request::new_with_init("http://do/check", &init) {
        Ok(r) => r,
        Err(_) => return true,
    };
    let mut res = match stub.fetch_with_request(req).await {
        Ok(r) => r,
        Err(_) => return true,
    };
    if res.status_code() != 200 {
        return true;
    }
    match res.json::<serde_json::Value>().await {
        Ok(v) => v.get("allowed").and_then(|a| a.as_bool()).unwrap_or(true),
        Err(_) => true,
    }
}

/// Deterministic tiny hash (FNV-1a 64) for DO sharding — stable across isolates
/// and releases, unlike `DefaultHasher` whose algorithm is unspecified.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
