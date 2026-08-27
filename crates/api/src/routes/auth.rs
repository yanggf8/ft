//! Auth routes — mirrors backend/src/routes/auth.ts.
//! register / login / logout, with a 10 req/min per-IP in-memory rate limit that
//! resets on cold start (same semantics as the TS Map).

use std::sync::{Arc, Mutex};
use worker::*;

use super::common::{client_ip, ok_json};
use super::super::error;
use super::super::services::billing;
use super::super::services::clock;
use super::super::services::db;
use super::super::services::uuid;
use super::R;

const RATE_LIMIT: u32 = 10;
const WINDOW_MS: f64 = 60000.0;

#[derive(Default)]
struct RateLimiter {
    entries: std::collections::HashMap<String, (u32, f64)>,
}
impl RateLimiter {
    fn check(&mut self, ip: &str) -> bool {
        let now = clock::now_ms();
        match self.entries.get(ip).copied() {
            Some((count, reset)) if now <= reset => {
                if count >= RATE_LIMIT {
                    false
                } else {
                    self.entries.get_mut(ip).unwrap().0 = count + 1;
                    true
                }
            }
            _ => {
                self.entries.insert(ip.to_string(), (1, now + WINDOW_MS));
                true
            }
        }
    }
}

fn rate_limit(limiter: &Arc<Mutex<RateLimiter>>, req: &Request) -> bool {
    limiter.lock().unwrap().check(&client_ip(req))
}

pub fn register(router: R<'static>) -> R<'static> {
    // Single shared 10/min map — mirrors TS single Map for all auth endpoints.
    let auth_limiter = Arc::new(Mutex::new(RateLimiter::default()));

    router
        .post_async("/api/auth/register", {
            let limiter = auth_limiter.clone();
            move |mut req, ctx| {
                let limiter = limiter.clone();
                async move {
                    if !rate_limit(&limiter, &req) {
                        return Ok(error::error("Too many requests", 429));
                    }
                    let body: RegisterBody = match req.json().await {
                        Ok(b) => b,
                        Err(_) => return Ok(error::error("Invalid JSON", 400)),
                    };
                    let email = match body.email {
                        Some(e) if is_valid_email(&e) => e,
                        _ => return Ok(error::error("Validation failed", 400)),
                    };
                    let full_name = body.full_name;

                    let db = match ctx.env.d1("DB") {
                        Ok(d) => d,
                        Err(_) => return Ok(error::error("db unavailable", 500)),
                    };
                    // Existing user?
                    let e = db::text(&email);
                    match db::first::<UserRow>(&db, "SELECT id FROM users WHERE email = ?1", &[&e]).await {
                        Ok(Some(_)) => return Ok(error::error("User already exists", 409)),
                        Ok(None) => {}
                        Err(_) => return Ok(error::error("db error", 500)),
                    }

                    let user_id = uuid::random_uuid();
                    let trial_ends_at = billing::get_trial_end_date();
                    let uid = db::text(&user_id);
                    let em = db::text(&email);
                    let name = db::opt_text(full_name.as_deref());
                    let trial = db::text(&trial_ends_at);
                    if let Err(e) = db::exec(
                        &db,
                        "INSERT INTO users (id, email, full_name, trial_ends_at) VALUES (?1, ?2, ?3, ?4)",
                        &[&uid, &em, &name, &trial],
                    ).await {
                        return Ok(error::error(format!("db error: {}", e), 500));
                    }

                    let session_id = create_session(&ctx, &user_id, &email).await?;
                    Ok(ok_json(
                        &serde_json::json!({ "sessionId": session_id, "userId": user_id, "email": email }),
                        201,
                    ))
                }
            }
        })
        .post_async("/api/auth/login", {
            let limiter = auth_limiter.clone();
            move |mut req, ctx| {
                let limiter = limiter.clone();
                async move {
                    if !rate_limit(&limiter, &req) {
                        return Ok(error::error("Too many requests", 429));
                    }
                    let body: LoginBody = match req.json().await {
                        Ok(b) => b,
                        Err(_) => return Ok(error::error("Invalid JSON", 400)),
                    };
                    let email = match body.email {
                        Some(e) if is_valid_email(&e) => e,
                        _ => return Ok(error::error("Validation failed", 400)),
                    };
                    let db = match ctx.env.d1("DB") {
                        Ok(d) => d,
                        Err(_) => return Ok(error::error("db unavailable", 500)),
                    };
                    let e = db::text(&email);
                    let user = match db::first::<UserRow>(&db, "SELECT id, email FROM users WHERE email = ?1", &[&e]).await {
                        Ok(Some(u)) => u,
                        Ok(None) => return Ok(error::error("User not found", 404)),
                        Err(e) => return Ok(error::error(format!("db error: {}", e), 500)),
                    };
                    let session_id = create_session(&ctx, &user.id, &user.email).await?;
                    Ok(ok_json(
                        &serde_json::json!({ "sessionId": session_id, "userId": user.id, "email": user.email }),
                        200,
                    ))
                }
            }
        })
        .post_async("/api/auth/logout", |req, ctx| async move {
            let auth = req.headers().get("authorization").ok().flatten().unwrap_or_default();
            if let Some(sid) = auth.strip_prefix("Bearer ") {
                destroy_session(&ctx, sid).await?;
            }
            Ok(ok_json(&serde_json::json!({ "success": true }), 200))
        })
}

async fn create_session(ctx: &RouteContext<()>, user_id: &str, email: &str) -> Result<String> {
    let session_id = uuid::random_uuid();
    let ns = ctx.env.durable_object("SESSION_DO")?;
    let stub = ns.id_from_name(&session_id)?.get_stub()?;
    let body = serde_json::json!({ "userId": user_id, "email": email });
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_body(Some(body.to_string().into()));
    let res = stub
        .fetch_with_request(Request::new_with_init("http://do/create", &init)?)
        .await?;
    if res.status_code() != 200 {
        return Err(worker::Error::from("create session failed"));
    }
    Ok(session_id)
}

async fn destroy_session(ctx: &RouteContext<()>, session_id: &str) -> Result<()> {
    let ns = ctx.env.durable_object("SESSION_DO")?;
    let stub = ns.id_from_name(session_id)?.get_stub()?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    let _ = stub
        .fetch_with_request(Request::new_with_init("http://do/destroy", &init)?)
        .await?;
    Ok(())
}

#[derive(serde::Deserialize)]
struct RegisterBody {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    full_name: Option<String>,
}

#[derive(serde::Deserialize)]
struct LoginBody {
    #[serde(default)]
    email: Option<String>,
}

#[derive(serde::Deserialize)]
struct UserRow {
    id: String,
    email: String,
}

/// Stricter email check — mirrors z.email(): non-empty, <=255, contains exactly one '@'
/// with non-empty local/domain, domain contains '.', no spaces.
fn is_valid_email(s: &str) -> bool {
    if s.is_empty() || s.len() > 255 || s.contains(' ') {
        return false;
    }
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return false;
    }
    if domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }
    // No consecutive dots, at least one char between dots
    if s.contains("..") {
        return false;
    }
    true
}
