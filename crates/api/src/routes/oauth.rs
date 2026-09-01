//! Google OAuth — authorization-code flow for FT.

use std::collections::HashMap;

use serde_json::{Value, json};
use worker::wasm_bindgen::JsValue;
use worker::{Fetch, Headers, Method, Request, RequestInit, Response, Result, RouteContext};

use crate::services::billing;
use crate::services::clock;
use crate::services::db;
use crate::services::oauth::{
    CallbackAction, GOOGLE_CALLBACK_PATH, GOOGLE_TOKEN_URL, GOOGLE_USERINFO_URL, GoogleProfile,
    STATE_COOKIE_NAME, evaluate_callback, failure_redirect_url, google_consent_url,
    parse_google_userinfo, random_oauth_state, read_cookie, state_clear_cookie, state_set_cookie,
    token_exchange_body,
};
use crate::services::uuid;

use super::R;

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
    let client_id = env_var(ctx, "GOOGLE_CLIENT_ID").or_else(|| secret_var(ctx, "GOOGLE_CLIENT_ID"))?;
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

fn redirect_with_cookie(location: &str, cookie: Option<String>) -> Result<Response> {
    let mut headers = Headers::new();
    headers.set("Location", location)?;
    if let Some(c) = cookie {
        headers.set("Set-Cookie", &c)?;
    }
    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

fn failure_redirect(frontend_origin: &str, error: &str) -> Result<Response> {
    let location = failure_redirect_url(frontend_origin, error);
    redirect_with_cookie(&location, Some(state_clear_cookie()))
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
    redirect_with_cookie(&consent_url, Some(state_set_cookie(&oauth_state)))
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
    let url_parsed = url::Url::parse(&url_str).unwrap_or_else(|_| url::Url::parse("http://localhost/").unwrap());
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
    let Some(user) = upsert_google_user(ctx, &profile).await else {
        worker::console_log!("oauth/callback: upsert_google_user failed for {}", profile.email);
        return failure_redirect(&origin, "oauth_error");
    };
    let Some(session_id) = create_session(ctx, &user.id, &user.email).await else {
        worker::console_log!("oauth/callback: create_session failed for {}", user.id);
        return failure_redirect(&origin, "oauth_error");
    };

    let location = format!("{origin}/login?sessionId={session_id}");
    redirect_with_cookie(&location, Some(state_clear_cookie()))
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

async fn upsert_google_user(
    ctx: &RouteContext<()>,
    profile: &GoogleProfile,
) -> Option<UserRow> {
    let db = db::Turso::from_env(&ctx.env).ok()?;
    let email = db::text(&profile.email);
    let existing: Option<UserRow> = db::first(&db, "SELECT id, email FROM users WHERE email = ?1", &[&email])
        .await
        .ok()?;

    if let Some(existing) = existing {
        if let Some(picture) = &profile.picture {
            if !picture.is_empty() {
                let pic = db::text(picture);
                let now_str = clock::now_iso();
                let now = db::text(&now_str);
                let uid = db::text(&existing.id);
                let _ = db::exec(
                    &db,
                    "UPDATE users SET avatar_url = COALESCE(?1, avatar_url), updated_at = ?2 WHERE id = ?3",
                    &[&pic, &now, &uid],
                )
                .await;
            }
        }
        return Some(existing);
    }

    let user_id = uuid::random_uuid();
    let trial_ends_at = billing::get_trial_end_date();
    let now_str = clock::now_iso();
    let uid = db::text(&user_id);
    let em = db::text(&profile.email);
    let name = db::opt_text(Some(&profile.full_name));
    let avatar = db::opt_text(profile.picture.as_deref());
    let trial = db::text(&trial_ends_at);
    let now_text = db::text(&now_str);

    db::exec(
        &db,
        "INSERT INTO users (id, email, full_name, avatar_url, trial_ends_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        &[&uid, &em, &name, &avatar, &trial, &now_text, &now_text],
    )
    .await
    .ok()?;

    Some(UserRow {
        id: user_id,
        email: profile.email.clone(),
    })
}

async fn create_session(
    ctx: &RouteContext<()>,
    user_id: &str,
    email: &str,
) -> Option<String> {
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
