//! Engine service-binding client — mirrors backend/src/routes/charts.ts helpers
//! `fetchEngineChart` and `jdFromBirth`.
//!
//! The Rust `fortunet-engine` Worker is reached via the FT_ENGINE service binding.
//! Its `Fetcher.fetch(url)` forwards path + query (host is ignored, per service
//! bindings); the URL must be a single slash (the engine normalizes `//` too).

use wasm_bindgen::JsCast;
use worker::*;

/// A minimal birth-data view used to build the engine query. Matches the TS
/// `EngineBirth` interface in charts.ts.
#[derive(Debug, Clone)]
pub struct EngineBirth {
    pub year: Option<i64>,
    pub month: Option<i64>,
    pub day: Option<i64>,
    pub hour: i64,
    pub gender: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
}

/// Call the engine for a chart type and return the unwrapped `chart` object.
pub async fn fetch_engine_chart(
    fetcher: &Fetcher,
    chart_type: &str,
    birth: &EngineBirth,
) -> Result<serde_json::Value> {
    let date = format!(
        "{}-{:02}-{:02}",
        birth.year.unwrap_or(2000),
        birth.month.unwrap_or(1),
        birth.day.unwrap_or(1)
    );
    let url = match chart_type {
        "ziwei" => {
            let gender = birth.gender.as_deref().unwrap_or("male");
            format!(
                "/engine/ziwei?date={}&hour={}&gender={}&fixLeap=true",
                urlencode(&date),
                birth.hour,
                urlencode(gender)
            )
        }
        "western" => {
            let jd = jd_from_birth(birth);
            let lat = birth.latitude.unwrap_or(25.0);
            let lon = birth.longitude.unwrap_or(121.5);
            format!("/engine/western?jdUtc={}&lat={}&lon={}", jd, lat, lon)
        }
        _ => return Err(worker::Error::from("unknown chart type")),
    };

    // Service binding: host ignored; ensure single leading slash.
    let full_url = if url.starts_with('/') {
        format!("https://ft-engine{}", url)
    } else {
        format!("https://ft-engine/{}", url)
    };

    let mut res = fetcher.fetch(full_url, None).await?;
    let status = res.status_code();
    if status != 200 {
        let body = res.text().await.unwrap_or_default();
        let preview: String = body.chars().take(200).collect();
        return Err(worker::Error::from(format!(
            "ft-engine {} failed: {} {}",
            chart_type, status, preview
        )));
    }
    let data: serde_json::Value = res.json().await?;
    let chart = if let Some(c) = data.get("chart") {
        c.clone()
    } else {
        return Err(worker::Error::from(format!(
            "ft-engine {}: no chart in response {:?}",
            chart_type,
            &data.to_string().chars().take(200).collect::<String>()
        )));
    };
    Ok(chart)
}

/// `jdFromBirth` — converts local civil date + hour + IANA timezone to a Julian Day (UT)
/// via `Intl` tz-offset reversal, then Fliegel–Van Flandern. Reproduces charts.ts.
pub fn jd_from_birth(birth: &EngineBirth) -> f64 {
    let y = birth.year.unwrap_or(2000);
    let m = birth.month.unwrap_or(1);
    let d = birth.day.unwrap_or(1);
    let h = birth.hour;
    let tz = birth.timezone.as_deref().unwrap_or("Asia/Taipei");

    // Build a UTC instant guessed as if the local wall-clock were UTC (padded 24→0).
    let local_as_utc = date_utc_ms(y, m, d, if h == 24 { 0 } else { h }, 0, 0);

    // Ask Intl what time this instant has in `tz`, and measure the offset it reports.
    let offset_ms = tz_offset_ms(tz, local_as_utc);
    // If Intl failed (NaN) — e.g. an unknown tz — fall back to treating the local
    // wall-clock as UTC (the TS code did the same via its catch → offsetMs = 0).
    let offset_ms = if offset_ms.is_nan() { 0.0 } else { offset_ms };

    let utc_ms = local_as_utc - offset_ms;
    let u = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(utc_ms));
    let yy = u.get_utc_full_year() as i64;
    let mm = u.get_utc_month() as i64 + 1;
    let dd = u.get_utc_date() as i64;
    let hh = u.get_utc_hours() as i64;

    // Fliegel–Van Flandern
    let a = (14 - mm) / 12;
    let yyq = yy + 4800 - a;
    let mmq = mm + 12 * a - 3;
    let jdn = dd + (153 * mmq + 2) / 5 + 365 * yyq + yyq / 4 - yyq / 100 + yyq / 400 - 32045;
    jdn as f64 + (hh as f64 - 12.0) / 24.0
}

