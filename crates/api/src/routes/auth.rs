//! Auth routes — magic-link login (audit P0-01 + A-02 + A-03).
//!
//! `register` / `login` no longer create a session directly: they mint a
//! single-use login token (only its SHA-256 hash is stored, 10-minute expiry)
//! and email a `{WEB_ORIGIN}/auth/verify?token=...` link. The session is
//! created only by `POST /api/auth/verify` after the token is atomically
//! consumed. Login and register answer with an identical 202 body whether or
//! not the address exists — no account enumeration (A-03).

use worker::*;

use super::super::error;
use super::super::services::billing;
use super::super::services::clock;
use super::super::services::db;
use super::super::services::email;
use super::super::services::login_token;
use super::super::services::uuid;
use super::common::{client_ip, ok_json, rate_limit};
use super::R;

/// Per-window limits: 10 requests/min per IP, 5 per email address.
const RATE_LIMIT_IP: u32 = 10;
const RATE_LIMIT_EMAIL: u32 = 5;
const WINDOW_MS: f64 = 60000.0;

/// Frontend origin the emailed link points at (`WEB_ORIGIN` var overrides).
const DEFAULT_WEB_ORIGIN: &str = "https://fortunet.pages.dev";

/// Identical 202 body for login + register — never reveals account existence.
const LINK_SENT_BODY: &str = "If that email exists, a login link has been sent";

pub fn register(router: R<'static>) -> R<'static> {
    router
        .post_async("/api/auth/register", |mut req, ctx| async move {
            let body: RegisterBody = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error("Invalid JSON", 400)),
            };
            let email_addr = match body.email {
                Some(e) if is_valid_email(&e) => e,
                _ => return Ok(error::error("Validation failed", 400)),
            };
            let ip = client_ip(&req);
            if !rate_limited(
                &ctx,
                &[
                    (format!("login:ip:{}", ip), RATE_LIMIT_IP),
                    (format!("login:email:{}", email_addr.to_lowercase()), RATE_LIMIT_EMAIL),
                ],
            )
            .await
            {
                return Ok(error::error("Too many requests", 429));
            }
            issue_login_link(&ctx, &email_addr, body.full_name.as_deref()).await
        })
        .post_async("/api/auth/login", |mut req, ctx| async move {
            let body: LoginBody = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error("Invalid JSON", 400)),
            };
            let email_addr = match body.email {
                Some(e) if is_valid_email(&e) => e,
                _ => return Ok(error::error("Validation failed", 400)),
            };
            let ip = client_ip(&req);
            if !rate_limited(
                &ctx,
                &[
                    (format!("login:ip:{}", ip), RATE_LIMIT_IP),
                    (format!("login:email:{}", email_addr.to_lowercase()), RATE_LIMIT_EMAIL),
                ],
            )
            .await
            {
                return Ok(error::error("Too many requests", 429));
            }
            // Login never creates an account: an unknown address gets the same
            // 202 and NO email (A-03).
            issue_login_link(&ctx, &email_addr, None).await
        })
        .post_async("/api/auth/verify", |mut req, ctx| async move {
            let ip = client_ip(&req);
            if !rate_limited(&ctx, &[(format!("verify:ip:{}", ip), RATE_LIMIT_IP)]).await {
                return Ok(error::error("Too many requests", 429));
            }
            let body: VerifyBody = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error("Invalid JSON", 400)),
            };
            let token = match body.token {
                Some(t) if !t.is_empty() && t.len() <= 256 => t,
                _ => return Ok(error::error("Validation failed", 400)),
            };
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let hash = login_token::hash_token(&token);
            let h = db::text(&hash);
            let now = clock::now_iso();
            let n = db::text(&now);
            // Opportunistic cleanup of expired tokens (ISO-vs-ISO comparison —
            // never `datetime('now')`, whose space format breaks ordering).
            let _ = db::exec(
                &db,
                "DELETE FROM login_tokens WHERE expires_at <= ?1",
                &[&n],
            )
            .await;
            // Single-use atomic consume: only a valid, unused, unexpired hash
            // is marked used. 0 rows affected = invalid.
            let stmt = match db
                .prepare(
                    "UPDATE login_tokens SET used_at = datetime('now') \
                     WHERE token_hash = ?1 AND used_at IS NULL AND expires_at > ?2",
                )
                .bind_refs([&h, &n].into_iter())
            {
                Ok(s) => s,
                Err(_) => return Ok(error::error("db error", 500)),
            };
            let consumed = match stmt.run().await {
                Ok(r) => r.meta().ok().flatten().and_then(|m| m.changes).unwrap_or(0),
                Err(_) => return Ok(error::error("db error", 500)),
            };
            if consumed == 0 {
                return Ok(error::error("Invalid or expired token", 401));
            }
            // The consumed row carries the address the link was minted for.
            let row = match db::first::<TokenRow>(
                &db,
                "SELECT email, pending_full_name FROM login_tokens WHERE token_hash = ?1",
                &[&h],
            )
            .await
            {
                Ok(Some(r)) => r,
                Ok(None) => return Ok(error::error("Invalid or expired token", 401)),
                Err(_) => return Ok(error::error("db error", 500)),
            };
            let e = db::text(&row.email);
            let user = match db::first::<UserRow>(
                &db,
                "SELECT id, email FROM users WHERE email = ?1",
                &[&e],
            )
            .await
            {
                Ok(Some(u)) => u,
                // Register flow: the account is created HERE, only after the
                // link proved email ownership (review risk 4). A consumed
                // token with no pending name and no user is a stale
                // unknown-address link — fail closed.
                Ok(None) => match row.pending_full_name.as_deref() {
                    Some(full_name) => {
                        let user_id = uuid::random_uuid();
                        let trial_ends_at = billing::get_trial_end_date();
                        let uid = db::text(&user_id);
                        let em = db::text(&row.email);
                        let name = db::opt_text(Some(full_name));
                        let trial = db::text(&trial_ends_at);
                        if let Err(_) = db::exec(
                            &db,
                            "INSERT INTO users (id, email, full_name, trial_ends_at) \
                             VALUES (?1, ?2, ?3, ?4)",
                            &[&uid, &em, &name, &trial],
                        )
                        .await
                        {
                            return Ok(error::error("db error", 500));
                        }
                        UserRow {
                            id: user_id,
                            email: row.email.clone(),
                        }
                    }
                    None => return Ok(error::error("Invalid or expired token", 401)),
                },
                Err(_) => return Ok(error::error("db error", 500)),
            };
            let session_id = create_session(&ctx, &user.id, &user.email).await?;
            Ok(ok_json(
                &serde_json::json!({ "sessionId": session_id, "userId": user.id, "email": user.email }),
                200,
            ))
        })
        .post_async("/api/auth/logout", |req, ctx| async move {
            let auth = req
                .headers()
                .get("authorization")
                .ok()
                .flatten()
                .unwrap_or_default();
            if let Some(sid) = auth.strip_prefix("Bearer ") {
                destroy_session(&ctx, sid).await?;
            }
            Ok(ok_json(&serde_json::json!({ "success": true }), 200))
        })
}

