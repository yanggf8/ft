//! ft-worker — Phase A engine Worker, exposed via service binding to the TS Worker.
//! Entry points mirror what the TS charts.ts route needs:
//!   GET /engine/ziwei   -> ft-ziwei.calculate()
//!   GET /engine/western -> ft-western.calculate()

use serde::Deserialize;
use worker::*;

#[derive(Deserialize)]
struct ZiweiQuery {
    date: String,
    hour: u8,
    gender: Option<String>,
    #[serde(rename = "fixLeap")]
    fix_leap: Option<bool>,
}

#[derive(Deserialize)]
struct WesternQuery {
    #[serde(rename = "jdUtc")]
    jd_utc: f64,
    lat: Option<f64>,
    lon: Option<f64>,
}

#[event(fetch)]
async fn fetch(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let url = req.url()?;
    // Normalize duplicate slashes (service bindings may forward `//engine/...`).
    let path = url.path().replace("//", "/");
    match path.as_str() {
        "/engine/ziwei" => handle_ziwei(&req).await,
        "/engine/western" => handle_western(&req).await,
        "/health" => {
            Response::from_json(&serde_json::json!({ "status": "ok", "engine": "ft-worker" }))
        }
        _ => Response::error("not found", 404),
    }
}

fn cors_headers(res: &mut Response) -> Result<()> {
    res.headers_mut()
        .append("Access-Control-Allow-Origin", "*")?;
    Ok(())
}

async fn handle_ziwei(req: &Request) -> Result<Response> {
    let q: ZiweiQuery = match req.query() {
        Ok(q) => q,
        Err(_) => {
            let mut r = Response::from_json(&serde_json::json!({
                "error": "missing required params: date, hour",
                "code": "MISSING_PARAMS"
            }))?;
            r = r.with_status(400);
            return Ok(r);
        }
    };
    let gender = q.gender.unwrap_or_else(|| "male".to_string());
    let fix_leap = q.fix_leap.unwrap_or(true);
    let time_index = ft_ziwei::hour_to_time_index(q.hour);
    match ft_ziwei::calculate(&q.date, time_index, &gender, fix_leap) {
        Ok(chart) => {
            let mut r = Response::from_json(&serde_json::json!({
                "chart": chart,
                "engineVersionZiwei": "4.0.0"
            }))?;
            cors_headers(&mut r)?;
            Ok(r)
        }
        Err(e) => {
            let mut r = Response::from_json(&serde_json::json!({ "error": e }))?;
            r = r.with_status(500);
            Ok(r)
        }
    }
}

async fn handle_western(req: &Request) -> Result<Response> {
    let q: WesternQuery = match req.query() {
        Ok(q) => q,
        Err(_) => {
            let mut r = Response::from_json(&serde_json::json!({
                "error": "missing required params: jdUtc",
                "code": "MISSING_PARAMS"
            }))?;
            r = r.with_status(400);
            return Ok(r);
        }
    };
    let lat = q.lat.unwrap_or(25.0);
    let lon = q.lon.unwrap_or(121.5);
    // A non-finite jdUtc (NaN/Inf, e.g. a failed tz conversion on the caller side)
    // must not reach the ephemeris math — it panics and produces a 1101 exception.
    if !q.jd_utc.is_finite() {
        let mut r = Response::from_json(&serde_json::json!({
            "error": "invalid jdUtc",
            "code": "INVALID_JD"
        }))?;
        r = r.with_status(400);
        return Ok(r);
    }
    let chart = ft_western::calculate(q.jd_utc, lat, lon);
    let mut r = Response::from_json(&serde_json::json!({
        "chart": chart,
        "engineVersionWestern": "4.0.0"
    }))?;
    cors_headers(&mut r)?;
    Ok(r)
}
