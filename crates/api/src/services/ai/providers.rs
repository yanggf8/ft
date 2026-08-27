//! AI provider calls — mirrors backend/src/services/ai/iflow.ts + the inline Groq/
//! Cerebras calls in ai-mutex-do.ts:callProvider.
//!
//! iFlow (GLM-4.6) is a dedicated adapter; Groq & Cerebras go through the
//! OpenAI-compatible /chat/completions endpoint with the same JSON envelope.
//! All three use `Fetch::Request` with a JSON body and a 45s timeout (mirrors
//! `AbortSignal.timeout(45000)` in the TS version).

use std::time::Duration;

use super::prompts::{build_prompt, get_system_prompt};
use worker::{Delay, Error, Fetch, Headers, Method, Request, RequestInit};

pub const IFLOW_API_URL: &str = "https://apis.iflow.cn/v1/chat/completions";
pub const GROQ_BASE: &str = "https://api.groq.com/openai/v1";
pub const CEREBRAS_BASE: &str = "https://api.cerebras.ai/v1";

const PROVIDER_TIMEOUT_MS: u64 = 45000;

#[derive(Debug, Clone)]
pub struct ProviderResult {
    pub interpretation: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<f64>,
}

/// Call a single provider. Returns a `ProviderResult` or an error string that the
/// AIMutexDO uses to fail over to the next provider.
pub async fn call_provider(
    name: &str,
    model: &str,
    api_key: &str,
    chart_type: &str,
    chart_data: &serde_json::Value,
    language: Option<&str>,
    focus: Option<&str>,
    max_tokens: u32,
) -> Result<ProviderResult, String> {
    let system = get_system_prompt(chart_type, language);
    let user = build_prompt(chart_type, chart_data, language, focus);
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
        "max_tokens": max_tokens,
        "temperature": 0.7,
    });

    let (url, auth) = match name {
        "iflow" => (IFLOW_API_URL.to_string(), format!("Bearer {}", api_key)),
        "groq" => (
            format!("{}/chat/completions", GROQ_BASE),
            format!("Bearer {}", api_key),
        ),
        "cerebras" => (
            format!("{}/chat/completions", CEREBRAS_BASE),
            format!("Bearer {}", api_key),
        ),
        other => return Err(format!("unknown provider {}", other)),
    };

    let mut res = match build_request(&url, &auth, &body.to_string()).await {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };

    let status = res.status_code();
    if status != 200 {
        let text = res.text().await.unwrap_or_default();
        let preview: String = text.chars().take(200).collect();
        return Err(format!("{} {}: {}", name, status, preview));
    }
    let data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let content = data
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let content = match content {
        Some(c) if !c.is_empty() => c,
        _ => return Err(format!("{}: empty response", name)),
    };
    let tokens_used = data
        .pointer("/usage/total_tokens")
        .and_then(|v| v.as_number())
        .and_then(|n| n.as_f64());

    Ok(ProviderResult {
        interpretation: content,
        provider: name.to_string(),
        model: model.to_string(),
        tokens_used,
    })
}

async fn build_request(url: &str, auth: &str, body_json: &str) -> Result<worker::Response, Error> {
    let headers = Headers::new();
    headers.set("Authorization", auth)?;
    headers.set("Content-Type", "application/json")?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body_json.into()));
    let req = Request::new_with_init(url, &init)?;
    send_with_timeout(req).await
}

/// Wraps `Fetch::Request(req).send()` with a 45s timeout — mirrors
/// `signal: AbortSignal.timeout(45000)` in TS. Uses `worker::Delay` + `futures_util::future::select`
/// because `worker::RequestInit` (0.8.5) does not expose a `signal` field; this
/// is the Promise-race equivalent that guarantees we never hang indefinitely.
async fn send_with_timeout(req: Request) -> Result<worker::Response, Error> {
    use futures_util::future::{select, Either};
    let fetch_fut = Box::pin(async move { Fetch::Request(req).send().await });
    let timeout_fut = Box::pin(Delay::from(Duration::from_millis(PROVIDER_TIMEOUT_MS)));
    let x = match select(fetch_fut, timeout_fut).await {
        Either::Left((res, _)) => res,
        Either::Right((_, _)) => Err(Error::from(format!(
            "provider timeout after {}ms",
            PROVIDER_TIMEOUT_MS
        ))),
    };
    x
}