/// Shared limiter check: every (key, limit) pair must pass inside the window.
/// Sequential short-circuit mirrors the old `iter().all()` semantics — a denied
/// key stops the walk, later keys are not consumed. P2-02: the counters now live
/// in RateLimitDO (cross-isolate) instead of an isolate-local OnceLock.
async fn rate_limited(ctx: &RouteContext<()>, keys: &[(String, u32)]) -> bool {
    for (k, limit) in keys {
        if !rate_limit(ctx, k, *limit, WINDOW_MS).await {
            return false;
        }
    }
    true
}

/// Magic-link issuance shared by login + register (A-03).
///
/// - `new_full_name = Some` (register): the user row is NOT created here — the
///   name rides on the token row (`pending_full_name`) and the account is
///   created by `POST /api/auth/verify` only after email ownership is proven
///   (review finding: pre-verify INSERT let anyone pre-register arbitrary
///   addresses with attacker-chosen names).
/// - `new_full_name = None` (login): an unknown address is silently accepted.
///
/// Both paths perform exactly one SELECT + one INSERT before answering, so the
/// 202 latency does not distinguish existing from unknown addresses (F2
/// mitigation; the Resend call — sent for existing addresses and for register
/// requests, which always carry a name — is the documented residual; worker
/// 0.8.5 exposes no `wait_until` on `RouteContext`).
///
/// Success is always the identical 202 `LINK_SENT_BODY`. Only infrastructure
/// failures (db, unconfigured email, Resend non-2xx) diverge.
async fn issue_login_link(
    ctx: &RouteContext<()>,
    email_addr: &str,
    new_full_name: Option<&str>,
) -> Result<Response> {
    if !email_delivery_configured(ctx) {
        return Ok(error::error("email delivery not configured", 503));
    }
    let db = match ctx.env.d1("DB") {
        Ok(d) => d,
        Err(_) => return Ok(error::error("db unavailable", 500)),
    };
    let e = db::text(email_addr);
    let exists = match db::first::<UserRow>(
        &db,
        "SELECT id, email FROM users WHERE email = ?1",
        &[&e],
    )
    .await
    {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(_) => return Ok(error::error("db error", 500)),
    };

    // Mint the token — only the hash is persisted. A register for an unknown
    // address carries the requested name; verify creates the account only
    // after the link proves ownership. Unknown-address logins mint an orphan
    // token (no email): bounded by the rate limits and swept by verify's
    // opportunistic DELETE.
    let (plain, hash) = match login_token::new_token() {
        Some(p) => p,
        None => return Ok(error::error("unable to create login token", 500)),
    };
    let expires_at = clock::now_plus_ms(login_token::TOKEN_TTL_MS as f64);
    let pending = match (!exists, new_full_name) {
        // Fresh register: the name rides the token; verify creates the account.
        (true, Some(full_name)) => db::opt_text(Some(full_name)),
        _ => db::opt_text(None),
    };
    let h = db::text(&hash);
    let em = db::text(email_addr);
    let exp = db::text(&expires_at);
    if let Err(_) = db::exec(
        &db,
        "INSERT INTO login_tokens (token_hash, email, expires_at, pending_full_name) \
         VALUES (?1, ?2, ?3, ?4)",
        &[&h, &em, &exp, &pending],
    )
    .await
    {
        return Ok(error::error("db error", 500));
    }

    if exists || new_full_name.is_some() {
        let origin = ctx
            .env
            .var("WEB_ORIGIN")
            .map(|v| v.to_string())
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_WEB_ORIGIN.to_string());
        let link = format!("{}/auth/verify?token={}", origin, plain);
        let expires_at_ms = clock::now_ms() + login_token::TOKEN_TTL_MS as f64;
        if let Err(e) = email::send_login_link(&ctx.env, email_addr, &link, expires_at_ms).await {
            // Honest failure — never a fake "sent" (the token just expires
            // unused). The reason goes to the worker log, not the client.
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                "login email to {} failed: {}",
                email_addr, e
            )));
            return Ok(error::error("failed to send login email", 502));
        }
    }
    Ok(link_sent())
}

