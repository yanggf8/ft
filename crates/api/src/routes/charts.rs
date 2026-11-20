//! Charts routes — mirrors backend/src/routes/charts.ts.
//! GET /api/charts, GET /api/charts/story, POST /api/charts/story/generate,
//! GET /api/charts/:type, POST /api/charts/:type/interpret.
//! Chart calculations come from the engine Worker (FT_ENGINE service binding); D1
//! holds the per-(user,type) cache; AIMutexDO serializes AI.

use worker::*;

use super::super::error;
use super::super::services::ai::ProviderResult;
use super::super::services::db;
use super::super::services::engine::{self, EngineBirth};
use super::super::services::engine_version::{
    CHART_SCHEMA_VERSION, ENGINE_VERSION_WESTERN, ENGINE_VERSION_ZIWEI,
};
use super::super::services::generation;
use super::super::services::uuid;
use super::common::{
    apply_cache_headers, auth_user, client_ip, create_etag, embed_meta, extracted_version,
    is_story_chart_current, ok_json, parse_chart, rate_limit,
};
use super::R;

const AI_RATE_LIMIT: u32 = 10;
const AI_WINDOW_MS: f64 = 60000.0;

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
    #[serde(default)]
    generation_tags: Option<String>,
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
    router
        .get_async("/api/charts", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(r) => return Ok(r),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let u = db::text(&user);
            let rows: Vec<serde_json::Value> = match db::all(&db, "SELECT * FROM interpretations WHERE user_id = ?1 ORDER BY created_at DESC", &[&u]).await {
                Ok(v) => v,
                Err(_) => return Ok(error::error("db error", 500)),
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
            // P2 telemetry: list hit/miss + count
            worker::console_log!(
                "metric: route=GET /api/charts count={} cache_hit={} fromCache=false",
                interpretations.len(),
                !interpretations.is_empty()
            );
            Ok(ok_json(&serde_json::json!({ "interpretations": interpretations }), 200))
        })
        .get_async("/api/charts/story", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(r) => return Ok(r),
            };
            let db = match db::Turso::from_env(&ctx.env) {
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
                Err(_) => return Ok(error::error("db error", 500)),
            };
            let row = match row {
                Some(r) => r,
                None => return Ok(error::error_code("No story yet. POST /story/generate first", "NO_STORY", 404)),
            };
            // 舊的離線範本故事視同尚未產生（前端會走 generate 重跑）
            if row.ai_interpretation.as_deref().is_some_and(is_stub_text) {
                return Ok(error::error_code("No story yet. POST /story/generate first", "NO_STORY", 404));
            }
            if let Some(ai) = &row.ai_interpretation {
                if !is_story_chart_current(row.chart_data.as_deref()) {
                    return Ok(error::error_code("Chart version stale, regeneration required", "RECALC_REQUIRED", 409));
                }
                let etag = create_etag(format!("{}-story", hash_of), row.updated_at.as_deref());
                if let Some(inm) = req.headers().get("if-none-match").ok().flatten() {
                    if inm == etag {
                        let mut res = Response::empty()?.with_status(304);
                        let _ = res.headers_mut().set("ETag", &etag);
                        apply_cache_headers(&mut res, 86400, true);
                        return Ok(res);
                    }
                }
                let mut res = ok_json(&serde_json::json!({ "story": ai, "fromCache": true }), 200);
                let _ = res.headers_mut().set("ETag", &etag);
                apply_cache_headers(&mut res, 86400, true);
                {
                    let tags_len = birth
                        .generation_tags
                        .as_deref()
                        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let has_generation_story = if tags_len == 0 {
                        false
                    } else {
                        birth
                            .generation_tags
                            .as_deref()
                            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                            .map(|tags| {
                                tags.iter().any(|t| {
                                    generation::generation_story_for_tag(t).is_some()
                                })
                            })
                            .unwrap_or(false)
                    };
                    let is_stub = ai.contains("離線模板") || ai.contains("offline-template");
                    worker::console_log!(
                        "metric: route=GET /api/charts/story cache_hit=true fromCache=true tags_len={} has_generation_story={} is_stub={} ai_story_chars={}",
                        tags_len,
                        has_generation_story,
                        is_stub,
                        ai.chars().count()
                    );
                }
                return Ok(res);
            }
            {
                let tags_len = birth
                    .generation_tags
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                    .map(|v| v.len())
                    .unwrap_or(0);
                worker::console_log!(
                    "metric: route=GET /api/charts/story cache_hit=false fromCache=false tags_len={} has_generation_story=false",
                    tags_len
                );
            }
            Ok(error::error_code("No story yet. POST /story/generate first", "NO_STORY", 404))
        })
        .post_async("/api/charts/story/generate", {
            move |req, ctx| {
                async move {
                    let user = match auth_user(&req, &ctx).await {
                        Ok(u) => u,
                        Err(r) => return Ok(r),
                    };
                    // P2-02：AI limiter 收編進 RateLimitDO（key "ai:{ip}"，limit 不變）。
                    // 舊 per-register Arc 在 lib.rs 每請求重建 router 時形同 per-request
                    //（baicodex F2 同款 bug），DO 版跨 isolate 且計數持久。
                    if !rate_limit(
                        &ctx,
                        &format!("ai:{}", client_ip(&req)),
                        AI_RATE_LIMIT,
                        AI_WINDOW_MS,
                    )
                    .await
                    {
                        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
                    }
                    let db = match db::Turso::from_env(&ctx.env) {
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
                        Err(_) => return Ok(error::error("db error", 500)),
                    };
                    if let Some(ex) = existing {
                        if let Some(ai) = ex.ai_interpretation.as_ref().filter(|a| !is_stub_text(a)) {
                            if is_story_chart_current(ex.chart_data.as_deref()) {
                                let mut res = ok_json(&serde_json::json!({ "story": ai, "fromCache": true }), 200);
                                apply_cache_headers(&mut res, 86400, true);
                                {
                                    let tags: Vec<String> = birth
                                        .generation_tags
                                        .as_deref()
                                        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                                        .unwrap_or_default();
                                    let tags_len = tags.len();
                                    let has_generation_story = tags
                                        .iter()
                                        .any(|t| generation::generation_story_for_tag(t).is_some());
                                    let is_stub = ai.contains("離線模板")
                                        || ai.contains("offline-template");
                                    worker::console_log!(
                                        "metric: route=POST /api/charts/story/generate cache_hit=true fromCache=true tags_len={} has_generation_story={} is_stub={} ai_story_chars={}",
                                        tags_len,
                                        has_generation_story,
                                        is_stub,
                                        ai.chars().count()
                                    );
                                }
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
                    let mut merged = serde_json::json!({
                        "ziwei": ziwei, "western": western,
                        "meta": {
                            "engineVersionZiwei": ENGINE_VERSION_ZIWEI,
                            "engineVersionWestern": ENGINE_VERSION_WESTERN,
                            "chartSchemaVersion": CHART_SCHEMA_VERSION,
                        }
                    });
                    // P0: embed generation_tags + generation_stories so prompt can render 【世代語境】
                    {
                        let tags: Vec<String> = birth
                            .generation_tags
                            .as_deref()
                            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                            .unwrap_or_default();
                        if !tags.is_empty() {
                            if let Some(obj) = merged.as_object_mut() {
                                obj.insert(
                                    "generation_tags".to_string(),
                                    serde_json::to_value(&tags).unwrap_or(serde_json::Value::Null),
                                );
                                // Build per-tag stories preserving tag order; skip unknown tags.
                                let mut stories: Vec<serde_json::Value> = Vec::new();
                                for tag in &tags {
                                    if let Some((title, story)) = generation::generation_story_for_tag(tag) {
                                        stories.push(serde_json::json!({
                                            "tag": tag,
                                            "title": title,
                                            "story": story
                                        }));
                                    }
                                }
                                // Also exercise combined_generation_story to keep the helper covered;
                                // prompts can use the per-tag array, but we also expose the combined
                                // title/story for debugging / future use.
                                let _combined = generation::combined_generation_story(&tags);
                                if !stories.is_empty() {
                                    obj.insert("generation_stories".to_string(), serde_json::Value::Array(stories));
                                }
                                // Keep a top-level combined string for convenience (optional).
                                if let Some((c_title, c_desc)) = _combined {
                                    obj.insert(
                                        "generation_combined".to_string(),
                                        serde_json::json!({ "title": c_title, "story": c_desc }),
                                    );
                                }
                            }
                        }
                    }

                    if !anything_configured(&ctx) {
                        return Ok(error::error("AI service not configured", 503));
                    }
                    let ai_resp = match call_ai_mutex(&ctx, "story", &merged).await {
                        Ok(r) => r,
                        Err(r) => return Ok(r),
                    };
                    // TS returns 502 EMPTY_STORY when the AI returns an empty string
                    if ai_resp.result.interpretation.trim().is_empty() {
                        return Ok(error::error_code("AI returned an empty story, please try again", "EMPTY_STORY", 502));
                    }
                    // 離線範本（全環失敗的 stub）不寫快取，照常回給前端
                    if ai_resp.stub {
                        let mut res = ok_json(&serde_json::json!({
                            "story": ai_resp.result.interpretation, "provider": "stub", "model": "offline-template", "fromCache": false, "stub": true,
                        }), 200);
                        apply_cache_headers(&mut res, 0, true);
                        return Ok(res);
                    }
                    let ai_resp = ai_resp.result;
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
                        return Ok(error::error("db error", 500));
                    }
                    let mut res = ok_json(&serde_json::json!({
                        "story": ai_resp.interpretation, "provider": ai_resp.provider, "model": ai_resp.model, "fromCache": false,
                    }), 200);
                    apply_cache_headers(&mut res, 86400, true);
                    {
                        let tags: Vec<String> = birth
                            .generation_tags
                            .as_deref()
                            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                            .unwrap_or_default();
                        let tags_len = tags.len();
                        let has_generation_story = tags
                            .iter()
                            .any(|t| generation::generation_story_for_tag(t).is_some());
                        let is_stub = ai_resp.provider == "stub"
                            || ai_resp.model == "offline-template"
                            || ai_resp.interpretation.contains("離線模板");
                        worker::console_log!(
                            "metric: route=POST /api/charts/story/generate cache_hit=false fromCache=false tags_len={} has_generation_story={} is_stub={} ai_story_chars={} provider={}",
                            tags_len,
                            has_generation_story,
                            is_stub,
                            ai_resp.interpretation.chars().count(),
                            ai_resp.provider
                        );
                    }
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
            let db = match db::Turso::from_env(&ctx.env) {
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
                "SELECT id, chart_data, ai_interpretation, birth_data_hash, created_at, updated_at FROM interpretations WHERE user_id = ?1 AND divination_type = ?2 AND birth_data_hash = ?3",
                &[&u, &dt, &bh],
            ).await {
                Ok(r) => r,
                Err(_) => return Ok(error::error("db error", 500)),
            };
            // 舊離線 stub 不回給前端（視同未解讀 → 前端顯示「產生 AI 解讀」按鈕，
            // 走 interpret 路由自癒）；ETag 帶標記，讓先前拿到 stub 的瀏覽器不會 304。
            let cached_is_stub = cached
                .as_ref()
                .and_then(|c| c.ai_interpretation.as_deref())
                .is_some_and(is_stub_text);
            let etag = create_etag(
                format!(
                    "{}-{}-{}{}",
                    hash,
                    expected_version,
                    CHART_SCHEMA_VERSION,
                    if cached_is_stub { "-nostub" } else { "" }
                ),
                cached.as_ref().and_then(|c| c.updated_at.as_deref()),
            );
            if let Some(inm) = req.headers().get("if-none-match").ok().flatten() {
                if inm == etag {
                    let mut res = Response::empty()?.with_status(304);
                    let _ = res.headers_mut().set("ETag", &etag);
                    apply_cache_headers(&mut res, if cached_is_stub { 0 } else { 3600 }, false);
                    return Ok(res);
                }
            }
            if let Some(c) = &cached {
                let parsed = parse_chart(c.chart_data.as_deref());
                let stored_version = extracted_version(&parsed);
                if stored_version == expected_version {
                    // 外殼與 fresh 路徑逐欄一致（版本欄在本分支僅於
                    // stored_version == expected_version 時可達，寫 expected 誠實）。
                    let mut res = ok_json(&serde_json::json!({
                        "id": c.id, "user_id": user, "divination_type": div_type,
                        "chart_data": parsed,
                        "ai_interpretation": if cached_is_stub { None } else { c.ai_interpretation.clone() },
                        "birth_data_hash": c.birth_data_hash, "fromCache": true,
                        "engineVersion": expected_version,
                        "chartSchemaVersion": CHART_SCHEMA_VERSION,
                    }), 200);
                    let _ = res.headers_mut().set("ETag", &etag);
                    apply_cache_headers(&mut res, if cached_is_stub { 0 } else { 3600 }, false);
                    {
                        let tags: Vec<String> = birth
                            .generation_tags
                            .as_deref()
                            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                            .unwrap_or_default();
                        let tags_len = tags.len();
                        let has_generation_story = tags
                            .iter()
                            .any(|t| generation::generation_story_for_tag(t).is_some());
                        worker::console_log!(
                            "metric: route=GET /api/charts/:type divination_type={} cache_hit=true fromCache=true tags_len={} has_generation_story={}",
                            div_type,
                            tags_len,
                            has_generation_story
                        );
                    }
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
                return Ok(error::error("db error", 500));
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
            {
                let tags: Vec<String> = birth
                    .generation_tags
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                    .unwrap_or_default();
                let tags_len = tags.len();
                let has_generation_story = tags
                    .iter()
                    .any(|t| generation::generation_story_for_tag(t).is_some());
                worker::console_log!(
                    "metric: route=GET /api/charts/:type divination_type={} cache_hit=false fromCache=false tags_len={} has_generation_story={}",
                    div_type,
                    tags_len,
                    has_generation_story
                );
            }
            Ok(res)
        })
        .post_async("/api/charts/:type/interpret", {
            move |req, ctx| {
                async move {
                    let user = match auth_user(&req, &ctx).await {
                        Ok(u) => u,
                        Err(r) => return Ok(r),
                    };
                    // P2-02：AI limiter 收編進 RateLimitDO（key "ai:{ip}"，limit 不變）。
                    // 舊 per-register Arc 在 lib.rs 每請求重建 router 時形同 per-request
                    //（baicodex F2 同款 bug），DO 版跨 isolate 且計數持久。
                    if !rate_limit(
                        &ctx,
                        &format!("ai:{}", client_ip(&req)),
                        AI_RATE_LIMIT,
                        AI_WINDOW_MS,
                    )
                    .await
                    {
                        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
                    }
                    let div_type = match ctx.param("type") {
                        Some(v) if v == "ziwei" || v == "western" => v.to_string(),
                        _ => return Ok(error::error("Invalid type", 400)),
                    };
                    let db = match db::Turso::from_env(&ctx.env) {
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
                        Err(_) => return Ok(error::error("db error", 500)),
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
                    // 舊資料若是離線範本（stub）視同未命中，重跑 AI 自我修復
                    if let Some(ai) = interp.ai_interpretation.as_ref().filter(|a| !is_stub_text(a)) {
                        let etag = create_etag(format!("{}-ai", interp.birth_data_hash.clone().unwrap_or_default()), interp.updated_at.as_deref());
                        if let Some(inm) = req.headers().get("if-none-match").ok().flatten() {
                            if inm == etag {
                                let mut res = Response::empty()?.with_status(304);
                                let _ = res.headers_mut().set("ETag", &etag);
                                apply_cache_headers(&mut res, 86400, true);
                                return Ok(res);
                            }
                        }
                        let mut res = ok_json(&serde_json::json!({ "interpretation": ai, "fromCache": true }), 200);
                        let _ = res.headers_mut().set("ETag", &etag);
                        apply_cache_headers(&mut res, 86400, true);
                        {
                            let tags_len = parsed
                                .get("generation_tags")
                                .and_then(|v| v.as_array())
                                .map(|a| a.len())
                                .unwrap_or(0);
                            let has_generation_story = if tags_len == 0 {
                                false
                            } else if let Some(arr) =
                                parsed.get("generation_stories").and_then(|v| v.as_array())
                            {
                                !arr.is_empty()
                            } else {
                                parsed
                                    .get("generation_tags")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| {
                                        arr.iter().filter_map(|v| v.as_str()).any(|t| {
                                            generation::generation_story_for_tag(t).is_some()
                                        })
                                    })
                                    .unwrap_or(false)
                            };
                            let is_stub = ai.contains("離線模板") || ai.contains("offline-template");
                            worker::console_log!(
                                "metric: route=POST /api/charts/:type/interpret divination_type={} cache_hit=true fromCache=true tags_len={} has_generation_story={} is_stub={} ai_story_chars={}",
                                div_type,
                                tags_len,
                                has_generation_story,
                                is_stub,
                                ai.chars().count()
                            );
                        }
                        return Ok(res);
                    }
                    if !anything_configured(&ctx) {
                        return Ok(error::error("AI service not configured", 503));
                    }
                    let chart_data = parse_chart(interp.chart_data.as_deref());
                    let ai_resp = match call_ai_mutex(&ctx, &div_type, &chart_data).await {
                        Ok(r) => r,
                        Err(r) => return Ok(r),
                    };
                    if ai_resp.stub {
                        let mut res = ok_json(&serde_json::json!({
                            "interpretation": ai_resp.result.interpretation, "provider": "stub", "model": "offline-template", "fromCache": false, "stub": true,
                        }), 200);
                        apply_cache_headers(&mut res, 0, true);
                        return Ok(res);
                    }
                    let ai_resp = ai_resp.result;
                    let id_t = db::text(&interp.id);
                    let ai_t = db::text(&ai_resp.interpretation);
                    if let Err(e) = db::exec(&db, "UPDATE interpretations SET ai_interpretation = ?1, updated_at = datetime('now') WHERE id = ?2", &[&ai_t, &id_t]).await {
                        return Ok(error::error("db error", 500));
                    }
                    let mut res = ok_json(&serde_json::json!({
                        "interpretation": ai_resp.interpretation, "provider": ai_resp.provider, "model": ai_resp.model, "fromCache": false,
                    }), 200);
                    apply_cache_headers(&mut res, 86400, true);
                    {
                        let tags_len = chart_data
                            .get("generation_tags")
                            .and_then(|v| v.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        let has_generation_story = if tags_len == 0 {
                            false
                        } else if let Some(arr) =
                            chart_data.get("generation_stories").and_then(|v| v.as_array())
                        {
                            !arr.is_empty()
                        } else {
                            chart_data
                                .get("generation_tags")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter().filter_map(|v| v.as_str()).any(|t| {
                                        generation::generation_story_for_tag(t).is_some()
                                    })
                                })
                                .unwrap_or(false)
                        };
                        let is_stub = ai_resp.provider == "stub"
                            || ai_resp.model == "offline-template"
                            || ai_resp.interpretation.contains("離線模板");
                        worker::console_log!(
                            "metric: route=POST /api/charts/:type/interpret divination_type={} cache_hit=false fromCache=false tags_len={} has_generation_story={} is_stub={} ai_story_chars={} provider={}",
                            div_type,
                            tags_len,
                            has_generation_story,
                            is_stub,
                            ai_resp.interpretation.chars().count(),
                            ai_resp.provider
                        );
                    }
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

async fn get_birth_data(db: &db::Turso, user: &str) -> Result<UserBirthRow, Response> {
    let u = db::text(user);
    let row: Option<UserBirthRow> = db::first(
        db,
        "SELECT birth_year, birth_month, birth_day, birth_hour, birth_minute, gender, timezone, latitude, longitude, birth_data_hash, generation_tags FROM users WHERE id = ?1",
        &[&u],
    ).await.map_err(|_| error::error("db error", 500))?;
    row.ok_or_else(|| error::error("User not found", 404))
}

/// Workers AI binding 是否存在（2026-11-20 起文字 AI 全走 `[ai]` binding，免 key）。
fn anything_configured(ctx: &RouteContext<()>) -> bool {
    ctx.env.ai("AI").is_ok()
}

/// 離線範本（DO 全環失敗時的 stub）判定——寫入前擋、讀快取時視同未命中。
fn is_stub_text(ai: &str) -> bool {
    ai.contains("離線模板") || ai.contains("offline-template")
}

/// DO 回應：成功結果＋是否為離線範本。
struct AiOutcome {
    result: ProviderResult,
    stub: bool,
}

/// DO 503 的 `code` → 前端可辨識的錯誤碼（只轉發白名單，其餘收斂為
/// `AI_UNAVAILABLE`；訊息 generic，不帶上游細節）。
fn map_ai_503(code: Option<&str>) -> Response {
    match code {
        Some("AI_QUOTA") => error::error_code("Daily AI quota exceeded", "AI_QUOTA", 503),
        Some(code @ ("AI_QUEUE_FULL" | "AI_QUEUE_TIMEOUT")) => {
            error::error_code("AI is busy, please try again", code, 503)
        }
        _ => error::error_code(
            "AI service temporarily unavailable, please try again",
            "AI_UNAVAILABLE",
            503,
        ),
    }
}

/// Call the AI_MUTEX DO. `Ok` on success (含 stub 旗標)；DO 503（額度、佇列、
/// 全環失敗）轉成帶 `code` 的 503；其他狀態原樣轉發。
async fn call_ai_mutex(
    ctx: &RouteContext<()>,
    chart_type: &str,
    chart_data: &serde_json::Value,
) -> Result<AiOutcome, Response> {
    let unavailable = || map_ai_503(None);
    let ns = ctx
        .env
        .durable_object("AI_MUTEX")
        .map_err(|_| unavailable())?;
    let stub = ns
        .id_from_name("global")
        .and_then(|id| id.get_stub())
        .map_err(|_| unavailable())?;
    let body = serde_json::json!({
        "interpretRequest": { "chartType": chart_type, "chartData": chart_data, "language": "zh" },
    });
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(body.to_string().into()));
    let req =
        Request::new_with_init("https://ai-mutex/interpret", &init).map_err(|_| unavailable())?;
    let mut res = stub
        .fetch_with_request(req)
        .await
        .map_err(|_| unavailable())?;
    let status = res.status_code();
    if status == 503 {
        let err: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        return Err(map_ai_503(err.get("code").and_then(|v| v.as_str())));
    }
    if status != 200 {
        let err: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        return Err(Response::from_json(&err)
            .unwrap_or_else(|_| error::error("ai error", 500))
            .with_status(status));
    }
    let data: serde_json::Value = res
        .json()
        .await
        .map_err(|_| error::error("ai bad response", 502))?;
    let field = |k: &str| {
        data.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let interpretation = field("interpretation");
    let provider = field("provider");
    let model = field("model");
    let stub = data.get("stub").and_then(|v| v.as_bool()).unwrap_or(false)
        || provider == "stub"
        || is_stub_text(&interpretation);
    Ok(AiOutcome {
        result: ProviderResult {
            interpretation,
            provider,
            model,
            tokens_used: None,
            finish_reason: None,
        },
        stub,
    })
}

fn validate_ziwei_v3(resp: &serde_json::Value) -> Result<(), String> {
    let palaces = resp
        .pointer("/chart_data/palaces")
        .and_then(|v| v.as_array());
    if palaces.map(|a| a.len()) != Some(12) {
        return Err("palaces.length != 12".to_string());
    }
    let meta = resp.pointer("/chart_data/meta");
    if meta.is_none() {
        return Err("meta missing".to_string());
    }
    Ok(())
}
