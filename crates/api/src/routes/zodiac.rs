//! 星座配對 AI 解讀（階段 B，plan 2026-11-19-zodiac-match §2.4–2.5、§6）。
//!
//! 處理順序：auth → body cap → 每分鐘限流（便宜、先擋腳本）→ parse → 驗 target
//! （400 `INVALID_SIGN`）→ **伺服器讀西洋盤**（`resolve_western_chart`；未填生辰 400、
//! DB 500、引擎 502——皆在扣 AI 額度之前）→ 查快取（命中不計次）→ IP／user 24h
//! strict 可退次限流 → AIMutexDO 專屬實例 `id_from_name("zodiac-match")` → 成功才寫入。
//!
//! - **失敗不落庫、一律退次**（站長 D1）：AI 呼叫的任何失敗（stub、空內容、DO 例外／
//!   非 200、輸出檢查不過）都退 IP＋user 兩把票根；額度用盡／佇列滿／佇列逾時回 503 原碼，
//!   其餘回 503 `AI_UNAVAILABLE`。
//! - 落庫前輸出檢查 [`output_ok`]：必含 `## 核心價值`、無 rising 不得提上升、無 moon 不得有
//!   情感需求節、不得出現角色混淆字（你們的／對方的月亮、上升）或百分比。
//! - 429：user 24h 上限回 `AI_DAILY_LIMIT`（前端顯示 3 次上限），其餘限流回 `RATE_LIMIT`。
//! - chartData 不含生日、出生地、時辰；未填時辰不送 rising（引擎以正午算的上升不可用）。
//! - 快取鍵＝(user_id, target_sign, sign_key, corpus_version, prompt_version)；先查後插，
//!   併發雙寫最多多一筆，讀取取最新。
//! - 紅線：配對資料永不寫入 predictions／prediction_feedback／f8_assignments。

use super::super::{
    error,
    services::{
        chart_resolver::{resolve_western_chart, WesternResolveError},
        clock, db, uuid,
    },
};
use super::common::{
    auth_user, body_too_large, client_ip, ok_json, rate_limit, rate_limit_strict_refundable,
    refund_strict,
};
use super::R;
use ft_schema::api::{ZodiacMatchInterpretRequest, ZodiacMatchInterpretResponse};
use ft_schema::zodiac_match::{
    match_rows, normalize_sign, sign_key, MatchBody, MatchRow, ZODIAC_MATCH_CORPUS_VERSION,
    ZODIAC_MATCH_PROMPT_VERSION,
};

use serde::Deserialize;
use worker::*;

/// 每分鐘每用戶 20 次（便宜的先行限流；含快取命中）。
const MINUTE_LIMIT: u32 = 20;
const MINUTE_WINDOW_MS: f64 = 60_000.0;
/// AI 額度：24h 固定視窗、fail-closed、可退次（比照 tarot v2）。
const AI_USER_LIMIT: u32 = 3;
const AI_IP_LIMIT: u32 = 10;
const AI_WINDOW_MS: f64 = 86_400_000.0;

#[derive(Deserialize)]
struct CachedRow {
    /// 表無 NOT NULL（站長規則）→ 讀取端容忍 NULL；空值視同未命中
    ai_interpretation: Option<String>,
    ai_provider: Option<String>,
    ai_model: Option<String>,
}

fn json(v: &ZodiacMatchInterpretResponse) -> Response {
    let mut res = ok_json(
        &serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
        200,
    );
    let _ = res.headers_mut().set("Cache-Control", "private, no-store");
    res
}

fn db_error() -> Response {
    error::error_code("db error", "DB_ERROR", 500)
}

/// 使用者這一側的星座（標準英文名）。`rising` 只在有出生時辰時才有值。
#[derive(Debug, Clone, PartialEq)]
struct MySigns {
    sun: &'static str,
    moon: Option<&'static str>,
    rising: Option<&'static str>,
}

fn my_signs(sun: &str, moon: &str, rising: &str, has_birth_hour: bool) -> Option<MySigns> {
    Some(MySigns {
        sun: normalize_sign(sun)?,
        moon: normalize_sign(moon),
        rising: if has_birth_hour {
            normalize_sign(rising)
        } else {
            None
        },
    })
}

fn rows_for(my: &MySigns, target: &str) -> Vec<MatchRow> {
    let mut mine: Vec<(MatchBody, &str)> = vec![(MatchBody::Sun, my.sun)];
    if let Some(m) = my.moon {
        mine.push((MatchBody::Moon, m));
    }
    if let Some(r) = my.rising {
        mine.push((MatchBody::Rising, r));
    }
    match_rows(&mine, target)
}

