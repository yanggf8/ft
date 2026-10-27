//! Voluntary weekly practice. No personality, chart, prediction or experiment inputs.

use super::super::{
    error,
    services::{clock, db, uuid},
};
use super::common::{
    apply_cache_headers, auth_user, body_too_large, client_ip, ok_json, rate_limit,
};
use super::R;
use ft_schema::practice::{
    valid_goal_action, CreatePracticeAttemptRequest, CreatePracticePlanRequest, PracticeAttempt,
    PracticeAttemptPage, PracticeHistoryPage, PracticeOverview, PracticePlan, PracticePlanHistory,
    PracticeSummary, PRACTICE_CATALOG_VERSION,
};

const RATE_LIMIT: u32 = 20;
const WINDOW_MS: f64 = 60_000.0;

#[derive(serde::Deserialize)]
struct PlanRow {
    id: String,
    goal_code: String,
    action_code: String,
    catalog_version: String,
    replace_active: i64,
    replaces_plan_id: Option<String>,
    created_at: String,
    ended_at: Option<String>,
}

impl PlanRow {
    fn view(&self) -> PracticePlan {
        PracticePlan {
            id: self.id.clone(),
            goal_code: self.goal_code.clone(),
            action_code: self.action_code.clone(),
            catalog_version: self.catalog_version.clone(),
            created_at: self.created_at.clone(),
            ended_at: self.ended_at.clone(),
        }
    }
}

#[derive(serde::Deserialize)]
struct AttemptRow {
    id: String,
    plan_id: String,
    cycle_id: String,
    tried: i64,
    helpfulness: Option<String>,
    obstacle: Option<String>,
    created_at: String,
}

impl AttemptRow {
    fn view(&self) -> PracticeAttempt {
        PracticeAttempt {
            id: self.id.clone(),
            plan_id: self.plan_id.clone(),
            cycle_id: self.cycle_id.clone(),
            tried: self.tried == 1,
            helpfulness: self.helpfulness.clone(),
            obstacle: self.obstacle.clone(),
            created_at: self.created_at.clone(),
        }
    }
}

#[derive(serde::Deserialize)]
struct EpochRow {
    practice_epoch: i64,
}

#[derive(serde::Deserialize)]
struct CycleRow {
    cycle_id: String,
}

#[derive(serde::Deserialize)]
struct CountRow {
    total_plans: i64,
    total_attempts: i64,
}

#[derive(serde::Deserialize)]
struct SummaryRow {
    attempted: i64,
    helped: i64,
    unsure: i64,
    not_helped: i64,
    not_tried: i64,
}

#[derive(serde::Deserialize)]
struct HistoryRow {
    #[serde(flatten)]
    plan: PlanRow,
    attempted: i64,
    helped: i64,
    unsure: i64,
    not_helped: i64,
    not_tried: i64,
}

impl HistoryRow {
    fn view(&self) -> PracticePlanHistory {
        PracticePlanHistory {
            plan: self.plan.view(),
            attempted: self.attempted as u32,
            helped: self.helped as u32,
            unsure: self.unsure as u32,
            not_helped: self.not_helped as u32,
            not_tried: self.not_tried as u32,
        }
    }
}

fn summary(cycle_id: String, row: SummaryRow) -> PracticeSummary {
    PracticeSummary {
        cycle_id,
        attempted: row.attempted as u32,
        helped: row.helped as u32,
        unsure: row.unsure as u32,
        not_helped: row.not_helped as u32,
        not_tried: row.not_tried as u32,
    }
}

fn valid_request_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

fn valid_epoch(value: i64) -> bool {
    (0..=i32::MAX as i64).contains(&value)
}

fn valid_attempt(tried: bool, helpfulness: Option<&str>, obstacle: Option<&str>) -> bool {
    match (tried, helpfulness, obstacle) {
        (true, Some("helped" | "unsure" | "not_helped"), None) => true,
        (false, None, Some("no_chance" | "forgot" | "too_big" | "other")) => true,
        _ => false,
    }
}

fn parse_cursor(cursor: &str) -> Option<(&str, &str)> {
    let (stamp, id) = cursor.split_once('|')?;
    let valid_stamp = stamp.len() == 24
        && stamp.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            10 => b == b'T',
            13 | 16 => b == b':',
            19 => b == b'.',
            23 => b == b'Z',
            _ => b.is_ascii_digit(),
        })
        && ft_schema::cycle::week_start_asia_taipei(stamp).is_some();
    if !valid_stamp || !valid_request_id(id) {
        return None;
    }
    Some((stamp, id))
}