/// The identical no-enumeration response for login + register.
fn link_sent() -> Response {
    ok_json(
        &serde_json::json!({ "ok": true, "message": LINK_SENT_BODY }),
        202,
    )
}

/// Missing `RESEND_API_KEY` secret or `MAIL_FROM` var = delivery not
/// configured -> 503, fail honestly.
fn email_delivery_configured(ctx: &RouteContext<()>) -> bool {
    let key_ok = ctx
        .env
        .secret("RESEND_API_KEY")
        .map(|s| !s.to_string().is_empty())
        .unwrap_or(false);
    let from_ok = ctx
        .env
        .var("MAIL_FROM")
        .map(|v| !v.to_string().is_empty())
        .unwrap_or(false);
    if !(key_ok && from_ok) {
        // Surface WHICH binding the runtime is missing — this cost a live
        // debugging session once (secret listed via wrangler but invisible
        // to the isolate). Never log the secret value itself.
        let key_present = ctx.env.secret("RESEND_API_KEY").is_ok();
        let from_present = ctx.env.var("MAIL_FROM").is_ok();
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!(
            "email_delivery_configured: key_ok={} (binding_present={}) from_ok={} (binding_present={})",
            key_ok, key_present, from_ok, from_present
        )));
    }
    key_ok && from_ok
}

async fn create_session(ctx: &RouteContext<()>, user_id: &str, email: &str) -> Result<String> {
    let session_id = uuid::random_uuid();
    let ns = ctx.env.durable_object("SESSION_DO")?;
    let stub = ns.id_from_name(&session_id)?.get_stub()?;
    let body = serde_json::json!({ "userId": user_id, "email": email });
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(body.to_string().into()));
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
struct VerifyBody {
    #[serde(default)]
    token: Option<String>,
}

#[derive(serde::Deserialize)]
struct UserRow {
    id: String,
    email: String,
}

#[derive(serde::Deserialize)]
struct TokenRow {
    email: String,
    pending_full_name: Option<String>,
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
