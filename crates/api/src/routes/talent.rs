//! 上手小實驗（天賦線 v1；spec docs/superpowers/specs/2026-11-22-talent-tryout-design.md，
//! §8／§9／§13.2／§13.3 為契約，§13 優先）。
//!
//! - 全部需登入（[`auth_user`]）；寫入路由（POST／DELETE）登入後共用限流
//!   `talent:user:{uid}` 每小時 60 次（[`rate_limit`]，fail-open）→ 429 `RATE_LIMITED`。
//! - `GET /api/talent/trials`：本人紀錄（新到舊）＋猜測傾向。`status` 由讀取時計算：
//!   已回報 → reported／skipped；未回報 → `is_expired` ? expired : open。`gap` 只給 reported。
//! - `POST /api/talent/trials` `{itemId, predicted, requestId}`：驗 body → 同 (user, requestId)
//!   已存在：內容相同 200 原筆、不同 409 `REQUEST_ID_CONFLICT` → 原子插入（`INSERT … SELECT …
//!   WHERE NOT EXISTS` 未過期的進行中）；affected==0 → 重查 requestId（併發）否則 409
//!   `OPEN_TRIAL_EXISTS`。成功 201。
//! - `POST /api/talent/trials/:id/report` `{reached: 0..3|null, stuck?, requestId}`：驗 body →
//!   SELECT (id, user)（無或非本人 → 404 `NOT_FOUND`）→ 已回報：同 requestId 200、否則 409
//!   `ALREADY_REPORTED` → 過期 410 `EXPIRED` → 條件 UPDATE；affected==0 重查依同規則回應。
//! - `DELETE /api/talent/trials/:id`（無 → 404）／`DELETE /api/talent/trials`：硬刪本人資料。
//! - 過期判斷唯一來源 `ft_schema::talent::is_expired`；時間戳一律 `clock::now_iso()`。
//! - DB 錯誤一律 500 `"db error"`，不回細節。
//! - 紅線：天賦線資料永不寫入 predictions／prediction_feedback／f8_assignments；不 import `symbolic`。

use super::super::{
    error,
    services::{clock, db, uuid},
};
use super::common::{auth_user, body_too_large, ok_json, rate_limit};
use super::R;
use ft_schema::api::{TalentDeleteResponse, TalentTendency, TalentTrial, TalentTrialsResponse};
use ft_schema::talent::{
    find, gap, is_expired, tendency, valid_request_id, STUCK_REASONS, TALENT_CATALOG_VERSION,
    WINDOW_SECS,
};

use serde::Deserialize;
use serde_json::Value;
use worker::*;

/// 每位使用者每小時 60 次寫入。
const USER_LIMIT: u32 = 60;
const USER_WINDOW_MS: f64 = 3_600_000.0;

const COLUMNS: &str = "id, item_id, catalog_version, predicted, created_at, \
                       reached, skipped, stuck, reported_at, report_request_id";

#[derive(Debug, Clone, Default, Deserialize)]
struct TrialRow {
    id: String,
    item_id: Option<String>,
    catalog_version: Option<String>,
    predicted: Option<i64>,
    created_at: Option<String>,
    reached: Option<i64>,
    skipped: Option<i64>,
    stuck: Option<String>,
    reported_at: Option<String>,
    report_request_id: Option<String>,
}

/// 0–3 的整數；浮點（含 1.0）、負數、字串、超界 → None。
fn step_value(v: &Value) -> Option<u8> {
    v.as_u64().filter(|n| *n <= 3).map(|n| n as u8)
}

fn small_u8(v: Option<i64>) -> Option<u8> {
    v.and_then(|n| u8::try_from(n).ok())
}