pub(crate) fn page_query(
    query: Option<&str>,
) -> Result<(usize, Option<(String, String)>), worker::Response> {
    let mut limit = 20usize;
    let mut cursor = None;
    if let Some(q) = query {
        for (k, v) in url::form_urlencoded::parse(q.as_bytes()) {
            match k.as_ref() {
                "limit" => {
                    limit = v
                        .parse::<usize>()
                        .ok()
                        .filter(|n| (1..=20).contains(n))
                        .ok_or_else(|| error::error_code("Invalid limit", "INVALID_PAGE", 400))?
                }
                "cursor" => {
                    let (stamp, id) = parse_cursor(&v)
                        .ok_or_else(|| error::error_code("Invalid cursor", "INVALID_PAGE", 400))?;
                    cursor = Some((stamp.to_string(), id.to_string()));
                }
                _ => {}
            }
        }
    }
    Ok((limit, cursor))
}

fn json<T: serde::Serialize>(v: &T, status: u16) -> worker::Response {
    let mut res = ok_json(
        &serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
        status,
    );
    apply_cache_headers(&mut res, 0, true);
    let _ = res.headers_mut().set("Cache-Control", "private, no-store");
    res
}

fn db_error() -> worker::Response {
    error::error_code("db error", "DB_ERROR", 500)
}
fn conflict(code: &str) -> worker::Response {
    error::error_code("Practice state changed", code, 409)
}
fn unique_error(e: &worker::Error) -> bool {
    e.to_string().contains("UNIQUE constraint failed")
}

async fn epoch(db: &db::Turso, user: &str) -> worker::Result<i64> {
    let row: Option<EpochRow> = db::first(
        db,
        "SELECT practice_epoch FROM users WHERE id=?1",
        &[&db::text(user)],
    )
    .await?;
    row.map(|r| r.practice_epoch)
        .ok_or_else(|| worker::Error::RustError("user missing".into()))
}

async fn plan_by_request(
    db: &db::Turso,
    user: &str,
    request: &str,
) -> worker::Result<Option<PlanRow>> {
    db::first(
        db,
        "SELECT * FROM practice_plans WHERE user_id=?1 AND request_id=?2",
        &[&db::text(user), &db::text(request)],
    )
    .await
}

async fn plan_by_id(db: &db::Turso, user: &str, id: &str) -> worker::Result<Option<PlanRow>> {
    db::first(
        db,
        "SELECT * FROM practice_plans WHERE user_id=?1 AND id=?2",
        &[&db::text(user), &db::text(id)],
    )
    .await
}

async fn active_plan(db: &db::Turso, user: &str) -> worker::Result<Option<PlanRow>> {
    db::first(
        db,
        "SELECT * FROM practice_plans WHERE user_id=?1 AND ended_at IS NULL LIMIT 1",
        &[&db::text(user)],
    )
    .await
}

async fn attempt_by_request(
    db: &db::Turso,
    user: &str,
    request: &str,
) -> worker::Result<Option<AttemptRow>> {
    db::first(
        db,
        "SELECT * FROM practice_attempts WHERE user_id=?1 AND request_id=?2",
        &[&db::text(user), &db::text(request)],
    )
    .await
}

const SUMMARY_SQL: &str = "SELECT COUNT(*) FILTER (WHERE tried=1) AS attempted, \
  COUNT(*) FILTER (WHERE helpfulness='helped') AS helped, \
  COUNT(*) FILTER (WHERE helpfulness='unsure') AS unsure, \
  COUNT(*) FILTER (WHERE helpfulness='not_helped') AS not_helped, \
  COUNT(*) FILTER (WHERE tried=0) AS not_tried \
  FROM practice_attempts WHERE user_id=?1 AND cycle_id=?2";

async fn week_summary(db: &db::Turso, user: &str, cycle: &str) -> worker::Result<PracticeSummary> {
    let row: SummaryRow = db::first(db, SUMMARY_SQL, &[&db::text(user), &db::text(cycle)])
        .await?
        .ok_or_else(|| worker::Error::RustError("summary missing".into()))?;
    Ok(summary(cycle.to_string(), row))
}

