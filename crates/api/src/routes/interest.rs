//! 興趣探索 v1（RIASEC）。No personality, chart, prediction, experiment or AI inputs;
//! interest data is never written to predictions/prediction_feedback/f8_assignments.
//! Spec: docs/superpowers/specs/2026-10-27-interest-explore-design.md §4.

use super::super::{
    error,
    services::{clock, db, uuid},
};
use super::common::{
    apply_cache_headers, auth_user, body_too_large, client_ip, ok_json, rate_limit,
};
use super::practice::{page_query, valid_epoch, valid_request_id};
use super::R;
use ft_schema::interest::{
    score, valid_answers, CreateInterestResultRequest, InterestHistoryPage, InterestOverview,
    InterestResult, InterestScores, FORM, INTEREST_INSTRUMENT_VERSION,
};

const RATE_LIMIT: u32 = 20;
const WINDOW_MS: f64 = 60_000.0;

#[derive(serde::Deserialize)]
struct ResultRow {
    id: String,
    instrument_version: String,
    form: String,
    /// 只在冪等比對時 SELECT；列表與最新結果不讀原始作答。
    #[serde(default)]
    answers: Option<String>,
    scores: String,
    created_at: String,
}

impl ResultRow {
    fn view(&self) -> Option<InterestResult> {
        Some(InterestResult {
            id: self.id.clone(),
            instrument_version: self.instrument_version.clone(),
            form: self.form.clone(),
            scores: serde_json::from_str(&self.scores).ok()?,
            created_at: self.created_at.clone(),
        })
    }
}

#[derive(serde::Deserialize)]
struct EpochRow {
    interest_epoch: i64,
}

#[derive(serde::Deserialize)]
struct CountRow {
    total_results: i64,
}

/// §4.2 步驟 (0)：形狀驗證，先於冪等查詢（practice/tarot/palm 前例）。
fn valid_shape(body: &CreateInterestResultRequest) -> bool {
    valid_request_id(&body.request_id)
        && valid_epoch(body.expected_epoch)
        && body.form == FORM
        && valid_answers(&body.answers)
}

/// §4.2 步驟 (1)：舊列比對 instrumentVersion＋form＋answers，**不含 epoch**。
fn matches_result(row: &ResultRow, body: &CreateInterestResultRequest) -> bool {
    let stored: Option<Vec<u8>> = row
        .answers
        .as_deref()
        .and_then(|a| serde_json::from_str(a).ok());
    row.instrument_version == body.instrument_version
        && row.form == body.form
        && stored.as_deref() == Some(body.answers.as_slice())
}

/// §4.2 步驟 (2)：僅新鍵才檢查語意——版本先於 epoch。
fn new_key_conflict(
    body: &CreateInterestResultRequest,
    current_epoch: i64,
) -> Option<&'static str> {
    if body.instrument_version != INTEREST_INSTRUMENT_VERSION {
        Some("STALE_INSTRUMENT")
    } else if current_epoch != body.expected_epoch {
        Some("STALE_INTEREST_STATE")
    } else {
        None
    }
}

