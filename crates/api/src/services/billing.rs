//! Billing service — mirrors backend/src/services/billing.ts.
//! 30-day free trial + subscription tier access. Native IAP planned (no web checkout).

use serde::Serialize;

use super::clock;

const TRIAL_DAYS_MS: f64 = 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBilling {
    pub tier: String, // 'free' | 'premium' | 'professional'
    pub is_trialing: bool,
    pub trial_ends_at: Option<String>,
    pub has_access: bool,
}
// TS emitted camelCase `trialEndsAt` / `isTrialing` / `hasAccess`.

/// `getTrialEndDate()` — Date.now() + 30 days as an ISO string.
pub fn get_trial_end_date() -> String {
    clock::now_plus_ms(TRIAL_DAYS_MS)
}

/// `checkUserAccess(user)` — true when premium/professional, or still within trial.
pub fn check_user_access(subscription_tier: &str, trial_ends_at: Option<&str>) -> UserBilling {
    let now = clock::now_ms();
    let is_trialing = trial_ends_at
        .map(|t| parse_iso_ms(t).map(|ms| ms > now).unwrap_or(false))
        .unwrap_or(false);
    let tier = subscription_tier;
    let has_access = tier != "free" || is_trialing;
    UserBilling {
        tier: tier.to_string(),
        is_trialing,
        trial_ends_at: trial_ends_at.map(|s| s.to_string()),
        has_access,
    }
}

fn parse_iso_ms(iso: &str) -> Option<f64> {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
    let ms = d.get_time();
    if ms.is_nan() {
        None
    } else {
        Some(ms)
    }
}
