//! Users routes — mirrors backend/src/routes/users.ts.
//! /me GET, /me/birth PUT, /me PUT. Auth middleware guards each handler.

use worker::*;

use super::super::error;
use super::super::services::{billing, birth_hash, db};
use super::common::{apply_cache_headers, auth_user, ok_json};
use super::R;

#[derive(Debug, serde::Deserialize)]
struct UserRow {
    id: String,
    email: String,
    full_name: Option<String>,
    avatar_url: Option<String>,
    birth_year: Option<i64>,
    birth_month: Option<i64>,
    birth_day: Option<i64>,
    birth_hour: Option<i64>,
    birth_minute: Option<i64>,
    gender: Option<String>,
    timezone: Option<String>,
    subscription_tier: Option<String>,
    trial_ends_at: Option<String>,
    created_at: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    generation_tags: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct BirthBody {
    #[serde(default)]
    birth_year: Option<i64>,
    #[serde(default)]
    birth_month: Option<i64>,
    #[serde(default)]
    birth_day: Option<i64>,
    #[serde(default)]
    birth_hour: Option<i64>,
    #[serde(default)]
    birth_minute: Option<i64>,
    #[serde(default)]
    gender: Option<String>,
    #[serde(default)]
    timezone: Option<String>,
    #[serde(default)]
    latitude: Option<f64>,
    #[serde(default)]
    longitude: Option<f64>,
    #[serde(default)]
    generation_tags: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
struct ProfileBody {
    #[serde(default)]
    full_name: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct MinimalUser {
    id: String,
    email: String,
    full_name: Option<String>,
    avatar_url: Option<String>,
    subscription_tier: String,
    created_at: Option<String>,
}

pub fn register(router: R<'static>) -> R<'static> {
    router
        .get_async("/api/users/me", |req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(resp) => return Ok(resp),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let u = db::text(&user);
            let row: Option<UserRow> = match db::first(
                &db,
                "SELECT id, email, full_name, avatar_url, birth_year, birth_month, birth_day, birth_hour, birth_minute, gender, timezone, subscription_tier, trial_ends_at, created_at, role, generation_tags FROM users WHERE id = ?1",
                &[&u],
            ).await {
                Ok(r) => r,
                Err(e) => return Ok(error::error(format!("db error: {}", e), 500)),
            };
            let row = match row {
                Some(r) => r,
                None => return Ok(error::error("User not found", 404)),
            };

            let tier = row.subscription_tier.clone().unwrap_or_else(|| "free".to_string());
            let trial_ends_at = row.trial_ends_at.clone();
            let billing_info = billing::check_user_access(&tier, trial_ends_at.as_deref());
            let has_birth_data = row.birth_year.is_some() && row.birth_month.is_some() && row.birth_day.is_some();
            let is_admin = {
                // hesocial-style: DB role takes precedence, env-var is bootstrap fallback
                if let Some(role) = row.role.as_deref() {
                    if role == "admin" || role == "super_admin" {
                        true
                    } else {
                        let admin = ctx
                            .env
                            .var("ADMIN_EMAIL")
                            .map(|v| v.to_string())
                            .unwrap_or_default();
                        crate::routes::admin_invites::is_admin_email(&admin, &row.email)
                    }
                } else {
                    let admin = ctx
                        .env
                        .var("ADMIN_EMAIL")
                        .map(|v| v.to_string())
                        .unwrap_or_default();
                    crate::routes::admin_invites::is_admin_email(&admin, &row.email)
                }
            };

            let mut res = ok_json(
                &serde_json::json!({
                    "id": row.id, "email": row.email, "full_name": row.full_name, "avatar_url": row.avatar_url,
                    "birth_year": row.birth_year, "birth_month": row.birth_month, "birth_day": row.birth_day,
                    "birth_hour": row.birth_hour, "birth_minute": row.birth_minute, "gender": row.gender,
                    "timezone": row.timezone, "generation_tags": row.generation_tags.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()).unwrap_or(serde_json::Value::Null),
                    "subscription_tier": tier, "trial_ends_at": trial_ends_at,
                    "created_at": row.created_at, "billing": billing_info, "hasBirthData": has_birth_data,
                    "isAdmin": is_admin,
                }),
                200,
            );
            apply_cache_headers(&mut res, 300, false);
            Ok(res)
        })
        .put_async("/api/users/me/birth", |mut req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(resp) => return Ok(resp),
            };
            let body: BirthBody = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error("Invalid JSON", 400)),
            };
            let (Some(by), Some(bm), Some(bd)) = (body.birth_year, body.birth_month, body.birth_day) else {
                return Ok(error::error("birth_year, birth_month, birth_day required", 400));
            };
            if !(1900..=2100).contains(&by) {
                return Ok(error::error("birth_year must be 1900-2100", 400));
            }
            if !(1..=12).contains(&bm) {
                return Ok(error::error("birth_month must be 1-12", 400));
            }
            if !(1..=31).contains(&bd) {
                return Ok(error::error("birth_day must be 1-31", 400));
            }
            let is_leap_year = by % 4 == 0 && (by % 100 != 0 || by % 400 == 0);
            let days_in_month = match bm {
                2 if is_leap_year => 29,
                2 => 28,
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            };
            if bd > days_in_month {
                return Ok(error::error("Invalid date", 400));
            }
            if let Some(h) = body.birth_hour {
                if !(0..=23).contains(&h) {
                    return Ok(error::error("birth_hour must be 0-23", 400));
                }
            }

