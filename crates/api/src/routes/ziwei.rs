//! 紫微子單元：流年大運＋今年運勢（spec docs/superpowers/specs/2026-11-21-sub-units-design.md
//! §A.4–A.5）。
//!
//! - `GET /api/ziwei/horoscope?year=Y`：引擎流年（不落庫）。省略 `year`＝台北今天
//!   （春節前屬前一個農曆年）。Y 以 `Y-7-1` 計算（7/1 必在農曆 Y 年內）；範圍＝出生農曆年
//!   到 +100，超出 400 `INVALID_YEAR`。`Cache-Control: private, max-age=300`＋`Vary`。
//! - `GET /api/ziwei/year`：今年的規則摘要＋已有的 AI 解讀（沒有則 null）。
//! - `POST /api/ziwei/year/interpret`（無 body）：auth → 每分鐘 IP 預限流 → 生辰檢查
//!   （含 `NO_BIRTH_HOUR`）→ 引擎流年 → 查快取（命中不計次）→ 本命盤 → IP 10／user 3
//!   per 24h strict 可退次限流 → AIMutexDO 專屬實例 `id_from_name("ziwei-year")` →
//!   **任何失敗不落庫、退兩把、回 503 `AI_UNAVAILABLE`** → 成功才寫入。
//! - 落庫前 [`output_ok`]：禁「一定／注定／必然／保證」、月份字樣、「預測」「本週」、
//!   去空白後少於 200 字。結尾提醒句 [`ZIWEI_YEAR_CLOSING`] 由 route 附加，不靠 AI。
//! - chartData 不含生日、出生地、時辰、虛歲（R23）；route 一律送 `language: "zh"`。
//! - 錯誤不 echo 引擎／DB 細節。
//! - 紅線：今年運勢資料永不寫入 predictions／prediction_feedback／f8_assignments；
//!   不讀也不寫 F4 strengths。

use super::super::{
    error,
    services::{
        chart_resolver::{resolve_ziwei_birth, resolve_ziwei_natal, ZiweiBirth, ZiweiBirthError},
        clock, db, engine,
        engine_version::{ENGINE_VERSION_ZIWEI, ENGINE_VERSION_ZIWEI_HOROSCOPE},
        uuid,
    },
};
use super::common::{
    apply_cache_headers, auth_user, client_ip, ok_json, rate_limit, rate_limit_strict_refundable,
    refund_strict,
};
use super::R;
use ft_schema::tarot::taipei_today;
use ft_schema::ziwei_luck::{
    ZiWeiHoroscopeV1, ZiweiHoroscopeResponse, ZiweiYearReading, ZiweiYearResponse,
    ZiweiYearSummary, ZIWEI_YEAR_CLOSING, ZIWEI_YEAR_PROMPT_VERSION,
};
use ft_schema::ZiWeiChartV3;

use serde::Deserialize;
use worker::*;

/// 每分鐘每 IP 20 次（便宜的先行限流；含快取命中）。
const MINUTE_LIMIT: u32 = 20;
const MINUTE_WINDOW_MS: f64 = 60_000.0;
/// AI 額度：24h 固定視窗、fail-closed、可退次（站長 §8-3：與紫微命盤 AI 分開計）。
const AI_USER_LIMIT: u32 = 3;
const AI_IP_LIMIT: u32 = 10;
const AI_WINDOW_MS: f64 = 86_400_000.0;
/// 可查的年份：出生農曆年到 +100（虛歲 1–101）。
const MAX_NOMINAL_AGE: u32 = 101;
/// 去空白後的最少字數。
const MIN_CHARS: usize = 200;

#[derive(Deserialize)]
struct CachedRow {
    /// 表無 NOT NULL（站長規則）→ 讀取端容忍 NULL；空值視同未命中
    ai_interpretation: Option<String>,
    ai_provider: Option<String>,
    ai_model: Option<String>,
}

fn db_error() -> Response {
    error::error_code("db error", "DB_ERROR", 500)
}