/// AI 用 chartData（plan §2.4／§6.4）：`rules[].pair` 明寫「你的月亮（水瓶座）×
/// 對方的太陽（獅子座）」；無 rising 時不送該鍵。
fn build_chart_data(my: &MySigns, target: &str) -> serde_json::Value {
    let rules: Vec<serde_json::Value> = rows_for(my, target)
        .iter()
        .map(|r| {
            serde_json::json!({
                "pair": r.frame(),
                "theme": r.theme(),
                "aspect": r.aspect.plain(),
                "label": r.label.zh(),
                "line": r.label.line(),
                "elementLine": r.element_line,
                "modalityLine": r.modality_line,
            })
        })
        .collect();
    let mut signs = serde_json::json!({ "sun": my.sun });
    if let Some(m) = my.moon {
        signs["moon"] = serde_json::json!(m);
    }
    if let Some(r) = my.rising {
        signs["rising"] = serde_json::json!(r);
    }
    serde_json::json!({
        "mySigns": signs,
        "target": target,
        "rules": rules,
        "corpusVersion": ZODIAC_MATCH_CORPUS_VERSION,
    })
}

struct AiOk {
    interpretation: String,
    provider: String,
    model: String,
}

/// DO 失敗 `(status, code)`。
type AiErr = (u16, String);

/// AI 失敗 → 回前端的 503 code。**所有失敗都退次**（D1）；這裡只決定 code：
/// 額度用盡／佇列滿／佇列逾時保留原碼，其餘（stub、空內容、DO 例外、輸出檢查不過…）
/// 一律 `AI_UNAVAILABLE`。
fn failure_code(err: &AiErr) -> &'static str {
    match (err.0, err.1.as_str()) {
        (503, "AI_QUOTA") => "AI_QUOTA",
        (503, "AI_QUEUE_FULL") => "AI_QUEUE_FULL",
        (503, "AI_QUEUE_TIMEOUT") => "AI_QUEUE_TIMEOUT",
        _ => "AI_UNAVAILABLE",
    }
}

/// 落庫前輸出檢查（純函式）：結構與角色紅線。不過＝當成 AI 失敗（退次、不寫入、503）。
fn output_ok(text: &str, has_moon: bool, has_rising: bool) -> bool {
    if !text.contains("## 核心價值") {
        return false;
    }
    if !has_rising && text.contains("上升") {
        return false;
    }
    if !has_moon && (text.contains("## 情感需求") || text.contains("月亮")) {
        return false;
    }
    const BANNED: [&str; 6] = [
        "你們的月亮",
        "你們的上升",
        "對方的月亮",
        "對方的上升",
        "%",
        "％",
    ];
    !BANNED.iter().any(|b| text.contains(b))
}

/// 呼 AIMutexDO 專屬實例 `id_from_name("zodiac-match")`；stub／空內容一律視為失敗。
async fn call_match_ai(
    ctx: &RouteContext<()>,
    chart_data: &serde_json::Value,
) -> std::result::Result<AiOk, AiErr> {
    let fail = |_| (0u16, String::new());
    let ns = ctx.env.durable_object("AI_MUTEX").map_err(fail)?;
    let stub = ns
        .id_from_name("zodiac-match")
        .and_then(|id| id.get_stub())
        .map_err(fail)?;
    let body = serde_json::json!({
        "interpretRequest": {
            "chartType": "zodiac_match",
            "chartData": chart_data,
            "language": "zh",
        },
    });
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(body.to_string().into()));
    let req = Request::new_with_init("https://ai-mutex/interpret", &init).map_err(fail)?;
    let mut res = stub.fetch_with_request(req).await.map_err(fail)?;
    let status = res.status_code();
    if status != 200 {
        let err: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        let code = err
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return Err((status, code));
    }
    let data: serde_json::Value = res.json().await.map_err(fail)?;
    if data.get("stub").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Err((200, "STUB".to_string()));
    }
    let interpretation = data
        .get("interpretation")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if interpretation.trim().is_empty() {
        return Err((200, "EMPTY".to_string()));
    }
    let s = |k: &str| {
        data.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    Ok(AiOk {
        interpretation,
        provider: s("provider"),
        model: s("model"),
    })
}

async fn cached(db: &db::Turso, user: &str, target: &str, key: &str) -> Result<Option<CachedRow>> {
    db::first(
        db,
        "SELECT ai_interpretation, ai_provider, ai_model FROM zodiac_match_readings \
         WHERE user_id=?1 AND target_sign=?2 AND sign_key=?3 AND corpus_version=?4 AND prompt_version=?5 \
         AND ai_interpretation IS NOT NULL AND ai_interpretation != '' \
         ORDER BY created_at DESC LIMIT 1",
        &[
            &db::text(user),
            &db::text(target),
            &db::text(key),
            &db::text(ZODIAC_MATCH_CORPUS_VERSION),
            &db::text(ZODIAC_MATCH_PROMPT_VERSION),
        ],
    )
    .await
}

