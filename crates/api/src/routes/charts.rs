//! Charts routes — mirrors backend/src/routes/charts.ts.
//! GET /api/charts, GET /api/charts/story, POST /api/charts/story/generate,
//! GET /api/charts/:type, POST /api/charts/:type/interpret.
//! Chart calculations come from the engine Worker (FT_ENGINE service binding); D1
//! holds the per-(user,type) cache; AIMutexDO serializes AI.

use std::sync::{Arc, Mutex};
use worker::*;

use super::common::{apply_cache_headers, auth_user, client_ip, create_etag, embed_meta, extracted_version, is_story_chart_current, ok_json, parse_chart};
use super::super::error;
use super::super::services::ai::ProviderResult;
use super::super::services::clock;
use super::super::services::db;
use super::super::services::engine::{self, EngineBirth};
use super::super::services::engine_version::{CHART_SCHEMA_VERSION, ENGINE_VERSION_WESTERN, ENGINE_VERSION_ZIWEI};
use super::super::services::uuid;
use super::R;

const AI_RATE_LIMIT: u32 = 10;
const AI_WINDOW_MS: f64 = 60000.0;

#[derive(Default)]
struct AiRateLimiter {
    entries: std::collections::HashMap<String, (u32, f64)>,
}
impl AiRateLimiter {
    fn check(&mut self, ip: &str) -> bool {
        let now = clock::now_ms();
        match self.entries.get(ip).copied() {
            Some((count, reset)) if now <= reset => {
                if count >= AI_RATE_LIMIT {
                    false
                } else {
                    self.entries.get_mut(ip).unwrap().0 = count + 1;
                    true
                }
            }
            _ => {
                self.entries.insert(ip.to_string(), (1, now + AI_WINDOW_MS));
                true
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct UserBirthRow {
    birth_year: Option<i64>,
    birth_month: Option<i64>,
    birth_day: Option<i64>,
    birth_hour: Option<i64>,
    birth_minute: Option<i64>,
    gender: Option<String>,
    timezone: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    birth_data_hash: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct InterpRow {
    id: String,
    chart_data: Option<String>,
    ai_interpretation: Option<String>,
    birth_data_hash: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

pub fn register(router: R<'static>) -> R<'static> {
    let ai_limiter = Arc::new(Mutex::new(AiRateLimiter::default()));
    let ai_limiter2 = ai_limiter.clone();

    router
        .get_async("/api/charts", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(r) => return Ok(r),
            };
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let u = db::text(&user);
            let rows: Vec<serde_json::Value> = match db::all(&db, "SELECT * FROM interpretations WHERE user_id = ?1 ORDER BY created_at DESC", &[&u]).await {
                Ok(v) => v,
                Err(e) => return Ok(error::error(format!("db error: {}", e), 500)),
            };
            let interpretations: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|mut row| {
                    if let Some(map) = row.as_object_mut() {
                        if let Some(cd) = map.get("chart_data").and_then(|v| v.as_str()) {
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(cd) {
                                map.insert("chart_data".to_string(), parsed);
                            }
                        }
                    }
                    row
                })
                .collect();
            Ok(ok_json(&serde_json::json!({ "interpretations": interpretations }), 200))
        })
        .get_async("/api/charts/story", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(r) => return Ok(r),
            };
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let birth = match get_birth_data(&db, &user).await {
                Ok(b) => b,
                Err(r) => return Ok(r),
            };
            let hash_of = birth.birth_data_hash.as_deref().unwrap_or("");
            let u = db::text(&user);
            let bh = db::opt_text(birth.birth_data_hash.as_deref());
            let row: Option<StoryRow> = match db::first(
                &db,
                "SELECT id, chart_data, ai_interpretation, updated_at FROM interpretations WHERE user_id = ?1 AND divination_type = 'story' AND birth_data_hash = ?2",
                &[&u, &bh],
            ).await {
                Ok(r) => r,
                Err(e) => return Ok(error::error(format!("db error: {}", e), 500)),
            };
            let row = match row {
                Some(r) => r,
                None => return Ok(error::error_code("No story yet. POST /story/generate first", "NO_STORY", 404)),
            };
            if let Some(ai) = &row.ai_interpretation {
                if !is_story_chart_current(row.chart_data.as_deref()) {
                    return Ok(error::error_code("Chart version stale, regeneration required", "RECALC_REQUIRED", 409));
                }
                let etag = create_etag(format!("{}-story", hash_of), row.updated_at.as_deref());
                if let Some(inm) = req.headers().get("if-none-match").ok().flatten() {
                    if inm == etag {
                        let mut res = Response::empty().unwrap().with_status(304);
                        let _ = res.headers_mut().set("ETag", &etag);
                        apply_cache_headers(&mut res, 86400, true);
                        return Ok(res);
                    }
                }
                let mut res = ok_json(&serde_json::json!({ "story": ai, "fromCache": true }), 200);
                let _ = res.headers_mut().set("ETag", &etag);
                apply_cache_headers(&mut res, 86400, true);
                return Ok(res);
            }
            Ok(error::error_code("No story yet. POST /story/generate first", "NO_STORY", 404))
        })
        .post_async("/api/charts/story/generate", {
            let limiter = ai_limiter.clone();
            move |req, ctx| {
                let limiter = limiter.clone();
                async move {
                    let user = match auth_user(&req, &ctx).await {
                        Ok(u) => u,
                        Err(r) => return Ok(r),
                    };
                    if !limiter.lock().unwrap().check(&client_ip(&req)) {
                        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
                    }
                    let db = match ctx.env.d1("DB") {
                        Ok(d) => d,
                        Err(_) => return Ok(error::error("db unavailable", 500)),
                    };
                    let birth = match get_birth_data(&db, &user).await {
                        Ok(b) => b,
                        Err(r) => return Ok(r),
                    };
                    if birth.birth_year.is_none() || birth.birth_month.is_none() || birth.birth_day.is_none() {
                        return Ok(error::error_code("Birth data required", "NO_BIRTH_DATA", 400));
                    }
                    if birth.gender.is_none() {
                        return Ok(error::error_code("Gender required", "NO_GENDER", 400));
                    }

                    let hash = birth.birth_data_hash.clone().unwrap_or_default();
                    let u = db::text(&user);
                    let bh = db::opt_text(birth.birth_data_hash.as_deref());
                    let existing: Option<StoryRow> = match db::first(
                        &db,
                        "SELECT chart_data, ai_interpretation FROM interpretations WHERE user_id = ?1 AND divination_type = 'story' AND birth_data_hash = ?2",
                        &[&u, &bh],
                    ).await {
                        Ok(r) => r,
                        Err(e) => return Ok(error::error(format!("db error: {}", e), 500)),
                    };
                    if let Some(ex) = existing {
                        if let Some(ai) = &ex.ai_interpretation {
                            if is_story_chart_current(ex.chart_data.as_deref()) {
                                let mut res = ok_json(&serde_json::json!({ "story": ai, "fromCache": true }), 200);
                                apply_cache_headers(&mut res, 86400, true);
                                return Ok(res);
                            }
                        }
                    }

                    let hour = birth.birth_hour.unwrap_or(12);
                    let engine = match ctx.env.service("FT_ENGINE") {
                        Ok(f) => f,
                        Err(_) => return Ok(error::error("engine unavailable", 500)),
                    };
                    let ziwei = match engine::fetch_engine_chart(&engine, "ziwei", &EngineBirth {
                        year: birth.birth_year, month: birth.birth_month, day: birth.birth_day,
                        hour, gender: birth.gender.clone(), latitude: None, longitude: None,
                        timezone: birth.timezone.clone(),
                    }).await {
                        Ok(v) => v,
                        Err(e) => return Ok(error::error(format!("{}", e), 502)),
                    };
                    let western = match engine::fetch_engine_chart(&engine, "western", &EngineBirth {
                        year: birth.birth_year, month: birth.birth_month, day: birth.birth_day,
                        hour, gender: None, latitude: birth.latitude, longitude: birth.longitude,
                        timezone: birth.timezone.clone(),
                    }).await {
                        Ok(v) => v,
                        Err(e) => return Ok(error::error(format!("{}", e), 502)),
                    };
                    let merged = serde_json::json!({
                        "ziwei": ziwei, "western": western,
                        "meta": {
                            "engineVersionZiwei": ENGINE_VERSION_ZIWEI,
                            "engineVersionWestern": ENGINE_VERSION_WESTERN,
                            "chartSchemaVersion": CHART_SCHEMA_VERSION,
                        }
                    });

                    if !anything_configured(&ctx) {
                        return Ok(error::error("AI service not configured", 503));
                    }
                    let ai_resp = match call_ai_mutex(&ctx, "story", &merged).await {
                        Ok(Some(r)) => r,
                        Ok(None) => return Ok(error::error("AI service temporarily unavailable, please try again", 503)),
                        Err(r) => return Ok(r),
                    };
                    // TS returns 502 EMPTY_STORY when the AI returns an empty string
                    if ai_resp.interpretation.trim().is_empty() {
                        return Ok(error::error_code("AI returned an empty story, please try again", "EMPTY_STORY", 502));
                    }
                    let id = uuid::random_uuid();
                    let id_t = db::text(&id);
                    let uid_t = db::text(&user);
                    let merged_str = merged.to_string();
                    let chart_t = db::text(&merged_str);
                    let ai_t = db::text(&ai_resp.interpretation);
                    let bh_t = db::text(&hash);
                    if let Err(e) = db::exec(
                        &db,
                        "INSERT INTO interpretations (id, user_id, divination_type, chart_data, ai_interpretation, birth_data_hash) VALUES (?1, ?2, 'story', ?3, ?4, ?5) ON CONFLICT(user_id, divination_type) DO UPDATE SET chart_data = excluded.chart_data, ai_interpretation = excluded.ai_interpretation, birth_data_hash = excluded.birth_data_hash, updated_at = datetime('now')",
                        &[&id_t, &uid_t, &chart_t, &ai_t, &bh_t],
                    ).await {
                        return Ok(error::error(format!("db error: {}", e), 500));
                    }
                    let mut res = ok_json(&serde_json::json!({
                        "story": ai_resp.interpretation, "provider": ai_resp.provider, "model": ai_resp.model, "fromCache": false,
                    }), 200);
                    apply_cache_headers(&mut res, 86400, true);
                    Ok(res)
                }
            }
        })
        .get_async("/api/charts/:type", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(r) => return Ok(r),
            };
            let div_type = match ctx.param("type") {
                Some(v) if v == "ziwei" || v == "western" => v.to_string(),
                _ => return Ok(error::error("Invalid type. Use: ziwei, western", 400)),
            };
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let birth = match get_birth_data(&db, &user).await {
                Ok(b) => b,
                Err(r) => return Ok(r),
            };
            if birth.birth_year.is_none() || birth.birth_month.is_none() || birth.birth_day.is_none() {
                return Ok(error::error_code("Birth data required", "NO_BIRTH_DATA", 400));
            }
            let expected_version = if div_type == "ziwei" { ENGINE_VERSION_ZIWEI } else { ENGINE_VERSION_WESTERN };
            let hash = birth.birth_data_hash.clone().unwrap_or_default();

            let u = db::text(&user);
            let dt = db::text(&div_type);
            let bh = db::opt_text(birth.birth_data_hash.as_deref());
            let cached: Option<InterpRow> = match db::first(
                &db,
                "SELECT id, chart_data, ai_interpretation, created_at, updated_at FROM interpretations WHERE user_id = ?1 AND divination_type = ?2 AND birth_data_hash = ?3",
                &[&u, &dt, &bh],
            ).await {
                Ok(r) => r,
                Err(e) => return Ok(error::error(format!("db error: {}", e), 500)),
            };
            let etag = create_etag(
                format!("{}-{}-{}", hash, expected_version, CHART_SCHEMA_VERSION),
                cached.as_ref().and_then(|c| c.updated_at.as_deref()),
            );
            if let Some(inm) = req.headers().get("if-none-match").ok().flatten() {
                if inm == etag {
                    let mut res = Response::empty().unwrap().with_status(304);
                    let _ = res.headers_mut().set("ETag", &etag);
                    apply_cache_headers(&mut res, 3600, false);
                    return Ok(res);
                }
            }
            if let Some(c) = &cached {
                let parsed = parse_chart(c.chart_data.as_deref());
                let stored_version = extracted_version(&parsed);
                if stored_version == expected_version {
                    let mut res = ok_json(&serde_json::json!({
                        "id": c.id, "user_id": user, "divination_type": div_type,
                        "chart_data": parsed, "ai_interpretation": c.ai_interpretation,
                        "birth_data_hash": c.birth_data_hash, "fromCache": true,
                    }), 200);
                    let _ = res.headers_mut().set("ETag", &etag);
                    apply_cache_headers(&mut res, 3600, false);
                    return Ok(res);
                }
            }

            let engine = match ctx.env.service("FT_ENGINE") {
                Ok(f) => f,
                Err(_) => return Ok(error::error("engine unavailable", 500)),
            };
            let hour = birth.birth_hour.unwrap_or(12);
            let chart_data = if div_type == "ziwei" {
                if birth.gender.is_none() {
                    return Ok(error::error_code("Gender required for ZiWei", "NO_GENDER", 400));
                }
                match engine::fetch_engine_chart(&engine, "ziwei", &EngineBirth {
                    year: birth.birth_year, month: birth.birth_month, day: birth.birth_day,
                    hour, gender: birth.gender.clone(), latitude: None, longitude: None,
                    timezone: birth.timezone.clone(),
                }).await {
                    Ok(v) => v,
                    Err(e) => return Ok(error::error(format!("{}", e), 502)),
                }
            } else {
                match engine::fetch_engine_chart(&engine, "western", &EngineBirth {
                    year: birth.birth_year, month: birth.birth_month, day: birth.birth_day,
                    hour, gender: None, latitude: birth.latitude, longitude: birth.longitude,
                    timezone: birth.timezone.clone(),
                }).await {
                    Ok(v) => v,
                    Err(e) => return Ok(error::error(format!("{}", e), 502)),
                }
            };
            let chart_with_version = embed_meta(chart_data, &div_type, expected_version);

            let id = uuid::random_uuid();
            let id_t = db::text(&id);
            let uid_t = db::text(&user);
            let dt_t = db::text(&div_type);
            let chart_str = chart_with_version.to_string();
            let chart_t = db::text(&chart_str);
            let bh_t = db::text(&hash);
            if let Err(e) = db::exec(
                &db,
                "INSERT INTO interpretations (id, user_id, divination_type, chart_data, birth_data_hash) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(user_id, divination_type) DO UPDATE SET chart_data = excluded.chart_data, birth_data_hash = excluded.birth_data_hash, ai_interpretation = NULL, updated_at = datetime('now')",
                &[&id_t, &uid_t, &dt_t, &chart_t, &bh_t],
            ).await {
                return Ok(error::error(format!("db error: {}", e), 500));
            }

            let response = serde_json::json!({
                "id": id, "user_id": user, "divination_type": div_type,
                "chart_data": chart_with_version, "ai_interpretation": serde_json::Value::Null,
                "birth_data_hash": hash, "fromCache": false,
                "engineVersion": expected_version, "chartSchemaVersion": CHART_SCHEMA_VERSION,
            });
            if div_type == "ziwei" {
                if let Err(e) = validate_ziwei_v3(&response) {
                    return Ok(error::error(format!("Chart schema violation: {}", e), 500));
                }
            }
            let mut res = ok_json(&response, 200);
            let _ = res.headers_mut().set("ETag", &etag);
            apply_cache_headers(&mut res, 3600, false);
            Ok(res)
        })
        .post_async("/api/charts/:type/interpret", {
            let limiter = ai_limiter2.clone();
            move |req, ctx| {
                let limiter = limiter.clone();
                async move {
                    let user = match auth_user(&req, &ctx).await {
                        Ok(u) => u,
                        Err(r) => return Ok(r),
                    };
                    if !limiter.lock().unwrap().check(&client_ip(&req)) {
                        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
                    }
                    let div_type = match ctx.param("type") {
                        Some(v) if v == "ziwei" || v == "western" => v.to_string(),
                        _ => return Ok(error::error("Invalid type", 400)),
                    };
                    let db = match ctx.env.d1("DB") {
                        Ok(d) => d,
                        Err(_) => return Ok(error::error("db unavailable", 500)),
                    };
                    let u = db::text(&user);
                    let dt = db::text(&div_type);
                    let interp: Option<InterpRow> = match db::first(
                        &db,
                        "SELECT id, chart_data, ai_interpretation, updated_at, birth_data_hash FROM interpretations WHERE user_id = ?1 AND divination_type = ?2",
                        &[&u, &dt],
                    ).await {
                        Ok(r) => r,
                        Err(e) => return Ok(error::error(format!("db error: {}", e), 500)),
                    };
                    let interp = match interp {
                        Some(i) => i,
                        None => return Ok(error::error("Chart not found. Call GET /:type first", 404)),
                    };
                    let parsed = parse_chart(interp.chart_data.as_deref());
                    let stored_version = extracted_version(&parsed);
                    let expected_version = if div_type == "ziwei" { ENGINE_VERSION_ZIWEI } else { ENGINE_VERSION_WESTERN };
                    if stored_version != expected_version {
                        return Ok(error::error_code("Chart version stale, recalculation required", "RECALC_REQUIRED", 409));
                    }
                    if let Some(ai) = &interp.ai_interpretation {
                        let etag = create_etag(format!("{}-ai", interp.birth_data_hash.clone().unwrap_or_default()), interp.updated_at.as_deref());
                        if let Some(inm) = req.headers().get("if-none-match").ok().flatten() {
                            if inm == etag {
                                let mut res = Response::empty().unwrap().with_status(304);
                                let _ = res.headers_mut().set("ETag", &etag);
                                apply_cache_headers(&mut res, 86400, true);
                                return Ok(res);
                            }
                        }
                        let mut res = ok_json(&serde_json::json!({ "interpretation": ai, "fromCache": true }), 200);
                        let _ = res.headers_mut().set("ETag", &etag);
                        apply_cache_headers(&mut res, 86400, true);
                        return Ok(res);
                    }
                    if !anything_configured(&ctx) {
                        return Ok(error::error("AI service not configured", 503));
                    }
                    let chart_data = parse_chart(interp.chart_data.as_deref());
                    let ai_resp = match call_ai_mutex(&ctx, &div_type, &chart_data).await {
                        Ok(Some(r)) => r,
                        Ok(None) => return Ok(error::error("AI service temporarily unavailable, please try again", 503)),
                        Err(r) => return Ok(r),
                    };
                    let id_t = db::text(&interp.id);
                    let ai_t = db::text(&ai_resp.interpretation);
                    if let Err(e) = db::exec(&db, "UPDATE interpretations SET ai_interpretation = ?1, updated_at = datetime('now') WHERE id = ?2", &[&ai_t, &id_t]).await {
                        return Ok(error::error(format!("db error: {}", e), 500));
                    }
                    let mut res = ok_json(&serde_json::json!({
                        "interpretation": ai_resp.interpretation, "provider": ai_resp.provider, "model": ai_resp.model, "fromCache": false,
                    }), 200);
                    apply_cache_headers(&mut res, 86400, true);
                    Ok(res)
                }
            }
        })
}

