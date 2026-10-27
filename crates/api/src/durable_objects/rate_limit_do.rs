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
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
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
            // palm v1（spec P8a）：低品質 422 返還 uid 額度——僅核銷「原視窗」
            // 的已計次（跨視窗拒退、下限 0、重複退被下限吸收；冪等安全）。
            "/refund" => self.refund(&mut req).await,
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
            // palm v1：route 端 refund 需原視窗識別（Codex B3——防跨視窗扣到新視窗）。
            "resetAt": counter.reset_at,
        }))
    }

    /// POST /refund — body `{key, resetAt}` -> `{ok, count}`。核銷同一視窗的
    /// 一次計數（`apply_refund` 為純函式，見 tests）。
    async fn refund(&self, req: &mut Request) -> Result<Response> {
        let body: RefundBody = req.json().await.map_err(|_| Error::from("invalid body"))?;
        let storage_key = format!("rl:{}", body.key);
        let stored: Option<RateCounter> = self.state.storage().get(&storage_key).await?;
        let prev_count = stored.as_ref().map(|c| c.count).unwrap_or(0);
        let updated = apply_refund(stored, body.reset_at);
        let count = match &updated {
            Some(c) => {
                self.state.storage().put(&storage_key, c).await?;
                c.count
            }
            None => prev_count,
        };
        Response::from_json(&serde_json::json!({ "ok": true, "count": count }))
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefundBody {
    key: String,
    reset_at: f64,
}

/// refund 核心邏輯（純函式）：僅當儲存的視窗 `reset_at` 與票根完全相同才退
/// 一次（浮點原樣回傳往返不失真）；跨視窗／不存在／已見底 → `None`（不動）。
fn apply_refund(stored: Option<RateCounter>, ticket_reset_at: f64) -> Option<RateCounter> {
    let c = stored?;
    if c.reset_at != ticket_reset_at || c.count == 0 {
        return None;
    }
    Some(RateCounter {
        count: c.count - 1,
        reset_at: c.reset_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counter(count: u32, reset_at: f64) -> RateCounter {
        RateCounter { count, reset_at }
    }

    #[test]
    fn refund_same_window_decrements() {
        let c = counter(2, 1_000.0);
        assert_eq!(
            apply_refund(Some(c.clone()), 1_000.0),
            Some(counter(1, 1_000.0))
        );
    }

    #[test]
    fn refund_cross_window_is_noop() {
        // 視窗已輪替：退票不許扣新視窗（Codex B3）
        let c = counter(1, 2_000.0);
        assert_eq!(apply_refund(Some(c), 1_000.0), None);
    }

    #[test]
    fn refund_saturates_at_zero() {
        let c = counter(0, 1_000.0);
        assert_eq!(apply_refund(Some(c), 1_000.0), None);
        // 重複退第二次（count 已 1→0 之後）也被下限吸收
        let c2 = counter(1, 1_000.0);
        assert_eq!(apply_refund(Some(c2), 1_000.0), Some(counter(0, 1_000.0)));
    }

    #[test]
    fn refund_missing_counter_is_noop() {
        assert_eq!(apply_refund(None, 1_000.0), None);
    }

    #[test]
    fn refund_check_body_camel_case() {
        let b: RefundBody =
            serde_json::from_str(r#"{"key":"palm:ai:u1","resetAt":123.5}"#).unwrap();
        assert_eq!(b.key, "palm:ai:u1");
        assert_eq!(b.reset_at, 123.5);
    }
}
