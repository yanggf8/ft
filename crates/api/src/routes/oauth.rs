//! Google OAuth — authorization-code flow for FT.

use std::collections::HashMap;

use serde_json::{json, Value};
use worker::wasm_bindgen::JsValue;
use worker::{Fetch, Headers, Method, Request, RequestInit, Response, Result, RouteContext};

use crate::services::billing;
use crate::services::clock;
use crate::services::db;
use crate::services::login_token;
use crate::services::oauth::{
    evaluate_callback, failure_redirect_url, google_consent_url, invite_clear_cookie,
    invite_set_cookie, normalize_invite, parse_google_userinfo, random_oauth_state, read_cookie,
    state_clear_cookie, state_set_cookie, token_exchange_body, CallbackAction, GoogleProfile,
    GOOGLE_CALLBACK_PATH, GOOGLE_TOKEN_URL, GOOGLE_USERINFO_URL, INVITE_COOKIE_NAME,
    STATE_COOKIE_NAME,
};
use crate::services::uuid;

use super::super::error;
use super::common::{body_too_large, client_ip, ok_json, rate_limit};
use super::R;

/// The Google callback redirects with `?oauth_code=` — a single-use code with
/// this TTL — instead of the session id itself. URLs persist in browser
/// history, Referer headers and access logs, so the 7-day session id moves
/// only in the exchange endpoint's response body.
const EXCHANGE_TTL_MS: f64 = 60_000.0;
/// Per-window limit for the exchange endpoint (mirrors `/api/auth/verify`).
const EXCHANGE_RATE_LIMIT_IP: u32 = 10;
const EXCHANGE_WINDOW_MS: f64 = 60000.0;

fn env_var(ctx: &RouteContext<()>, name: &str) -> Option<String> {
    ctx.env
        .var(name)
        .ok()
        .map(|v| v.to_string())
        .filter(|v| !v.is_empty())
}

fn secret_var(ctx: &RouteContext<()>, name: &str) -> Option<String> {
    ctx.env
        .secret(name)
        .ok()
        .map(|v| v.to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| env_var(ctx, name))
}

fn google_config(ctx: &RouteContext<()>) -> Option<(String, String)> {
    let client_id =
        env_var(ctx, "GOOGLE_CLIENT_ID").or_else(|| secret_var(ctx, "GOOGLE_CLIENT_ID"))?;
    let client_secret =
        secret_var(ctx, "GOOGLE_CLIENT_SECRET").or_else(|| env_var(ctx, "GOOGLE_CLIENT_SECRET"))?;
    Some((client_id, client_secret))
}

fn google_oauth_unavailable() -> Result<Response> {
    Response::from_json(&json!({
        "error": "Google OAuth is not configured",
        "message": "GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET must be configured"
    }))
    .map(|r| r.with_status(503))
}

fn redirect_with_cookies(location: &str, cookies: &[String]) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Location", location)?;
    for c in cookies {
        headers.append("Set-Cookie", c)?;
    }
    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

fn failure_redirect(frontend_origin: &str, error: &str) -> Result<Response> {
    let location = failure_redirect_url(frontend_origin, error);
    redirect_with_cookies(&location, &[state_clear_cookie(), invite_clear_cookie()])
}

fn callback_url(req: &Request) -> Option<String> {
    let url = req.url().ok()?;
    let origin = format!("{}://{}", url.scheme(), url.host_str()?);
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    Some(format!("{origin}{port}{GOOGLE_CALLBACK_PATH}"))
}

fn frontend_origin(ctx: &RouteContext<()>) -> String {
    ctx.env
        .var("WEB_ORIGIN")
        .map(|v| v.to_string())
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://fortunet.pages.dev".to_string())
}

