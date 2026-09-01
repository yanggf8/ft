//! Google OAuth 2.0 helpers — pure logic, mirrors hesocial's core/oauth.rs.
//! The worker crate owns the HTTP calls (token exchange + userinfo).

use serde_json::Value;

pub const GOOGLE_AUTHORIZATION_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";
pub const GOOGLE_CALLBACK_PATH: &str = "/api/auth/google/callback";
pub const OAUTH_SCOPES: &str = "profile email";
pub const STATE_COOKIE_NAME: &str = "google_oauth_state";
pub const STATE_COOKIE_PATH: &str = "/api/auth/google";
pub const STATE_COOKIE_MAX_AGE_SECONDS: u64 = 600;

pub fn percent_encode(value: &str) -> String {
    const UNRESERVED: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if UNRESERVED.contains(byte) {
            encoded.push(char::from(*byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

pub fn google_consent_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{GOOGLE_AUTHORIZATION_URL}?response_type=code&redirect_uri={}&scope={}&state={}&client_id={}",
        percent_encode(redirect_uri),
        percent_encode(OAUTH_SCOPES),
        percent_encode(state),
        percent_encode(client_id),
    )
}

pub fn token_exchange_body(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> String {
    format!(
        "client_id={}&client_secret={}&code={}&grant_type=authorization_code&redirect_uri={}",
        percent_encode(client_id),
        percent_encode(client_secret),
        percent_encode(code),
        percent_encode(redirect_uri),
    )
}

pub fn state_set_cookie(state: &str) -> String {
    format!(
        "{STATE_COOKIE_NAME}={state}; Path={STATE_COOKIE_PATH}; HttpOnly; Secure; SameSite=Lax; Max-Age={STATE_COOKIE_MAX_AGE_SECONDS}"
    )
}

pub fn state_clear_cookie() -> String {
    format!(
        "{STATE_COOKIE_NAME}=; Path={STATE_COOKIE_PATH}; HttpOnly; Secure; SameSite=Lax; Max-Age=0"
    )
}

pub fn read_cookie<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').map(str::trim).find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then_some(value)
    })
}

pub fn random_oauth_state() -> Option<String> {
    let bytes = crate::services::uuid::secure_bytes(16)?;
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

pub enum CallbackAction {
    ExchangeCode(String),
    RedirectOauthFailed,
}

pub fn evaluate_callback(
    error: Option<&str>,
    code: Option<&str>,
    state: Option<&str>,
    cookie_state: Option<&str>,
) -> CallbackAction {
    if error.is_some() {
        return CallbackAction::RedirectOauthFailed;
    }
    let Some(code) = code.filter(|c| !c.is_empty()) else {
        return CallbackAction::RedirectOauthFailed;
    };
    let (Some(state), Some(cookie_state)) = (state, cookie_state) else {
        return CallbackAction::RedirectOauthFailed;
    };
    if state.is_empty() || state != cookie_state {
        return CallbackAction::RedirectOauthFailed;
    }
    CallbackAction::ExchangeCode(code.to_owned())
}

pub struct GoogleProfile {
    pub email: String,
    pub full_name: String,
    pub picture: Option<String>,
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.is_empty())
}

pub fn parse_google_userinfo(json: &Value) -> Option<GoogleProfile> {
    let email = json.get("email").and_then(Value::as_str)?;
    let display_name = json.get("name").and_then(Value::as_str).unwrap_or("");
    let first = non_empty(json.get("given_name").and_then(Value::as_str))
        .unwrap_or_else(|| display_name.split(' ').next().unwrap_or(""))
        .to_owned();
    let last = non_empty(json.get("family_name").and_then(Value::as_str))
        .map(str::to_owned)
        .unwrap_or_else(|| {
            display_name
                .split(' ')
                .skip(1)
                .collect::<Vec<_>>()
                .join(" ")
        });
    let full_name = if last.is_empty() {
        first.clone()
    } else {
        format!("{first} {last}").trim().to_string()
    };
    let picture = json
        .get("picture")
        .and_then(Value::as_str)
        .map(str::to_owned);
    Some(GoogleProfile {
        email: email.to_owned(),
        full_name: if full_name.is_empty() {
            display_name.to_owned()
        } else {
            full_name
        },
        picture,
    })
}

pub fn success_redirect_url(frontend_origin: &str, token: &str) -> String {
    format!("{frontend_origin}/auth/verify?token={token}")
}

pub fn failure_redirect_url(frontend_origin: &str, error: &str) -> String {
    format!("{frontend_origin}/login?error={error}")
}
