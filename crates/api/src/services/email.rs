//! Transactional email via Resend (https://resend.com/docs).
//!
//! Contract (audit P0-01 foundation): fire once, no retries, and **no fake
//! success** — a missing secret/variable or a non-2xx response returns `Err`
//! so the caller can honestly tell the user the link was not sent.

use worker::{Env, Error, Fetch, Headers, Method, Request, RequestInit};

pub const RESEND_ENDPOINT: &str = "https://api.resend.com/emails";

#[derive(Debug)]
pub enum EmailError {
    /// Deployment misconfiguration: secret/variable missing or empty.
    Config(String),
    /// Request build or transport failure.
    Transport(Error),
    /// Resend answered with a non-2xx status (deliberately not retried).
    Status(u16),
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // The payload names the missing BINDING, never a secret value.
            EmailError::Config(what) => write!(f, "email not configured: {} missing", what),
            EmailError::Transport(e) => write!(f, "email transport failure: {:?}", e),
            EmailError::Status(code) => write!(f, "resend answered {}", code),
        }
    }
}

/// Send the magic-link email. `link_url` is the absolute `/api/auth/verify` URL
/// carrying the plain token; `expires_at_ms` is the absolute expiry instant so
/// the mail can state WHEN it dies, not just how long it lasts (user feedback:
/// a relative "10 minutes" made people miss the window).
pub async fn send_login_link(
    env: &Env,
    to_email: &str,
    link_url: &str,
    expires_at_ms: f64,
) -> Result<(), EmailError> {
    let api_key = env_secret(env, "RESEND_API_KEY")?;
    let from = env_var(env, "MAIL_FROM")?;

    // Absolute expiry in UTC+8 (the product's home timezone). JsDate only
    // formats UTC, so the +8h shift is applied to the instant and the result
    // is presented as wall-clock Taiwan time.
    let mins = crate::services::login_token::TOKEN_TTL_MS / 60_000;
    let taipei = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(
        expires_at_ms + 8.0 * 3600.0 * 1000.0,
    ))
    .to_iso_string()
    .as_string()
    .unwrap_or_default()[..16]
        .replace('T', " ");
    let html = format!(
        "<p>點下面的連結登入 Fortunet。連結只能使用一次,將於 <b>{taipei}</b>(台灣時間,約 {mins} 分鐘後)失效。</p>\
         <p><a href=\"{link}\">{link}</a></p>\
         <p>如果你沒有申請這封信,請忽略本郵件。</p>",
        taipei = taipei,
        mins = mins,
        link = html_escape(link_url),
    );
    let body = serde_json::json!({
        "from": from,
        "to": [to_email],
        "subject": "Fortunet 登入連結(單次使用,即將過期)",
        "html": html,
    });

    let headers = Headers::new();
    headers
        .set("Authorization", &format!("Bearer {}", api_key))
        .map_err(EmailError::Transport)?;
    headers
        .set("Content-Type", "application/json")
        .map_err(EmailError::Transport)?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body.to_string().into()));
    let req = Request::new_with_init(RESEND_ENDPOINT, &init).map_err(EmailError::Transport)?;

    let mut res = Fetch::Request(req)
        .send()
        .await
        .map_err(EmailError::Transport)?;

    let status = res.status_code();
    if !(200..300).contains(&status) {
        let _ = res.text().await; // drain; body dropped, never logged
        return Err(EmailError::Status(status));
    }
    Ok(())
}

/// Secret lookup: empty/missing = fail closed (mirrors `secret_or` in
/// routes/charts.rs).
fn env_secret(env: &Env, name: &str) -> Result<String, EmailError> {
    match env.secret(name) {
        Ok(s) if !s.to_string().is_empty() => Ok(s.to_string()),
        _ => Err(EmailError::Config(name.to_string())),
    }
}

fn env_var(env: &Env, name: &str) -> Result<String, EmailError> {
    match env.var(name) {
        Ok(v) if !v.to_string().is_empty() => Ok(v.to_string()),
        _ => Err(EmailError::Config(name.to_string())),
    }
}

/// Minimal HTML escaping for attribute/text contexts of the link.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