pub fn register(router: R<'static>) -> R<'static> {
    router.post_async("/api/zodiac/match/interpret", |mut req, ctx| async move {
        let user = match auth_user(&req, &ctx).await {
            Ok(u) => u,
            Err(e) => return Ok(e),
        };
        if body_too_large(&req) {
            return Ok(error::error_code(
                "payload too large",
                "PAYLOAD_TOO_LARGE",
                413,
            ));
        }
        if !rate_limit(
            &ctx,
            &format!("zodiac:match:min:{}", user),
            MINUTE_LIMIT,
            MINUTE_WINDOW_MS,
        )
        .await
        {
            return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
        }
        let body: ZodiacMatchInterpretRequest = match req.json().await {
            Ok(v) => v,
            Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)),
        };
        let Some(target) = normalize_sign(&body.target) else {
            return Ok(error::error_code("Invalid sign", "INVALID_SIGN", 400));
        };

        // 伺服器讀西洋盤（不信任前端送來的星座）；失敗皆在計次之前
        let resolved = match resolve_western_chart(&ctx, &user).await {
            Ok(r) => r,
            Err(WesternResolveError::NoBirthData) => {
                return Ok(error::error_code(
                    "Birth data required",
                    "NO_BIRTH_DATA",
                    400,
                ))
            }
            Err(WesternResolveError::Db) => return Ok(db_error()),
            Err(WesternResolveError::Engine) => {
                return Ok(error::error_code(
                    "Chart temporarily unavailable",
                    "ENGINE_ERROR",
                    502,
                ))
            }
        };
        let c = &resolved.chart;
        let Some(my) = my_signs(
            &c.sun_sign.name,
            &c.moon_sign.name,
            &c.ascendant.sign,
            resolved.has_birth_hour,
        ) else {
            return Ok(error::error_code(
                "Chart temporarily unavailable",
                "ENGINE_ERROR",
                502,
            ));
        };
        let Some(key) = sign_key(my.sun, my.moon, my.rising) else {
            return Ok(error::error_code(
                "Chart temporarily unavailable",
                "ENGINE_ERROR",
                502,
            ));
        };

        let db = match db::Turso::from_env(&ctx.env) {
            Ok(d) => d,
            Err(_) => return Ok(db_error()),
        };
        match cached(&db, &user, target, &key).await {
            Ok(Some(CachedRow {
                ai_interpretation: Some(text),
                ai_provider,
                ai_model,
            })) if !text.trim().is_empty() => {
                return Ok(json(&ZodiacMatchInterpretResponse {
                    interpretation: text,
                    provider: ai_provider,
                    model: ai_model,
                    cached: true,
                }))
            }
            Ok(_) => {}
            Err(_) => return Ok(db_error()),
        }

        let ip_key = format!("zodiac:ai:ip:{}", client_ip(&req));
        let user_key = format!("zodiac:ai:{}", user);
        let Some(ip_reset) =
            rate_limit_strict_refundable(&ctx, &ip_key, AI_IP_LIMIT, AI_WINDOW_MS).await
        else {
            return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
        };
        let Some(user_reset) =
            rate_limit_strict_refundable(&ctx, &user_key, AI_USER_LIMIT, AI_WINDOW_MS).await
        else {
            refund_strict(&ctx, &ip_key, ip_reset).await;
            return Ok(error::error_code(
                "Daily AI limit reached",
                "AI_DAILY_LIMIT",
                429,
            ));
        };

        let chart_data = build_chart_data(&my, target);
        let outcome = match call_match_ai(&ctx, &chart_data).await {
            Ok(o) if output_ok(&o.interpretation, my.moon.is_some(), my.rising.is_some()) => {
                Ok(o)
            }
            Ok(_) => Err((200u16, "OUTPUT_CHECK".to_string())),
            Err(e) => Err(e),
        };
        let ok = match outcome {
            Ok(o) => o,
            Err(err) => {
                // D1：任何失敗都退兩把（每把各退一次，票根冪等）
                refund_strict(&ctx, &user_key, user_reset).await;
                refund_strict(&ctx, &ip_key, ip_reset).await;
                let code = failure_code(&err);
                let msg = match code {
                    "AI_QUOTA" => "Daily AI quota exceeded",
                    "AI_UNAVAILABLE" => "AI temporarily unavailable",
                    _ => "AI is busy, please try again",
                };
                return Ok(error::error_code(msg, code, 503));
            }
        };

        // 成功才落庫；寫入失敗仍回結果（使用者已等完 AI、已計次），只是下次不命中快取。
        // 不可靜默：記 log（不含細節）。
        let inserted = db::exec(
            &db,
            "INSERT INTO zodiac_match_readings \
             (id,user_id,target_sign,sign_key,corpus_version,prompt_version,ai_interpretation,ai_provider,ai_model,created_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            &[
                &db::text(&uuid::random_uuid()),
                &db::text(&user),
                &db::text(target),
                &db::text(&key),
                &db::text(ZODIAC_MATCH_CORPUS_VERSION),
                &db::text(ZODIAC_MATCH_PROMPT_VERSION),
                &db::text(&ok.interpretation),
                &db::opt_text(Some(ok.provider.as_str())),
                &db::opt_text(Some(ok.model.as_str())),
                &db::text(&clock::now_iso()),
            ],
        )
        .await;
        if inserted.is_err() {
            console_error!("zodiac match cache insert failed");
        }
        Ok(json(&ZodiacMatchInterpretResponse {
            interpretation: ok.interpretation,
            provider: Some(ok.provider),
            model: Some(ok.model),
            cached: false,
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rising_dropped_without_birth_hour() {
        let with = my_signs("Taurus", "Aquarius", "Virgo", true).unwrap();
        assert_eq!(with.rising, Some("Virgo"));
        let without = my_signs("Taurus", "Aquarius", "Virgo", false).unwrap();
        assert_eq!(without.rising, None, "未填時辰：引擎以正午算的上升不可用");
        assert!(my_signs("bogus", "Aquarius", "Virgo", true).is_none());
    }

    #[test]
    fn chart_data_has_no_birth_info_and_frames_pairs() {
        let my = my_signs("Taurus", "Aquarius", "Virgo", false).unwrap();
        let v = build_chart_data(&my, "Leo");
        assert!(v.pointer("/mySigns/rising").is_none(), "無時辰不送 rising");
        assert_eq!(v["target"], "Leo");
        let rules = v["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[1]["pair"], "你的月亮（水瓶座）× 對方的太陽（獅子座）");
        let s = v.to_string();
        for banned in ["birth", "latitude", "longitude", "hour"] {
            assert!(!s.contains(banned), "chartData 不得含 {banned}");
        }
    }

    #[test]
    fn merged_rows_when_same_sign() {
        let my = my_signs("Taurus", "Taurus", "Virgo", true).unwrap();
        let v = build_chart_data(&my, "Scorpio");
        let rules = v["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(
            rules[0]["pair"],
            "你的太陽＋月亮（金牛座）× 對方的太陽（天蠍座）"
        );
        assert_eq!(rules[0]["theme"], "核心價值・情感需求");
    }

    #[test]
    fn failure_codes_all_refunded_paths() {
        // 所有 Err 都退次（route 無條件 refund）；這裡驗回前端的 code
        assert_eq!(failure_code(&(503, "AI_QUOTA".into())), "AI_QUOTA");
        assert_eq!(
            failure_code(&(503, "AI_QUEUE_FULL".into())),
            "AI_QUEUE_FULL"
        );
        assert_eq!(
            failure_code(&(503, "AI_QUEUE_TIMEOUT".into())),
            "AI_QUEUE_TIMEOUT"
        );
        for e in [
            (200, "STUB"),
            (200, "EMPTY"),
            (200, "OUTPUT_CHECK"),
            (0, ""),
            (400, "INVALID_REQUEST"),
            (500, ""),
            (503, "AI_UNAVAILABLE"),
            (502, "X"),
        ] {
            assert_eq!(failure_code(&(e.0, e.1.into())), "AI_UNAVAILABLE", "{e:?}");
        }
    }

    #[test]
    fn output_check_rules() {
        let good = "## 核心價值\n…\n## 情感需求\n…\n## 第一印象\n你的上升…\n## 小提醒\n…";
        assert!(output_ok(good, true, true));
        assert!(
            !output_ok("## 核心價值\n你的月亮很敏感\n## 小提醒\n…", false, false),
            "無月亮時內文不得出現月亮"
        );
        assert!(!output_ok("## 情感需求\n…", true, true), "缺核心價值");
        assert!(!output_ok(good, true, false), "無 rising 不得提上升");
        let no_rise = "## 核心價值\n…\n## 情感需求\n…\n## 小提醒\n…";
        assert!(output_ok(no_rise, true, false));
        assert!(
            !output_ok(no_rise, false, false),
            "無 moon 不得有情感需求節"
        );
        for bad in [
            "你們的月亮",
            "你們的上升",
            "對方的月亮",
            "對方的上升",
            "80%",
            "80％",
        ] {
            assert!(
                !output_ok(&format!("## 核心價值\n{bad}"), true, true),
                "{bad}"
            );
        }
    }
}
