//! API request / response contract shared by the Worker (`ft-api`) and the
//! Leptos frontend (`ft-web`).
//!
//! This module is the whole point of Phase C: the frontend and backend stop
//! drifting because they deserialize the *same* structs. Field names here are the
//! JSON wire names — keep the `serde(rename)` annotations aligned with what
//! `ft-api` actually emits (see backend routes), not with Rust naming taste.

#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

// ── Auth ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
}

/// `POST /api/auth/register` (201) and `/api/auth/login` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub sessionId: String,
    pub userId: String,
    pub email: String,
}

// ── User / billing ──

/// The `billing` object nested in `GET /api/users/me`.
/// Emitted camelCase by `ft-api::services::billing::UserBilling`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Billing {
    /// `free` | `premium` | `professional`
    pub tier: String,
    pub isTrialing: bool,
    pub trialEndsAt: Option<String>,
    pub hasAccess: bool,
}

/// `GET /api/users/me`. Birth fields are snake_case (they come straight from the
/// D1 `users` row); `billing` / `hasBirthData` are camelCase (computed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub birth_year: Option<i64>,
    #[serde(default)]
    pub birth_month: Option<i64>,
    #[serde(default)]
    pub birth_day: Option<i64>,
    #[serde(default)]
    pub birth_hour: Option<i64>,
    #[serde(default)]
    pub birth_minute: Option<i64>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub subscription_tier: Option<String>,
    #[serde(default)]
    pub trial_ends_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    pub billing: Billing,
    pub hasBirthData: bool,
}

impl UserProfile {
    /// Human-readable birth line, e.g. `1990年5月15日 14時 · 男`.
    /// Mirrors the ProfilePage rendering in the React version.
    pub fn birth_summary(&self) -> Option<String> {
        let (y, m, d) = (self.birth_year?, self.birth_month?, self.birth_day?);
        let hour = match self.birth_hour {
            Some(h) => format!(" {}時", h),
            None => " (時辰不詳)".to_string(),
        };
        let gender = match self.gender.as_deref() {
            Some("male") => " · 男",
            Some("female") => " · 女",
            _ => "",
        };
        Some(format!("{}年{}月{}日{}{}", y, m, d, hour, gender))
    }
}

/// `PUT /api/users/me/birth` body. `birth_hour` is nullable on purpose — the UI
/// lets the user mark the hour as unknown, and the backend defaults it to 12.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BirthDataRequest {
    pub birth_year: i64,
    pub birth_month: i64,
    pub birth_day: i64,
    pub birth_hour: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_minute: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

/// `PUT /api/users/me/birth` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirthDataResponse {
    pub success: bool,
    pub birth_data_hash: String,
}

// ── Charts ──

/// `GET /api/charts/:type`. `chart_data` stays a raw `Value` because the two
/// divination types have different shapes; use `as_ziwei` / `as_western`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartResponse {
    pub id: String,
    pub user_id: String,
    pub divination_type: String,
    pub chart_data: serde_json::Value,
    #[serde(default)]
    pub ai_interpretation: Option<String>,
    #[serde(default)]
    pub birth_data_hash: Option<String>,
    #[serde(default)]
    pub fromCache: bool,
}

impl ChartResponse {
    pub fn as_ziwei(&self) -> Option<crate::ZiWeiChartV3> {
        serde_json::from_value(self.chart_data.clone()).ok()
    }
    pub fn as_western(&self) -> Option<crate::WesternChartV3> {
        serde_json::from_value(self.chart_data.clone()).ok()
    }
}

/// `POST /api/charts/:type/interpret` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpretResponse {
    pub interpretation: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub fromCache: bool,
}

/// `GET /api/charts/story` and `POST /api/charts/story/generate` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryResponse {
    pub story: String,
    #[serde(default)]
    pub fromCache: bool,
}

/// `GET /api/charts` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpretationsResponse {
    #[serde(default)]
    pub interpretations: Vec<serde_json::Value>,
}

// ── Errors ──

/// Every non-2xx body from `ft-api` is `{ error, code? }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    #[serde(default)]
    pub code: Option<String>,
}

impl ApiError {
    pub fn is(&self, code: &str) -> bool {
        self.code.as_deref() == Some(code)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.code {
            Some(c) => write!(f, "{} ({})", self.error, c),
            None => write!(f, "{}", self.error),
        }
    }
}
