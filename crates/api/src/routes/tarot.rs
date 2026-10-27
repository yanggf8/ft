//! 塔羅 v1b — 三張牌陣（登入）。設計定稿：docs/ARCHITECTURE.md §Tarot。
//!
//! 處理順序（定稿，比照 practice／predictions 全站先例——IP 限流最先）：
//! IP 限流 → auth → body cap → parse → 驗證 → 冪等查詢（在 uid 限流前，
//! 重放不受 429 擋）→ uid 24h 限流 → 伺服端抽牌（`secure_bytes` fail-closed，
//! 明文禁 `Math::random`）→ 寫入 → 201。
//! 紅線：塔羅資料永不寫入 predictions／prediction_feedback／f8_assignments。

use super::super::{
    error,
    services::{clock, db, uuid},
};
use super::common::{
    apply_cache_headers, auth_user, body_too_large, client_ip, ok_json, rate_limit,
};
use super::practice::page_query;
use super::R;
use ft_schema::tarot::{
    assemble_interpretation, shuffle_three, valid_tarot_request_id, validate_question,
    QuestionTooLong, TarotCardView, TarotDrawRequest, TarotHistoryPage, TarotReadingView,
    MIN_ENTROPY_BYTES, TAROT_CORPUS_VERSION,
};
use serde::{Deserialize, Serialize};
use worker::*;

/// 每分鐘每 IP 20 次（防腳本灌端點——比照 practice RATE_LIMIT 檔次）。
const IP_LIMIT: u32 = 20;
const IP_WINDOW_MS: f64 = 60_000.0;
/// 每用戶 24 小時固定視窗 20 次（濫用緩衝，非曆日、fail-open 非硬上限——
/// RateLimitDO 語意，設計定稿明文取捨）。
const USER_LIMIT: u32 = 20;
const USER_WINDOW_MS: f64 = 86_400_000.0;

const POSITIONS: [&str; 3] = ["situation", "obstacle", "advice"];

