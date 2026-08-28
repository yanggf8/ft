//! HTTP client for `fortunet-api` — the Rust counterpart of `frontend/src/lib/api.ts`.
//!
//! Session handling matches the React client exactly: the session id lives in
//! `localStorage` under `sessionId` and rides on every request as
//! `Authorization: Bearer <id>`.

use ft_schema::api::*;
use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Same default as the React client's `VITE_API_URL` fallback.
pub const API_URL: &str = "https://fortunet-api.yanggf.workers.dev";

const SESSION_KEY: &str = "sessionId";

/// What a failed call gives back. `Api` carries the structured `{error, code}`
/// body so callers can branch on `code` (NO_BIRTH_DATA, NO_STORY, RATE_LIMIT…)
/// instead of substring-matching an error string like the React version did.
#[derive(Debug, Clone)]
pub enum ApiErr {
    /// Non-2xx with a parsed `{ error, code? }` body.
    Api { status: u16, body: ApiError },
    /// Non-2xx whose body was not the expected JSON shape.
    Status { status: u16, text: String },
    /// Transport / serialization failure.
    Network(String),
}

impl ApiErr {
    /// True when the server sent this exact `code`.
    pub fn is_code(&self, code: &str) -> bool {
        matches!(self, ApiErr::Api { body, .. } if body.is(code))
    }
    pub fn status(&self) -> Option<u16> {
        match self {
            ApiErr::Api { status, .. } | ApiErr::Status { status, .. } => Some(*status),
            ApiErr::Network(_) => None,
        }
    }
    /// Missing birth data / gender — both send the user to the profile page.
    pub fn needs_birth_data(&self) -> bool {
        self.is_code("NO_BIRTH_DATA") || self.is_code("NO_GENDER")
    }
}

impl std::fmt::Display for ApiErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiErr::Api { body, .. } => write!(f, "{}", body),
            ApiErr::Status { status, text } => write!(f, "HTTP {}: {}", status, text),
            ApiErr::Network(e) => write!(f, "{}", e),
        }
    }
}

// ── session storage ──

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn get_session() -> Option<String> {
    storage()?
        .get_item(SESSION_KEY)
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
}

pub fn set_session(session_id: Option<&str>) {
    let Some(s) = storage() else { return };
    match session_id {
        Some(id) => {
            let _ = s.set_item(SESSION_KEY, id);
        }
        None => {
            let _ = s.remove_item(SESSION_KEY);
        }
    }
}

// ── request plumbing ──

fn url(path: &str) -> String {
    format!("{}{}", API_URL, path)
}

/// Turn a finished response into `T`, or into a structured `ApiErr`.
async fn decode<T: DeserializeOwned>(resp: gloo_net::http::Response) -> Result<T, ApiErr> {
    let status = resp.status();
    if (200..300).contains(&status) {
        return resp
            .json::<T>()
            .await
            .map_err(|e| ApiErr::Network(format!("decode failed: {}", e)));
    }
    let text = resp.text().await.unwrap_or_default();
    match serde_json::from_str::<ApiError>(&text) {
        Ok(body) => Err(ApiErr::Api { status, body }),
        Err(_) => Err(ApiErr::Status { status, text }),
    }
}

fn with_auth(mut builder: gloo_net::http::RequestBuilder) -> gloo_net::http::RequestBuilder {
    if let Some(sid) = get_session() {
        builder = builder.header("Authorization", &format!("Bearer {}", sid));
    }
    builder
}

async fn get_json<T: DeserializeOwned>(path: &str, no_cache: bool) -> Result<T, ApiErr> {
    let mut b = with_auth(Request::get(&url(path)));
    if no_cache {
        b = b.header("Cache-Control", "no-cache");
    }
    let resp = b.send().await.map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

async fn send_json<B: Serialize, T: DeserializeOwned>(
    builder: gloo_net::http::RequestBuilder,
    body: &B,
) -> Result<T, ApiErr> {
    let req = with_auth(builder)
        .header("Content-Type", "application/json")
        .json(body)
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    let resp = req
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

/// POST with no request body (interpret / story generate / logout).
async fn post_empty<T: DeserializeOwned>(path: &str) -> Result<T, ApiErr> {
    let resp = with_auth(Request::post(&url(path)))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

// ── endpoints ──

pub async fn register(email: &str, full_name: Option<&str>) -> Result<SessionResponse, ApiErr> {
    let body = RegisterRequest {
        email: email.to_string(),
        full_name: full_name.filter(|s| !s.is_empty()).map(|s| s.to_string()),
    };
    let res: SessionResponse = send_json(Request::post(&url("/api/auth/register")), &body).await?;
    set_session(Some(&res.sessionId));
    Ok(res)
}

pub async fn login(email: &str) -> Result<SessionResponse, ApiErr> {
    let body = LoginRequest {
        email: email.to_string(),
    };
    let res: SessionResponse = send_json(Request::post(&url("/api/auth/login")), &body).await?;
    set_session(Some(&res.sessionId));
    Ok(res)
}

/// Clears the local session even when the server call fails — same forgiving
/// behavior as the React `logout()`.
pub async fn logout() {
    let _ = post_empty::<serde_json::Value>("/api/auth/logout").await;
    set_session(None);
}

pub async fn get_me(no_cache: bool) -> Result<UserProfile, ApiErr> {
    get_json("/api/users/me", no_cache).await
}

pub async fn update_birth_data(data: &BirthDataRequest) -> Result<BirthDataResponse, ApiErr> {
    send_json(Request::put(&url("/api/users/me/birth")), data).await
}

pub async fn get_chart(chart_type: &str, no_cache: bool) -> Result<ChartResponse, ApiErr> {
    get_json(&format!("/api/charts/{}", chart_type), no_cache).await
}

/// `POST /api/charts/:type/interpret`, retrying once through a fresh chart when
/// the server reports the cached chart is stale (409 RECALC_REQUIRED).
pub async fn interpret(chart_type: &str) -> Result<InterpretResponse, ApiErr> {
    let path = format!("/api/charts/{}/interpret", chart_type);
    match post_empty::<InterpretResponse>(&path).await {
        Err(e) if e.status() == Some(409) => {
            get_chart(chart_type, true).await?;
            post_empty::<InterpretResponse>(&path).await
        }
        other => other,
    }
}

pub async fn get_story(no_cache: bool) -> Result<StoryResponse, ApiErr> {
    get_json("/api/charts/story", no_cache).await
}

pub async fn generate_story() -> Result<StoryResponse, ApiErr> {
    post_empty("/api/charts/story/generate").await
}

// ── Big5 personality (F1) ──

/// `POST /api/personality/quiz`. A 422 `CARELESS_SUSPECTED` is handled by the
/// caller as the one post-submit path that does not re-fetch the read model.
pub async fn submit_quiz(body: &QuizSubmission) -> Result<QuizResponse, ApiErr> {
    send_json(Request::post(&url("/api/personality/quiz")), body).await
}

pub async fn get_personality(no_cache: bool) -> Result<PersonalityMeResponse, ApiErr> {
    get_json("/api/personality/me", no_cache).await
}

/// Bodyless DELETE: some intermediaries reject DELETE request bodies.
pub async fn delete_personality() -> Result<PersonalityDeleteResponse, ApiErr> {
    let resp = with_auth(Request::delete(&url("/api/personality/me")))
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}