// ── helpers ─────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct StoryRow {
    id: Option<String>,
    chart_data: Option<String>,
    ai_interpretation: Option<String>,
    updated_at: Option<String>,
}

async fn get_birth_data(db: &worker::D1Database, user: &str) -> Result<UserBirthRow, Response> {
    let u = db::text(user);
    let row: Option<UserBirthRow> = db::first(
        db,
        "SELECT birth_year, birth_month, birth_day, birth_hour, birth_minute, gender, timezone, latitude, longitude, birth_data_hash FROM users WHERE id = ?1",
        &[&u],
    ).await.map_err(|e| error::error(format!("db error: {}", e), 500))?;
    row.ok_or_else(|| error::error("User not found", 404))
}

fn anything_configured(ctx: &RouteContext<()>) -> bool {
    ["IFLOW_API_KEY", "GROQ_API_KEY", "CEREBRAS_API_KEY"]
        .iter()
        .any(|name| ctx.env.secret(name).map(|s| !s.to_string().is_empty()).unwrap_or(false))
}

/// Call the AI_MUTEX DO. `Ok(Some(..))` on success, `Ok(None)` on 503 (all providers
/// failed / queue full), `Err(Response)` carries a status for other failures.
async fn call_ai_mutex(ctx: &RouteContext<()>, chart_type: &str, chart_data: &serde_json::Value) -> Result<Option<ProviderResult>, Response> {
    let ns = ctx.env.durable_object("AI_MUTEX").map_err(|_| error::error("ai unavailable", 503))?;
    let stub = ns.id_from_name("global").and_then(|id| id.get_stub()).map_err(|_| error::error("ai unavailable", 503))?;
    let mut keys_map = serde_json::Map::new();
    if let Some(v) = secret_or(&ctx.env, "IFLOW_API_KEY") {
        keys_map.insert("iflow".to_string(), serde_json::Value::String(v));
    }
    if let Some(v) = secret_or(&ctx.env, "GROQ_API_KEY") {
        keys_map.insert("groq".to_string(), serde_json::Value::String(v));
    }
    if let Some(v) = secret_or(&ctx.env, "CEREBRAS_API_KEY") {
        keys_map.insert("cerebras".to_string(), serde_json::Value::String(v));
    }
    let keys = serde_json::Value::Object(keys_map);
    let body = serde_json::json!({
        "keys": keys,
        "interpretRequest": { "chartType": chart_type, "chartData": chart_data, "language": "zh" },
    });
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_body(Some(body.to_string().into()));
    let req = Request::new_with_init("https://ai-mutex/interpret", &init)
        .map_err(|_| error::error("ai unavailable", 503))?;
    let mut res = stub
        .fetch_with_request(req)
        .await
        .map_err(|_| error::error("ai unavailable", 503))?;
    if res.status_code() == 503 {
        return Ok(None);
    }
    let status = res.status_code();
    if status != 200 {
        let err: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        return Err(Response::from_json(&err).unwrap_or_else(|_| error::error("ai error", 500)).with_status(status));
    }
    let data: serde_json::Value = res.json().await.map_err(|_| error::error("ai bad response", 502))?;
    let interpretation = data.get("interpretation").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let provider = data.get("provider").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let model = data.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string();
    Ok(Some(ProviderResult { interpretation, provider, model, tokens_used: None }))
}

fn secret_or(env: &Env, name: &str) -> Option<String> {
    match env.secret(name) {
        Ok(s) if !s.to_string().is_empty() => Some(s.to_string()),
        _ => None,
    }
}

fn validate_ziwei_v3(resp: &serde_json::Value) -> Result<(), String> {
    let palaces = resp.pointer("/chart_data/palaces").and_then(|v| v.as_array());
    if palaces.map(|a| a.len()) != Some(12) {
        return Err("palaces.length != 12".to_string());
    }
    let meta = resp.pointer("/chart_data/meta");
    if meta.is_none() {
        return Err("meta missing".to_string());
    }
    Ok(())
}