/// 列 → wire。`now` 用於未回報列的過期判斷。
fn to_trial(row: &TrialRow, now: &str) -> TalentTrial {
    let created_at = row.created_at.clone().unwrap_or_default();
    let predicted = small_u8(row.predicted).unwrap_or(0);
    let reached = small_u8(row.reached);
    let status = if row.reported_at.is_some() {
        if row.skipped == Some(1) {
            "skipped"
        } else {
            "reported"
        }
    } else if is_expired(&created_at, now) {
        "expired"
    } else {
        "open"
    };
    let gap = match (status, reached) {
        ("reported", Some(r)) => Some(gap(predicted, r)),
        _ => None,
    };
    TalentTrial {
        id: row.id.clone(),
        item_id: row.item_id.clone().unwrap_or_default(),
        catalog_version: row.catalog_version.clone().unwrap_or_default(),
        predicted,
        created_at,
        status: status.to_string(),
        reached,
        stuck: row.stuck.clone(),
        reported_at: row.reported_at.clone(),
        gap,
    }
}

/// 只吃 reported 的差距（skipped／expired／open 不算）。
fn tendency_of(trials: &[TalentTrial]) -> TalentTendency {
    let gaps: Vec<i8> = trials
        .iter()
        .filter(|t| t.status == "reported")
        .filter_map(|t| t.gap)
        .collect();
    let t = tendency(&gaps);
    TalentTendency {
        reported_count: t.reported,
        need: t.need,
        under: t.under,
        exact: t.exact,
        over: t.over,
        kind: t.kind.map(|k| k.wire().to_string()),
    }
}

#[derive(Debug, PartialEq)]
struct CreateInput {
    item_id: &'static str,
    predicted: u8,
    request_id: String,
}

/// 驗 POST body；錯誤回 code。順序：requestId → itemId → predicted。
fn parse_create(body: &Value) -> std::result::Result<CreateInput, &'static str> {
    let request_id = body
        .get("requestId")
        .and_then(Value::as_str)
        .filter(|s| valid_request_id(s))
        .ok_or("INVALID_REQUEST_ID")?;
    let item = body
        .get("itemId")
        .and_then(Value::as_str)
        .and_then(find)
        .ok_or("INVALID_ITEM")?;
    let predicted = body
        .get("predicted")
        .and_then(step_value)
        .ok_or("INVALID_PREDICTION")?;
    Ok(CreateInput {
        item_id: item.id,
        predicted,
        request_id: request_id.to_string(),
    })
}

#[derive(Debug, PartialEq)]
struct ReportInput {
    /// None＝這次沒做
    reached: Option<u8>,
    stuck: Option<&'static str>,
    request_id: String,
}

/// 驗 report body。`reached` key 必填（null＝沒做）；`stuck` 可省略／null，有值須為卡點 wire，
/// 且 reached 為 null 時不得帶。
fn parse_report(body: &Value) -> std::result::Result<ReportInput, &'static str> {
    let request_id = body
        .get("requestId")
        .and_then(Value::as_str)
        .filter(|s| valid_request_id(s))
        .ok_or("INVALID_REQUEST_ID")?;
    let reached = match body.get("reached") {
        None => return Err("INVALID_REACHED"),
        Some(Value::Null) => None,
        Some(v) => Some(step_value(v).ok_or("INVALID_REACHED")?),
    };
    let stuck = match body.get("stuck") {
        None | Some(Value::Null) => None,
        Some(v) => {
            let wire = v.as_str().ok_or("INVALID_STUCK")?;
            // 取 STUCK_REASONS 內的 &'static 版本（同時完成白名單驗證）
            let wire = STUCK_REASONS
                .iter()
                .map(|(w, _)| *w)
                .find(|w| *w == wire)
                .ok_or("INVALID_STUCK")?;
            Some(wire)
        }
    };
    if reached.is_none() && stuck.is_some() {
        return Err("INVALID_STUCK");
    }
    Ok(ReportInput {
        reached,
        stuck,
        request_id: request_id.to_string(),
    })
}

/// 同 requestId 的既有列：內容相同 → 冪等重送；不同 → 衝突。
fn create_replay_matches(row: &TrialRow, input: &CreateInput) -> bool {
    row.item_id.as_deref() == Some(input.item_id)
        && row.predicted == Some(i64::from(input.predicted))
}

