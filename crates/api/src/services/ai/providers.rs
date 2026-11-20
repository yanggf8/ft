//! AI provider calls — all on Cloudflare Workers AI（`[ai]` binding、免 key、
//! 不出站）。
//!
//! - 文字解讀（ziwei／western／story／tarot／zodiac_match）：`call_workers_ai_text`，
//!   鏈與模型在 `ai_mutex_do.rs TEXT_CHAIN`（2026-11-20 起；取代 Groq→Cerebras，
//!   後者備援已 402 失效）。
//! - 手相影像抽取：`call_workers_ai`（palm 專用，契約不變）。

use std::time::Duration;

use super::prompts::{build_prompt, get_system_prompt};
use worker::{Ai, Delay};

/// palm 影像呼叫逾時（spec §6.2，不變）。
const PALM_TIMEOUT_MS: u64 = 45000;
/// gemma-4 主環：關閉思考（實測長 prompt 30–34s；開思考 67–83s）。
pub const GEMMA_MODEL: &str = "@cf/google/gemma-4-26b-a4b-it";
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
    /// B5：回應截斷旗標透傳（choices 形 finish_reason；Responses 形
    /// `status=="incomplete"` 正規化為 `"incomplete"`）。
    pub finish_reason: Option<String>,
}

/// 文字鏈 Workers AI 請求 body（純函式）。messages 形；gemma 系列關閉思考
/// （`chat_template_kwargs.enable_thinking=false`），其他模型不帶該鍵。
pub fn build_text_payload(
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
        "max_tokens": max_tokens,
    });
    if model.starts_with("@cf/google/gemma") {
        body["chat_template_kwargs"] = serde_json::json!({ "enable_thinking": false });
    }
    body
}

/// Workers AI 每日免費額度用盡（10,000 neurons／日）。官方主碼 3036；4006 為
/// 變體；訊息含 "daily free allocation"／"10,000 neurons"。**3040（容量暫滿）
/// 與一般 429 不算**——那些要換下一環，不是全站額度。
pub fn is_daily_quota_error(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    // 自家逾時訊息（`workers-ai timeout after 23036ms`）的毫秒數可能含 3036／4006——排除。
    if m.contains("timeout") {
        return false;
    }
    has_isolated_code(&m, "3036")
        || has_isolated_code(&m, "4006")
        || m.contains("daily free allocation")
        || m.contains("10,000 neurons")
        || m.contains("10000 neurons")
}

/// `code` 出現且前後都不是英數字（`AiError: 3036: …`、`"code":3036` 命中；
/// `23036ms`、hex request id `a3036f` 不命中）。
fn has_isolated_code(m: &str, code: &str) -> bool {
    let bytes = m.as_bytes();
    m.match_indices(code).any(|(i, _)| {
        let before = i.checked_sub(1).map(|j| bytes[j]);
        let after = bytes.get(i + code.len()).copied();
        !before.is_some_and(|c| c.is_ascii_alphanumeric())
            && !after.is_some_and(|c| c.is_ascii_alphanumeric())
    })
}

// ── Workers AI（palm v1 抽取；spec §6.2 選型終局）─────────────────────────────
/// B4：Workers AI 回應解析——`result.response`／`response`、OpenAI 形
/// `result.choices[0].message.content`／頂層 `choices`，以及 Responses 形
/// （gpt-oss 系列：`output[]`）。逐一檢查候選位置、取**第一個非空字串**
/// （空欄位不短路，Muse 審 Minor）。（`Ai` binding 版無 result 外殼、REST 版有。）
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
    .or_else(|| parse_responses_output(v))
}

