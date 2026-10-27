//! 手相 v1 — 四端點（spec P1；設計：docs/superpowers/specs/2026-10-27-palm-v1-design.md）。
//!
//! analyze 處理順序（P2）：cheap IP fail-open → auth → 512KiB 專用 cap →
//! parse/驗證 → image_sha256 → 冪等查詢（strict 限流**前**，重放不受 429 擋）→
//! IP strict → uid strict（refundable 票根）→ AIMutexDO palm 專屬實例 →
//! fence 剝除＋strict parse → 品質閘（422 走 P8a 退款閉環）→ 規則式解讀快照落庫。
//! 紅線：手相資料永不寫入 predictions/prediction_feedback/f8_assignments；
//! 照片與 image_sha256 指紋不進 wire 不進 log（P5/P12）。

use super::super::{
    error,
    services::{clock, db, uuid},
};
use super::common::{
    apply_cache_headers, auth_user, body_too_large_with, client_ip, ok_json, rate_limit,
    rate_limit_strict, rate_limit_strict_refundable, refund_strict,
};
use super::practice::page_query;
use super::R;
use ft_schema::palm::wire::{
    gate_min_confidence, strip_json_fence, valid_hand, valid_image_b64, valid_mime,
    valid_palm_request_id, PalmAnalyzeRequest, PalmHistoryPage, PalmReadingView,
};
use ft_schema::palm::{
    assemble_palm_interpretation, passes_gate, PalmExtraction, PalmFeatures, PALM_CORPUS_VERSION,
    PALM_FEATURES_VERSION,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use worker::*;

/// cheap fail-open：每分鐘每 IP 20（P2/P20——防腳本灌端點，非成本上限）。
const CHEAP_IP_LIMIT: u32 = 20;
const CHEAP_IP_WINDOW_MS: f64 = 60_000.0;
/// AI 成本路徑（P20：皆 24h 固定視窗、fail-closed strict）：IP 15、uid 3、refund 3。
const AI_IP_LIMIT: u32 = 15;
const AI_USER_LIMIT: u32 = 3;
const REFUND_LIMIT: u32 = 3;
const AI_WINDOW_MS: f64 = 86_400_000.0;
/// P3：palm 專用 body cap（512KiB；單一實質約束另在 imageBase64 682,667 字）。
const PALM_MAX_BODY_BYTES: u64 = 512 * 1024;

#[derive(Deserialize)]
struct ReadingRow {
    id: String,
    hand: String,
    image_sha256: String,
    features_json: String,
    interpretation: String,
    corpus_version: String,
    created_at: String,
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
    error::error_code("Palm state changed", code, 409)
}

fn unique_error(e: &Error) -> bool {
    e.to_string().contains("UNIQUE constraint failed")
}

/// P12：sha256(base64 字串) hex——冪等比對指紋（RC1：不對解碼後 bytes）。
/// 不進 wire 不進 log。
fn image_sha256(b64: &str) -> String {
    let digest = Sha256::digest(b64.as_bytes());
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// DO 失敗結局分類（P8a 計數表的鍵）：決定 refund 路徑。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiFailure {
    /// 佇列滿/逾時（未進 provider）——退，**不占** refund 額度（基礎設施非使用者責任）。
    Queue,
    /// provider 全敗（含 DO rpm 觸頂，見批次B log 註記）——不退。
    AllFailed,
    /// finish_reason=length 截斷（DO 判定回 500）——已花不退。
    Truncated,
    /// 無法歸類（DO 不可達/未知碼/空回應）——保守不退。
    Unknown,
}

fn classify_ai_failure(status: u16, code: &str) -> AiFailure {
    match (status, code) {
        (503, "AI_QUEUE_FULL") | (503, "AI_QUEUE_TIMEOUT") => AiFailure::Queue,
        (503, "AI_ALL_FAILED") => AiFailure::AllFailed,
        (500, "AI_TRUNCATED") => AiFailure::Truncated,
        _ => AiFailure::Unknown,
    }
}

/// P8a：只有 Queue 結局「退款且不占 refund bucket」（品質閘 422 的退款另在
/// route 內占 `palm:refund:{uid}` 額度，不經此函式）。
fn refunds_without_bucket(f: AiFailure) -> bool {
    f == AiFailure::Queue
}

/// P8a：outcome code 轉發封閉集（Muse 審 Important：集合不封閉會讓未來 DO
/// 內部碼洩漏給前端）——僅四個契約碼原樣轉發（前端重試語意有別），其餘
/// （未知碼/空碼/狀態不符）一律收斂 503 AI_UNAVAILABLE。
fn forward_ai_error(status: u16, code: &str) -> (u16, &'static str) {
    match (status, code) {
        (503, "AI_QUEUE_FULL") => (503, "AI_QUEUE_FULL"),
        (503, "AI_QUEUE_TIMEOUT") => (503, "AI_QUEUE_TIMEOUT"),
        (503, "AI_ALL_FAILED") => (503, "AI_ALL_FAILED"),
        (500, "AI_TRUNCATED") => (500, "AI_TRUNCATED"),
        _ => (503, "AI_UNAVAILABLE"),
    }
}

/// 冪等比對（P13）：hand＋image_sha256（同 requestId 異 body → 409）。
fn matches_reading(row: &ReadingRow, hand: &str, sha: &str) -> bool {
    row.hand == hand && row.image_sha256 == sha
}

/// DO 請求體（批次B 契約）：**不帶 keys**（Workers AI binding 在 DO 端）、
/// chartData 只有 hand、image 內嵌 mime/base64。
fn build_interpret_body(req: &PalmAnalyzeRequest) -> serde_json::Value {
    serde_json::json!({
        "interpretRequest": {
            "chartType": "palm",
            "chartData": { "hand": req.hand },
            "image": { "mime": req.mime_type, "base64": req.image_base64 },
            "language": "zh",
        },
    })
}

struct AiOk {
    interpretation: String,
    model: String,
}

/// 呼 AIMutexDO **palm 專屬實例**（P9：`id_from_name("palm")`）。失敗回
/// `(status, code)` 供 [`classify_ai_failure`] 分類；DO 錯誤細節不透傳前端（P5）。
async fn call_palm_ai(
    ctx: &RouteContext<()>,
    req: &PalmAnalyzeRequest,
) -> Result<AiOk, (u16, String)> {
    let ns = ctx
        .env
        .durable_object("AI_MUTEX")
        .map_err(|_| (503, String::new()))?;
    let stub = ns
        .id_from_name("palm")
        .and_then(|id| id.get_stub())
        .map_err(|_| (503, String::new()))?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(build_interpret_body(req).to_string().into()));
    let request = Request::new_with_init("https://ai-mutex/interpret", &init)
        .map_err(|_| (503, String::new()))?;
    let mut res = stub
        .fetch_with_request(request)
        .await
        .map_err(|_| (503, String::new()))?;
    let status = res.status_code();
    if status != 200 {
        let code = res
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v.get("code").and_then(|c| c.as_str()).map(String::from))
            .unwrap_or_default();
        return Err((status, code));
    }
    let data: serde_json::Value = res.json().await.map_err(|_| (503, String::new()))?;
    let interpretation = data
        .get("interpretation")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if interpretation.trim().is_empty() {
        return Err((503, String::new()));
    }
    Ok(AiOk {
        interpretation,
        model: data
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

fn row_view(row: &ReadingRow) -> Result<PalmReadingView> {
    let features: PalmFeatures = serde_json::from_str(&row.features_json)?;
    Ok(PalmReadingView {
        id: row.id.clone(),
        hand: row.hand.clone(),
        features,
        interpretation: row.interpretation.clone(),
        corpus_version: row.corpus_version.clone(),
        created_at: row.created_at.clone(),
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
        "SELECT * FROM palm_readings WHERE user_id=?1 AND request_id=?2 LIMIT 1",
        &[&db::text(user), &db::text(request_id)],
    )
    .await
}

pub fn register(router: R<'static>) -> R<'static> {
    router
    .post_async("/api/palm/analyze", |mut req, ctx| async move {
      // P2-1：cheap fail-open（20/min/IP）
      if !rate_limit(&ctx, &format!("palm:cheap-ip:{}", client_ip(&req)), CHEAP_IP_LIMIT, CHEAP_IP_WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      // P2-3／P3：512KiB 專用 cap
      if body_too_large_with(&req, PALM_MAX_BODY_BYTES) { return Ok(error::error_code("payload too large", "PAYLOAD_TOO_LARGE", 413)) }
      let body: PalmAnalyzeRequest = match req.json().await { Ok(v) => v, Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)) };
      if !valid_palm_request_id(&body.request_id) || !valid_hand(&body.hand) {
        return Ok(error::error_code("Invalid palm request", "INVALID_PALM", 400));
      }
      if !valid_mime(&body.mime_type) || !valid_image_b64(&body.image_base64) {
        return Ok(error::error_code("Invalid image", "INVALID_IMAGE", 400));
      }
      // P2-5：指紋（P12 措辭：不可重建影像，不進 wire/log）→ 冪等查詢（strict 前）
      let sha = image_sha256(&body.image_base64);
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      match reading_by_request(&db, &user, &body.request_id).await {
        Ok(Some(existing)) => return Ok(if matches_reading(&existing, &body.hand, &sha) {
          match row_view(&existing) { Ok(v) => json(&v, 200), Err(_) => db_error() }
        } else { conflict("IDEMPOTENCY_CONFLICT") }),
        Ok(None) => {}, Err(_) => return Ok(db_error())
      }
      // P2-7/8：strict fail-closed（IP 15/24h → uid 3/24h refundable）
      if !rate_limit_strict(&ctx, &format!("palm:ai:ip:{}", client_ip(&req)), AI_IP_LIMIT, AI_WINDOW_MS).await {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      }
      let Some(reset_at) = rate_limit_strict_refundable(&ctx, &format!("palm:ai:{}", user), AI_USER_LIMIT, AI_WINDOW_MS).await else {
        return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
      };
      // P2-9：VLM（DO palm 專屬實例；失敗依 P8a 分類決定退款）
      let ai = match call_palm_ai(&ctx, &body).await {
        Ok(a) => a,
        Err((status, code)) => {
          if refunds_without_bucket(classify_ai_failure(status, &code)) {
            refund_strict(&ctx, &format!("palm:ai:{}", user), reset_at).await;
          }
          // P8a：outcome code 屬 wire 契約（無 provider 細節，P5 安全）——
          // 封閉集白名單轉發（前端可區分截斷/佇列/全敗的重試語意），其餘收斂。
          let (fwd_status, fwd_code) = forward_ai_error(status, &code);
          let msg = if fwd_code == "AI_UNAVAILABLE" {
            "AI service temporarily unavailable, please try again"
          } else {
            "AI service error"
          };
          return Ok(error::error_code(msg, fwd_code, fwd_status));
        }
      };
      // P8b：剝圍籬 → strict serde（deny_unknown_fields 擋幻覺欄位）
      let ext: PalmExtraction = match serde_json::from_str(strip_json_fence(&ai.interpretation)) {
        Ok(v) => v,
        Err(_) => return Ok(error::error_code("AI parse failed", "AI_PARSE_FAILED", 500)),
      };
      // P8 品質閘：未過 → 422＋P8a 退款閉環（占 palm:refund:{uid} 額度才退）
      if !passes_gate(&ext, gate_min_confidence(&ai.model)) {
        if rate_limit(&ctx, &format!("palm:refund:{}", user), REFUND_LIMIT, AI_WINDOW_MS).await {
          refund_strict(&ctx, &format!("palm:ai:{}", user), reset_at).await;
        }
        return Ok(error::error_code("Photo quality too low, please retake", "PALM_QUALITY_LOW", 422));
      }
      // §3.3：規則式解讀快照（照片不落庫，只有特徵與解讀）
      let interpretation = assemble_palm_interpretation(&ext.features);
      let features_json = match serde_json::to_string(&ext.features) {
        Ok(s) => s,
        Err(_) => return Ok(error::error_code("Internal error", "INTERNAL", 500)),
      };
      let id = uuid::random_uuid();
      let now = clock::now_iso();
      let n = db::exec_changes(&db,
        "INSERT INTO palm_readings (id,user_id,request_id,hand,image_sha256,features_json,interpretation,corpus_version,features_version,created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        &[&db::text(&id),&db::text(&user),&db::text(&body.request_id),&db::text(&body.hand),
          &db::text(&sha),&db::text(&features_json),&db::text(&interpretation),
          &db::text(PALM_CORPUS_VERSION),&db::text(PALM_FEATURES_VERSION),&db::text(&now)]).await;
      match n {
        Ok(1) => Ok(json(&PalmReadingView {
          id, hand: body.hand, features: ext.features, interpretation,
          corpus_version: PALM_CORPUS_VERSION.into(), created_at: now, ai_chart_context: None,
        }, 201)),
        failed => {
          // UNIQUE 衝突或傳輸錯誤後的回查（tarot 先例：先讀冪等鍵再分類）
          match reading_by_request(&db, &user, &body.request_id).await {
            Ok(Some(existing)) => return Ok(if matches_reading(&existing, &body.hand, &sha) {
              match row_view(&existing) { Ok(v) => json(&v, 200), Err(_) => db_error() }
            } else { conflict("IDEMPOTENCY_CONFLICT") }),
            Err(_) => return Ok(db_error()), Ok(None) => {}
          }
          if let Err(e) = &failed { if !unique_error(e) { return Ok(db_error()) } }
          Ok(db_error())
        }
      }
    })
    .get_async("/api/palm/readings", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let url = match req.url() { Ok(u) => u, Err(_) => return Ok(error::error_code("Invalid URL", "INVALID_PAGE", 400)) };
      let (limit, cursor) = match page_query(url.query()) { Ok(p) => p, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let rows: Vec<ReadingRow> = match db::all(&db,
        "SELECT * FROM palm_readings WHERE user_id=?1 AND (?2 IS NULL OR (created_at,id)<(?2,?3)) \
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
      Ok(json(&PalmHistoryPage { items, next_cursor }, 200))
    })
    .delete_async("/api/palm/readings/:id", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let id = ctx.param("id").cloned().unwrap_or_default();
      if !valid_palm_request_id(&id) { return Ok(error::error_code("Invalid reading", "INVALID_PALM", 400)) }
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      // 不存在／他人／重刪一律 404（不洩漏存在性）
      let n = match db::exec_changes(&db,
        "DELETE FROM palm_readings WHERE id=?1 AND user_id=?2",
        &[&db::text(&id),&db::text(&user)]).await {
          Ok(n) => n, Err(_) => return Ok(db_error())
        };
      if n == 1 { Ok(json(&Deleted { deleted: true }, 200)) }
      else { Ok(error::error_code("Reading not found", "NOT_FOUND", 404)) }
    })
    .delete_async("/api/palm/readings", |req, ctx| async move {
      let user = match auth_user(&req, &ctx).await { Ok(u) => u, Err(e) => return Ok(e) };
      let db = match db::Turso::from_env(&ctx.env) { Ok(d) => d, Err(_) => return Ok(db_error()) };
      let n = match db::exec_changes(&db,
        "DELETE FROM palm_readings WHERE user_id=?1",
        &[&db::text(&user)]).await {
          Ok(n) => n, Err(_) => return Ok(db_error())
        };
      Ok(json(&DeletedCount { deleted: n as i64 }, 200))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(hand: &str, sha: &str) -> ReadingRow {
        ReadingRow {
            id: "01234567-89ab-cdef-0123-456789abcdef".into(),
            hand: hand.into(),
            image_sha256: sha.into(),
            features_json: r#"{"handShape":"unknown","fingerLengths":{"thumb":"unknown","index":"unknown","middle":"unknown","ring":"unknown","pinky":"unknown"},"lines":{},"mounts":{}}"#.into(),
            interpretation: "解讀".into(),
            corpus_version: "1".into(),
            created_at: "2026-10-27T00:00:00.000Z".into(),
            ai_chart_context: None,
        }
    }

    fn req() -> PalmAnalyzeRequest {
        PalmAnalyzeRequest {
            request_id: "01234567-89ab-cdef-0123-456789abcdef".into(),
            hand: "left".into(),
            image_base64: "QUJD".into(),
            mime_type: "image/webp".into(),
        }
    }

    /// sha256 已知向量（sha256sum 實算）——同時釘「對 base64 字串雜湊」契約。
    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            image_sha256("QUJD"),
            "d9cae0dbdbf078b2020e2abe5fcd74bc1edba83c35f6b8a86d638ed9b8d3d1f9"
        );
        assert_eq!(
            image_sha256("A"),
            "559aead08264d5795d3909718cdd05abd49572e84fe55590eef31a88a08fdffd"
        );
        let h = image_sha256("QUJD");
        assert_eq!(h.len(), 64);
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    /// P8a 計數表鍵：503 三碼＋500 截斷＋未知保守。
    #[test]
    fn failure_classification_matrix() {
        assert_eq!(classify_ai_failure(503, "AI_QUEUE_FULL"), AiFailure::Queue);
        assert_eq!(
            classify_ai_failure(503, "AI_QUEUE_TIMEOUT"),
            AiFailure::Queue
        );
        assert_eq!(
            classify_ai_failure(503, "AI_ALL_FAILED"),
            AiFailure::AllFailed
        );
        assert_eq!(
            classify_ai_failure(500, "AI_TRUNCATED"),
            AiFailure::Truncated
        );
        assert_eq!(
            classify_ai_failure(503, ""),
            AiFailure::Unknown,
            "DO 不可達無碼"
        );
        assert_eq!(
            classify_ai_failure(503, "SOMETHING_NEW"),
            AiFailure::Unknown,
            "未知碼保守不退"
        );
        assert_eq!(
            classify_ai_failure(400, "INVALID_IMAGE"),
            AiFailure::Unknown
        );
    }

    /// P8a 轉發封閉集：僅四個契約碼原樣轉發；未知/空碼收斂 503 AI_UNAVAILABLE。
    #[test]
    fn forward_ai_error_closed_set() {
        assert_eq!(
            forward_ai_error(503, "AI_QUEUE_FULL"),
            (503, "AI_QUEUE_FULL")
        );
        assert_eq!(
            forward_ai_error(503, "AI_QUEUE_TIMEOUT"),
            (503, "AI_QUEUE_TIMEOUT")
        );
        assert_eq!(
            forward_ai_error(503, "AI_ALL_FAILED"),
            (503, "AI_ALL_FAILED")
        );
        assert_eq!(forward_ai_error(500, "AI_TRUNCATED"), (500, "AI_TRUNCATED"));
        assert_eq!(
            forward_ai_error(500, "SOMETHING_NEW"),
            (503, "AI_UNAVAILABLE"),
            "未知 500 碼不轉發"
        );
        assert_eq!(
            forward_ai_error(503, "SOMETHING_NEW"),
            (503, "AI_UNAVAILABLE"),
            "未知 503 碼不轉發"
        );
        assert_eq!(
            forward_ai_error(503, ""),
            (503, "AI_UNAVAILABLE"),
            "DO 不可達無碼"
        );
    }

    /// P8a：僅 Queue 結局「退且不占 refund 額度」。
    #[test]
    fn only_queue_refunds_without_bucket() {
        assert!(refunds_without_bucket(AiFailure::Queue));
        for f in [
            AiFailure::AllFailed,
            AiFailure::Truncated,
            AiFailure::Unknown,
        ] {
            assert!(!refunds_without_bucket(f), "{f:?} 不得無條件退");
        }
    }

    /// P13：hand＋image_sha256 冪等比對。
    #[test]
    fn idempotency_matches_hand_and_sha() {
        let r = row("left", "abc");
        assert!(matches_reading(&r, "left", "abc"));
        assert!(!matches_reading(&r, "right", "abc"), "hand 異 → 409");
        assert!(!matches_reading(&r, "left", "xyz"), "sha 異 → 409");
    }

    /// DO 請求體：無 keys、image 內嵌、chartData 只有 hand。
    #[test]
    fn interpret_body_shape() {
        let v = build_interpret_body(&req());
        let ir = v.get("interpretRequest").expect("interpretRequest 必在");
        assert_eq!(ir["chartType"], "palm");
        assert_eq!(ir["chartData"]["hand"], "left");
        assert_eq!(ir["image"]["mime"], "image/webp");
        assert_eq!(ir["image"]["base64"], "QUJD");
        assert_eq!(ir["language"], "zh");
        assert!(v.get("keys").is_none(), "Workers AI 免 key——不帶 keys");
    }

    /// P8b：模型包圍籬也能 strict parse；垃圾回應失敗（→ AI_PARSE_FAILED 路徑）。
    #[test]
    fn parse_extraction_through_fence() {
        let ok = "```json\n{\"quality\":\"ok\",\"confidence\":0.9,\"features\":{\"handShape\":\"earth\",\"fingerLengths\":{\"thumb\":\"short\",\"index\":\"medium\",\"middle\":\"long\",\"ring\":\"medium\",\"pinky\":\"short\"},\"lines\":{\"life\":{\"presence\":\"present\",\"traits\":{\"depth\":\"deep\",\"length\":\"long\",\"curve\":\"slight\"}}},\"mounts\":{\"venus\":\"prominent\"}}}\n```";
        let ext: PalmExtraction =
            serde_json::from_str(strip_json_fence(ok)).expect("圍籬內有效 JSON 必須可解");
        assert_eq!(ext.confidence, 0.9);
        assert!(
            serde_json::from_str::<PalmExtraction>(strip_json_fence("not json at all")).is_err()
        );
    }

    /// 落庫列 → wire view（camelCase、ai_chart_context 三段）。
    #[test]
    fn row_view_roundtrip() {
        let v = row_view(&row("right", "abc")).expect("合法 features_json 必須可解");
        assert_eq!(v.hand, "right");
        assert_eq!(v.corpus_version, "1");
        assert!(v.ai_chart_context.is_none());
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains("\"interpretation\"") && s.contains("\"features\""));
        assert!(!s.contains("imageSha256"), "指紋不進 wire（P12）");
    }
}