            let hash = birth_hash::compute_birth_hash(&birth_hash::BirthHashInput {
                birth_year: body.birth_year,
                birth_month: body.birth_month,
                birth_day: body.birth_day,
                birth_hour: body.birth_hour.or(Some(12)),
                birth_minute: body.birth_minute.or(Some(0)),
                gender: body.gender.clone(),
                timezone: body.timezone.clone(),
                latitude: body.latitude,
                longitude: body.longitude,
                generation_tags: body.generation_tags.clone(),
            });

            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let timezone = body.timezone.clone().unwrap_or_else(|| "Asia/Taipei".to_string());
            let gen_tags = body.generation_tags.as_ref().map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()));
            let gen_t = db::opt_text(gen_tags.as_deref());
            let by_t = db::opt_int(body.birth_year);
            let bm_t = db::opt_int(body.birth_month);
            let bd_t = db::opt_int(body.birth_day);
            let bh_t = db::opt_int(body.birth_hour);
            let bmi_t = db::opt_int(body.birth_minute);
            let g_t = db::opt_text(body.gender.as_deref());
            let tz_t = db::text(&timezone);
            let lat_t = db::opt_real(body.latitude);
            let lon_t = db::opt_real(body.longitude);
            let h_t = db::text(&hash);
            let u_t = db::text(&user);
            if let Err(e) = db::exec(
                &db,
                "UPDATE users SET birth_year = ?1, birth_month = ?2, birth_day = ?3, birth_hour = ?4, birth_minute = ?5, gender = ?6, timezone = ?7, latitude = ?8, longitude = ?9, birth_data_hash = ?10, generation_tags = ?11, updated_at = datetime('now') WHERE id = ?12",
                &[&by_t, &bm_t, &bd_t, &bh_t, &bmi_t, &g_t, &tz_t, &lat_t, &lon_t, &h_t, &gen_t, &u_t],
            ).await {
                return Ok(error::error(format!("db error: {}", e), 500));
            }
            let du_t = db::text(&user);
            if let Err(e) = db::exec(&db, "DELETE FROM interpretations WHERE user_id = ?1", &[&du_t]).await {
                return Ok(error::error(format!("db error: {}", e), 500));
            }
            let mut res = ok_json(&serde_json::json!({ "success": true, "birth_data_hash": hash }), 200);
            apply_cache_headers(&mut res, 0, false);
            Ok(res)
        })
        .put_async("/api/users/me", |mut req, ctx| async move {
            let user = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(resp) => return Ok(resp),
            };
            let body: ProfileBody = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error("Invalid JSON", 400)),
            };
            // Validation — mirrors updateProfileSchema: full_name max 100, avatar_url url max 500
            if let Some(name) = body.full_name.as_deref() {
                if name.len() > 100 {
                    return Ok(error::error("Validation failed: full_name too long", 400));
                }
            }
            if let Some(url) = body.avatar_url.as_deref() {
                if url.len() > 500 {
                    return Ok(error::error("Validation failed: avatar_url too long", 400));
                }
                if !is_valid_url(url) {
                    return Ok(error::error("Validation failed: avatar_url must be a valid URL", 400));
                }
            }
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let name_t = db::opt_text(body.full_name.as_deref());
            let avatar_t = db::opt_text(body.avatar_url.as_deref());
            let u_t = db::text(&user);
            if let Err(e) = db::exec(
                &db,
                "UPDATE users SET full_name = ?1, avatar_url = ?2, updated_at = datetime('now') WHERE id = ?3",
                &[&name_t, &avatar_t, &u_t],
            ).await {
                return Ok(error::error(format!("db error: {}", e), 500));
            }
            let u2_t = db::text(&user);
            let row: Option<MinimalUser> = match db::first(
                &db,
                "SELECT id, email, full_name, avatar_url, subscription_tier, created_at FROM users WHERE id = ?1",
                &[&u2_t],
            ).await {
                Ok(r) => r,
                Err(e) => return Ok(error::error(format!("db error: {}", e), 500)),
            };
            let row = match row {
                Some(r) => r,
                None => return Ok(error::error("User not found", 404)),
            };
            let mut res = ok_json(&serde_json::to_value(row).unwrap_or_default(), 200);
            apply_cache_headers(&mut res, 0, false);
            Ok(res)
        })
}

fn is_valid_url(s: &str) -> bool {
    // Mirrors z.url(): must be a valid http/https URL.
    if s.is_empty() {
        return false;
    }
    let lower = s.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return false;
    }
    // Require at least one dot after the scheme and no spaces.
    if s.contains(' ') {
        return false;
    }
    let after_scheme = if lower.starts_with("https://") {
        &s[8..]
    } else {
        &s[7..]
    };
    after_scheme.contains('.') && !after_scheme.is_empty()
}