fn cycle_now() -> worker::Result<String> {
    ft_schema::cycle::week_start_asia_taipei(&clock::now_iso())
        .ok_or_else(|| worker::Error::RustError("clock invalid".into()))
}

fn matches_plan(row: &PlanRow, body: &CreatePracticePlanRequest) -> bool {
    row.goal_code == body.goal_code
        && row.action_code == body.action_code
        && row.replace_active == i64::from(body.replace_active)
        && row.replaces_plan_id == body.replaces_plan_id
}

fn matches_attempt(row: &AttemptRow, plan_id: &str, body: &CreatePracticeAttemptRequest) -> bool {
    row.plan_id == plan_id
        && (row.tried == 1) == body.tried
        && row.helpfulness == body.helpfulness
        && row.obstacle == body.obstacle
}

pub fn register(router: R<'static>) -> R<'static> {
    router
    .get_async("/api/practice/history", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let url = match req.url() { Ok(u) => u, Err(_) => return Ok(error::error_code("Invalid URL", "INVALID_PAGE", 400)) };
      let (limit, cursor) = match page_query(url.query()) { Ok(p) => p, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let rows: Vec<HistoryRow> = match db::all(&db,
        "SELECT p.*, COUNT(a.id) FILTER (WHERE a.tried=1) AS attempted, \
          COUNT(a.id) FILTER (WHERE a.helpfulness='helped') AS helped, \
          COUNT(a.id) FILTER (WHERE a.helpfulness='unsure') AS unsure, \
          COUNT(a.id) FILTER (WHERE a.helpfulness='not_helped') AS not_helped, \
          COUNT(a.id) FILTER (WHERE a.tried=0) AS not_tried \
          FROM practice_plans p LEFT JOIN practice_attempts a ON a.plan_id=p.id AND a.user_id=p.user_id \
          WHERE p.user_id=?1 AND p.ended_at IS NOT NULL AND (?2 IS NULL OR (p.created_at,p.id)<(?2,?3)) \
          GROUP BY p.id ORDER BY p.created_at DESC,p.id DESC LIMIT ?4",
        &[&db::text(&user),&db::opt_text(cursor.as_ref().map(|x| x.0.as_str())),
          &db::opt_text(cursor.as_ref().map(|x| x.1.as_str())),&db::int((limit+1) as i32)]).await {
          Ok(r) => r, Err(_) => return Ok(db_error())
        };
      let next_cursor = if rows.len()>limit { rows.get(limit-1).map(|r| format!("{}|{}",r.plan.created_at,r.plan.id)) } else { None };
      Ok(json(&PracticeHistoryPage { plans: rows.iter().take(limit).map(HistoryRow::view).collect(), next_cursor }, 200))
    })
    .get_async("/api/practice/plans/:id/attempts", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let id = ctx.param("id").cloned().unwrap_or_default();
      if !valid_request_id(&id) { return Ok(error::error_code("Invalid plan", "INVALID_PRACTICE", 400)) }
      let url = match req.url() { Ok(u) => u, Err(_) => return Ok(error::error_code("Invalid URL", "INVALID_PAGE", 400)) };
      let (limit, cursor) = match page_query(url.query()) { Ok(p) => p, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      match plan_by_id(&db, &user, &id).await { Ok(Some(_)) => {}, Ok(None) => return Ok(error::error_code("Plan not found", "NOT_FOUND", 404)), Err(_) => return Ok(db_error()) }
      let rows: Vec<AttemptRow> = match db::all(&db,
        "SELECT * FROM practice_attempts WHERE user_id=?1 AND plan_id=?2 \
          AND (?3 IS NULL OR (created_at,id)<(?3,?4)) ORDER BY created_at DESC,id DESC LIMIT ?5",
        &[&db::text(&user),&db::text(&id),&db::opt_text(cursor.as_ref().map(|x| x.0.as_str())),
          &db::opt_text(cursor.as_ref().map(|x| x.1.as_str())),&db::int((limit+1) as i32)]).await {
          Ok(r) => r, Err(_) => return Ok(db_error())
        };
      let next_cursor = if rows.len()>limit { rows.get(limit-1).map(|r| format!("{}|{}",r.created_at,r.id)) } else { None };
      Ok(json(&PracticeAttemptPage { attempts: rows.iter().take(limit).map(AttemptRow::view).collect(), next_cursor }, 200))
    })
    .get_async("/api/practice", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let cycle = match cycle_now() { Ok(c) => c, Err(_) => return Ok(db_error()) };
      let prior: Option<CycleRow> = db::first(&db, "SELECT date(?1, '-7 days') AS cycle_id", &[&db::text(&cycle)]).await.ok().flatten();
      let Some(prior) = prior else { return Ok(db_error()) };
      let counts: CountRow = match db::first(&db, "SELECT (SELECT COUNT(*) FROM practice_plans WHERE user_id=?1) AS total_plans, (SELECT COUNT(*) FROM practice_attempts WHERE user_id=?1) AS total_attempts", &[&db::text(&user)]).await {
        Ok(Some(c)) => c, _ => return Ok(db_error())
      };
      let (e, active, current, previous) = match (epoch(&db, &user).await, active_plan(&db, &user).await,
        week_summary(&db, &user, &cycle).await, week_summary(&db, &user, &prior.cycle_id).await) {
        (Ok(e), Ok(a), Ok(c), Ok(p)) => (e, a, c, p), _ => return Ok(db_error())
      };
      let recent: Vec<AttemptRow> = if let Some(a) = &active {
        match db::all(&db, "SELECT * FROM practice_attempts WHERE user_id=?1 AND plan_id=?2 ORDER BY created_at DESC, id DESC LIMIT 10",
          &[&db::text(&user), &db::text(&a.id)]).await { Ok(v) => v, Err(_) => return Ok(db_error()) }
      } else { Vec::new() };
      Ok(json(&PracticeOverview { cycle_id: cycle, practice_epoch: e,
        total_plans: counts.total_plans as u32, total_attempts: counts.total_attempts as u32, current_week: current,
        previous_week: previous, active_plan: active.map(|a| a.view()),
        recent_attempts: recent.iter().map(AttemptRow::view).collect() }, 200))
    })
    .post_async("/api/practice/plans", |mut req, ctx| async move {
      if !rate_limit(&ctx, &format!("practice:ip:{}", client_ip(&req)), RATE_LIMIT, WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      if body_too_large(&req) { return Ok(error::error_code("payload too large", "PAYLOAD_TOO_LARGE", 413)) }
      let body: CreatePracticePlanRequest = match req.json().await { Ok(v) => v, Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)) };
      if !valid_request_id(&body.request_id) || !valid_epoch(body.expected_epoch) ||
        !valid_goal_action(&body.goal_code, &body.action_code) ||
        body.replace_active != body.replaces_plan_id.is_some() ||
        body.replaces_plan_id.as_deref().is_some_and(|id| !valid_request_id(id)) {
        return Ok(error::error_code("Invalid practice plan", "INVALID_PRACTICE", 400));
      }
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      match plan_by_request(&db, &user, &body.request_id).await {
        Ok(Some(existing)) => return Ok(if matches_plan(&existing, &body) { json(&existing.view(), 200) } else { conflict("IDEMPOTENCY_CONFLICT") }),
        Ok(None) => {}, Err(_) => return Ok(db_error())
      }
      let current_epoch = match epoch(&db, &user).await { Ok(e) => e, Err(_) => return Ok(db_error()) };
      if current_epoch != body.expected_epoch { return Ok(conflict("STALE_PRACTICE_STATE")) }
      let id = uuid::random_uuid();
      let now = clock::now_iso();
      let id_p = db::text(&id); let user_p = db::text(&user); let goal_p = db::text(&body.goal_code);
      let action_p = db::text(&body.action_code); let version_p = db::text(PRACTICE_CATALOG_VERSION);
      let request_p = db::text(&body.request_id); let now_p = db::text(&now);
      let epoch_p = db::int(body.expected_epoch as i32);
      let old_p = db::opt_text(body.replaces_plan_id.as_deref());
      let result = if body.replace_active {
        db::batch(&db, &[
          ("UPDATE practice_plans SET ended_at=?1, ended_by_request_id=?2 WHERE id=?3 AND user_id=?4 AND ended_at IS NULL AND EXISTS (SELECT 1 FROM users WHERE id=?4 AND practice_epoch=?5)",
            &[&now_p, &request_p, &old_p, &user_p, &epoch_p]),
          ("INSERT INTO practice_plans (id,user_id,goal_code,action_code,catalog_version,replace_active,replaces_plan_id,request_id,created_at) \
            SELECT ?1,?2,?3,?4,?5,1,?6,?7,?8 FROM practice_plans WHERE id=?6 AND user_id=?2 AND ended_at=?8 AND ended_by_request_id=?7",
            &[&id_p, &user_p, &goal_p, &action_p, &version_p, &old_p, &request_p, &now_p])
        ]).await.and_then(|v| match v.as_slice() {
          [1, 1] => Ok(1), [0, 0] => Ok(0),
          _ => Err(worker::Error::RustError("practice replace unexpected batch counts".into())),
        })
      } else {
        db::exec_changes(&db, "INSERT INTO practice_plans (id,user_id,goal_code,action_code,catalog_version,replace_active,replaces_plan_id,request_id,created_at) \
          SELECT ?1,?2,?3,?4,?5,0,NULL,?6,?7 FROM users WHERE id=?2 AND practice_epoch=?8 AND NOT EXISTS \
          (SELECT 1 FROM practice_plans WHERE user_id=?2 AND ended_at IS NULL)",
          &[&id_p,&user_p,&goal_p,&action_p,&version_p,&request_p,&now_p,&epoch_p]).await
      };
      match result {
        Ok(1) => Ok(json(&PracticePlan { id, goal_code: body.goal_code, action_code: body.action_code,
          catalog_version: PRACTICE_CATALOG_VERSION.into(), created_at: now, ended_at: None }, 201)),
        failed => {
          // A transport error might occur after a successful commit; read by idempotency key first.
          match plan_by_request(&db, &user, &body.request_id).await {
            Ok(Some(existing)) => return Ok(if matches_plan(&existing, &body) { json(&existing.view(), 200) } else { conflict("IDEMPOTENCY_CONFLICT") }),
            Err(_) => return Ok(db_error()), Ok(None) => {}
          }
          if let Err(e) = &failed { if !unique_error(e) { return Ok(db_error()) } }
          match epoch(&db, &user).await { Ok(e) if e != body.expected_epoch => return Ok(conflict("STALE_PRACTICE_STATE")),
            Err(_) => return Ok(db_error()), _ => {} }
          match active_plan(&db, &user).await {
            Ok(Some(active)) => Ok(conflict(if body.replace_active && Some(active.id.as_str()) != body.replaces_plan_id.as_deref() {
              "NO_ACTIVE_PLAN" } else { "ACTIVE_PLAN_EXISTS" })),
            Ok(None) => Ok(conflict("NO_ACTIVE_PLAN")), Err(_) => Ok(db_error())
          }
        }
      }
    })
    .post_async("/api/practice/plans/:id/attempts", |mut req, ctx| async move {
      if !rate_limit(&ctx, &format!("practice:ip:{}", client_ip(&req)), RATE_LIMIT, WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let plan_id = ctx.param("id").cloned().unwrap_or_default();
      if !valid_request_id(&plan_id) { return Ok(error::error_code("Invalid plan", "INVALID_PRACTICE", 400)) }
      if body_too_large(&req) { return Ok(error::error_code("payload too large", "PAYLOAD_TOO_LARGE", 413)) }
      let body: CreatePracticeAttemptRequest = match req.json().await { Ok(v) => v, Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)) };
      if !valid_request_id(&body.request_id) || !valid_attempt(body.tried, body.helpfulness.as_deref(), body.obstacle.as_deref()) {
        return Ok(error::error_code("Invalid attempt", "INVALID_PRACTICE", 400));
      }
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      match attempt_by_request(&db, &user, &body.request_id).await {
        Ok(Some(existing)) => return Ok(if matches_attempt(&existing, &plan_id, &body) { json(&existing.view(), 200) } else { conflict("IDEMPOTENCY_CONFLICT") }),
        Ok(None) => {}, Err(_) => return Ok(db_error())
      }
      let now = clock::now_iso(); let id = uuid::random_uuid();
      let cycle = match ft_schema::cycle::week_start_asia_taipei(&now) { Some(c) => c, None => return Ok(db_error()) };
      let n = db::exec_changes(&db, "INSERT INTO practice_attempts (id,plan_id,user_id,cycle_id,tried,helpfulness,obstacle,request_id,created_at) \
        SELECT ?1,?2,?3,?4,?5,?6,?7,?8,?9 FROM practice_plans WHERE id=?2 AND user_id=?3 AND ended_at IS NULL",
        &[&db::text(&id),&db::text(&plan_id),&db::text(&user),&db::text(&cycle),&db::bool(body.tried),
          &db::opt_text(body.helpfulness.as_deref()),&db::opt_text(body.obstacle.as_deref()),
          &db::text(&body.request_id),&db::text(&now)]).await;
      match n {
        Ok(1) => Ok(json(&PracticeAttempt { id, plan_id, cycle_id: cycle, tried: body.tried,
          helpfulness: body.helpfulness, obstacle: body.obstacle, created_at: now }, 201)),
        failed => {
          match attempt_by_request(&db, &user, &body.request_id).await {
            Ok(Some(existing)) => return Ok(if matches_attempt(&existing, &plan_id, &body) { json(&existing.view(), 200) } else { conflict("IDEMPOTENCY_CONFLICT") }),
            Err(_) => return Ok(db_error()), Ok(None) => {}
          }
          if failed.is_err() { return Ok(db_error()) }
          match plan_by_id(&db, &user, &plan_id).await {
            Ok(None) => Ok(error::error_code("Plan not found", "NOT_FOUND", 404)),
            Ok(Some(_)) => Ok(conflict("NO_ACTIVE_PLAN")), Err(_) => Ok(db_error())
          }
        }
      }
    })
    .post_async("/api/practice/plans/:id/end", |req, ctx| async move {
      if !rate_limit(&ctx, &format!("practice:ip:{}", client_ip(&req)), RATE_LIMIT, WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let id = ctx.param("id").cloned().unwrap_or_default();
      if !valid_request_id(&id) { return Ok(error::error_code("Invalid plan", "INVALID_PRACTICE", 400)) }
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let now = clock::now_iso();
      if db::exec_changes(&db, "UPDATE practice_plans SET ended_at=?1 WHERE id=?2 AND user_id=?3 AND ended_at IS NULL",
        &[&db::text(&now),&db::text(&id),&db::text(&user)]).await.is_err() { return Ok(db_error()) }
      match plan_by_id(&db, &user, &id).await { Ok(Some(plan)) => Ok(json(&plan.view(), 200)),
        Ok(None) => Ok(error::error_code("Plan not found", "NOT_FOUND", 404)), Err(_) => Ok(db_error()) }
    })
    .delete_async("/api/practice", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let u = db::text(&user);
      let counts = db::batch(&db, &[
        ("DELETE FROM practice_attempts WHERE user_id=?1", &[&u]),
        ("DELETE FROM practice_plans WHERE user_id=?1", &[&u]),
        ("UPDATE users SET practice_epoch=practice_epoch+1 WHERE id=?1", &[&u]),
      ]).await;
      if !matches!(counts, Ok(ref v) if v.get(2)==Some(&1)) { return Ok(db_error()) }
      match epoch(&db, &user).await { Ok(e) => Ok(json(&serde_json::json!({"deleted":true,"practiceEpoch":e}), 200)), Err(_) => Ok(db_error()) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_id_requires_uuid_shape() {
        assert!(valid_request_id("123e4567-e89b-42d3-a456-426614174000"));
        assert!(!valid_request_id("reuse"));
        assert!(!valid_request_id("123e4567-e89b-42d3-a456-42661417400z"));
    }

    #[test]
    fn attempt_feedback_is_exclusive() {
        assert!(valid_attempt(true, Some("helped"), None));
        assert!(valid_attempt(false, None, Some("too_big")));
        assert!(!valid_attempt(true, None, None));
        assert!(!valid_attempt(false, Some("helped"), Some("forgot")));
        assert!(!valid_attempt(true, Some("helped"), Some("other")));
    }

    #[test]
    fn cursor_rejects_non_iso_or_non_uuid() {
        assert!(
            parse_cursor("2026-10-20T12:00:00.000Z|123e4567-e89b-42d3-a456-426614174000").is_some()
        );
        assert!(parse_cursor("2026-10-20|123e4567-e89b-42d3-a456-426614174000").is_none());
        assert!(parse_cursor("2026-10-20T12:00:00.000Z|bad").is_none());
        assert!(
            parse_cursor("202X-10-20T12:00:00.000Z|123e4567-e89b-42d3-a456-426614174000").is_none()
        );
    }

    #[test]
    fn epoch_cannot_wrap_sql_integer_binding() {
        assert!(valid_epoch(0));
        assert!(valid_epoch(i32::MAX as i64));
        assert!(!valid_epoch(-1));
        assert!(!valid_epoch(i32::MAX as i64 + 1));
    }
}