pub fn register(router: R<'static>) -> R<'static> {
    router
        .get_async("/api/auth/google", |req, ctx| async move {
            match google_start(&ctx, &req).await {
                Ok(r) => Ok(r),
                Err(e) => Response::error(e.to_string(), 500),
            }
        })
        .get_async("/api/auth/google/callback", |req, ctx| async move {
            match google_callback(&ctx, &req).await {
                Ok(r) => Ok(r),
                Err(e) => Response::error(e.to_string(), 500),
            }
        })
        .post_async("/api/auth/oauth/exchange", |mut req, ctx| async move {
            let ip = client_ip(&req);
            if !rate_limit(
                &ctx,
                &format!("oauthx:ip:{ip}"),
                EXCHANGE_RATE_LIMIT_IP,
                EXCHANGE_WINDOW_MS,
            )
            .await
            {
                return Ok(error::error("Too many requests", 429));
            }
            if body_too_large(&req) {
                return Ok(error::error_code(
                    "payload too large",
                    "PAYLOAD_TOO_LARGE",
                    413,
                ));
            }
            let body: ExchangeBody = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error("Invalid JSON", 400)),
            };
            let code = match body.code {
                Some(c) if !c.is_empty() && c.len() <= 256 => c,
                _ => return Ok(error::error("Validation failed", 400)),
            };
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let hash = login_token::hash_token(&code);
            let h = db::text(&hash);
            let now = clock::now_iso();
            let n = db::text(&now);
            // Opportunistic cleanup of expired codes (ISO-vs-ISO comparison —
            // never `datetime('now')`, whose space format breaks ordering).
            let _ = db::exec(
                &db,
                "DELETE FROM oauth_exchanges WHERE expires_at <= ?1",
                &[&n],
            )
            .await;
            // Single-use atomic consume: only a valid, unused, unexpired hash
            // is marked used. 0 rows affected = invalid, expired or replayed.
            let consumed = match db::exec_changes(
                &db,
                "UPDATE oauth_exchanges SET used_at = datetime('now') \
                 WHERE token_hash = ?1 AND used_at IS NULL AND expires_at > ?2",
                &[&h, &n],
            )
            .await
            {
                Ok(changes) => changes,
                Err(_) => return Ok(error::error("db error", 500)),
            };
            if consumed == 0 {
                return Ok(error::error("Invalid or expired code", 401));
            }
            let row = match db::first::<ExchangeRow>(
                &db,
                "SELECT user_id, email FROM oauth_exchanges WHERE token_hash = ?1",
                &[&h],
            )
            .await
            {
                Ok(Some(r)) => r,
                Ok(None) => return Ok(error::error("Invalid or expired code", 401)),
                Err(_) => return Ok(error::error("db error", 500)),
            };
            let Some(session_id) = create_session(&ctx, &row.user_id, &row.email).await else {
                worker::console_log!("oauth/exchange: create_session failed for {}", row.user_id);
                return Ok(error::error("oauth_error", 500));
            };
            Ok(ok_json(
                &serde_json::json!({
                    "sessionId": session_id,
                    "userId": row.user_id,
                    "email": row.email
                }),
                200,
            ))
        })
}

async fn google_start(ctx: &RouteContext<()>, req: &Request) -> Result<Response> {
    let Some((client_id, _)) = google_config(ctx) else {
        return google_oauth_unavailable();
    };
    let origin = frontend_origin(ctx);
    let Some(redirect_uri) = callback_url(req) else {
        worker::console_log!("oauth/start: could not derive redirect_uri");
        return failure_redirect(&origin, "oauth_error");
    };
    let Some(oauth_state) = random_oauth_state() else {
        return failure_redirect(&origin, "oauth_error");
    };
    let consent_url = google_consent_url(&client_id, &redirect_uri, &oauth_state);
    // An invite code forwarded by the register page rides a short-lived cookie
    // through Google's redirects; it is only trusted at the callback's atomic
    // consume, never here.
    let mut cookies = vec![state_set_cookie(&oauth_state)];
    if let Some(invite) = req
        .url()
        .ok()
        .and_then(|u| {
            u.query_pairs()
                .find(|(k, _)| k == "invite")
                .map(|(_, v)| v.to_string())
        })
        .as_deref()
        .and_then(normalize_invite)
    {
        cookies.push(invite_set_cookie(&invite));
    }
    redirect_with_cookies(&consent_url, &cookies)
}