fn engine_error() -> Response {
    error::error_code("Chart temporarily unavailable", "ENGINE_ERROR", 502)
}

fn invalid_year() -> Response {
    error::error_code("Year out of range", "INVALID_YEAR", 400)
}

fn birth_error(e: ZiweiBirthError) -> Response {
    match e {
        ZiweiBirthError::NoBirthData => {
            error::error_code("Birth data required", "NO_BIRTH_DATA", 400)
        }
        ZiweiBirthError::NoGender => error::error_code("Gender required", "NO_GENDER", 400),
        ZiweiBirthError::NoBirthHour => {
            error::error_code("Birth hour required", "NO_BIRTH_HOUR", 400)
        }
        ZiweiBirthError::Db => db_error(),
    }
}

/// `YYYY-MM-DD` → `YYYY-M-D`（x-iztro 慣用格式；去前導零）。
fn unpad_date(iso: &str) -> Option<String> {
    let mut it = iso.split('-');
    let y: i32 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some(format!("{y}-{m}-{d}"))
}

/// 台北今天（`YYYY-M-D`）。
fn today_target() -> Option<String> {
    unpad_date(&taipei_today(&clock::now_iso())?)
}

/// `?year=` 解析：省略 → `Ok(None)`；非整數 → `Err`。
fn parse_year_param(raw: Option<&str>) -> std::result::Result<Option<i32>, ()> {
    match raw {
        None => Ok(None),
        Some(s) => s.trim().parse::<i32>().map(Some).map_err(|_| ()),
    }
}

/// 呼叫引擎前的便宜範圍檢查：出生陽曆年的前一年（春節前出生屬前一農曆年）到 +101。
fn year_roughly_in_range(year: i32, birth_solar_year: i64) -> bool {
    let y = i64::from(year);
    y >= birth_solar_year - 1 && y <= birth_solar_year + 101
}

/// 引擎回傳後的精確檢查：以 `Y-7-1` 計算時 luckYear 必等於 Y（Y 早於出生農曆年時虛歲被
/// 夾成 1、luckYear ≠ Y），且虛歲 ≤ 101。
fn year_exactly_in_range(year: i32, h: &ZiWeiHoroscopeV1) -> bool {
    h.luck_year == year && h.nominal_age >= 1 && h.nominal_age <= MAX_NOMINAL_AGE
}

async fn fetch_horoscope(
    ctx: &RouteContext<()>,
    birth: &ZiweiBirth,
    target: &str,
) -> std::result::Result<ZiWeiHoroscopeV1, Response> {
    let fetcher = ctx.env.service("FT_ENGINE").map_err(|_| engine_error())?;
    engine::fetch_engine_horoscope(&fetcher, &birth.engine, target)
        .await
        .map_err(|_| {
            console_error!("ziwei horoscope engine call failed");
            engine_error()
        })
}

/// 本命盤上某顆星所在的宮名（找不到回空字串）。
fn natal_palace_of_star(chart: &ZiWeiChartV3, star: &str) -> String {
    chart
        .palaces
        .iter()
        .find(|p| p.stars.iter().any(|s| s.name == star))
        .map(|p| p.name.clone())
        .unwrap_or_default()
}

fn natal_palace_name(chart: &ZiWeiChartV3, branch: &str) -> String {
    chart
        .palaces
        .iter()
        .find(|p| p.branch == branch)
        .map(|p| p.name.clone())
        .unwrap_or_default()
}

