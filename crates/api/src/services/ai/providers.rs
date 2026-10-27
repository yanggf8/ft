//! AI provider calls — one adapter per provider in the failover chain.
//!
//! Groq & Cerebras both go through the OpenAI-compatible /chat/completions
//! endpoint with the same JSON envelope. Each uses `Fetch::Request` with a JSON
//! body and a 45s timeout. The chain shrank to these two on 2026-10-27, when
//! the former primary's free tier ended; Groq is now priority 1 and Cerebras
//! priority 2.

use std::time::Duration;

use super::prompts::{build_prompt, get_system_prompt};
use worker::{Ai, Delay, Error, Fetch, Headers, Method, Request, RequestInit};

pub const GROQ_BASE: &str = "https://api.groq.com/openai/v1";
pub const CEREBRAS_BASE: &str = "https://api.cerebras.ai/v1";

const PROVIDER_TIMEOUT_MS: u64 = 45000;
/// palm v1 影像 payload（B1：`InterpretRequest.image` 的形狀；wire 鍵全小寫）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ImagePayload {
    pub mime: String,
    pub base64: String,
}

#[derive(Debug, Clone)]
pub struct ProviderResult {
    pub interpretation: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<f64>,
    /// B5：回應截斷旗標透傳（choices 形的 finish_reason）。
    /// 僅 Workers AI 分支填值；groq/cerebras 一律 `None`。
    pub finish_reason: Option<String>,
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
        finish_reason: None,
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

// ── Workers AI（palm v1 抽取；spec §6.2 選型終局）─────────────────────────────
/// B4：Workers AI 回應雙形解析——`result.response`／`response` 與 OpenAI 形
/// `result.choices[0].message.content`／頂層 `choices` 都要解；逐一檢查候選
/// 位置、取**第一個非空字串**（空欄位不短路，Muse 審 Minor）。
/// （`Ai` binding 版回應無 result 外殼、REST 版有——四個位置都容忍。）
pub fn parse_workers_ai_content(v: &serde_json::Value) -> Option<String> {
    [
        "/result/response",
        "/response",
        "/result/choices/0/message/content",
        "/choices/0/message/content",
    ]
    .iter()
    .filter_map(|p| v.pointer(p).and_then(|x| x.as_str()))
    .find(|s| !s.is_empty())
    .map(str::to_string)
}

/// B4/B5：finish_reason 只在 choices 形出現（`choices[0].finish_reason`）；
/// response 形 → None。
pub fn parse_workers_ai_finish_reason(v: &serde_json::Value) -> Option<String> {
    v.pointer("/result/choices/0/finish_reason")
        .or_else(|| v.pointer("/choices/0/finish_reason"))
        .and_then(|x| x.as_str())
        .map(str::to_string)
}

/// B7：Workers AI llama vision 的 user content 陣列（文字＋data URL 圖片）。
/// data URL 格式集中於此單一純函式（測形狀）；錯誤字串永不嵌入 base64（P5）。
pub fn build_messages(text: &str, image: &ImagePayload) -> serde_json::Value {
    serde_json::json!([
        { "type": "text", "text": text },
        {
            "type": "image_url",
            "image_url": { "url": format!("data:{};base64,{}", image.mime, image.base64) },
        },
    ])
}

/// B3/B7：Workers AI 呼叫（palm 抽取專用）。`Ai` binding 就地、不出站、免 API
/// key、不帶 temperature（模型預設，spike 實測條件）；45s timeout 比照
/// `send_with_timeout` 的 select＋Delay 模式。錯誤字串只含 model 名＋截斷摘要
/// （P5 單點堵漏：DO 的 console_error!/record_error/錯誤回應三路都繼承自這裡）。
pub async fn call_workers_ai(
    ai: &Ai,
    model: &str,
    chart_type: &str,
    chart_data: &serde_json::Value,
    image: &ImagePayload,
    language: Option<&str>,
    max_tokens: u32,
) -> Result<ProviderResult, String> {
    let system = get_system_prompt(chart_type, language);
    let user_text = build_prompt(chart_type, chart_data, language, None);
    let input = serde_json::json!({
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": build_messages(&user_text, image) },
        ],
        "max_tokens": max_tokens,
    });
    let data = run_with_timeout(ai, model, input).await?;
    let content =
        parse_workers_ai_content(&data).ok_or_else(|| format!("{}: empty response", model))?;
    let tokens_used = ["/usage/total_tokens", "/result/usage/total_tokens"]
        .iter()
        .filter_map(|p| {
            data.pointer(p)
                .and_then(|v| v.as_number())
                .and_then(|n| n.as_f64())
        })
        .next();
    Ok(ProviderResult {
        interpretation: content,
        provider: "workers-ai".to_string(),
        model: model.to_string(),
        tokens_used,
        finish_reason: parse_workers_ai_finish_reason(&data),
    })
}

