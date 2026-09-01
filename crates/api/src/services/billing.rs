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

/// Tier/trial access mapping, extracted so native tests can pin it (the
/// js_sys::Date clock read cannot run outside wasm). `is_trialing` is the
/// parsed-stamp > now comparison; see check_user_access.
fn trial_access_for(tier: &str, is_trialing: bool) -> UserBilling {
    let has_access = tier != "free" || is_trialing;
    UserBilling {
        tier: tier.to_string(),
        is_trialing,
        trial_ends_at: None,
        has_access,
    }
}

/// `checkUserAccess(user)` — true when premium/professional, or still within trial.
pub fn check_user_access(subscription_tier: &str, trial_ends_at: Option<&str>) -> UserBilling {
    let now = clock::now_ms();
    let is_trialing = trial_ends_at
        .map(|t| parse_iso_ms(t).map(|ms| ms > now).unwrap_or(false))
        .unwrap_or(false);
    let mut billing = trial_access_for(subscription_tier, is_trialing);
    billing.trial_ends_at = trial_ends_at.map(|s| s.to_string());
    billing
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

#[cfg(test)]
mod tests {
    // check_user_access itself calls js_sys::Date::now() through parse_iso_ms,
    // which panics on a native test target ("cannot call wasm-bindgen
    // imported functions on non-wasm targets"). The trialing math it wraps is
    // `parse_iso_ms(stamp) > now` — a single comparison — and the tier/
    // has_access mapping around it is the pure logic worth pinning. That
    // mapping lives in `access_for` below; keep it extracted so native tests
    // can pin it, and keep check_user_access a thin wrapper.
    use super::{trial_access_for, TRIAL_DAYS_MS};

    #[test]
    fn expired_trial_loses_access_on_free_tier() {
        let b = trial_access_for("free", false);
        assert!(!b.has_access);
        assert!(!b.is_trialing);
        assert_eq!(b.tier, "free");
    }

    #[test]
    fn paid_tiers_bypass_the_trial_window() {
        for tier in ["premium", "professional"] {
            let b = trial_access_for(tier, false);
            assert!(b.has_access, "{tier} should access despite expired trial");
            assert!(!b.is_trialing);
        }
        // Free tier with an active trial keeps access.
        let trialing = trial_access_for("free", true);
        assert!(trialing.has_access);
        assert!(trialing.is_trialing);
    }

    #[test]
    fn trial_window_constant_matches_thirty_days() {
        assert_eq!(TRIAL_DAYS_MS, 30.0 * 24.0 * 60.0 * 60.0 * 1000.0);
    }
}