/// Responses 形：只串接 `type=="message"` 下 `type=="output_text"` 的 `text`，
/// **跳過 `reasoning`**（否則會把模型英文思考當解讀輸出）；再以頂層
/// `output_text` 兜底。
fn parse_responses_output(v: &serde_json::Value) -> Option<String> {
    let output = v
        .pointer("/result/output")
        .or_else(|| v.pointer("/output"))
        .and_then(|x| x.as_array());
    if let Some(items) = output {
        let text: String = items
            .iter()
            .filter(|it| it.get("type").and_then(|t| t.as_str()) == Some("message"))
            .filter_map(|it| it.get("content").and_then(|c| c.as_array()))
            .flatten()
            .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("output_text"))
            .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
            .collect();
        if !text.is_empty() {
            return Some(text);
        }
    }
    ["/result/output_text", "/output_text"]
        .iter()
        .filter_map(|p| v.pointer(p).and_then(|x| x.as_str()))
        .find(|s| !s.is_empty())
        .map(str::to_string)
}

/// B4/B5：choices 形回 `choices[0].finish_reason`；Responses 形
/// `status=="incomplete"` → `"incomplete"`；response 形 → None。
pub fn parse_workers_ai_finish_reason(v: &serde_json::Value) -> Option<String> {
    if let Some(r) = v
        .pointer("/result/choices/0/finish_reason")
        .or_else(|| v.pointer("/choices/0/finish_reason"))
        .and_then(|x| x.as_str())
    {
        return Some(r.to_string());
    }
    let status = v
        .pointer("/result/status")
        .or_else(|| v.pointer("/status"))
        .and_then(|x| x.as_str());
    (status == Some("incomplete")).then(|| "incomplete".to_string())
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
/// key、不帶 temperature（模型預設，spike 實測條件）；45s timeout（select＋Delay）。錯誤字串只含 model 名＋截斷摘要
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
    let data = run_with_timeout(ai, model, input, PALM_TIMEOUT_MS).await?;
    result_from(model, &data)
}

/// 文字解讀呼叫（Workers AI）。`timeout_ms` 由 DO 依整筆請求 deadline 計算。
#[allow(clippy::too_many_arguments)]
pub async fn call_workers_ai_text(
    ai: &Ai,
    model: &str,
    chart_type: &str,
    chart_data: &serde_json::Value,
    language: Option<&str>,
    focus: Option<&str>,
    max_tokens: u32,
    timeout_ms: u64,
) -> Result<ProviderResult, String> {
    let system = get_system_prompt(chart_type, language);
    let user = build_prompt(chart_type, chart_data, language, focus);
    let input = build_text_payload(model, &system, &user, max_tokens);
    let data = run_with_timeout(ai, model, input, timeout_ms).await?;
    result_from(model, &data)
}

fn result_from(model: &str, data: &serde_json::Value) -> Result<ProviderResult, String> {
    let content =
        parse_workers_ai_content(data).ok_or_else(|| format!("{}: empty response", model))?;
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
        finish_reason: parse_workers_ai_finish_reason(data),
    })
}

