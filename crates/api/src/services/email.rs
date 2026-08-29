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

/// Send the magic-link email. `link_url` is the absolute `/api/auth/verify`
/// URL carrying the plain token, built by the caller (routes).
pub async fn send_login_link(env: &Env, to_email: &str, link_url: &str) -> Result<(), EmailError> {
    let api_key = env_secret(env, "RESEND_API_KEY")?;
    let from = env_var(env, "MAIL_FROM")?;

    let mins = crate::services::login_token::TOKEN_TTL_MS / 60_000;
    let html = format!(
        "<p>Click the link below to sign in to Fortunet. It expires in {} minutes and can be used once.</p>\
         <p><a href=\"{}\">{}</a></p>\
         <p>If you did not request this email, you can ignore it.</p>",
        mins,
        html_escape(link_url),
        html_escape(link_url),
    );
    let body = serde_json::json!({
        "from": from,
        "to": [to_email],
        "subject": "Your Fortunet sign-in link",
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