/// 冪等鍵命中或寫入失敗後的判定；route 再轉成 Response。
#[derive(Debug, PartialEq)]
enum Outcome {
    /// 回原結果 200
    Replay,
    Conflict(&'static str),
    DbError,
}

/// §4.2 (1)／(4)：按 requestId 重讀的結果。`None`＝無列，需再看 epoch（`stale_outcome`）。
fn reread_outcome(
    reread: Result<Option<&ResultRow>, ()>,
    body: &CreateInterestResultRequest,
) -> Option<Outcome> {
    match reread {
        Err(()) => Some(Outcome::DbError),
        Ok(None) => None,
        Ok(Some(row)) if matches_result(row, body) => Some(Outcome::Replay),
        Ok(Some(_)) => Some(Outcome::Conflict("IDEMPOTENCY_CONFLICT")),
    }
}

/// §4.2 (4)：寫入失敗（`Ok(0)`、UNIQUE 敗者或任何錯誤）且重讀無列——epoch 不符回 409，
/// 其餘無法判定者（含 epoch 讀取失敗）回 500。不依錯誤類型提早分岔（Kimi review S1）。
fn stale_outcome(epoch_now: Result<i64, ()>, expected_epoch: i64) -> Outcome {
    match epoch_now {
        Ok(e) if e != expected_epoch => Outcome::Conflict("STALE_INTEREST_STATE"),
        _ => Outcome::DbError,
    }
}

/// history 分頁：查 `limit+1` 判斷有無下一頁，游標由最後一筆**已回傳**資料產生（practice 契約）。
fn next_cursor(rows: &[ResultRow], limit: usize) -> Option<String> {
    if rows.len() <= limit {
        return None;
    }
    rows.get(limit.checked_sub(1)?)
        .map(|r| format!("{}|{}", r.created_at, r.id))
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
    error::error_code("Interest state changed", code, 409)
}

async fn epoch(db: &db::Turso, user: &str) -> worker::Result<i64> {
    let row: Option<EpochRow> = db::first(
        db,
        "SELECT interest_epoch FROM users WHERE id=?1",
        &[&db::text(user)],
    )
    .await?;
    row.map(|r| r.interest_epoch)
        .ok_or_else(|| worker::Error::RustError("user missing".into()))
}

async fn result_by_request(
    db: &db::Turso,
    user: &str,
    request: &str,
) -> worker::Result<Option<ResultRow>> {
    db::first(
        db,
        "SELECT id, instrument_version, form, answers, scores, created_at FROM interest_results \
         WHERE user_id=?1 AND request_id=?2",
        &[&db::text(user), &db::text(request)],
    )
    .await
}

fn outcome_response(outcome: Outcome, existing: Option<&ResultRow>) -> worker::Response {
    match outcome {
        Outcome::Replay => match existing.and_then(ResultRow::view) {
            Some(v) => json(&v, 200),
            None => db_error(),
        },
        Outcome::Conflict(code) => conflict(code),
        Outcome::DbError => db_error(),
    }
}

/// 冪等鍵重讀後若已可判定（有列或重讀失敗）即回應；`None`＝無列。
fn reread_response(
    existing: &worker::Result<Option<ResultRow>>,
    body: &CreateInterestResultRequest,
) -> Option<worker::Response> {
    let row = existing.as_ref().map(Option::as_ref).map_err(|_| ());
    reread_outcome(row, body).map(|o| outcome_response(o, row.ok().flatten()))
}

pub fn register(router: R<'static>) -> R<'static> {
    router
    .get_async("/api/interest/history", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let url = match req.url() { Ok(u) => u, Err(_) => return Ok(error::error_code("Invalid URL", "INVALID_PAGE", 400)) };
      let (limit, cursor) = match page_query(url.query()) { Ok(p) => p, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let rows: Vec<ResultRow> = match db::all(&db,
        "SELECT id, instrument_version, form, scores, created_at FROM interest_results \
          WHERE user_id=?1 AND (?2 IS NULL OR (created_at,id)<(?2,?3)) ORDER BY created_at DESC,id DESC LIMIT ?4",
        &[&db::text(&user),&db::opt_text(cursor.as_ref().map(|x| x.0.as_str())),
          &db::opt_text(cursor.as_ref().map(|x| x.1.as_str())),&db::int((limit+1) as i32)]).await {
          Ok(r) => r, Err(_) => return Ok(db_error())
        };
      let next_cursor = next_cursor(&rows, limit);
      let results: Option<Vec<InterestResult>> = rows.iter().take(limit).map(ResultRow::view).collect();
      match results {
        Some(results) => Ok(json(&InterestHistoryPage { results, next_cursor }, 200)),
        None => Ok(db_error()),
      }
    })
    .get_async("/api/interest", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let counts: CountRow = match db::first(&db, "SELECT COUNT(*) AS total_results FROM interest_results WHERE user_id=?1", &[&db::text(&user)]).await {
        Ok(Some(c)) => c, _ => return Ok(db_error())
      };
      let latest: Option<ResultRow> = match db::first(&db,
        "SELECT id, instrument_version, form, scores, created_at FROM interest_results \
          WHERE user_id=?1 ORDER BY created_at DESC, id DESC LIMIT 1", &[&db::text(&user)]).await {
        Ok(r) => r, Err(_) => return Ok(db_error())
      };
      let e = match epoch(&db, &user).await { Ok(e) => e, Err(_) => return Ok(db_error()) };
      let latest = match latest { Some(row) => match row.view() { Some(v) => Some(v), None => return Ok(db_error()) }, None => None };
      Ok(json(&InterestOverview { interest_epoch: e, total_results: counts.total_results as u32, latest }, 200))
    })
    .post_async("/api/interest/results", |mut req, ctx| async move {
      if !rate_limit(&ctx, &format!("interest:ip:{}", client_ip(&req)), RATE_LIMIT, WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      if body_too_large(&req) { return Ok(error::error_code("payload too large", "PAYLOAD_TOO_LARGE", 413)) }
      let body: CreateInterestResultRequest = match req.json().await { Ok(v) => v, Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)) };
      // (0) 形狀 → 400
      if !valid_shape(&body) { return Ok(error::error_code("Invalid interest result", "INVALID_INTEREST", 400)) }
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      // (1) 冪等鍵
      if let Some(res) = reread_response(&result_by_request(&db, &user, &body.request_id).await, &body) { return Ok(res) }
      // (2) 新鍵語意：版本 → epoch
      let current_epoch = match epoch(&db, &user).await { Ok(e) => e, Err(_) => return Ok(db_error()) };
      if let Some(code) = new_key_conflict(&body, current_epoch) { return Ok(conflict(code)) }
      // 伺服器重算，不信任前端分數
      let Some(scores) = score(&body.answers) else { return Ok(error::error_code("Invalid interest result", "INVALID_INTEREST", 400)) };
      let scores = InterestScores::from_array(scores);
      let (Ok(scores_json), Ok(answers_json)) = (serde_json::to_string(&scores), serde_json::to_string(&body.answers)) else { return Ok(db_error()) };
      let id = uuid::random_uuid();
      let now = clock::now_iso();
      // (3) 條件寫入：epoch 於同一句檢查
      let n = db::exec_changes(&db, "INSERT INTO interest_results (id,user_id,instrument_version,form,answers,scores,request_id,created_at) \
        SELECT ?1,?2,?3,?4,?5,?6,?7,?8 FROM users WHERE id=?2 AND interest_epoch=?9",
        &[&db::text(&id),&db::text(&user),&db::text(INTEREST_INSTRUMENT_VERSION),&db::text(FORM),
          &db::text(&answers_json),&db::text(&scores_json),&db::text(&body.request_id),&db::text(&now),
          &db::int(body.expected_epoch as i32)]).await;
      match n {
        Ok(1) => Ok(json(&InterestResult { id, instrument_version: INTEREST_INSTRUMENT_VERSION.into(),
          form: FORM.into(), scores, created_at: now }, 201)),
        // (4) Ok(0)、UNIQUE 敗者或任何寫入錯誤同一路徑（傳輸錯誤可能發生在 commit 之後）：
        // 先重讀冪等鍵；無列再看 epoch，不依錯誤類型提早分岔
        _ => {
          if let Some(res) = reread_response(&result_by_request(&db, &user, &body.request_id).await, &body) { return Ok(res) }
          Ok(outcome_response(stale_outcome(epoch(&db, &user).await.map_err(|_| ()), body.expected_epoch), None))
        }
      }
    })
    .delete_async("/api/interest", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let u = db::text(&user);
      let counts = db::batch(&db, &[
        ("DELETE FROM interest_results WHERE user_id=?1", &[&u]),
        ("UPDATE users SET interest_epoch=interest_epoch+1 WHERE id=?1", &[&u]),
      ]).await;
      if !matches!(counts, Ok(ref v) if v.get(1)==Some(&1)) { return Ok(db_error()) }
      match epoch(&db, &user).await { Ok(e) => Ok(json(&serde_json::json!({"deleted":true,"interestEpoch":e}), 200)), Err(_) => Ok(db_error()) }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RID: &str = "123e4567-e89b-42d3-a456-426614174000";

    fn body() -> CreateInterestResultRequest {
        CreateInterestResultRequest {
            instrument_version: INTEREST_INSTRUMENT_VERSION.into(),
            form: FORM.into(),
            answers: vec![3; 48],
            expected_epoch: 0,
            request_id: RID.into(),
        }
    }

    fn row(answers: &[u8]) -> ResultRow {
        ResultRow {
            id: "r1".into(),
            instrument_version: INTEREST_INSTRUMENT_VERSION.into(),
            form: FORM.into(),
            answers: Some(serde_json::to_string(answers).unwrap()),
            scores: serde_json::to_string(&InterestScores::from_array([3.0; 6])).unwrap(),
            created_at: "2026-10-30T00:00:00.000Z".into(),
        }
    }

    #[test]
    fn shape_accepts_well_formed_body() {
        assert!(valid_shape(&body()));
    }

    #[test]
    fn shape_rejects_bad_request_id() {
        let mut b = body();
        b.request_id = "reuse".into();
        assert!(!valid_shape(&b));
    }

    #[test]
    fn shape_rejects_epoch_out_of_i32_range() {
        let mut b = body();
        b.expected_epoch = i32::MAX as i64 + 1;
        assert!(!valid_shape(&b));
        b.expected_epoch = -1;
        assert!(!valid_shape(&b));
    }

    #[test]
    fn shape_rejects_form_other_than_a() {
        let mut b = body();
        b.form = "B".into();
        assert!(!valid_shape(&b));
    }

    #[test]
    fn shape_rejects_bad_answers() {
        let mut b = body();
        b.answers = vec![3; 47];
        assert!(!valid_shape(&b));
        b.answers = vec![6; 48];
        assert!(!valid_shape(&b));
    }

    #[test]
    fn shape_does_not_judge_instrument_version() {
        // 版本屬語意檢查（步驟 2），舊版重送同鍵仍須走冪等回原結果
        let mut b = body();
        b.instrument_version = "0".into();
        assert!(valid_shape(&b));
    }

    #[test]
    fn idempotent_resend_matches_ignoring_epoch() {
        let mut b = body();
        b.expected_epoch = 7;
        assert!(matches_result(&row(&[3; 48]), &b));
    }

    #[test]
    fn idempotent_resend_with_different_answers_conflicts() {
        let mut answers = vec![3u8; 48];
        answers[0] = 4;
        assert!(!matches_result(&row(&answers), &body()));
    }

    #[test]
    fn idempotent_resend_with_different_version_conflicts() {
        let mut b = body();
        b.instrument_version = "0".into();
        assert!(!matches_result(&row(&[3; 48]), &b));
    }

    #[test]
    fn new_key_with_current_state_passes() {
        assert_eq!(new_key_conflict(&body(), 0), None);
    }

    #[test]
    fn new_key_with_stale_version_is_stale_instrument() {
        let mut b = body();
        b.instrument_version = "0".into();
        // 版本先於 epoch：兩者皆不符時回 STALE_INSTRUMENT
        assert_eq!(new_key_conflict(&b, 3), Some("STALE_INSTRUMENT"));
    }

    #[test]
    fn resend_after_delete_is_stale_state() {
        // 刪除後舊鍵已不存在 → 走新鍵檢查；epoch 已 +1
        assert_eq!(new_key_conflict(&body(), 1), Some("STALE_INTEREST_STATE"));
    }

    fn row_at(id: &str, created_at: &str) -> ResultRow {
        let mut r = row(&[3; 48]);
        r.id = id.into();
        r.created_at = created_at.into();
        r
    }

    #[test]
    fn reread_after_committed_transport_error_replays() {
        let existing = row(&[3; 48]);
        assert_eq!(
            reread_outcome(Ok(Some(&existing)), &body()),
            Some(Outcome::Replay)
        );
    }

    #[test]
    fn reread_concurrent_loser_with_other_content_conflicts() {
        let mut answers = vec![3u8; 48];
        answers[47] = 1;
        let existing = row(&answers);
        assert_eq!(
            reread_outcome(Ok(Some(&existing)), &body()),
            Some(Outcome::Conflict("IDEMPOTENCY_CONFLICT"))
        );
    }

    #[test]
    fn reread_failure_is_db_error() {
        assert_eq!(reread_outcome(Err(()), &body()), Some(Outcome::DbError));
    }

    #[test]
    fn reread_without_row_defers_to_epoch() {
        assert_eq!(reread_outcome(Ok(None), &body()), None);
    }

    #[test]
    fn failed_write_with_changed_epoch_is_stale_state() {
        // 涵蓋 DELETE 競態下條件 INSERT 回 Ok(0)，以及非 UNIQUE 錯誤＋epoch 已變
        assert_eq!(
            stale_outcome(Ok(1), 0),
            Outcome::Conflict("STALE_INTEREST_STATE")
        );
    }

    #[test]
    fn failed_write_with_same_epoch_is_db_error() {
        assert_eq!(stale_outcome(Ok(0), 0), Outcome::DbError);
    }

    #[test]
    fn failed_write_with_unreadable_epoch_is_db_error() {
        assert_eq!(stale_outcome(Err(()), 0), Outcome::DbError);
    }

    #[test]
    fn next_cursor_only_when_extra_row_fetched() {
        let rows = vec![
            row_at("c", "2026-10-30T00:00:03.000Z"),
            row_at("b", "2026-10-30T00:00:02.000Z"),
            row_at("a", "2026-10-30T00:00:01.000Z"),
        ];
        assert_eq!(
            next_cursor(&rows, 2),
            Some("2026-10-30T00:00:02.000Z|b".to_string())
        );
        assert_eq!(next_cursor(&rows[..2], 2), None);
        assert_eq!(next_cursor(&[], 20), None);
    }
}