/// AI 用 chartData（spec §A.5）：本命十二宮主星＋亮度＋生年四化、五行局、性別、
/// 目前大限（或童限）、流年命宮、流年十二宮名、流年四化。**不含**生日、出生地、時辰、虛歲。
fn build_chart_data(chart: &ZiWeiChartV3, h: &ZiWeiHoroscopeV1) -> serde_json::Value {
    let palaces: Vec<serde_json::Value> = chart
        .palaces
        .iter()
        .map(|p| {
            let stars: Vec<serde_json::Value> = p
                .stars
                .iter()
                .filter(|s| s.star_type == "main" || s.sihua.is_some())
                .map(|s| {
                    let mut v = serde_json::json!({ "name": s.name });
                    if let Some(b) = &s.brightness {
                        v["brightness"] = serde_json::json!(b);
                    }
                    if let Some(sh) = &s.sihua {
                        v["sihua"] = serde_json::json!(sh);
                    }
                    v
                })
                .collect();
            serde_json::json!({ "branch": p.branch, "name": p.name, "stars": stars })
        })
        .collect();
    let d = &h.decadal;
    let mut decadal = serde_json::json!({
        "isChildhood": d.is_childhood,
        "stem": d.stem,
        "branch": d.branch,
        "natalPalace": chart
            .palaces
            .get(d.palace_index as usize)
            .map(|p| p.name.clone())
            .unwrap_or_default(),
    });
    if let (Some(s), Some(e)) = (d.start_age, d.end_age) {
        decadal["startAge"] = serde_json::json!(s);
        decadal["endAge"] = serde_json::json!(e);
    }
    let y = &h.yearly;
    let mutagen: Vec<serde_json::Value> = y
        .mutagen
        .labeled()
        .iter()
        .map(|(label, star)| {
            serde_json::json!({
                "label": label,
                "star": star,
                "natalPalace": natal_palace_of_star(chart, star),
            })
        })
        .collect();
    serde_json::json!({
        "natal": {
            "fiveElement": chart.five_element,
            "gender": chart.birth_info.gender,
            "palaces": palaces,
        },
        "decadal": decadal,
        "yearly": {
            "stem": y.stem,
            "branch": y.branch,
            "natalPalace": natal_palace_name(chart, &y.branch),
            "palaceNames": y.palace_names,
            "mutagen": mutagen,
        },
    })
}

/// 落庫前輸出檢查（純函式；spec §A.5 R07）。不過＝當成 AI 失敗（退次、不寫入、503）。
fn output_ok(text: &str) -> bool {
    const BANNED: [&str; 6] = ["一定", "注定", "必然", "保證", "預測", "本週"];
    if BANNED.iter().any(|b| text.contains(b)) {
        return false;
    }
    if has_month_word(text) {
        return false;
    }
    text.chars().filter(|c| !c.is_whitespace()).count() >= MIN_CHARS
}

/// regex `[一二三四五六七八九十０-９0-9]+月` 的等價判定：任一「月」緊接在數字字元之後。
fn has_month_word(text: &str) -> bool {
    let is_num = |c: char| {
        "一二三四五六七八九十".contains(c) || c.is_ascii_digit() || ('０'..='９').contains(&c)
    };
    let mut prev: Option<char> = None;
    for c in text.chars() {
        if c == '月' && prev.is_some_and(is_num) {
            return true;
        }
        prev = Some(c);
    }
    false
}

/// AI 文字＋固定結尾提醒（route 附加）。
fn with_closing(text: &str) -> String {
    format!("{}\n\n{}", text.trim_end(), ZIWEI_YEAR_CLOSING)
}

struct AiOk {
    interpretation: String,
    provider: String,
    model: String,
}

