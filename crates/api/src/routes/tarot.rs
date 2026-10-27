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
    rate_limit_strict,
};
use super::practice::page_query;
use super::R;
use ft_schema::tarot::{
    assemble_interpretation, card_by_id, shuffle_three, valid_tarot_request_id, validate_question,
    QuestionTooLong, TarotCardView, TarotDrawRequest, TarotHistoryPage, TarotReadingView,
    MIN_ENTROPY_BYTES, TAROT_CORPUS_VERSION, TAROT_PROMPT_VERSION,
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

/// v2 AI 限流（spec §Tarot v2-13）：皆 24h 固定視窗、`rate_limit_strict`
/// （fail-closed）；計次邊界＝進入 AI 呼叫即計（含 queue-full 與失敗）。
const AI_USER_LIMIT: u32 = 3;
const AI_IP_LIMIT: u32 = 10;
const AI_WINDOW_MS: f64 = 86_400_000.0;

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
    ai_interpretation: Option<String>,
    ai_status: Option<String>,
    ai_provider: Option<String>,
    ai_model: Option<String>,
    ai_chart_context: Option<i64>,
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
        ai_interpretation: row.ai_interpretation.clone(),
        ai_status: row.ai_status.clone(),
        ai_provider: row.ai_provider.clone(),
        ai_model: row.ai_model.clone(),
        ai_chart_context: row.ai_chart_context.map(|v| v != 0),
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

async fn reading_by_id(db: &db::Turso, user: &str, id: &str) -> Result<Option<ReadingRow>> {
    db::first(
        db,
        "SELECT * FROM tarot_readings WHERE id=?1 AND user_id=?2 LIMIT 1",
        &[&db::text(id), &db::text(user)],
    )
    .await
}

/// v2 AI 成功結果（DO 回應解析後）。
struct AiOk {
    interpretation: String,
    provider: String,
    model: String,
}

/// 呼 AIMutexDO **專屬實例**（`id_from_name("tarot")`，spec §Tarot v2-4）。
/// 任何失敗（DO 不可用／503 queue-full／非 200／**`stub:true`**——spec v2-11
/// 一律視為 failed／空解讀）回 `Err`；呼叫端記 `ai_status=failed`。
async fn call_tarot_ai(ctx: &RouteContext<()>, chart_data: &serde_json::Value) -> Result<AiOk, ()> {
    let ns = ctx.env.durable_object("AI_MUTEX").map_err(|_| ())?;
    let stub = ns
        .id_from_name("tarot")
        .and_then(|id| id.get_stub())
        .map_err(|_| ())?;
    let mut keys_map = serde_json::Map::new();
    for (env_key, name) in [
        ("IFLOW_API_KEY", "iflow"),
        ("GROQ_API_KEY", "groq"),
        ("CEREBRAS_API_KEY", "cerebras"),
    ] {
        if let Ok(v) = ctx.env.secret(env_key) {
            let v = v.to_string();
            if !v.is_empty() {
                keys_map.insert(name.to_string(), serde_json::Value::String(v));
            }
        }
    }
    let body = serde_json::json!({
        "keys": serde_json::Value::Object(keys_map),
        "interpretRequest": {
            "chartType": "tarot",
            "chartData": chart_data,
            "language": "zh",
        },
    });
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(body.to_string().into()));
    let req = Request::new_with_init("https://ai-mutex/interpret", &init).map_err(|_| ())?;
    let mut res = stub.fetch_with_request(req).await.map_err(|_| ())?;
    if res.status_code() != 200 {
        // 503 = queue full / 全 provider 失敗；其他非 200 同樣視為失敗
        return Err(());
    }
    let data: serde_json::Value = res.json().await.map_err(|_| ())?;
    if data.get("stub").and_then(|v| v.as_bool()).unwrap_or(false) {
        // spec v2-11：stub 文案不得入庫——一律視為 failed
        return Err(());
    }
    let interpretation = data
        .get("interpretation")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if interpretation.trim().is_empty() {
        return Err(());
    }
    Ok(AiOk {
        interpretation,
        provider: data
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        model: data
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

#[derive(Deserialize)]
struct StrengthRow {
    work: i32,
    love: i32,
    family: i32,
    money: i32,
    health: i32,
}

#[derive(Deserialize)]
struct ChartDataRow {
    chart_data: String,
}

/// 組 AI 用 chartData（spec §Tarot v2-5/6/8/9/10）：牌陣＋question＋
/// chartSummary（命盤摘要：紫微最新命宮主星／五行局＋F4 **當週**自評強度）。
/// 資料最小化：不含出生時地；無命盤／無當週強度 → 缺項跳過（降級矩陣）。
/// 回傳 (chartData, 是否使用命盤 context)——後者存 `ai_chart_context`
/// 供前端文案區分（Muse 前審 S2／契約 #9）。
async fn build_tarot_chart_data(
    db: &db::Turso,
    user: &str,
    row: &ReadingRow,
    include_chart: bool,
) -> Result<(serde_json::Value, bool)> {
    let cards: Vec<TarotCardView> = serde_json::from_str(&row.cards)?;
    let cards_json: Vec<serde_json::Value> = cards
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "name": c.name,
                "reversed": c.reversed,
                "position": c.position,
                "meaning": card_by_id(&c.id).map(|t| if c.reversed { t.reversed } else { t.upright }),
            })
        })
        .collect();

    let mut parts: Vec<String> = Vec::new();
    // 紫微命盤摘要（最新 ziwei chart；主星＋亮度）。契約 #9：查詢失敗降級為
    // 無摘要續跑（純塔羅仍可解），不讓摘要故障炸掉整支端點。
    let chart: Option<ChartDataRow> = if include_chart {
        db::first(
            db,
            "SELECT chart_data FROM interpretations WHERE user_id=?1 AND divination_type='ziwei' ORDER BY updated_at DESC LIMIT 1",
            &[&db::text(user)],
        )
        .await
        .unwrap_or(None)
    } else {
        None
    };
    if let Some(chart) = chart {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&chart.chart_data) {
            let five = v
                .pointer("/fiveElement")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            // isLifePalace 定位（較 name==命宮 穩）；只取主星並附亮度
            let ming_stars = v
                .pointer("/palaces")
                .and_then(|x| x.as_array())
                .and_then(|arr| {
                    arr.iter().find(|p| {
                        p.get("isLifePalace")
                            .and_then(|b| b.as_bool())
                            .unwrap_or(false)
                    })
                })
                .or_else(|| {
                    v.pointer("/palaces")
                        .and_then(|x| x.as_array())
                        .and_then(|arr| {
                            arr.iter()
                                .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("命宮"))
                        })
                })
                .and_then(|p| p.get("stars").and_then(|x| x.as_array()))
                .map(|stars| {
                    stars
                        .iter()
                        .filter(|st| st.get("type").and_then(|t| t.as_str()) == Some("main"))
                        .filter_map(|st| {
                            let name = st.get("name").and_then(|n| n.as_str())?;
                            let bright = st.get("brightness").and_then(|b| b.as_str());
                            Some(match bright {
                                Some(b) => format!("{name}（{b}）"),
                                None => name.to_string(),
                            })
                        })
                        .collect::<Vec<_>>()
                        .join("、")
                });
            match ming_stars {
                Some(stars) if !stars.is_empty() => {
                    parts.push(format!("命宮主星：{stars}"));
                }
                _ => parts.push("命宮無主星（借對宮）".to_string()),
            }
            if !five.is_empty() {
                parts.push(format!("五行局：{five}"));
            }
        }
    }
    // F4 當週自評強度（僅當週 cycle_id；無當週列＝無此 context）。純塔羅
    // （include_chart=false）一併跳過——命盤 context 定義含 F4 五域（契約
    // #8），勾「純塔羅」不得外送任何命盤衍生資料（Muse 複審 B1）。
    let now = clock::now_iso();
    if include_chart {
        if let Some(cycle) = ft_schema::cycle::week_start_asia_taipei(&now) {
            let s: Option<StrengthRow> = db::first(
            db,
            "SELECT work, love, family, money, health FROM prediction_strengths WHERE user_id=?1 AND cycle_id=?2",
            &[&db::text(user), &db::text(&cycle)],
        )
        .await
        .unwrap_or(None);
            if let Some(s) = s {
                parts.push(format!(
                    "本週自評強度（0–3）：工作{}、金錢{}、感情{}、家庭{}、健康{}",
                    s.work, s.money, s.love, s.family, s.health
                ));
            }
        }
    }

    let summary = if parts.is_empty() {
        None
    } else {
        Some(parts.join("；"))
    };
    let used = summary.is_some();
    Ok((
        serde_json::json!({
            "cards": cards_json,
            "question": row.question,
            "chartSummary": summary,
        }),
        used,
    ))
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
          id, question, spread: body.spread, cards, interpretation, corpus_version: TAROT_CORPUS_VERSION.into(), created_at: now,
          ai_interpretation: None, ai_status: None, ai_provider: None, ai_model: None,
          ai_chart_context: None,
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
    .post_async("/api/tarot/readings/:id/interpret", |req, ctx| async move {
      // v2 AI 整合解牌（spec §Tarot v2）：auth → 讀取（done 冪等回原結果，
      // 不計次——401/404/重放皆不消耗額度）→ IP strict → uid strict（兩者皆
      // 「進入 AI 呼叫即計」邊界之後才計）→ AIMutex 專屬實例 → stub 偵測 →
      // 更新 ai_* 欄位（規則式永不覆寫）。
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let id = ctx.param("id").cloned().unwrap_or_default();
      if !valid_tarot_request_id(&id) { return Ok(error::error_code("Invalid reading", "INVALID_TAROT", 400)) }
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let row = match reading_by_id(&db, &user, &id).await {
        Ok(Some(r)) => r,
        Ok(None) => return Ok(error::error_code("Reading not found", "NOT_FOUND", 404)),
        Err(_) => return Ok(db_error()),
      };
      if row.ai_status.as_deref() == Some("done") {
        return Ok(match row_view(&row) { Ok(v) => json(&v, 200), Err(_) => db_error() });
      }
      if !rate_limit_strict(&ctx, &format!("tarot:ai:ip:{}", client_ip(&req)), AI_IP_LIMIT, AI_WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      if !rate_limit_strict(&ctx, &format!("tarot:ai:{}", user), AI_USER_LIMIT, AI_WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      // include_chart=0 → 純塔羅（契約 #7：命盤感知為可控制選項）
      let include_chart = req
        .url()
        .ok()
        .and_then(|u| u.query().map(|q| q.to_string()))
        .map(|q| !q.split('&').any(|kv| kv == "chart=0"))
        .unwrap_or(true);
      let (chart_data, chart_used) =
        match build_tarot_chart_data(&db, &user, &row, include_chart).await {
          Ok(v) => v,
          Err(_) => return Ok(db_error()),
        };
      let outcome = call_tarot_ai(&ctx, &chart_data).await;
      let now = clock::now_iso();
      let (ai_text, status, provider, model) = match outcome {
        Ok(o) => (Some(o.interpretation), "done", Some(o.provider), Some(o.model)),
        Err(()) => (None, "failed", None, None),
      };
      // S8 伺服端防護（first-wins）：另一分頁已完成（done）則不覆寫，敗者
      // 回查回贏者結果。兩邊都進了 AI 呼叫＝都計次（同步無 pending 的取捨）。
      let n = db::exec_changes(
        &db,
        "UPDATE tarot_readings SET ai_interpretation=?1, ai_status=?2, ai_prompt_version=?3, ai_provider=?4, ai_model=?5, ai_generated_at=?6, ai_chart_context=?9          WHERE id=?7 AND user_id=?8 AND (ai_status IS NULL OR ai_status != 'done')",
        &[
          &db::opt_text(ai_text.as_deref()),
          &db::text(status),
          &db::text(TAROT_PROMPT_VERSION),
          &db::opt_text(provider.as_deref()),
          &db::opt_text(model.as_deref()),
          &db::text(&now),
          &db::text(&id),
          &db::text(&user),
          &db::int(i32::from(chart_used)),
        ],
      )
      .await;
      match n {
        Ok(_) => match reading_by_id(&db, &user, &id).await {
          Ok(Some(updated)) => match row_view(&updated) { Ok(v) => Ok(json(&v, 200)), Err(_) => Ok(db_error()) },
          _ => Ok(db_error()),
        },
        Err(_) => Ok(db_error()),
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
            ai_interpretation: None,
            ai_status: None,
            ai_provider: None,
            ai_model: None,
            ai_chart_context: None,
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
