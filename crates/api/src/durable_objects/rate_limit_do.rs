//! Rate limit Durable Object — finding P2-02.
//!
//! The old limiter (`routes/common.rs::RateLimiter`) was isolate-local: every
//! cold start zeroed the counters and each isolate had its own budget, so
//! "10 req/min per IP" never held globally. This DO keeps one counter per
//! caller-namespaced key (`login:ip:…`, `login:email:…`, `verify:ip:…`,
//! `personality:ip:…`, `ai:…`) with reset semantics identical to the old
//! `RateLimiter::check`: the window ends at first-hit + window_ms, a hit past
//! it resets in place, a hit inside it increments until `limit` denies.
//!
//! Instances are sharded by the CALLER (`rl:{fnv1a(key) % 8}`, see
//! `routes/common.rs::rate_limit`) so the keyspace does not funnel through a
//! single global DO. New class + new storage keys — additive only, nothing
//! existing is touched.

use std::time::Duration;

use worker::*;

use crate::services::clock;

/// One counter per namespaced key. Mirrors the old `(count, reset)` entry.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RateCounter {
    count: u32,
    reset_at: f64,
}

#[durable_object(fetch)]
pub struct RateLimitDO {
    state: State,
}

impl DurableObject for RateLimitDO {
    fn new(state: State, _env: Env) -> Self {
        Self { state }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        match req.path().as_str() {
            "/check" => self.check(&mut req).await,
            _ => Response::error("Not found", 404),
        }
    }

    /// F3: expired counters used to linger forever (unbounded storage growth —
    /// keys include attacker-chosen emails and per-visitor IPs). The alarm
    /// sweeps them off the request path; `check` re-arms it whenever it is not
    /// already scheduled, so each shard self-cleans at most ~once per minute.
    async fn alarm(&self) -> Result<Response> {
        let now = clock::now_ms();
        let map = self
            .state
            .storage()
            .list_with_options(ListOptions::new().prefix("rl:"))
            .await?;
        let mut expired: Vec<String> = Vec::new();
        for entry in map.entries() {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let arr = js_sys::Array::from(&entry);
            let key = arr.get(0).as_string().unwrap_or_default();
            if key.is_empty() {
                continue;
            }
            let json = match js_sys::JSON::stringify(&arr.get(1)) {
                Ok(s) => s.as_string().unwrap_or_default(),
                Err(_) => continue,
            };
            let Ok(counter) = serde_json::from_str::<RateCounter>(&json) else {
                continue;
            };
            if counter.reset_at < now {
                expired.push(key);
            }
        }
        if !expired.is_empty() {
            self.state
                .storage()
                .delete_multiple(expired.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .await?;
        }
        Ok(Response::empty()?)
    }
}

impl RateLimitDO {
    /// POST /check — body `{key, limit, windowMs}` -> `{allowed, retryAfterMs}`.
    async fn check(&self, req: &mut Request) -> Result<Response> {
        let body: CheckBody = req.json().await.map_err(|_| Error::from("invalid body"))?;
        let now = clock::now_ms();
        let storage_key = format!("rl:{}", body.key);
        let stored: Option<RateCounter> = self.state.storage().get(&storage_key).await?;
        let (counter, allowed, retry_after_ms) = match stored {
            // Window still open — count against the limit.
            Some(entry) if now <= entry.reset_at => {
                if entry.count >= body.limit {
                    let retry_after_ms = (entry.reset_at - now).max(0.0);
                    (entry, false, retry_after_ms)
                } else {
                    (
                        RateCounter {
                            count: entry.count + 1,
                            reset_at: entry.reset_at,
                        },
                        true,
                        0.0,
                    )
                }
            }
            // Window expired (or first hit) — reset in place with a fresh window.
            _ => (
                RateCounter {
                    count: 1,
                    reset_at: now + body.window_ms,
                },
                true,
                0.0,
            ),
        };
        // Persist only when the counter changed (a denial reads without writing).
        if allowed {
            self.state.storage().put(&storage_key, &counter).await?;
        }
        self.arm_sweep_if_due(now).await;
        Response::from_json(&serde_json::json!({
            "allowed": allowed,
            "retryAfterMs": retry_after_ms,
        }))
    }

    /// Arm the sweep alarm ~1s out when none is scheduled. `get_alarm` is a
    /// cheap runtime call; `set_alarm` only happens on arming, so the
    /// steady-state per-check cost is unchanged.
    async fn arm_sweep_if_due(&self, _now: f64) {
        let due = match self.state.storage().get_alarm().await {
            Ok(None) => true,
            Ok(Some(_)) => false,
            Err(_) => return,
        };
        if !due {
            return;
        }
        let _ = self.state.storage().set_alarm(Duration::from_secs(1));
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckBody {
    key: String,
    limit: u32,
    window_ms: f64,
}