/// 呼 AIMutexDO 專屬實例 `id_from_name("ziwei-year")`；stub／空內容／非 200 一律 `Err`。
async fn call_year_ai(
    ctx: &RouteContext<()>,
    chart_data: &serde_json::Value,
) -> std::result::Result<AiOk, ()> {
    let ns = ctx.env.durable_object("AI_MUTEX").map_err(|_| ())?;
    let stub = ns
        .id_from_name("ziwei-year")
        .and_then(|id| id.get_stub())
        .map_err(|_| ())?;
    let body = serde_json::json!({
        "interpretRequest": {
            "chartType": "ziwei_year",
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
        return Err(());
    }
    let data: serde_json::Value = res.json().await.map_err(|_| ())?;
    if data.get("stub").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Err(());
    }
    let s = |k: &str| {
        data.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let interpretation = s("interpretation");
    if interpretation.trim().is_empty() {
        return Err(());
    }
    Ok(AiOk {
        interpretation,
        provider: s("provider"),
        model: s("model"),
    })
}

async fn cached(db: &db::Turso, user: &str, luck_year: i32) -> Result<Option<CachedRow>> {
    db::first(
        db,
        "SELECT ai_interpretation, ai_provider, ai_model FROM ziwei_year_readings \
         WHERE user_id=?1 AND luck_year=?2 AND engine_version_ziwei=?3 AND horoscope_version=?4 \
         AND prompt_version=?5 AND ai_interpretation IS NOT NULL AND ai_interpretation != '' \
         ORDER BY created_at DESC LIMIT 1",
        &[
            &db::text(user),
            &db::int(luck_year),
            &db::text(ENGINE_VERSION_ZIWEI),
            &db::text(ENGINE_VERSION_ZIWEI_HOROSCOPE),
            &db::text(ZIWEI_YEAR_PROMPT_VERSION),
        ],
    )
    .await
}

/// 快取列 → 回應；空文字視同未命中。
fn reading_from_row(row: CachedRow, luck_year: i32) -> Option<ZiweiYearReading> {
    let text = row.ai_interpretation.filter(|t| !t.trim().is_empty())?;
    Some(ZiweiYearReading {
        luck_year,
        interpretation: with_closing(&text),
        provider: row.ai_provider,
        model: row.ai_model,
        cached: true,
    })
}

fn no_store_json<T: serde::Serialize>(v: &T) -> Response {
    let mut res = ok_json(
        &serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
        200,
    );
    apply_cache_headers(&mut res, 0, false);
    res
}

pub fn register(router: R<'static>) -> R<'static> {
    router
        .get_async("/api/ziwei/horoscope", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let url = req.url()?;
            let raw_year = url
                .query_pairs()
                .find(|(k, _)| k == "year")
                .map(|(_, v)| v.into_owned());
            let Ok(year) = parse_year_param(raw_year.as_deref()) else {
                return Ok(invalid_year());
            };
            let birth = match resolve_ziwei_birth(&ctx, &user).await {
                Ok(b) => b,
                Err(e) => return Ok(birth_error(e)),
            };
            let target = match year {
                Some(y) => {
                    if !year_roughly_in_range(y, birth.engine.year.unwrap_or_default()) {
                        return Ok(invalid_year());
                    }
                    format!("{y}-7-1")
                }
                None => match today_target() {
                    Some(t) => t,
                    None => return Ok(engine_error()),
                },
            };
            let h = match fetch_horoscope(&ctx, &birth, &target).await {
                Ok(h) => h,
                Err(r) => return Ok(r),
            };
            if let Some(y) = year {
                if !year_exactly_in_range(y, &h) {
                    return Ok(invalid_year());
                }
            }
            let body = ZiweiHoroscopeResponse {
                horoscope: h,
                engine_version_ziwei_horoscope: ENGINE_VERSION_ZIWEI_HOROSCOPE.to_string(),
            };
            let mut res = ok_json(
                &serde_json::to_value(&body).unwrap_or(serde_json::Value::Null),
                200,
            );
            apply_cache_headers(&mut res, 300, true);
            Ok(res)
        })
        .get_async("/api/ziwei/year", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let birth = match resolve_ziwei_birth(&ctx, &user).await {
                Ok(b) => b,
                Err(e) => return Ok(birth_error(e)),
            };
            let Some(target) = today_target() else {
                return Ok(engine_error());
            };
            let h = match fetch_horoscope(&ctx, &birth, &target).await {
                Ok(h) => h,
                Err(r) => return Ok(r),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            let reading = match cached(&db, &user, h.luck_year).await {
                Ok(row) => row.and_then(|r| reading_from_row(r, h.luck_year)),
                Err(_) => return Ok(db_error()),
            };
            Ok(no_store_json(&ZiweiYearResponse {
                summary: ZiweiYearSummary {
                    horoscope: h,
                    engine_version_ziwei_horoscope: ENGINE_VERSION_ZIWEI_HOROSCOPE.to_string(),
                },
                reading,
            }))
        })
        .post_async("/api/ziwei/year/interpret", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let ip = client_ip(&req);
            if !rate_limit(
                &ctx,
                &format!("ziwei:year:min:{}", ip),
                MINUTE_LIMIT,
                MINUTE_WINDOW_MS,
            )
            .await
            {
                return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
            }
            let birth = match resolve_ziwei_birth(&ctx, &user).await {
                Ok(b) => b,
                Err(e) => return Ok(birth_error(e)),
            };
            let Some(target) = today_target() else {
                return Ok(engine_error());
            };
            let h = match fetch_horoscope(&ctx, &birth, &target).await {
                Ok(h) => h,
                Err(r) => return Ok(r),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(db_error()),
            };
            match cached(&db, &user, h.luck_year).await {
                Ok(Some(row)) => {
                    if let Some(r) = reading_from_row(row, h.luck_year) {
                        return Ok(no_store_json(&r));
                    }
                }
                Ok(None) => {}
                Err(_) => return Ok(db_error()),
            }
            // 本命盤：失敗皆在扣 AI 額度之前
            let natal = match resolve_ziwei_natal(&ctx, &user, &birth).await {
                Ok(Some(c)) => c,
                Ok(None) => return Ok(engine_error()),
                Err(_) => return Ok(db_error()),
            };
            let chart_data = build_chart_data(&natal, &h);

            let ip_key = format!("ziwei:year:ai:ip:{}", ip);
            let user_key = format!("ziwei:year:ai:{}", user);
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

            let ok = match call_year_ai(&ctx, &chart_data).await {
                Ok(o) if output_ok(&o.interpretation) => o,
                _ => {
                    // 任何失敗（stub、空字串、輸出檢查不過、佇列滿／逾時、DO 例外）：
                    // 不落庫、兩把都退、503 AI_UNAVAILABLE
                    refund_strict(&ctx, &user_key, user_reset).await;
                    refund_strict(&ctx, &ip_key, ip_reset).await;
                    return Ok(error::error_code(
                        "AI temporarily unavailable",
                        "AI_UNAVAILABLE",
                        503,
                    ));
                }
            };

            // 成功才落庫；寫入失敗仍回結果（使用者已等完 AI、已計次），只是下次不命中快取。
            let inserted = db::exec(
                &db,
                "INSERT INTO ziwei_year_readings \
                 (id,user_id,luck_year,engine_version_ziwei,horoscope_version,prompt_version,ai_interpretation,ai_provider,ai_model,created_at) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                &[
                    &db::text(&uuid::random_uuid()),
                    &db::text(&user),
                    &db::int(h.luck_year),
                    &db::text(ENGINE_VERSION_ZIWEI),
                    &db::text(ENGINE_VERSION_ZIWEI_HOROSCOPE),
                    &db::text(ZIWEI_YEAR_PROMPT_VERSION),
                    &db::text(&ok.interpretation),
                    &db::opt_text(Some(ok.provider.as_str())),
                    &db::opt_text(Some(ok.model.as_str())),
                    &db::text(&clock::now_iso()),
                ],
            )
            .await;
            if inserted.is_err() {
                console_error!("ziwei year cache insert failed");
            }
            Ok(no_store_json(&ZiweiYearReading {
                luck_year: h.luck_year,
                interpretation: with_closing(&ok.interpretation),
                provider: Some(ok.provider),
                model: Some(ok.model),
                cached: false,
            }))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ft_schema::ziwei_luck::{
        BranchName, HoroscopeAge, HoroscopeDecadal, HoroscopeMutagen, HoroscopeYearly,
    };
    use ft_schema::{
        BirthInfoV3, LunarDate, SolarDate, StemBranch, StemBranchX4, ZiWeiMeta, ZiWeiPalaceV3,
        ZiWeiStarV3,
    };

    fn long_ok_text() -> String {
        "今年可以多留意的主題，傾向回到生活的節奏與自己的需要。".repeat(10)
    }

    #[test]
    fn output_check_rules() {
        let good = long_ok_text();
        assert!(output_ok(&good));
        for bad in ["一定", "注定", "必然", "保證", "預測", "本週"] {
            assert!(!output_ok(&format!("{good}{bad}")), "{bad}");
        }
        for m in ["三月", "十二月", "3月", "１２月", "正月三月"] {
            assert!(!output_ok(&format!("{good}{m}會有變化")), "{m}");
        }
        // 月字不接數字 → 不算月份
        assert!(output_ok(&format!("{good}歲月與月亮")));
        // 去空白後少於 200 字
        let short = "今年可以多留意的主題。".repeat(5);
        assert!(!output_ok(&short));
        let padded = format!("{}{}", "  \n".repeat(300), "短");
        assert!(!output_ok(&padded));
    }

    #[test]
    fn closing_is_appended_once() {
        let t = with_closing("內文\n\n");
        assert!(t.ends_with(ZIWEI_YEAR_CLOSING));
        assert_eq!(t.matches(ZIWEI_YEAR_CLOSING).count(), 1);
        assert!(t.starts_with("內文\n\n流年"));
    }

    #[test]
    fn year_param_and_ranges() {
        assert_eq!(parse_year_param(None), Ok(None));
        assert_eq!(parse_year_param(Some("2026")), Ok(Some(2026)));
        assert!(parse_year_param(Some("abc")).is_err());
        assert!(parse_year_param(Some("2026.5")).is_err());
        assert!(year_roughly_in_range(1989, 1990));
        assert!(year_roughly_in_range(2091, 1990));
        assert!(!year_roughly_in_range(1988, 1990));
        assert!(!year_roughly_in_range(2092, 1990));
        let mut h = horoscope_fixture(false);
        assert!(year_exactly_in_range(2026, &h));
        assert!(
            !year_exactly_in_range(2025, &h),
            "luckYear 不符＝早於出生農曆年"
        );
        h.nominal_age = 102;
        assert!(!year_exactly_in_range(2026, &h));
    }

    #[test]
    fn unpad_dates() {
        assert_eq!(unpad_date("2026-01-05"), Some("2026-1-5".to_string()));
        assert_eq!(unpad_date("2026-11-21"), Some("2026-11-21".to_string()));
        assert_eq!(unpad_date("2026-1"), None);
        assert_eq!(unpad_date("x-1-1"), None);
    }

    fn star(name: &str, t: &str, b: Option<&str>, sihua: Option<&str>) -> ZiWeiStarV3 {
        ZiWeiStarV3 {
            name: name.into(),
            star_type: t.into(),
            brightness: b.map(Into::into),
            sihua: sihua.map(Into::into),
        }
    }

    fn chart_fixture() -> ZiWeiChartV3 {
        const BR: [&str; 12] = [
            "子", "丑", "寅", "卯", "辰", "巳", "午", "未", "申", "酉", "戌", "亥",
        ];
        const NAMES: [&str; 12] = [
            "父母", "福德", "田宅", "官祿", "僕役", "遷移", "疾厄", "財帛", "子女", "夫妻", "兄弟",
            "命宮",
        ];
        let palaces = (0..12)
            .map(|i| ZiWeiPalaceV3 {
                index: i as u8,
                name: NAMES[i].into(),
                branch: BR[i].into(),
                stem: "甲".into(),
                stars: match i {
                    2 => vec![star("廉貞", "main", Some("廟"), None)],
                    5 => vec![
                        star("天同", "main", Some("廟"), Some("ji")),
                        star("地空", "auxiliary", None, None),
                    ],
                    _ => vec![],
                },
                is_life_palace: Some(i == 11),
                is_body_palace: Some(false),
            })
            .collect();
        let sb = || StemBranch {
            stem: "庚".into(),
            branch: "午".into(),
        };
        ZiWeiChartV3 {
            birth_info: BirthInfoV3 {
                solar: SolarDate {
                    year: 1990,
                    month: 5,
                    day: 15,
                },
                lunar: LunarDate {
                    year: 1990,
                    month: 4,
                    day: 21,
                    is_leap: Some(false),
                },
                hour: 6,
                hour_branch: "午".into(),
                gender: "男".into(),
            },
            four_pillars: StemBranchX4 {
                year: sb(),
                month: sb(),
                day: sb(),
                hour: sb(),
            },
            five_element: "土五局".into(),
            life_palace_index: 11,
            body_palace_index: 11,
            palaces,
            major_limits: vec![],
            meta: ZiWeiMeta {
                day_divide: "forward".into(),
                is_leap: false,
                fix_leap: true,
                time_index: 6,
                engine_version_ziwei: "4.0.0".into(),
                chart_schema_version: 3,
            },
        }
    }

    fn horoscope_fixture(childhood: bool) -> ZiWeiHoroscopeV1 {
        ZiWeiHoroscopeV1 {
            target_date: "2026-7-1".into(),
            luck_year: 2026,
            nominal_age: 37,
            decadal: HoroscopeDecadal {
                palace_index: 2,
                branch: "寅".into(),
                stem: "戊".into(),
                start_age: if childhood { None } else { Some(35) },
                end_age: if childhood { None } else { Some(44) },
                is_childhood: childhood,
            },
            age: HoroscopeAge {
                palace_index: 4,
                branch: "辰".into(),
                stem: "庚".into(),
            },
            yearly: HoroscopeYearly {
                palace_index: 6,
                branch: "午".into(),
                stem: "丙".into(),
                palace_names: vec![BranchName {
                    branch: "午".into(),
                    name: "命宮".into(),
                }],
                mutagen: HoroscopeMutagen {
                    lu: "天同".into(),
                    quan: "天機".into(),
                    ke: "文昌".into(),
                    ji: "廉貞".into(),
                },
            },
        }
    }

    #[test]
    fn chart_data_layers_without_birth_info() {
        let v = build_chart_data(&chart_fixture(), &horoscope_fixture(false));
        assert_eq!(v["natal"]["gender"], "男");
        assert_eq!(v["natal"]["fiveElement"], "土五局");
        let yi = &v["natal"]["palaces"][5];
        assert_eq!(yi["name"], "遷移");
        let stars = yi["stars"].as_array().unwrap();
        assert_eq!(stars.len(), 1, "只送主星與帶生年四化的星");
        assert_eq!(stars[0]["sihua"], "ji");
        assert_eq!(v["decadal"]["natalPalace"], "田宅");
        assert_eq!(v["decadal"]["startAge"], 35);
        assert_eq!(v["yearly"]["natalPalace"], "疾厄");
        assert_eq!(v["yearly"]["mutagen"][0]["label"], "流祿");
        assert_eq!(v["yearly"]["mutagen"][0]["natalPalace"], "遷移");
        assert_eq!(v["yearly"]["mutagen"][3]["natalPalace"], "田宅");
        assert_eq!(v["yearly"]["mutagen"][1]["natalPalace"], "", "本命無此星");
        let s = v.to_string();
        for banned in [
            "birth",
            "solar",
            "lunar",
            "hour",
            "nominalAge",
            "luckYear",
            "1990",
            "latitude",
        ] {
            assert!(!s.contains(banned), "chartData 不得含 {banned}");
        }
        let c = build_chart_data(&chart_fixture(), &horoscope_fixture(true));
        assert_eq!(c["decadal"]["isChildhood"], true);
        assert!(c["decadal"].get("startAge").is_none());
    }
}