#[derive(Debug, PartialEq)]
enum ReportDecision {
    /// 已以同 requestId 回報 → 200 原結果
    Replay,
    AlreadyReported,
    Expired,
    /// 未回報且未過期 → 可寫入
    Proceed,
}

fn report_decision(row: &TrialRow, request_id: &str, now: &str) -> ReportDecision {
    if row.reported_at.is_some() {
        if row.report_request_id.as_deref() == Some(request_id) {
            ReportDecision::Replay
        } else {
            ReportDecision::AlreadyReported
        }
    } else if is_expired(row.created_at.as_deref().unwrap_or_default(), now) {
        ReportDecision::Expired
    } else {
        ReportDecision::Proceed
    }
}

fn json_status<T: serde::Serialize>(v: &T, status: u16) -> Response {
    let mut res = ok_json(&serde_json::to_value(v).unwrap_or(Value::Null), status);
    let _ = res.headers_mut().set("Cache-Control", "private, no-store");
    res
}

fn db_error() -> Response {
    error::error_code("db error", "DB_ERROR", 500)
}

fn payload_too_large() -> Response {
    error::error_code("payload too large", "PAYLOAD_TOO_LARGE", 413)
}

fn rate_limited() -> Response {
    error::error_code("Too many requests", "RATE_LIMITED", 429)
}

fn not_found() -> Response {
    error::error_code("Not found", "NOT_FOUND", 404)
}

fn bad_request(code: &str) -> Response {
    let msg = match code {
        "INVALID_REQUEST_ID" => "Invalid requestId",
        "INVALID_ITEM" => "Invalid item",
        "INVALID_PREDICTION" => "Invalid prediction",
        "INVALID_REACHED" => "Invalid reached",
        "INVALID_STUCK" => "Invalid stuck",
        _ => "Invalid request",
    };
    error::error_code(msg, code, 400)
}

fn open_trial_exists() -> Response {
    error::error_code("An open trial already exists", "OPEN_TRIAL_EXISTS", 409)
}

fn request_id_conflict() -> Response {
    error::error_code(
        "requestId already used with different content",
        "REQUEST_ID_CONFLICT",
        409,
    )
}

fn report_answer(decision: ReportDecision, row: &TrialRow, now: &str) -> Response {
    match decision {
        ReportDecision::Replay => json_status(&to_trial(row, now), 200),
        ReportDecision::AlreadyReported => {
            error::error_code("Already reported", "ALREADY_REPORTED", 409)
        }
        // 第二輪仍顯示可寫卻 UPDATE 0 列＝剛跨過期限（秒／毫秒邊界）→ 過期
        ReportDecision::Expired | ReportDecision::Proceed => {
            error::error_code("Trial expired", "EXPIRED", 410)
        }
    }
}

/// 期限起點：now − WINDOW_SECS（ISO，與 `created_at` 同格式可直接字串比較）。
fn cutoff_iso() -> String {
    clock::now_plus_ms(-(WINDOW_SECS as f64) * 1000.0)
}

async fn read_json(req: &mut Request) -> std::result::Result<Value, Response> {
    req.json::<Value>()
        .await
        .map_err(|_| error::error_code("Invalid JSON", "INVALID_JSON", 400))
}

async fn trial_by_request_id(
    db: &db::Turso,
    user: &str,
    request_id: &str,
) -> worker::Result<Option<TrialRow>> {
    db::first(
        db,
        &format!(
            "SELECT {COLUMNS} FROM talent_trials WHERE user_id=?1 AND create_request_id=?2 \
             ORDER BY created_at LIMIT 1"
        ),
        &[&db::text(user), &db::text(request_id)],
    )
    .await
}