/// `Ai::run` 包 45s timeout（select＋Delay，比照 `send_with_timeout`）。
/// 錯誤只含 model 名＋截斷摘要——絕不嵌入 input（含 base64，P5）。
async fn run_with_timeout(
    ai: &Ai,
    model: &str,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use futures_util::future::{select, Either};
    let run_fut = Box::pin(ai.run::<_, serde_json::Value>(model, input));
    let timeout_fut = Box::pin(Delay::from(Duration::from_millis(PROVIDER_TIMEOUT_MS)));
    match select(run_fut, timeout_fut).await {
        Either::Left((res, _)) => res.map_err(|e| {
            let summary: String = e.to_string().chars().take(200).collect();
            format!("{}: {}", model, summary)
        }),
        Either::Right((_, _)) => Err(format!(
            "{}: workers-ai timeout after {}ms",
            model, PROVIDER_TIMEOUT_MS
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> ImagePayload {
        ImagePayload {
            mime: "image/webp".to_string(),
            base64: "QUJD".to_string(),
        }
    }

    // ── B9-3：Workers AI 回應雙形解析 ──
    #[test]
    fn parse_workers_ai_content_response_shape() {
        // REST 外殼形 {"result": {"response": ...}}
        let v = serde_json::json!({ "result": { "response": "{\"quality\":\"ok\"}" } });
        assert_eq!(
            parse_workers_ai_content(&v).as_deref(),
            Some("{\"quality\":\"ok\"}")
        );
        // binding 形（無 result 外殼）
        let v2 = serde_json::json!({ "response": "B" });
        assert_eq!(parse_workers_ai_content(&v2).as_deref(), Some("B"));
        // response 形沒有 finish_reason
        assert_eq!(parse_workers_ai_finish_reason(&v), None);
        assert_eq!(parse_workers_ai_finish_reason(&v2), None);
    }

    #[test]
    fn parse_workers_ai_content_choices_shapes() {
        // OpenAI 形（result.choices）
        let v = serde_json::json!({
            "result": {
                "choices": [
                    { "message": { "content": "A" }, "finish_reason": "length" }
                ]
            }
        });
        assert_eq!(parse_workers_ai_content(&v).as_deref(), Some("A"));
        assert_eq!(
            parse_workers_ai_finish_reason(&v).as_deref(),
            Some("length")
        );
        // 頂層 choices 形也容忍
        let v2 = serde_json::json!({
            "choices": [
                { "message": { "content": "B" }, "finish_reason": "stop" }
            ]
        });
        assert_eq!(parse_workers_ai_content(&v2).as_deref(), Some("B"));
        assert_eq!(parse_workers_ai_finish_reason(&v2).as_deref(), Some("stop"));
    }

    #[test]
    fn parse_workers_ai_content_empty_is_none() {
        let v = serde_json::json!({ "result": { "response": "" } });
        assert_eq!(parse_workers_ai_content(&v), None);
        // 認不得的形狀
        let v2 = serde_json::json!({ "errors": [{ "code": 400 }] });
        assert_eq!(parse_workers_ai_content(&v2), None);
    }

    #[test]
    fn empty_response_field_falls_through_to_choices() {
        // Muse 審 Minor：result.response 存在但為空字串時不得短路——
        // 續查 choices 形，取第一個非空內容。
        let v = serde_json::json!({
            "result": { "response": "" },
            "choices": [ { "message": { "content": "B" } } ],
        });
        assert_eq!(parse_workers_ai_content(&v).as_deref(), Some("B"));
    }

    // ── B9-5：build_messages 形狀（data URL 組裝、content array 結構）──
    #[test]
    fn build_messages_content_array_shape() {
        let v = build_messages("請依系統指示抽取這張手掌照片的手相特徵", &image());
        let arr = v.as_array().expect("content must be an array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["type"], "text");
        assert_eq!(arr[0]["text"], "請依系統指示抽取這張手掌照片的手相特徵");
        assert_eq!(arr[1]["type"], "image_url");
        assert_eq!(arr[1]["image_url"]["url"], "data:image/webp;base64,QUJD");
    }
}