async fn google_callback(ctx: &RouteContext<()>, req: &Request) -> Result<Response> {
    let Some((client_id, client_secret)) = google_config(ctx) else {
        return google_oauth_unavailable();
    };
    let origin = frontend_origin(ctx);

    let cookie_header = req
        .headers()
        .get("Cookie")
        .ok()
        .flatten()
        .unwrap_or_default();
    let cookie_state = read_cookie(&cookie_header, STATE_COOKIE_NAME).map(str::to_owned);

    let url_str = req.url().map(|u| u.to_string()).unwrap_or_default();
    let url_parsed =
        url::Url::parse(&url_str).unwrap_or_else(|_| url::Url::parse("http://localhost/").unwrap());
    let mut query: HashMap<String, String> = HashMap::new();
    for (k, v) in url_parsed.query_pairs() {
        query.insert(k.to_string(), v.to_string());
    }

    let action = evaluate_callback(
        query.get("error").map(String::as_str),
        query.get("code").map(String::as_str),
        query.get("state").map(String::as_str),
        cookie_state.as_deref(),
    );
    let CallbackAction::ExchangeCode(code) = action else {
        worker::console_log!(
            "oauth/callback: precondition failed (error={:?} code_present={} state_present={} cookie_present={})",
            query.get("error"),
            query.contains_key("code"),
            query.contains_key("state"),
            cookie_state.is_some(),
        );
        return failure_redirect(&origin, "oauth_failed");
    };

    let Some(redirect_uri) = callback_url(req) else {
        worker::console_log!("oauth/callback: could not derive redirect_uri");
        return failure_redirect(&origin, "oauth_error");
    };
    let Some(access_token) = exchange_code(&client_id, &client_secret, &code, &redirect_uri).await
    else {
        worker::console_log!("oauth/callback: token exchange failed (redirect_uri={redirect_uri})");
        return failure_redirect(&origin, "oauth_error");
    };
    let Some(profile) = fetch_userinfo(&access_token).await else {
        worker::console_log!("oauth/callback: userinfo fetch failed");
        return failure_redirect(&origin, "oauth_error");
    };
    let invite_code = read_cookie(&cookie_header, INVITE_COOKIE_NAME).and_then(normalize_invite);
    let user = match resolve_google_user(ctx, &profile, invite_code.as_deref()).await {
        Ok(user) => user,
        Err(code) => {
            worker::console_log!("oauth/callback: resolve_google_user -> {}", code);
            return failure_redirect(&origin, code);
        }
    };
    let Some(exchange_code) = create_exchange(ctx, &user.id, &user.email).await else {
        worker::console_log!("oauth/callback: create_exchange failed for {}", user.id);
        return failure_redirect(&origin, "oauth_error");
    };

    // The redirect carries a one-time short-lived exchange code, never the
    // session id itself (see EXCHANGE_TTL_MS above).
    let location = format!("{origin}/login?oauth_code={exchange_code}");
    redirect_with_cookies(&location, &[state_clear_cookie(), invite_clear_cookie()])
}

/// Mint a one-time exchange code for a just-authenticated Google user. Only
/// the SHA-256 hash is stored (`oauth_exchanges` — same discipline as
/// `login_tokens`); the plain value rides the redirect URL exactly once and
/// dies in 60 seconds or on first use, whichever comes first.
async fn create_exchange(ctx: &RouteContext<()>, user_id: &str, email: &str) -> Option<String> {
    let db = db::Turso::from_env(&ctx.env).ok()?;
    let (plain, hash) = login_token::new_token()?;
    let expires = clock::now_plus_ms(EXCHANGE_TTL_MS);
    let h = db::text(&hash);
    let uid = db::text(user_id);
    let em = db::text(email);
    let n = db::text(&expires);
    db::exec(
        &db,
        "INSERT INTO oauth_exchanges (token_hash, user_id, email, expires_at) \
         VALUES (?1, ?2, ?3, ?4)",
        &[&h, &uid, &em, &n],
    )
    .await
    .ok()?;
    Some(plain)
}

#[derive(serde::Deserialize)]
struct ExchangeBody {
    code: Option<String>,
}

#[derive(serde::Deserialize)]
struct ExchangeRow {
    user_id: String,
    email: String,
}

async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Option<String> {
    let headers = Headers::new();
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .ok()?;
    headers.set("Accept", "application/json").ok()?;
    let body = token_exchange_body(client_id, client_secret, code, redirect_uri);
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&body)));
    let request = Request::new_with_init(GOOGLE_TOKEN_URL, &init).ok()?;
    let mut response = Fetch::Request(request).send().await.ok()?;
    if response.status_code() != 200 {
        return None;
    }
    let json: Value = response.json().await.ok()?;
    json.get("access_token")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

async fn fetch_userinfo(access_token: &str) -> Option<GoogleProfile> {
    let headers = Headers::new();
    headers
        .set("Authorization", &format!("Bearer {access_token}"))
        .ok()?;
    headers.set("Accept", "application/json").ok()?;
    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);
    let request = Request::new_with_init(GOOGLE_USERINFO_URL, &init).ok()?;
    let mut response = Fetch::Request(request).send().await.ok()?;
    if response.status_code() != 200 {
        return None;
    }
    let json: Value = response.json().await.ok()?;
    parse_google_userinfo(&json)
}