/// `Ai::run` 包逾時（select＋Delay；`worker::RequestInit` 無 signal 欄位）。
/// 錯誤只含 model 名＋截斷摘要——絕不嵌入 input（含 base64，P5）。
async fn run_with_timeout(
    ai: &Ai,
    model: &str,
    input: serde_json::Value,
    timeout_ms: u64,
) -> Result<serde_json::Value, String> {
    use futures_util::future::{select, Either};
    let run_fut = Box::pin(ai.run::<_, serde_json::Value>(model, input));
    let timeout_fut = Box::pin(Delay::from(Duration::from_millis(timeout_ms)));
    match select(run_fut, timeout_fut).await {
        Either::Left((res, _)) => res.map_err(|e| {
            let summary: String = e.to_string().chars().take(200).collect();
            format!("{}: {}", model, summary)
        }),
        Either::Right((_, _)) => Err(format!(
            "{}: workers-ai timeout after {}ms",
            model, timeout_ms
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

    // ── 階段 0：Responses 形（gpt-oss）──
    #[test]
    fn responses_shape_skips_reasoning() {
        let v = serde_json::json!({ "result": { "status": "completed", "output": [
            { "type": "reasoning", "content": [ { "type": "reasoning_text", "text": "We need to think" } ] },
            { "type": "message", "content": [
                { "type": "output_text", "text": "## 命宮\n" },
                { "type": "output_text", "text": "內容" }
            ] }
        ] } });
        assert_eq!(
            parse_workers_ai_content(&v).as_deref(),
            Some("## 命宮\n內容")
        );
        assert_eq!(parse_workers_ai_finish_reason(&v), None);
    }

    #[test]
    fn responses_shape_reasoning_only_is_none() {
        let v = serde_json::json!({ "output": [
            { "type": "reasoning", "content": [ { "type": "reasoning_text", "text": "thinking..." } ] }
        ] });
        assert_eq!(parse_workers_ai_content(&v), None, "不得把思考當內容");
    }

    #[test]
    fn responses_shape_incomplete_is_truncation() {
        let v = serde_json::json!({ "result": { "status": "incomplete",
            "incomplete_details": { "reason": "max_output_tokens" },
            "output": [ { "type": "message", "content": [ { "type": "output_text", "text": "半" } ] } ] } });
        assert_eq!(
            parse_workers_ai_finish_reason(&v).as_deref(),
            Some("incomplete")
        );
    }

    #[test]
    fn real_gemma_rest_fixture_parses() {
        // 2026-11-20 以主環 body 形狀實打 REST 的真實回應（紫微 prompt）
        let raw = include_str!("../../../tests/fixtures/workers_ai_gemma4_ziwei.json");
        let v: serde_json::Value = serde_json::from_str(raw).expect("fixture json");
        let text = parse_workers_ai_content(&v).expect("must extract content");
        assert!(
            text.chars().count() > 300,
            "內容過短：{}",
            text.chars().count()
        );
        assert!(text.contains("紫微"));
        assert_eq!(parse_workers_ai_finish_reason(&v).as_deref(), Some("stop"));
    }

    #[test]
    fn text_payload_gemma_disables_thinking() {
        let v = build_text_payload(GEMMA_MODEL, "S", "U", 2048);
        assert_eq!(v["chat_template_kwargs"]["enable_thinking"], false);
        assert_eq!(v["max_tokens"], 2048);
        assert_eq!(v["messages"][0]["role"], "system");
        assert_eq!(v["messages"][1]["content"], "U");
        let l = build_text_payload("@cf/meta/llama-3.3-70b-instruct-fp8-fast", "S", "U", 1536);
        assert!(l.get("chat_template_kwargs").is_none());
        assert_eq!(l["max_tokens"], 1536);
    }

    #[test]
    fn daily_quota_error_detection() {
        assert!(is_daily_quota_error(
            "@cf/x: AiError: 3036: you have used up your daily free allocation of 10,000 neurons"
        ));
        assert!(is_daily_quota_error(
            "AiError: 4006: daily free allocation exceeded"
        ));
        assert!(!is_daily_quota_error(
            "AiError: 3040: Capacity temporarily exceeded, please try again (429)"
        ));
        assert!(!is_daily_quota_error("429 Too Many Requests"));
        assert!(!is_daily_quota_error(
            "@cf/x: workers-ai timeout after 55000ms"
        ));
        // 逾時毫秒數、hex request id 裡的 3036／4006 不得誤判
        assert!(!is_daily_quota_error(
            "@cf/x: workers-ai timeout after 23036ms"
        ));
        assert!(!is_daily_quota_error(
            "@cf/x: workers-ai timeout after 24006ms"
        ));
        assert!(!is_daily_quota_error(
            "AiError: 5000: upstream error (request id a3036f9e40061b)"
        ));
        assert!(!is_daily_quota_error("InferenceUpstreamError: 13036 bytes"));
        // 帶界符的代碼仍命中
        assert!(is_daily_quota_error(r#"{"code":3036,"message":"quota"}"#));
        assert!(is_daily_quota_error("AiError: 4006: something"));
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