#[derive(Deserialize)]
struct ReadingRow {
    id: String,
    question: Option<String>,
    spread: String,
    cards: String, // JSON 字串（wire 形狀 [TarotCardView]，站上 TEXT 存 JSON 慣例）
    interpretation: String,
    corpus_version: String,
    created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Deleted {
    deleted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeletedCount {
    deleted: i64,
}

fn json<T: Serialize>(v: &T, status: u16) -> Response {
    let mut res = ok_json(
        &serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
        status,
    );
    apply_cache_headers(&mut res, 0, true);
    let _ = res.headers_mut().set("Cache-Control", "private, no-store");
    res
}

fn db_error() -> Response {
    error::error_code("db error", "DB_ERROR", 500)
}

fn conflict(code: &str) -> Response {
    error::error_code("Tarot state changed", code, 409)
}

fn unique_error(e: &Error) -> bool {
    e.to_string().contains("UNIQUE constraint failed")
}

fn wire_cards(drawn: &[ft_schema::tarot::DrawnCard]) -> Vec<TarotCardView> {
    drawn
        .iter()
        .enumerate()
        .map(|(i, d)| TarotCardView {
            id: d.card.id.to_string(),
            name: d.card.name.to_string(),
            reversed: d.reversed,
            position: POSITIONS[i].to_string(),
        })
        .collect()
}

fn row_view(row: &ReadingRow) -> Result<TarotReadingView> {
    let cards: Vec<TarotCardView> = serde_json::from_str(&row.cards)?;
    Ok(TarotReadingView {
        id: row.id.clone(),
        question: row.question.clone(),
        spread: row.spread.clone(),
        cards,
        interpretation: row.interpretation.clone(),
        corpus_version: row.corpus_version.clone(),
        created_at: row.created_at.clone(),
    })
}

async fn reading_by_request(
    db: &db::Turso,
    user: &str,
    request_id: &str,
) -> Result<Option<ReadingRow>> {
    db::first(
        db,
        "SELECT * FROM tarot_readings WHERE user_id=?1 AND request_id=?2 LIMIT 1",
        &[&db::text(user), &db::text(request_id)],
    )
    .await
}

/// 冪等比對：spread＋正規化後 question（設計定稿）。
fn matches_reading(row: &ReadingRow, spread: &str, question: Option<&str>) -> bool {
    row.spread == spread && row.question.as_deref() == question
}

pub fn register(router: R<'static>) -> R<'static> {
    router
    .post_async("/api/tarot/draw", |mut req, ctx| async move {
      if !rate_limit(&ctx, &format!("tarot:ip:{}", client_ip(&req)), IP_LIMIT, IP_WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      if body_too_large(&req) { return Ok(error::error_code("payload too large", "PAYLOAD_TOO_LARGE", 413)) }
      let body: TarotDrawRequest = match req.json().await { Ok(v) => v, Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)) };
      // spec：trim 後空＝未填存 NULL；僅 >200 字回 400
      let question = match validate_question(body.question.as_deref()) {
        Ok(q) => q,
        Err(QuestionTooLong) => {
            return Ok(error::error_code("Invalid question", "INVALID_TAROT", 400))
        }
      };
      if body.spread != "three" || !valid_tarot_request_id(&body.request_id) {
        return Ok(error::error_code("Invalid tarot draw", "INVALID_TAROT", 400));
      }
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      // 冪等查詢（在 uid 限流之前——重放回原結果，不受 429 擋）
      match reading_by_request(&db, &user, &body.request_id).await {
        Ok(Some(existing)) => return Ok(if matches_reading(&existing, &body.spread, question.as_deref()) {
          match row_view(&existing) { Ok(v) => json(&v, 200), Err(_) => db_error() }
        } else { conflict("IDEMPOTENCY_CONFLICT") }),
        Ok(None) => {}, Err(_) => return Ok(db_error())
      }
      if !rate_limit(&ctx, &format!("tarot:draw:{}", user), USER_LIMIT, USER_WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      // 伺服端抽牌：CSPRNG fail-closed（crypto 不可用即整單 500，絕不降級）
      let entropy = match uuid::secure_bytes(MIN_ENTROPY_BYTES) {
        Some(e) => e,
        None => return Ok(error::error_code("Internal error", "INTERNAL", 500)),
      };
      let drawn = match shuffle_three(&entropy) {
        Some(d) => d,
        None => return Ok(error::error_code("Internal error", "INTERNAL", 500)),
      };
      let interpretation = match assemble_interpretation(&drawn) {
        Some(i) => i,
        None => return Ok(error::error_code("Internal error", "INTERNAL", 500)),
      };
      let cards = wire_cards(&drawn);
      let cards_json = match serde_json::to_string(&cards) {
        Ok(s) => s,
        Err(_) => return Ok(error::error_code("Internal error", "INTERNAL", 500)),
      };
      let id = uuid::random_uuid();
      let now = clock::now_iso();
      let n = db::exec_changes(&db,
        "INSERT INTO tarot_readings (id,user_id,request_id,question,spread,cards,interpretation,corpus_version,created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        &[&db::text(&id),&db::text(&user),&db::text(&body.request_id),
          &db::opt_text(question.as_deref()),&db::text(&body.spread),&db::text(&cards_json),
          &db::text(&interpretation),&db::text(TAROT_CORPUS_VERSION),&db::text(&now)]).await;
      match n {
        Ok(1) => Ok(json(&TarotReadingView {
          id, question, spread: body.spread, cards, interpretation, corpus_version: TAROT_CORPUS_VERSION.into(), created_at: now
        }, 201)),
        failed => {
          // UNIQUE 衝突或傳輸錯誤後的回查（同 practice 先例：先讀冪等鍵再分類）
          match reading_by_request(&db, &user, &body.request_id).await {
            Ok(Some(existing)) => return Ok(if matches_reading(&existing, &body.spread, question.as_deref()) {
              match row_view(&existing) { Ok(v) => json(&v, 200), Err(_) => db_error() }
            } else { conflict("IDEMPOTENCY_CONFLICT") }),
            Err(_) => return Ok(db_error()), Ok(None) => {}
          }
          if let Err(e) = &failed { if !unique_error(e) { return Ok(db_error()) } }
          Ok(db_error())
        }
      }
    })
    .get_async("/api/tarot/readings", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let url = match req.url() { Ok(u) => u, Err(_) => return Ok(error::error_code("Invalid URL", "INVALID_PAGE", 400)) };
      let (limit, cursor) = match page_query(url.query()) { Ok(p) => p, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let rows: Vec<ReadingRow> = match db::all(&db,
        "SELECT * FROM tarot_readings WHERE user_id=?1 AND (?2 IS NULL OR (created_at,id)<(?2,?3)) \
         ORDER BY created_at DESC, id DESC LIMIT ?4",
        &[&db::text(&user),&db::opt_text(cursor.as_ref().map(|x| x.0.as_str())),
          &db::opt_text(cursor.as_ref().map(|x| x.1.as_str())),&db::int((limit+1) as i32)]).await {
          Ok(r) => r, Err(_) => return Ok(db_error())
        };
      let next_cursor = if rows.len() > limit {
        rows.get(limit-1).map(|r| format!("{}|{}", r.created_at, r.id))
      } else { None };
      let mut items = Vec::with_capacity(rows.len().min(limit));
      for r in rows.iter().take(limit) {
        match row_view(r) { Ok(v) => items.push(v), Err(_) => return Ok(db_error()) }
      }
      Ok(json(&TarotHistoryPage { items, next_cursor }, 200))
    })
    .delete_async("/api/tarot/readings/:id", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let id = ctx.param("id").cloned().unwrap_or_default();
      if !valid_tarot_request_id(&id) { return Ok(error::error_code("Invalid reading", "INVALID_TAROT", 400)) }
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      // 不存在／他人／重刪一律 404（不洩漏存在性）
      let n = match db::exec_changes(&db,
        "DELETE FROM tarot_readings WHERE id=?1 AND user_id=?2",
        &[&db::text(&id),&db::text(&user)]).await {
          Ok(n) => n, Err(_) => return Ok(db_error())
        };
      if n == 1 { Ok(json(&Deleted { deleted: true }, 200)) }
      else { Ok(error::error_code("Reading not found", "NOT_FOUND", 404)) }
    })
    .delete_async("/api/tarot/readings", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let n = match db::exec_changes(&db,
        "DELETE FROM tarot_readings WHERE user_id=?1",
        &[&db::text(&user)]).await {
          Ok(n) => n, Err(_) => return Ok(db_error())
        };
      Ok(json(&DeletedCount { deleted: n as i64 }, 200))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(question: Option<&str>) -> ReadingRow {
        ReadingRow {
            id: "x".repeat(36),
            question: question.map(|s| s.to_string()),
            spread: "three".into(),
            cards: "[]".into(),
            interpretation: "i".into(),
            corpus_version: "1".into(),
            created_at: "2026-10-27T00:00:00.000Z".into(),
        }
    }

    #[test]
    fn idempotency_matches_spread_and_normalized_question() {
        let r = row(Some("我想問事業"));
        assert!(matches_reading(&r, "three", Some("我想問事業")));
        assert!(
            !matches_reading(&r, "three", Some("我想問感情")),
            "question 不同 → 409"
        );
        assert!(!matches_reading(&r, "single", None), "spread 不同 → 409");
        assert!(matches_reading(&row(None), "three", None), "皆空 → 重放");
    }

    #[test]
    fn cards_json_roundtrip() {
        let entropy = [7u8; 32];
        let drawn = ft_schema::tarot::shuffle_three(&entropy).unwrap();
        let cards = wire_cards(&drawn);
        let s = serde_json::to_string(&cards).unwrap();
        let parsed: Vec<TarotCardView> = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].id, "swords_08"); // golden 對齊 schema 測試
        assert_eq!(parsed[0].position, "situation");
    }
}
