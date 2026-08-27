//! AI Mutex Durable Object — mirrors backend/src/durable-objects/ai-mutex-do.ts.
//!
//! - 3-provider failover: iFlow → Groq → Cerebras
//! - Serializes requests (1 concurrent at a time; backpressure sheds load)
//! - Tracks exresource (requests/tokens/errors/latency/failovers) per provider/day
//! - rpm / rpd limits enforced before each provider call
//!
//! Storage keys (`rpm:{provider}`, `exresource:{provider}:{date}`) match the JS
//! version so the Stage B in-place deploy keeps AI metrics intact (see
//! `ft_schema::storage::MinuteRecord` / `ExResource`).

// Field names below are semantic (JSON keys from the TS AIMutexDO request body:
// chartType / chartData). Keep them as-is to match the wire contract.
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use ft_schema::storage::{exresource_key, rpm_key, ExResource, ExResourceError, MinuteRecord};
use futures_channel::oneshot;
use worker::*;

use crate::services::ai::{call_provider, ProviderResult};
use crate::services::clock;

const MAX_QUEUE_DEPTH: usize = 8;
const MAX_QUEUE_WAIT_MS: f64 = 60000.0;
const RPM_LIMITS: [(&str, u32, f64); 3] = [
    ("iflow", 1, f64::INFINITY),
    ("groq", 30, 14400.0),
    ("cerebras", 30, 14400.0),
];
const RPD_LIMITS: [(&str, f64); 3] = [
    ("iflow", f64::INFINITY),
    ("groq", 14400.0),
    ("cerebras", 14400.0),
];

struct QueuedEntry {
    queued_at: f64,
    req: Request,
    tx: oneshot::Sender<Result<Response>>,
}

std::thread_local! {
    static QUEUE: RefCell<VecDeque<QueuedEntry>> = RefCell::new(VecDeque::new());
    static PROCESSING: Cell<bool> = Cell::new(false);
}

#[durable_object(fetch)]
pub struct AIMutexDO {
    state: State,
    _env: Env,
}

impl DurableObject for AIMutexDO {
    fn new(state: State, env: Env) -> Self {
        Self { state, _env: env }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        // Backpressure: check queue depth before enqueuing (mirrors TS MAX_QUEUE_DEPTH).
        let depth = QUEUE.with(|q| q.borrow().len());
        if depth >= MAX_QUEUE_DEPTH {
            let mut r = Response::from_json(&serde_json::json!({
                "error": "AI queue is full, please try again shortly",
                "code": "AI_QUEUE_FULL",
                "queueDepth": depth,
            }))?
            .with_status(503);
            r.headers_mut().append("Retry-After", "30")?;
            return Ok(r);
        }

        let (tx, rx) = oneshot::channel();
        let queued_at = clock::now_ms();
        QUEUE.with(|q| {
            q.borrow_mut().push_back(QueuedEntry {
                queued_at,
                req,
                tx,
            })
        });

        // Drive the queue. If another fetch is already processing, this returns immediately
        // and the current request will be picked up by the in-flight loop.
        self.process_queue().await;

        match rx.await {
            Ok(res) => res,
            Err(_) => {
                // Sender dropped before responding — should not happen, treat as queue timeout.
                Ok(Response::from_json(&serde_json::json!({
                    "error": "AI request timed out while queued",
                    "code": "AI_QUEUE_TIMEOUT",
                    "waitedMs": clock::now_ms() - queued_at,
                }))
                .unwrap()
                .with_status(503))
            }
        }
    }
}

impl AIMutexDO {
    /// Mirrors TS `processQueue()`: ensures only one handle_request runs at a time,
    /// respects MAX_QUEUE_WAIT_MS, and drains the queue sequentially.
    async fn process_queue(&self) {
        // If already processing, let the active loop drain the queue.
        if PROCESSING.with(|p| p.replace(true)) {
            return;
        }

        loop {
            let entry = QUEUE.with(|q| q.borrow_mut().pop_front());
            let Some(entry) = entry else {
                // Queue drained — release lock and double-check for races where a new
                // entry arrived between pop_front returning None and clearing the flag.
                PROCESSING.with(|p| p.set(false));
                if QUEUE.with(|q| !q.borrow().is_empty()) {
                    // Another entry slipped in; reclaim the lock and continue.
                    if !PROCESSING.with(|p| p.replace(true)) {
                        continue;
                    }
                }
                return;
            };

            let waited = clock::now_ms() - entry.queued_at;
            if waited > MAX_QUEUE_WAIT_MS {
                let resp = Response::from_json(&serde_json::json!({
                    "error": "AI request timed out while queued",
                    "code": "AI_QUEUE_TIMEOUT",
                    "waitedMs": waited,
                }))
                .map(|r| r.with_status(503));
                let resp = match resp {
                    Ok(mut r) => {
                        let _ = r.headers_mut().append("Retry-After", "30");
                        Ok(r)
                    }
                    Err(e) => Err(e),
                };
                let _ = entry.tx.send(resp);
                continue;
            }

            let res = self.handle_request(entry.req).await;
            let _ = entry.tx.send(res);
        }
    }