#[derive(serde::Deserialize)]
struct UserRow {
    id: String,
    email: String,
}

/// Find-or-create the Google user. Existing addresses sign in untouched; NEW
/// addresses must present a valid invite while `INVITE_REQUIRED` holds (beta
/// gate, spec 2026-08-30 — the magic-link path enforces the same rule; this
/// was the unguarded second door). The invite is consumed atomically here,
/// immediately before INSERT, mirroring routes/auth.rs verify, so a stale
/// register-time check cannot let one code overshoot its `max_uses`. `Err`
/// carries the failure_redirect error code.
async fn resolve_google_user(
    ctx: &RouteContext<()>,
    profile: &GoogleProfile,
    invite_code: Option<&str>,
) -> std::result::Result<UserRow, &'static str> {
    let db = db::Turso::from_env(&ctx.env).map_err(|_| "oauth_error")?;
    let email = db::text(&profile.email);
    let existing: Option<UserRow> = db::first(
        &db,
        "SELECT id, email FROM users WHERE email = ?1",
        &[&email],
    )
    .await
    .ok()
    .flatten();

    if let Some(user) = existing {
        if let Some(picture) = &profile.picture {
            if !picture.is_empty() {
                let pic = db::text(picture);
                let now_str = clock::now_iso();
                let now = db::text(&now_str);
                let uid = db::text(&user.id);
                let _ = db::exec(
                    &db,
                    "UPDATE users SET avatar_url = COALESCE(?1, avatar_url), updated_at = ?2 WHERE id = ?3",
                    &[&pic, &now, &uid],
                )
                .await;
            }
        }
        return Ok(user);
    }

    let invited_by = match invite_code {
        // A supplied code is always validated + consumed (channel
        // attribution), whether or not the beta gate demands one — mirroring
        // the magic-link verify path.
        Some(code) => {
            let c = db::text(code);
            let now_str = clock::now_iso();
            let n = db::text(&now_str);
            let consumed = db::exec_changes(
                &db,
                "UPDATE invites SET used_count = used_count + 1 \
                 WHERE code = ?1 AND used_count < max_uses \
                 AND (expires_at IS NULL OR expires_at > ?2) \
                 AND revoked_at IS NULL",
                &[&c, &n],
            )
            .await
            .map_err(|_| "oauth_error")?;
            if consumed == 0 {
                worker::console_log!("oauth/callback: invite unusable for {}", profile.email);
                return Err("invite_invalid");
            }
            Some(code)
        }
        None if crate::services::invite::INVITE_REQUIRED => {
            worker::console_log!(
                "oauth/callback: unknown address {} without invite",
                profile.email
            );
            return Err("invite_required");
        }
        None => None,
    };

    let user_id = uuid::random_uuid();
    let trial_ends_at = billing::get_trial_end_date();
    let uid = db::text(&user_id);
    let em = db::text(&profile.email);
    let name = db::opt_text(Some(&profile.full_name));
    let avatar = db::opt_text(profile.picture.as_deref());
    let trial = db::text(&trial_ends_at);
    let ib = db::opt_text(invited_by);
    let now_str = clock::now_iso();
    let now_text = db::text(&now_str);

    db::exec(
        &db,
        "INSERT INTO users (id, email, full_name, avatar_url, trial_ends_at, invited_by, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        &[&uid, &em, &name, &avatar, &trial, &ib, &now_text],
    )
    .await
    .map_err(|_| "oauth_error")?;

    Ok(UserRow {
        id: user_id,
        email: profile.email.clone(),
    })
}

async fn create_session(ctx: &RouteContext<()>, user_id: &str, email: &str) -> Option<String> {
    let session_id = uuid::random_uuid();
    let ns = ctx.env.durable_object("SESSION_DO").ok()?;
    let stub = ns.id_from_name(&session_id).ok()?.get_stub().ok()?;
    let body = serde_json::json!({ "userId": user_id, "email": email });
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(body.to_string().into()));
    let req = Request::new_with_init("http://do/create", &init).ok()?;
    let res = stub.fetch_with_request(req).await.ok()?;
    if res.status_code() != 200 {
        return None;
    }
    Some(session_id)
}