/// `Date.UTC(y, mo, d, h, mi, se)` via js_sys Reflect — matches the TS `Date.UTC`
/// call byte-for-byte. `m` is 1-based; JS UTC is 0-based. Returns epoch ms.
///
/// NB: `Date.UTC` is a STATIC method on the `Date` constructor, so we grab it as a
/// property (`.UTC`) and call it with the global as `this` — NOT the constructor
/// itself (calling `Date(...)` returns a Date object, not a number).
#[allow(clippy::too_many_arguments)]
fn date_utc_ms(y: i64, m: i64, d: i64, h: i64, mi: i64, se: i64) -> f64 {
    let global = js_sys::global();
    let date_ctor = js_sys::Reflect::get(&global, &"Date".into()).unwrap_or_default();
    let utc_fn = js_sys::Reflect::get(&date_ctor, &"UTC".into()).unwrap_or_default();
    let utc_fn: js_sys::Function = utc_fn.dyn_into().unwrap_or_default();
    let args = js_sys::Array::new();
    args.push(&(y as f64).into());
    args.push(&((m - 1) as f64).into());
    args.push(&(d as f64).into());
    args.push(&(h as f64).into());
    args.push(&(mi as f64).into());
    args.push(&(se as f64).into());
    let result = js_sys::Reflect::apply(&utc_fn, &global, &args).unwrap_or_default();
    result.as_f64().unwrap_or(f64::NAN)
}

/// Replicates the TS `Intl.DateTimeFormat('en-US', {...}).formatToParts(...)` offset
/// measurement. Returns `localAsUtc - localAsUtc2` (ms).
fn tz_offset_ms(tz: &str, local_as_utc: f64) -> f64 {
    // Build options object: { timeZone, year, month, day, hour, minute, second, hour12:false }
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"timeZone".into(), &tz.into());
    let _ = js_sys::Reflect::set(&opts, &"year".into(), &"numeric".into());
    let _ = js_sys::Reflect::set(&opts, &"month".into(), &"2-digit".into());
    let _ = js_sys::Reflect::set(&opts, &"day".into(), &"2-digit".into());
    let _ = js_sys::Reflect::set(&opts, &"hour".into(), &"2-digit".into());
    let _ = js_sys::Reflect::set(&opts, &"minute".into(), &"2-digit".into());
    let _ = js_sys::Reflect::set(&opts, &"second".into(), &"2-digit".into());
    let _ = js_sys::Reflect::set(&opts, &"hour12".into(), &false.into());

    let locales = js_sys::Array::new();
    locales.push(&"en-US".into());

    let dtf = js_sys::Intl::DateTimeFormat::new(&locales, &opts);
    let parts = dtf.format_to_parts(&js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(
        local_as_utc,
    )));

    // Build {year, month, day, hour, minute, second} from the parts array.
    let mut year = 0f64;
    let mut month = 1f64;
    let mut day = 1f64;
    let mut hour = 0f64;
    let mut minute = 0f64;
    let mut second = 0f64;
    let len = parts.length();
    for idx in 0..len {
        let v = parts.get(idx);
        let typ = js_sys::Reflect::get(&v, &"type".into())
            .ok()
            .and_then(|x| x.as_string())
            .unwrap_or_default();
        let val = js_sys::Reflect::get(&v, &"value".into())
            .ok()
            .and_then(|x| x.as_string())
            .unwrap_or_default();
        match typ.as_str() {
            "year" => year = val.parse().unwrap_or(0.0),
            "month" => month = val.parse().unwrap_or(1.0),
            "day" => day = val.parse().unwrap_or(1.0),
            "hour" => {
                hour = if val == "24" {
                    0.0
                } else {
                    val.parse().unwrap_or(0.0)
                }
            }
            "minute" => minute = val.parse().unwrap_or(0.0),
            "second" => second = val.parse().unwrap_or(0.0),
            _ => {}
        }
    }

    // TS: `Date.UTC(parts.year, parts.month-1, parts.day, localHour, parts.minute, parts.second)`.
    let local_as_utc2 = date_utc_ms(
        year as i64,
        month as i64,
        day as i64,
        hour as i64,
        minute as i64,
        second as i64,
    );
    local_as_utc2 - local_as_utc
}

/// Percent-encode for a query value (kept simple; inputs are ASCII-safe dates/gender).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}