    async fn handle_request(&self, mut req: Request) -> Result<Response> {
        let body: InterpretMsg = req.json().await.map_err(|_| Error::from("invalid body"))?;
        let interpret = body.interpretRequest;
        let today = clock::today_utc();
        let mut last_error: Option<(String, String, String)> = None;
        let mut failover_count = 0u32;

        let max_tokens = if interpret.chartType == "story" { 2048 } else { 1024 };

        for (name, rpm, _) in RPM_LIMITS.iter() {
            let api_key = match body.keys.get(*name) {
                Some(Some(v)) if !v.is_empty() => v.clone(),
                _ => continue,
            };
            let model = model_for(name);

            if self.rpd_blocked(name, &today).await? {
                continue;
            }
            if !self.check_rpm(name, *rpm).await? {
                continue;
            }

            let start = clock::now_ms();
            match call_provider(
                name,
                model,
                &api_key,
                &interpret.chartType,
                &interpret.chartData,
                interpret.language.as_deref(),
                interpret.focus.as_deref(),
                max_tokens,
            )
            .await
            {
                Ok(result) => {
                    let latency = clock::now_ms() - start;
                    let tokens = result.tokens_used.unwrap_or(0.0);
                    self.record(RecordCtx {
                        provider: name.to_string(),
                        tokens,
                        latency,
                        failovers: failover_count,
                    }, &today).await?;
                    return response_for(result, name, model, latency, failover_count, &today);
                }
                Err(e) => {
                    let code = classify(&e);
                    let code_owned = code.to_string();
                    self.record_error(name, &today, &code_owned, &e).await?;
                    last_error = Some((name.to_string(), code_owned, e.clone()));
                    failover_count += 1;
                }
            }
        }

        Response::from_json(&serde_json::json!({
            "error": "All providers failed",
            "code": "ALL_PROVIDERS_FAILED",
            "lastError": last_error.map(|(provider, code, message)| serde_json::json!({ "provider": provider, "code": code, "message": message })),
            "failovers": failover_count,
        }))
        .map(|r| r.with_status(503))
    }

    async fn check_rpm(&self, provider: &str, limit: u32) -> Result<bool> {
        let now = clock::now_ms();
        let key = rpm_key(provider);
        let rec: Option<MinuteRecord> = self.state.storage().get(&key).await?;
        if rec.is_none() || now > rec.as_ref().unwrap().reset {
            self.state.storage().put(&key, &MinuteRecord { count: 1.0, reset: now + 60000.0 }).await?;
            return Ok(true);
        }
        let rec = rec.unwrap();
        if rec.count >= limit as f64 {
            return Ok(false);
        }
        self.state.storage().put(&key, &MinuteRecord { count: rec.count + 1.0, reset: rec.reset }).await?;
        Ok(true)
    }

    async fn rpd_blocked(&self, provider: &str, today: &str) -> Result<bool> {
        let limit = RPD_LIMITS.iter().find(|(p, _)| *p == provider).map(|(_, l)| *l).unwrap_or(f64::INFINITY);
        if limit.is_infinite() {
            return Ok(false);
        }
        let key = exresource_key(provider, today);
        let rec: Option<ExResource> = self.state.storage().get(&key).await?;
        Ok(rec.map(|r| r.requests >= limit).unwrap_or(false))
    }

    async fn record(&self, r: RecordCtx, today: &str) -> Result<()> {
        let key = exresource_key(&r.provider, today);
        let cur: Option<ExResource> = self.state.storage().get(&key).await?;
        let mut v = cur.unwrap_or(empty_exresource());
        v.requests += 1.0;
        v.tokens += r.tokens;
        v.latencySum += r.latency;
        v.failovers += r.failovers as f64;
        self.state.storage().put(&key, &v).await?;
        Ok(())
    }

    async fn record_error(&self, provider: &str, today: &str, code: &str, message: &str) -> Result<()> {
        let key = exresource_key(provider, today);
        let cur: Option<ExResource> = self.state.storage().get(&key).await?;
        let mut v = cur.unwrap_or(empty_exresource());
        v.errors += 1.0;
        v.lastError = Some(ExResourceError {
            time: clock::now_iso(),
            code: code.to_string(),
            message: message.chars().take(200).collect(),
        });
        self.state.storage().put(&key, &v).await?;
        Ok(())
    }
}

// ── statics / helpers ─────────────────────────────────────────────────────────

fn model_for(provider: &str) -> &'static str {
    match provider {
        "iflow" => "GLM-4.6",
        "groq" => "moonshotai/kimi-k2-instruct-0905",
        "cerebras" => "llama-3.3-70b",
        _ => "",
    }
}

fn classify(e: &str) -> &'static str {
    if e.contains("429") {
        "RATE_LIMIT"
    } else if e.contains("401") {
        "AUTH"
    } else {
        "API_ERROR"
    }
}

fn empty_exresource() -> ExResource {
    ExResource {
        requests: 0.0,
        tokens: 0.0,
        errors: 0.0,
        latencySum: 0.0,
        failovers: 0.0,
        lastError: None,
    }
}

struct RecordCtx {
    provider: String,
    tokens: f64,
    latency: f64,
    failovers: u32,
}

fn response_for(result: ProviderResult, provider: &str, model: &str, latency: f64, failovers: u32, today: &str) -> Result<Response> {
    Response::from_json(&serde_json::json!({
        "interpretation": result.interpretation,
        "provider": provider,
        "model": model,
        "exresource": {
            "provider": provider,
            "model": model,
            "latency": latency,
            "failovers": failovers,
            "date": today,
        },
    }))
}

#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct InterpretMsg {
    #[serde(default)]
    keys: std::collections::HashMap<String, Option<String>>,
    interpretRequest: InterpretRequest,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct InterpretRequest {
    #[serde(rename = "chartType")]
    chartType: String,
    #[serde(rename = "chartData")]
    chartData: serde_json::Value,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    focus: Option<String>,
}
