//! Typed chart resolution for the overlay endpoint (spec 2026-09-07-f2-f3 §1.4).
//! Read-only: a fresh cache row is read; absent/stale/invalid is recomputed via
//! the FT_ENGINE binding but NOT persisted — the chart route keeps its own write
//! path (rev.3 ruling: duplicate computation is acceptable in v1).
//!
//! Red line (spec §0.4): this module must not be imported by
//! `services/predictions.rs` or `services::ai`.

use crate::services::{db, engine, engine_version};
use ft_schema::{WesternChartV3, ZiWeiChartV3};
use serde_json::Value;
use worker::RouteContext;

/// 出生資料是否齊備與嘗試順序(spec §1.4)。gender 缺失 → 跳過紫微、不發明預設
/// (engine client 會靜默 default male;Codex [3])。這是「嘗試資格」——紫微可用
/// 即早返回,西洋僅作 fallback(Codex [P11])。
pub(crate) fn resolve_plan(has_birth: bool, has_gender: bool) -> (bool, bool) {
    (has_birth && has_gender, has_birth)
}

/// 新鮮快取的 Value 層檢查(Codex [P10]):缺 raw / 壞 JSON / 列上無雜湊 /
/// 雜湊不合 / meta 缺 key / engine 或 schema 版本不合 → false。
/// typed 反序列化只對通過者執行(`WesternChartV3` 結構體無 meta 欄位,
/// 新鮮度必須在 Value 層判)。
pub(crate) fn cached_payload_current(
    raw: Option<&str>,
    stored_birth_hash: Option<&str>,
    birth_hash: &str,
    engine_version_key: &str,
    expected_engine_version: &str,
    expected_schema_version: u32,
) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    let Some(stored) = stored_birth_hash else {
        return false;
    };
    if stored != birth_hash {
        return false;
    }
    let ev = v
        .get("meta")
        .and_then(|m| m.get(engine_version_key))
        .and_then(Value::as_str);
    let sv = v
        .get("meta")
        .and_then(|m| m.get("chartSchemaVersion"))
        .and_then(Value::as_u64);
    ev == Some(expected_engine_version) && sv == Some(expected_schema_version as u64)
}

pub(crate) struct ResolvedCharts {
    pub ziwei: Option<ZiWeiChartV3>,
    pub western: Option<WesternChartV3>,
    /// 使用者是否已填生辰 — UI 以此區分「未填生辰」與「命盤暫時無法計算」
    /// 兩態(spec §4 不得共用文案;Codex [3])。
    pub birth_known: bool,
}

#[derive(serde::Deserialize)]
struct BirthRow {
    birth_year: Option<i64>,
    birth_month: Option<i64>,
    birth_day: Option<i64>,
    birth_hour: Option<i64>,
    gender: Option<String>,
    timezone: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    birth_data_hash: Option<String>,
}

#[derive(serde::Deserialize)]
struct CacheRow {
    chart_data: Option<String>,
    birth_data_hash: Option<String>,
}

/// meta 內嵌(engine 版本 + schema 版本),與 `routes::common::embed_meta` 等價;
/// 內聯以免 services → routes 的可見性耦合。
fn with_meta(mut chart: Value, engine_version_key: &str, engine_version: &str) -> Value {
    if !chart.get("meta").map(|m| m.is_object()).unwrap_or(false) {
        chart["meta"] = Value::Object(serde_json::Map::new());
    }
    if let Some(meta) = chart.get_mut("meta").and_then(|m| m.as_object_mut()) {
        meta.insert(
            engine_version_key.to_string(),
            Value::String(engine_version.to_string()),
        );
        meta.insert(
            "chartSchemaVersion".to_string(),
            serde_json::json!(engine_version::CHART_SCHEMA_VERSION),
        );
    }
    chart
}

/// 紫微盤結構有效性(Codex 二輪 #2):宮位非空且命宮 index 真有對應宮;
/// 空宮借對宮是合法路徑,但「整個 palaces 空陣列」會產出全 50 的假 prior。
fn ziwei_chart_structurally_valid(c: &ZiWeiChartV3) -> bool {
    !c.palaces.is_empty() && c.palaces.iter().any(|p| p.index == c.life_palace_index)
}

