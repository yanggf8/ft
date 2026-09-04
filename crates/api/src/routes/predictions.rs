//! F5 predictions routes — 週期生成 + F6 兩段式回報。
//! Spec: docs/superpowers/specs/2026-09-04-f5-api-predictions-design.md §3–§4

use ft_schema::api::{
    CheckSituationRequest, FeedbackRequest, GeneratePredictionsResponse, ListPredictionsResponse,
};
use ft_schema::cycle::is_monday_cycle_id;

use super::super::error;
use super::super::services::{db, predictions};
use super::common::{apply_cache_headers, auth_user, client_ip, ok_json, rate_limit};
use super::R;

const RATE_LIMIT: u32 = 10;
const WINDOW_MS: f64 = 60000.0;

/// 服務錯誤 → `{ error, code }` response。
fn to_err(e: predictions::PredictionsError) -> worker::Response {
    use predictions::PredictionsError::*;
    match e {
        ProfileIncomplete => error::error_code(
            "Complete the personality quiz first",
            "PROFILE_INCOMPLETE",
            409,
        ),
        NotFound => error::error_code("Prediction not found", "NOT_FOUND", 404),
        StaleCycle => error::error_code(
            "This cycle is no longer the current week",
            "STALE_CYCLE",
            409,
        ),
        SituationRequired => error::error_code(
            "Answer the situation check for this trigger first",
            "SITUATION_REQUIRED",
            409,
        ),
        SituationAbsent => error::error_code(
            "Situation was absent — stage 2 not allowed",
            "SITUATION_ABSENT",
            409,
        ),
        SituationLocked => error::error_code(
            "Situation is locked once feedback exists",
            "SITUATION_LOCKED",
            409,
        ),
        FeedbackExists => error::error_code(
            "Feedback already submitted for this prediction",
            "FEEDBACK_EXISTS",
            409,
        ),
        UnknownTrigger => error::error_code(
            "No prediction for this trigger this week",
            "UNKNOWN_TRIGGER",
            400,
        ),
        Db(e) => error::error_code(format!("db error: {e}"), "DB_ERROR", 500),
    }
}

/// P2-01：plain-data DTO 序列化不可失敗，fallback 到 JSON null（不 panic）。
fn to_json<T: serde::Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

/// GET 的可選 ?cycleId=；省略或空 → 當週。非週一格式 → 400 INVALID_CYCLE。
fn requested_cycle(query: Option<&str>) -> Result<String, worker::Response> {
    if let Some(q) = query {
        for (k, v) in url::form_urlencoded::parse(q.as_bytes()) {
            if k == "cycleId" {
                let s = v.into_owned();
                if s.is_empty() {
                    continue;
                }
                if !is_monday_cycle_id(&s) {
                    return Err(error::error_code(
                        "cycleId must be a Monday YYYY-MM-DD",
                        "INVALID_CYCLE",
                        400,
                    ));
                }
                return Ok(s);
            }
        }
    }
    predictions::current_cycle_id().map_err(to_err)
}

pub fn register(router: R<'static>) -> R<'static> {
    router
        .get_async("/api/predictions", |req, ctx| async move {
            let user_id = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error_code("db unavailable", "DB_UNAVAILABLE", 500)),
            };
            let url = match req.url() {
                Ok(u) => u,
                Err(_) => return Ok(error::error_code("bad url", "INVALID_JSON", 400)),
            };
            let cycle = match requested_cycle(url.query()) {
                Ok(c) => c,
                Err(e) => return Ok(e),
            };
            let mut view = match predictions::list_cycle(&db, &user_id, &cycle).await {
                Ok(v) => v,
                Err(e) => return Ok(to_err(e)),
            };
            predictions::redact_view(&mut view);
            let resp = ListPredictionsResponse {
                cycleId: cycle,
                checks: view.checks,
                predictions: view.predictions,
                feedback: view.feedback,
            };
            let mut res = ok_json(&to_json(&resp), 200);
            apply_cache_headers(&mut res, 0, true);
            Ok(res)
        })
        .post_async("/api/predictions/generate", |req, ctx| async move {
            if !rate_limit(
                &ctx,
                &format!("predictions:ip:{}", client_ip(&req)),
                RATE_LIMIT,
                WINDOW_MS,
            )
            .await
            {
                return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
            }
            let user_id = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error_code("db unavailable", "DB_UNAVAILABLE", 500)),
            };
            let cycle = match predictions::current_cycle_id() {
                Ok(c) => c,
                Err(e) => return Ok(to_err(e)),
            };
            let outcome = match predictions::generate(&db, &user_id, &cycle).await {
                Ok(o) => o,
                Err(e) => return Ok(to_err(e)),
            };
            let mut view = outcome.view;
            predictions::redact_view(&mut view);
            let resp = GeneratePredictionsResponse {
                cycleId: cycle,
                generated: outcome.generated,
                predictions: view.predictions,
            };
            Ok(ok_json(&to_json(&resp), 200))
        })
        .put_async("/api/predictions/checks", |mut req, ctx| async move {
            if !rate_limit(
                &ctx,
                &format!("predictions:ip:{}", client_ip(&req)),
                RATE_LIMIT,
                WINDOW_MS,
            )
            .await
            {
                return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
            }
            let user_id = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error_code("db unavailable", "DB_UNAVAILABLE", 500)),
            };
            let body: CheckSituationRequest = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)),
            };
            let now = match predictions::current_cycle_id() {
                Ok(c) => c,
                Err(e) => return Ok(to_err(e)),
            };
            let cycle = match &body.cycleId {
                Some(c) => {
                    // 格式先驗（Grok 二審 P2 #5）：非週一日期 → 400，不是 409
                    if !is_monday_cycle_id(c) {
                        return Ok(error::error_code(
                            "cycleId must be a Monday YYYY-MM-DD",
                            "INVALID_CYCLE",
                            400,
                        ));
                    }
                    if c != &now {
                        return Ok(error::error_code(
                            "Writes are only allowed for the current week",
                            "STALE_CYCLE",
                            409,
                        ));
                    }
                    c.clone()
                }
                None => now,
            };
            let check = match predictions::upsert_check(
                &db,
                &user_id,
                &cycle,
                body.trigger,
                body.situation,
            )
            .await
            {
                Ok(c) => c,
                Err(e) => return Ok(to_err(e)),
            };
            Ok(ok_json(&to_json(&check), 200))
        })
        .post_async("/api/predictions/:id/feedback", |mut req, ctx| async move {
            if !rate_limit(
                &ctx,
                &format!("predictions:ip:{}", client_ip(&req)),
                RATE_LIMIT,
                WINDOW_MS,
            )
            .await
            {
                return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
            }
            let user_id = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error_code("db unavailable", "DB_UNAVAILABLE", 500)),
            };
            let id = ctx.param("id").cloned().unwrap_or_default();
            if id.is_empty() {
                return Ok(error::error_code("Prediction not found", "NOT_FOUND", 404));
            }
            let body: FeedbackRequest = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)),
            };
            let cycle = match predictions::current_cycle_id() {
                Ok(c) => c,
                Err(e) => return Ok(to_err(e)),
            };
            let feedback =
                match predictions::record_feedback(&db, &user_id, &id, &cycle, body.response).await
                {
                    Ok(f) => f,
                    Err(e) => return Ok(to_err(e)),
                };
            Ok(ok_json(&to_json(&feedback), 200))
        })
}