async fn trial_by_id(db: &db::Turso, user: &str, id: &str) -> worker::Result<Option<TrialRow>> {
    db::first(
        db,
        &format!("SELECT {COLUMNS} FROM talent_trials WHERE id=?1 AND user_id=?2"),
        &[&db::text(id), &db::text(user)],
    )
    .await
}

/// 同 requestId 既有列 → Some(回應)；無 → None。
fn replay_response(row: Option<TrialRow>, input: &CreateInput, now: &str) -> Option<Response> {
    let row = row?;
    Some(if create_replay_matches(&row, input) {
        json_status(&to_trial(&row, now), 200)
    } else {
        request_id_conflict()
    })
}

pub fn register(router: R<'static>) -> R<'static> {
    router
        .get_async("/api/talent/trials", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            let rows: Vec<TrialRow> = match db::all(
                &db,
                &format!(
                    "SELECT {COLUMNS} FROM talent_trials WHERE user_id=?1 \
                     ORDER BY created_at DESC, id DESC"
                ),
                &[&db::text(&user)],
            )
            .await
            {
                Ok(r) => r,
                Err(_) => return Ok(db_error()),
            };
            let now = clock::now_iso();
            let trials: Vec<TalentTrial> = rows.iter().map(|r| to_trial(r, &now)).collect();
            let tendency = tendency_of(&trials);
            Ok(json_status(&TalentTrialsResponse { trials, tendency }, 200))
        })
        .post_async("/api/talent/trials", |mut req, ctx| async move {
            if body_too_large(&req) {
                return Ok(payload_too_large());
            }
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            if !rate_limit(
                &ctx,
                &format!("talent:user:{user}"),
                USER_LIMIT,
                USER_WINDOW_MS,
            )
            .await
            {
                return Ok(rate_limited());
            }
            let body = match read_json(&mut req).await {
                Ok(v) => v,
                Err(e) => return Ok(e),
            };
            let input = match parse_create(&body) {
                Ok(i) => i,
                Err(code) => return Ok(bad_request(code)),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            let now = clock::now_iso();
            match trial_by_request_id(&db, &user, &input.request_id).await {
                Ok(existing) => {
                    if let Some(res) = replay_response(existing, &input, &now) {
                        return Ok(res);
                    }
                }
                Err(_) => return Ok(db_error()),
            }

            let id = uuid::random_uuid();
            let cutoff = cutoff_iso();
            let inserted = match db::exec_changes(
                &db,
                "INSERT INTO talent_trials \
                 (id, user_id, item_id, catalog_version, predicted, created_at, create_request_id) \
                 SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7 \
                 WHERE NOT EXISTS (SELECT 1 FROM talent_trials \
                   WHERE user_id=?2 AND reported_at IS NULL AND created_at > ?8)",
                &[
                    &db::text(&id),
                    &db::text(&user),
                    &db::text(input.item_id),
                    &db::text(TALENT_CATALOG_VERSION),
                    &db::int(i32::from(input.predicted)),
                    &db::text(&now),
                    &db::text(&input.request_id),
                    &db::text(&cutoff),
                ],
            )
            .await
            {
                Ok(n) => n,
                Err(_) => return Ok(db_error()),
            };
            if inserted == 0 {
                // 併發同 requestId 的另一請求可能剛插入 → 先依冪等規則回應
                return match trial_by_request_id(&db, &user, &input.request_id).await {
                    Ok(existing) => {
                        Ok(replay_response(existing, &input, &now)
                            .unwrap_or_else(open_trial_exists))
                    }
                    Err(_) => Ok(db_error()),
                };
            }
            let row = TrialRow {
                id,
                item_id: Some(input.item_id.to_string()),
                catalog_version: Some(TALENT_CATALOG_VERSION.to_string()),
                predicted: Some(i64::from(input.predicted)),
                created_at: Some(now.clone()),
                ..TrialRow::default()
            };
            Ok(json_status(&to_trial(&row, &now), 201))
        })
        .post_async("/api/talent/trials/:id/report", |mut req, ctx| async move {
            if body_too_large(&req) {
                return Ok(payload_too_large());
            }
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            if !rate_limit(
                &ctx,
                &format!("talent:user:{user}"),
                USER_LIMIT,
                USER_WINDOW_MS,
            )
            .await
            {
                return Ok(rate_limited());
            }
            let trial_id = ctx.param("id").cloned().unwrap_or_default();
            let body = match read_json(&mut req).await {
                Ok(v) => v,
                Err(e) => return Ok(e),
            };
            let input = match parse_report(&body) {
                Ok(i) => i,
                Err(code) => return Ok(bad_request(code)),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            let now = clock::now_iso();
            let row = match trial_by_id(&db, &user, &trial_id).await {
                Ok(Some(r)) => r,
                Ok(None) => return Ok(not_found()),
                Err(_) => return Ok(db_error()),
            };
            let decision = report_decision(&row, &input.request_id, &now);
            if decision != ReportDecision::Proceed {
                return Ok(report_answer(decision, &row, &now));
            }

            let cutoff = cutoff_iso();
            let changed = match db::exec_changes(
                &db,
                "UPDATE talent_trials SET reached=?1, skipped=?2, stuck=?3, reported_at=?4, \
                 report_request_id=?5 \
                 WHERE id=?6 AND user_id=?7 AND reported_at IS NULL AND created_at > ?8",
                &[
                    &db::opt_int(input.reached.map(i64::from)),
                    &db::int(if input.reached.is_none() { 1 } else { 0 }),
                    &db::opt_text(input.stuck),
                    &db::text(&now),
                    &db::text(&input.request_id),
                    &db::text(&trial_id),
                    &db::text(&user),
                    &db::text(&cutoff),
                ],
            )
            .await
            {
                Ok(n) => n,
                Err(_) => return Ok(db_error()),
            };
            let row = match trial_by_id(&db, &user, &trial_id).await {
                Ok(Some(r)) => r,
                Ok(None) => return Ok(not_found()),
                Err(_) => return Ok(db_error()),
            };
            if changed == 0 {
                let decision = report_decision(&row, &input.request_id, &now);
                return Ok(report_answer(decision, &row, &now));
            }
            Ok(json_status(&to_trial(&row, &now), 200))
        })
        .delete_async("/api/talent/trials/:id", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            if !rate_limit(
                &ctx,
                &format!("talent:user:{user}"),
                USER_LIMIT,
                USER_WINDOW_MS,
            )
            .await
            {
                return Ok(rate_limited());
            }
            let trial_id = ctx.param("id").cloned().unwrap_or_default();
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            let n = match db::exec_changes(
                &db,
                "DELETE FROM talent_trials WHERE id=?1 AND user_id=?2",
                &[&db::text(&trial_id), &db::text(&user)],
            )
            .await
            {
                Ok(n) => n,
                Err(_) => return Ok(db_error()),
            };
            if n == 0 {
                return Ok(not_found());
            }
            Ok(json_status(
                &TalentDeleteResponse { deleted: n as u32 },
                200,
            ))
        })
        .delete_async("/api/talent/trials", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            if !rate_limit(
                &ctx,
                &format!("talent:user:{user}"),
                USER_LIMIT,
                USER_WINDOW_MS,
            )
            .await
            {
                return Ok(rate_limited());
            }
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            let n = match db::exec_changes(
                &db,
                "DELETE FROM talent_trials WHERE user_id=?1",
                &[&db::text(&user)],
            )
            .await
            {
                Ok(n) => n,
                Err(_) => return Ok(db_error()),
            };
            Ok(json_status(
                &TalentDeleteResponse { deleted: n as u32 },
                200,
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const C: &str = "2026-11-22T00:00:00.000Z";
    const NOW: &str = "2026-11-23T00:00:00.000Z";
    const LATE: &str = "2026-11-29T00:00:00.000Z";

    fn open_row() -> TrialRow {
        TrialRow {
            id: "t1".into(),
            item_id: Some("juggle".into()),
            catalog_version: Some("1".into()),
            predicted: Some(1),
            created_at: Some(C.into()),
            ..TrialRow::default()
        }
    }

    fn reported_row(predicted: i64, reached: Option<i64>) -> TrialRow {
        TrialRow {
            predicted: Some(predicted),
            reached,
            skipped: Some(if reached.is_none() { 1 } else { 0 }),
            reported_at: Some(NOW.into()),
            report_request_id: Some("req-report-1".into()),
            ..open_row()
        }
    }

    #[test]
    fn status_open_expired_reported_skipped() {
        let t = to_trial(&open_row(), NOW);
        assert_eq!((t.status.as_str(), t.gap, t.reached), ("open", None, None));
        assert_eq!(to_trial(&open_row(), LATE).status, "expired");
        let bad = TrialRow {
            created_at: Some("garbage".into()),
            ..open_row()
        };
        assert_eq!(to_trial(&bad, NOW).status, "expired", "壞時間不擋新實驗");

        let r = to_trial(&reported_row(1, Some(3)), LATE);
        assert_eq!(r.status, "reported", "已回報不因時間過期");
        assert_eq!((r.reached, r.gap), (Some(3), Some(2)));
        assert_eq!(r.reported_at.as_deref(), Some(NOW));

        let s = to_trial(&reported_row(2, None), NOW);
        assert_eq!(
            (s.status.as_str(), s.gap, s.reached),
            ("skipped", None, None)
        );
    }

    #[test]
    fn to_trial_wire_shape() {
        let v = serde_json::to_value(to_trial(&reported_row(3, Some(0)), NOW)).unwrap();
        assert_eq!(v["itemId"], "juggle");
        assert_eq!(v["catalogVersion"], "1");
        assert_eq!(v["createdAt"], C);
        assert_eq!(v["gap"], -3);
        assert_eq!(v["reportedAt"], NOW);
    }

    #[test]
    fn tendency_counts_only_reported() {
        let mut trials = vec![
            to_trial(&reported_row(1, Some(2)), NOW),
            to_trial(&reported_row(1, None), NOW),
            to_trial(&open_row(), NOW),
            to_trial(&open_row(), LATE),
        ];
        let t = tendency_of(&trials);
        assert_eq!((t.reported_count, t.need, t.kind), (1, 2, None));
        trials.push(to_trial(&reported_row(0, Some(1)), NOW));
        trials.push(to_trial(&reported_row(2, Some(2)), NOW));
        let t = tendency_of(&trials);
        assert_eq!((t.reported_count, t.need), (3, 0));
        assert_eq!((t.under, t.exact, t.over), (2, 1, 0));
        assert_eq!(t.kind.as_deref(), Some("conservative"));
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["reportedCount"], 3);
    }

    #[test]
    fn step_value_rejects_non_integers() {
        assert_eq!(step_value(&json!(0)), Some(0));
        assert_eq!(step_value(&json!(3)), Some(3));
        for bad in [
            json!(4),
            json!(-1),
            json!(1.5),
            json!(1.0),
            json!("1"),
            json!(null),
            json!(true),
        ] {
            assert_eq!(step_value(&bad), None, "{bad}");
        }
    }

    #[test]
    fn parse_create_rules() {
        let ok = json!({"itemId":"crane","predicted":2,"requestId":"abcd1234"});
        assert_eq!(
            parse_create(&ok),
            Ok(CreateInput {
                item_id: "crane",
                predicted: 2,
                request_id: "abcd1234".into()
            })
        );
        let cases = [
            (
                json!({"itemId":"crane","predicted":2,"requestId":"short"}),
                "INVALID_REQUEST_ID",
            ),
            (
                json!({"itemId":"crane","predicted":2}),
                "INVALID_REQUEST_ID",
            ),
            (
                json!({"itemId":"nope","predicted":2,"requestId":"abcd1234"}),
                "INVALID_ITEM",
            ),
            (
                json!({"predicted":2,"requestId":"abcd1234"}),
                "INVALID_ITEM",
            ),
            (
                json!({"itemId":"crane","predicted":1.5,"requestId":"abcd1234"}),
                "INVALID_PREDICTION",
            ),
            (
                json!({"itemId":"crane","predicted":-1,"requestId":"abcd1234"}),
                "INVALID_PREDICTION",
            ),
            (
                json!({"itemId":"crane","predicted":"2","requestId":"abcd1234"}),
                "INVALID_PREDICTION",
            ),
            (
                json!({"itemId":"crane","requestId":"abcd1234"}),
                "INVALID_PREDICTION",
            ),
            (json!([1, 2]), "INVALID_REQUEST_ID"),
        ];
        for (body, code) in cases {
            assert_eq!(parse_create(&body), Err(code), "{body}");
        }
    }

    #[test]
    fn parse_report_rules() {
        assert_eq!(
            parse_report(&json!({"reached":2,"stuck":"time","requestId":"abcd1234"})),
            Ok(ReportInput {
                reached: Some(2),
                stuck: Some("time"),
                request_id: "abcd1234".into()
            })
        );
        assert_eq!(
            parse_report(&json!({"reached":null,"requestId":"abcd1234"})),
            Ok(ReportInput {
                reached: None,
                stuck: None,
                request_id: "abcd1234".into()
            }),
            "null＝這次沒做"
        );
        assert_eq!(
            parse_report(&json!({"reached":null,"stuck":null,"requestId":"abcd1234"}))
                .map(|r| r.stuck),
            Ok(None)
        );
        let cases = [
            (json!({"reached":1,"requestId":"x"}), "INVALID_REQUEST_ID"),
            (json!({"requestId":"abcd1234"}), "INVALID_REACHED"),
            (
                json!({"reached":4,"requestId":"abcd1234"}),
                "INVALID_REACHED",
            ),
            (
                json!({"reached":0.5,"requestId":"abcd1234"}),
                "INVALID_REACHED",
            ),
            (
                json!({"reached":"1","requestId":"abcd1234"}),
                "INVALID_REACHED",
            ),
            (
                json!({"reached":1,"stuck":"lazy","requestId":"abcd1234"}),
                "INVALID_STUCK",
            ),
            (
                json!({"reached":1,"stuck":3,"requestId":"abcd1234"}),
                "INVALID_STUCK",
            ),
            (
                json!({"reached":null,"stuck":"time","requestId":"abcd1234"}),
                "INVALID_STUCK",
            ),
        ];
        for (body, code) in cases {
            assert_eq!(parse_report(&body), Err(code), "{body}");
        }
    }

    #[test]
    fn create_replay_compares_item_and_prediction() {
        let row = open_row();
        let same = CreateInput {
            item_id: "juggle",
            predicted: 1,
            request_id: "req-create-1".into(),
        };
        assert!(create_replay_matches(&row, &same));
        let other_pred = CreateInput {
            predicted: 2,
            ..same
        };
        assert!(!create_replay_matches(&row, &other_pred));
        let other_item = CreateInput {
            item_id: "crane",
            predicted: 1,
            request_id: "req-create-1".into(),
        };
        assert!(!create_replay_matches(&row, &other_item));
    }

    #[test]
    fn report_decision_order() {
        assert_eq!(
            report_decision(&open_row(), "abcd1234", NOW),
            ReportDecision::Proceed
        );
        assert_eq!(
            report_decision(&open_row(), "abcd1234", LATE),
            ReportDecision::Expired
        );
        let done = reported_row(1, Some(1));
        assert_eq!(
            report_decision(&done, "req-report-1", LATE),
            ReportDecision::Replay,
            "已回報優先於過期"
        );
        assert_eq!(
            report_decision(&done, "other-req-1", NOW),
            ReportDecision::AlreadyReported
        );
    }
}