/// 西洋盤結構有效性:三個太陽/月亮/上升星座名皆非空。
fn western_chart_structurally_valid(c: &WesternChartV3) -> bool {
    !c.sun_sign.name.is_empty() && !c.moon_sign.name.is_empty() && !c.ascendant.sign.is_empty()
}

/// 解析單一盤種:新鮮快取 → typed+結構驗證通過才採用;否則 engine 現算
/// (不落快取,rev.3)。engine/parse 失敗或結構無效 = 該盤不可用(None);
/// DB 失敗 = Err(5xx 語意留給 route)。
async fn resolve_one<T: serde::de::DeserializeOwned>(
    db: &db::Turso,
    fetcher: &worker::Fetcher,
    user_id: &str,
    div_type: &str,
    engine_version_key: &str,
    expected_engine_version: &str,
    birth_hash: &str,
    birth: &BirthRow,
    validate: impl Fn(&T) -> bool,
) -> Result<Option<T>, worker::Error> {
    let uid = db::text(user_id);
    let dt = db::text(div_type);
    let bh = db::opt_text(Some(birth_hash));
    let cached: Option<CacheRow> = db::first(
        db,
        "SELECT chart_data, birth_data_hash FROM interpretations \
         WHERE user_id = ?1 AND divination_type = ?2 AND birth_data_hash = ?3",
        &[&uid, &dt, &bh],
    )
    .await?;
    if let Some(c) = &cached {
        if cached_payload_current(
            c.chart_data.as_deref(),
            c.birth_data_hash.as_deref(),
            birth_hash,
            engine_version_key,
            expected_engine_version,
            engine_version::CHART_SCHEMA_VERSION,
        ) {
            if let Some(raw) = c.chart_data.as_deref() {
                if let Ok(parsed) = serde_json::from_str::<T>(raw) {
                    if validate(&parsed) {
                        return Ok(Some(parsed));
                    }
                }
            }
        }
    }
    let eb = engine::EngineBirth {
        year: birth.birth_year,
        month: birth.birth_month,
        day: birth.birth_day,
        hour: birth.birth_hour.unwrap_or(12),
        gender: birth.gender.clone(),
        // charts.rs 對 ziwei 不帶經緯度 — resolver 必須同款,否則 overlay 的盤
        // 會與命盤頁悄悄分歧(Kimi 終審 #3)
        latitude: if div_type == "ziwei" {
            None
        } else {
            birth.latitude
        },
        longitude: if div_type == "ziwei" {
            None
        } else {
            birth.longitude
        },
        timezone: birth.timezone.clone(),
    };
    // engine 失敗 = 該盤不可用(None),絕不外拋 — spec §1.4:降級,不 5xx;
    // 紫微失敗才輪到西洋 fallback(Kimi 終審 #1)。
    // 非物件 payload(如 {"chart":[]})在 IndexMut 會 panic — 先擋(Codex 二輪 #1)
    let raw = match engine::fetch_engine_chart(fetcher, div_type, &eb).await {
        Ok(v) if v.is_object() => with_meta(v, engine_version_key, expected_engine_version),
        Ok(_) => return Ok(None),
        Err(_) => return Ok(None),
    };
    // 結構無效視同該盤不可用(Codex 二輪 #2)
    let Ok(parsed) = serde_json::from_value::<T>(raw) else {
        return Ok(None);
    };
    if !validate(&parsed) {
        return Ok(None);
    }
    Ok(Some(parsed))
}

/// 解析紫微/西洋命盤(spec §1.4)。使用者列不存在 → birth_known = false;
/// DB 失敗 → Err(worker::Error)(route 映射為 5xx,不外洩細節)。
pub(crate) async fn resolve_chart(
    ctx: &RouteContext<()>,
    user_id: &str,
) -> Result<ResolvedCharts, worker::Error> {
    let db = db::Turso::from_env(&ctx.env)?;
    let uid = db::text(user_id);
    let birth: Option<BirthRow> = db::first(
        &db,
        "SELECT birth_year, birth_month, birth_day, birth_hour, gender, timezone, \
         latitude, longitude, birth_data_hash FROM users WHERE id = ?1",
        &[&uid],
    )
    .await?;
    let Some(birth) = birth else {
        return Ok(ResolvedCharts {
            ziwei: None,
            western: None,
            birth_known: false,
        });
    };
    let has_birth =
        birth.birth_year.is_some() && birth.birth_month.is_some() && birth.birth_day.is_some();
    let has_gender = birth.gender.is_some();
    let birth_known = has_birth;
    let (try_ziwei, try_western) = resolve_plan(has_birth, has_gender);
    let birth_hash = birth.birth_data_hash.clone().unwrap_or_default();
    let fetcher = ctx.env.service("FT_ENGINE")?;

    if try_ziwei {
        if let Some(z) = resolve_one::<ZiWeiChartV3>(
            &db,
            &fetcher,
            user_id,
            "ziwei",
            "engineVersionZiwei",
            engine_version::ENGINE_VERSION_ZIWEI,
            &birth_hash,
            &birth,
            ziwei_chart_structurally_valid,
        )
        .await?
        {
            return Ok(ResolvedCharts {
                ziwei: Some(z),
                western: None,
                birth_known,
            });
        }
    }
    let western = if try_western {
        resolve_one::<WesternChartV3>(
            &db,
            &fetcher,
            user_id,
            "western",
            "engineVersionWestern",
            engine_version::ENGINE_VERSION_WESTERN,
            &birth_hash,
            &birth,
            western_chart_structurally_valid,
        )
        .await?
    } else {
        None
    };
    Ok(ResolvedCharts {
        ziwei: None,
        western,
        birth_known,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usable_requires_hash_and_versions_aligned() {
        assert!(chart_usable_inline("h1", "h1", "4.0.0", "4.0.0", 3, 3));
        assert!(!chart_usable_inline("h1", "h2", "4.0.0", "4.0.0", 3, 3)); // birth 變了
        assert!(!chart_usable_inline("h1", "h1", "3.0.0", "4.0.0", 3, 3)); // engine 過期
        assert!(!chart_usable_inline("h1", "h1", "4.0.0", "4.0.0", 2, 3)); // schema 過期
    }

    /// `cached_payload_current` 的 typed 常量版;兩者語意必須一致。
    fn chart_usable_inline(
        stored_birth_hash: &str,
        birth_hash: &str,
        stored_engine_version: &str,
        engine_version: &str,
        stored_schema_version: u32,
        schema_version: u32,
    ) -> bool {
        stored_birth_hash == birth_hash
            && stored_engine_version == engine_version
            && stored_schema_version == schema_version
    }

    #[test]
    fn plan_skips_ziwei_without_gender() {
        assert_eq!(resolve_plan(true, true), (true, true));
        assert_eq!(resolve_plan(true, false), (false, true)); // 不發明 gender
        assert_eq!(resolve_plan(false, true), (false, false)); // 無生辰 -> 全跳
        assert_eq!(resolve_plan(false, false), (false, false));
    }

    const FRESH_ZIWEI: &str = r#"{"meta":{"engineVersionZiwei":"4.0.0","chartSchemaVersion":3}}"#;

    #[test]
    fn cached_payload_fresh_stale_malformed_absent() {
        assert!(cached_payload_current(
            Some(FRESH_ZIWEI),
            Some("h1"),
            "h1",
            "engineVersionZiwei",
            "4.0.0",
            3
        ));
        assert!(!cached_payload_current(
            Some(FRESH_ZIWEI),
            Some("h1"),
            "h2",
            "engineVersionZiwei",
            "4.0.0",
            3
        )); // 雜湊不合
        assert!(!cached_payload_current(
            Some(FRESH_ZIWEI),
            None,
            "h1",
            "engineVersionZiwei",
            "4.0.0",
            3
        )); // 列上無雜湊 = miss
        assert!(!cached_payload_current(
            Some(r#"{"meta":{}}"#),
            Some("h1"),
            "h1",
            "engineVersionZiwei",
            "4.0.0",
            3
        )); // 缺 key
        assert!(!cached_payload_current(
            Some(r#"{"meta":{"engineVersionZiwei":"3.0.0","chartSchemaVersion":3}}"#),
            Some("h1"),
            "h1",
            "engineVersionZiwei",
            "4.0.0",
            3
        )); // 版本舊
        assert!(!cached_payload_current(
            Some("{not json"),
            Some("h1"),
            "h1",
            "engineVersionZiwei",
            "4.0.0",
            3
        )); // 壞 JSON
        assert!(!cached_payload_current(
            None,
            Some("h1"),
            "h1",
            "engineVersionZiwei",
            "4.0.0",
            3
        )); // 缺 raw
    }
}
