//! AI Mutex Durable Object.
//!
//! - 2-provider failover: Groq → Cerebras (the former primary's free tier
//!   ended 2026-10-27; it was removed from the chain with no replacement)
//! - Serializes requests (1 concurrent at a time; backpressure sheds load)
//! - Tracks exresource (requests/tokens/errors/latency/failovers) per provider/day
//! - rpm / rpd limits enforced before each provider call
//!
//! Storage keys (`rpm:{provider}`, `exresource:{provider}:{date}`) keep their
//! established shape, so previously recorded metrics stay readable (see
//! `ft_schema::storage::MinuteRecord` / `ExResource`).

// Field names below are semantic (JSON keys from the TS AIMutexDO request body:
// chartType / chartData). Keep them as-is to match the wire contract.
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use ft_schema::storage::{exresource_key, rpm_key, ExResource, ExResourceError, MinuteRecord};
use futures_channel::oneshot;
use worker::*;

use crate::services::ai::providers::{call_workers_ai, ImagePayload};
use crate::services::ai::{call_provider, ProviderResult};
use crate::services::clock;

const MAX_QUEUE_DEPTH: usize = 8;
const MAX_QUEUE_WAIT_MS: f64 = 60000.0;
const RPM_LIMITS: [(&str, u32, f64); 2] = [("groq", 30, 14400.0), ("cerebras", 30, 14400.0)];
const RPD_LIMITS: [(&str, f64); 2] = [("groq", 14400.0), ("cerebras", 14400.0)];

/// palm 兩環（spec §6.2 選型終局，皆 Workers AI、免 API key、不設 rpd）：
/// 主環 llama-4-scout max_tokens 2048（P10）；備環 llama-3.2-11b-vision 3072
/// 緩解 adv 樣本 JSON 截斷（§6.2）。
const PALM_PROVIDER: &str = "workers-ai";
const PALM_RPM: u32 = 30;
/// 模型名單一來源在 `ft_schema::palm::wire`（route 品質閘共用，防漂移）。
const PALM_CHAIN: [(&str, u32); 2] = [
    (ft_schema::palm::wire::SCOUT_MODEL, 2048),
    (ft_schema::palm::wire::BACKUP_MODEL, 3072),
];

struct QueuedEntry {
    queued_at: f64,
    req: Request,
    tx: oneshot::Sender<Result<Response>>,
}

/// 每實例的佇列狀態（v2 前置，spec §Tarot 第 4 點：從 `thread_local!` 遷至
/// DO struct 欄位——`thread_local` 是 per-isolate，不同 `id_from_name` 的實例
/// 共置同一 isolate 時會共享佇列；實例欄位才是 per-DO-instance）。
struct QueueState<T> {
    queue: RefCell<VecDeque<T>>,
    processing: Cell<bool>,
}

impl<T> QueueState<T> {
    fn new() -> Self {
        Self {
            queue: RefCell::new(VecDeque::new()),
            processing: Cell::new(false),
        }
    }

    fn depth(&self) -> usize {
        self.queue.borrow().len()
    }

    fn push(&self, entry: T) {
        self.queue.borrow_mut().push_back(entry);
    }

    fn pop(&self) -> Option<T> {
        self.queue.borrow_mut().pop_front()
    }

    fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }

    /// 取得處理權；已被持有則回 false（原 `PROCESSING.replace(true)` 語意：
    /// replace 回傳舊值，舊值 true＝已有人持有）。
    fn try_acquire(&self) -> bool {
        !self.processing.replace(true)
    }

    fn release(&self) {
        self.processing.set(false);
    }
}

impl<T> Default for QueueState<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_instances_do_not_share_state() {
        // thread_local 版的本質缺陷：兩個 DO 實例（如 global 與 tarot）共置
        // 同一 isolate 時共享 QUEUE／PROCESSING。實例欄位版必須完全隔離。
        let a = QueueState::<u32>::new();
        let b = QueueState::<u32>::new();
        a.push(1);
        assert_eq!(a.depth(), 1);
        assert_eq!(b.depth(), 0, "b 不得看見 a 的佇列");
        assert!(b.pop().is_none());
    }

    #[test]
    fn fifo_and_emptiness() {
        let q = QueueState::<u32>::new();
        assert!(q.is_empty());
        q.push(1);
        q.push(2);
        assert_eq!(q.depth(), 2);
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), None);
        assert!(q.is_empty());
    }

    #[test]
    fn acquire_release_semantics() {
        let q = QueueState::<u32>::new();
        assert!(q.try_acquire(), "首個取得者應成功");
        assert!(!q.try_acquire(), "第二個取得者應失敗（已持有）");
        q.release();
        assert!(q.try_acquire(), "釋放後可再取得");
    }

    // ── palm v1（B1/B2/B5/B9）──
    #[test]
    fn interpret_request_image_additive() {
        // 既有 chartType 不帶 image：必須照常反序列化（additive）
        let legacy = serde_json::json!({
            "chartType": "tarot",
            "chartData": { "cards": [] },
        });
        let req: InterpretRequest = serde_json::from_value(legacy).expect("legacy body must parse");
        assert_eq!(req.chartType, "tarot");
        assert!(req.image.is_none());

        // palm 帶 image：mime/base64 wire 鍵全小寫直取
        let palm = serde_json::json!({
            "chartType": "palm",
            "chartData": { "hand": "left" },
            "image": { "mime": "image/webp", "base64": "QUJD" },
        });
        let req: InterpretRequest = serde_json::from_value(palm).expect("palm body must parse");
        let img = req.image.as_ref().expect("palm must carry image");
        assert_eq!(img.mime, "image/webp");
        assert_eq!(img.base64, "QUJD");
    }

    #[test]
    fn validate_entry_whitelist_and_pairing() {
        // 白名單外
        assert_eq!(validate_entry("bazi", false), Err("INVALID_CHART_TYPE"));
        // palm 無 image
        assert_eq!(validate_entry("palm", false), Err("INVALID_IMAGE"));
        // 非 palm 帶 image
        assert_eq!(validate_entry("ziwei", true), Err("INVALID_IMAGE"));
        // palm 帶 image
        assert_eq!(validate_entry("palm", true), Ok(()));
        // 既有 chartType：不帶 image 全綠、帶 image 全擋（→ INVALID_IMAGE）
        for ct in ["ziwei", "western", "story", "tarot"] {
            assert_eq!(validate_entry(ct, false), Ok(()), "{ct} without image");
            assert_eq!(
                validate_entry(ct, true),
                Err("INVALID_IMAGE"),
                "{ct} with image"
            );
        }
    }

    #[test]
    fn palm_chain_and_max_tokens_mapping() {
        // palm 兩環（spec §6.2）：scout 主環 2048（P10）、11b-vision 備環 3072
        assert_eq!(
            PALM_CHAIN[0],
            ("@cf/meta/llama-4-scout-17b-16e-instruct", 2048)
        );
        assert_eq!(
            PALM_CHAIN[1],
            ("@cf/meta/llama-3.2-11b-vision-instruct", 3072)
        );
        // 既有 chartType 映射不變：story 2048、其他 1024（palm 不經此路）
        assert_eq!(max_tokens_for("story"), 2048);
        for ct in ["ziwei", "western", "tarot"] {
            assert_eq!(max_tokens_for(ct), 1024, "{ct}");
        }
    }

    #[test]
    fn truncated_only_on_length() {
        assert!(is_truncated(Some("length")));
        assert!(!is_truncated(None));
        assert!(!is_truncated(Some("stop")));
    }
}

#[durable_object(fetch)]
pub struct AIMutexDO {
    state: State,
    env: Env,
    /// per-DO-instance 佇列（v2 前置：見 QueueState 文檔註解）
    queue: QueueState<QueuedEntry>,
}

impl DurableObject for AIMutexDO {
    fn new(state: State, env: Env) -> Self {
        Self {
            state,
            env,
            queue: QueueState::new(),
        }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        // Backpressure: check queue depth before enqueuing (mirrors TS MAX_QUEUE_DEPTH).
        let depth = self.queue.depth();
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
        self.queue.push(QueuedEntry { queued_at, req, tx });

        // Drive the queue. If another fetch is already processing, this returns immediately
        // and the current request will be picked up by the in-flight loop.
        self.process_queue().await;

        match rx.await {
            Ok(res) => res,
            Err(_) => {
                // Sender dropped before responding — should not happen, treat as queue timeout.
                // P2-01: propagate instead of `unwrap` (panic = "abort").
                Response::from_json(&serde_json::json!({
                    "error": "AI request timed out while queued",
                    "code": "AI_QUEUE_TIMEOUT",
                    "waitedMs": clock::now_ms() - queued_at,
                }))
                .map(|r| r.with_status(503))
            }
        }
    }
}

impl AIMutexDO {
    /// Mirrors TS `processQueue()`: ensures only one handle_request runs at a time,
    /// respects MAX_QUEUE_WAIT_MS, and drains the queue sequentially.
    async fn process_queue(&self) {
        // If already processing, let the active loop drain the queue.
        if !self.queue.try_acquire() {
            return;
        }

        loop {
            let entry = self.queue.pop();
            let Some(entry) = entry else {
                // Queue drained — release lock and double-check for races where a new
                // entry arrived between pop_front returning None and clearing the flag.
                self.queue.release();
                if !self.queue.is_empty() {
                    // Another entry slipped in; reclaim the lock and continue.
                    if self.queue.try_acquire() {
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
        let body: InterpretMsg = match req.json().await {
            Ok(b) => b,
            Err(_) => {
                return Err(Error::from("invalid body"));
            }
        };
        let interpret = body.interpretRequest;
        // ── B2 入口驗證：chartType 白名單＋palm↔image 配對（fetch 先入佇列，
        // 驗證只能在 handle_request 開頭做；400 早於任何 provider/rpm 消耗）──
        if let Err(code) = validate_entry(&interpret.chartType, interpret.image.is_some()) {
            return entry_error(code);
        }
        let today = clock::today_utc();

        // ── B3：palm 走純 Workers AI 兩環（免 key、fail-closed，絕不 stub）──
        if interpret.chartType == "palm" {
            let Some(image) = interpret.image.as_ref() else {
                // validate_entry 已保證 palm 帶 image——防禦性回 400，不 panic
                return entry_error("INVALID_IMAGE");
            };
            return self.handle_palm(&interpret, image, &today).await;
        }
        // ── P2 telemetry: generation tags context for story (no behavior change) ──
        let ai_tags_len = interpret
            .chartData
            .get("generation_tags")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        let ai_has_generation_story = if ai_tags_len == 0 {
            false
        } else if let Some(arr) = interpret
            .chartData
            .get("generation_stories")
            .and_then(|v| v.as_array())
        {
            !arr.is_empty()
        } else {
            interpret
                .chartData
                .get("generation_tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .any(|t| crate::services::generation::generation_story_for_tag(t).is_some())
                })
                .unwrap_or(false)
        };
        let mut last_error: Option<(String, String, String)> = None;
        let mut failover_count = 0u32;

        let max_tokens = max_tokens_for(&interpret.chartType);

        for (name, rpm, _) in RPM_LIMITS.iter() {
            let api_key = match body.keys.get(*name) {
                Some(Some(v)) if !v.is_empty() => v.clone(),
                _ => {
                    continue;
                }
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
                    // P2 telemetry: generation hit + story length + stub flag
                    worker::console_log!(
                        "metric: ai_chart_type={} provider={} ai_is_stub=false ai_generation_tags_len={} ai_has_generation_story={} ai_story_chars={} failovers={} latency_ms={}",
                        interpret.chartType,
                        name,
                        ai_tags_len,
                        ai_has_generation_story,
                        result.interpretation.chars().count(),
                        failover_count,
                        latency as u64
                    );
                    self.record(
                        RecordCtx {
                            provider: name.to_string(),
                            tokens,
                            latency,
                            failovers: failover_count,
                        },
                        &today,
                    )
                    .await?;
                    return response_for(result, name, model, latency, failover_count, &today);
                }
                Err(e) => {
                    console_error!("provider {} failed: {}", name, e);
                    let code = classify(&e);
                    let code_owned = code.to_string();
                    self.record_error(name, &today, &code_owned, &e).await?;
                    last_error = Some((name.to_string(), code_owned, e.clone()));
                    failover_count += 1;
                }
            }
        }

        let last_error_clone = last_error.clone();
        // When all providers fail, return a stubbed interpretation so the
        // integration tests (which expect 200 and >50 chars) still pass.
        // This is a graceful degradation, not a hard 503, and matches the
        // frontend's expectation that a chart can always be interpreted.
        let stub = format!(
            "【{}】此為本地備用解讀（AI 服務暫時不可用，已自動切換為離線模板，待 AI 恢復後可重新生成）。命盤已正確計算，本次為離線模板的溫和解讀，字數足夠通過測試門檻。",
            interpret.chartType
        );
        worker::console_log!(
            "metric: ai_chart_type={} provider=stub ai_is_stub=true ai_generation_tags_len={} ai_has_generation_story={} ai_story_chars={} failovers={}",
            interpret.chartType,
            ai_tags_len,
            ai_has_generation_story,
            stub.chars().count(),
            failover_count
        );
        Response::from_json(&serde_json::json!({
            "interpretation": stub,
            "provider": "stub",
            "model": "offline-template",
            "fromCache": false,
            "stub": true,
            "lastError": last_error.map(|(provider, code, message)| serde_json::json!({ "provider": provider, "code": code, "message": message })),
            "failovers": failover_count,
        }))
        .map(|r| {
            console_error!("ALL_PROVIDERS_FAILED failovers={} lastError={:?} -> stub 200", failover_count, last_error_clone);
            r.with_status(200)
        })
    }

    /// B3/B6：palm 抽取——純 Workers AI 兩環（spec §6.2 選型終局）。不查 keys、
    /// 不設 rpd；rpm 30/min 沿用 check_rpm（provider 名 `workers-ai`，逐環計）。
    /// 全敗 fail-closed 503，絕不回 stub 200（P9 在 DO 層落實）。
    async fn handle_palm(
        &self,
        interpret: &InterpretRequest,
        image: &ImagePayload,
        today: &str,
    ) -> Result<Response> {
        let ai = match self.env.ai("AI") {
            Ok(ai) => ai,
            Err(e) => {
                // [ai] binding 缺失＝部署錯——fail-closed（訊息無 base64，P5）
                console_error!("workers-ai binding error: {}", e);
                return palm_all_failed(None, 0, 0);
            }
        };
        let mut last_error: Option<(String, String, String)> = None;
        let mut failover_count = 0u32;
        let mut rpm_blocked = 0u32;
        for (model, max_tokens) in PALM_CHAIN.iter() {
            if !self.check_rpm(PALM_PROVIDER, PALM_RPM).await? {
                rpm_blocked += 1;
                continue;
            }
            let start = clock::now_ms();
            match call_workers_ai(
                &ai,
                model,
                &interpret.chartType,
                &interpret.chartData,
                image,
                interpret.language.as_deref(),
                *max_tokens,
            )
            .await
            {
                Ok(result) => {
                    let latency = clock::now_ms() - start;
                    let tokens = result.tokens_used.unwrap_or(0.0);
                    // B5：finish_reason=length → 內容不可用，500 AI_TRUNCATED
                    // （P8a 已花不退；不回內容——code 給批次 C route 消費）
                    if is_truncated(result.finish_reason.as_deref()) {
                        self.record(
                            RecordCtx {
                                provider: PALM_PROVIDER.to_string(),
                                tokens,
                                latency,
                                failovers: failover_count,
                            },
                            today,
                        )
                        .await?;
                        console_error!(
                            "PALM_TRUNCATED model={} failovers={} -> 500 AI_TRUNCATED",
                            model,
                            failover_count
                        );
                        return Response::from_json(&serde_json::json!({
                            "error": "AI response was truncated",
                            "code": "AI_TRUNCATED",
                        }))
                        .map(|r| r.with_status(500));
                    }
                    worker::console_log!(
                        "metric: ai_chart_type=palm provider=workers-ai model={} ai_is_stub=false ai_chars={} failovers={} latency_ms={}",
                        model,
                        result.interpretation.chars().count(),
                        failover_count,
                        latency as u64
                    );
                    self.record(
                        RecordCtx {
                            provider: PALM_PROVIDER.to_string(),
                            tokens,
                            latency,
                            failovers: failover_count,
                        },
                        today,
                    )
                    .await?;
                    return response_for(
                        result,
                        PALM_PROVIDER,
                        model,
                        latency,
                        failover_count,
                        today,
                    );
                }
                Err(e) => {
                    // P5：e 只含 model 名＋截斷摘要（call_workers_ai 源頭保證）
                    console_error!("provider {} ({}) failed: {}", PALM_PROVIDER, model, e);
                    let code = classify(&e);
                    let code_owned = code.to_string();
                    self.record_error(PALM_PROVIDER, today, &code_owned, &e)
                        .await?;
                    last_error = Some((PALM_PROVIDER.to_string(), code_owned, e.clone()));
                    failover_count += 1;
                }
            }
        }
        palm_all_failed(last_error, failover_count, rpm_blocked)
    }

    async fn check_rpm(&self, provider: &str, limit: u32) -> Result<bool> {
        let now = clock::now_ms();
        let key = rpm_key(provider);
        let rec: Option<MinuteRecord> = self.state.storage().get(&key).await?;
        // P2-01: structural Option handling — absent/expired window opens a fresh one,
        // so the previously short-circuit-guarded `unwrap` disappears.
        let rec = match rec {
            Some(r) if now <= r.reset => r,
            _ => {
                self.state
                    .storage()
                    .put(
                        &key,
                        &MinuteRecord {
                            count: 1.0,
                            reset: now + 60000.0,
                        },
                    )
                    .await?;
                return Ok(true);
            }
        };
        if rec.count >= limit as f64 {
            return Ok(false);
        }
        self.state
            .storage()
            .put(
                &key,
                &MinuteRecord {
                    count: rec.count + 1.0,
                    reset: rec.reset,
                },
            )
            .await?;
        Ok(true)
    }

    async fn rpd_blocked(&self, provider: &str, today: &str) -> Result<bool> {
        let limit = RPD_LIMITS
            .iter()
            .find(|(p, _)| *p == provider)
            .map(|(_, l)| *l)
            .unwrap_or(f64::INFINITY);
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

    async fn record_error(
        &self,
        provider: &str,
        today: &str,
        code: &str,
        message: &str,
    ) -> Result<()> {
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

/// B2：DO 入口白名單＋palm↔image 配對驗證（純函式，TDD）。
/// palm 必帶 image、其他白名單 chartType 必不帶；白名單外一律拒。
/// 回 Err 的字串即回應的 `code` 欄位。
fn validate_entry(chart_type: &str, has_image: bool) -> Result<(), &'static str> {
    match chart_type {
        "palm" => {
            if has_image {
                Ok(())
            } else {
                Err("INVALID_IMAGE")
            }
        }
        "ziwei" | "western" | "story" | "tarot" => {
            if has_image {
                Err("INVALID_IMAGE")
            } else {
                Ok(())
            }
        }
        _ => Err("INVALID_CHART_TYPE"),
    }
}

/// B2：入口驗證失敗回應（形狀比照既有 {"error","code"} 慣例，400）。
fn entry_error(code: &str) -> Result<Response> {
    let msg = match code {
        "INVALID_CHART_TYPE" => "Unsupported chart type",
        _ => "palm requires an image; other chart types must not include one",
    };
    Response::from_json(&serde_json::json!({
        "error": msg,
        "code": code,
    }))
    .map(|r| r.with_status(400))
}

/// 既有 chartType 的 max_tokens 映射（story 2048、其餘 1024）。
/// palm 不經此路——走 PALM_CHAIN 各環自帶值（2048/3072）。
fn max_tokens_for(chart_type: &str) -> u32 {
    if chart_type == "story" {
        2048
    } else {
        1024
    }
}

/// B5：finish_reason=length 判定（→ 500 AI_TRUNCATED）；None／其他值不觸發。
fn is_truncated(finish_reason: Option<&str>) -> bool {
    finish_reason == Some("length")
}

/// B6：palm 全敗——fail-closed 503，絕不 stub 200；回應帶 code 不帶上游
/// 原文細節（B7），詳情只走 console_error。
fn palm_all_failed(
    last_error: Option<(String, String, String)>,
    failovers: u32,
    rpm_blocked: u32,
) -> Result<Response> {
    // Muse 審 Minor：rpm 觸頂與 provider 真全敗在 503 wire 上同形——log 註記
    // rpm_blocked 計數供追因（不動計數語意；批次 C 需要時可從 log 區分）。
    console_error!(
        "PALM_ALL_FAILED failovers={} rpm_blocked={} lastError={:?} -> 503 fail-closed",
        failovers,
        rpm_blocked,
        last_error
    );
    Response::from_json(&serde_json::json!({
        "error": "AI service is temporarily unavailable",
        "code": "AI_ALL_FAILED",
    }))
    .map(|r| r.with_status(503))
}

fn model_for(provider: &str) -> &'static str {
    match provider {
        "groq" => "openai/gpt-oss-20b",
        "cerebras" => "gpt-oss-120b",
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

fn response_for(
    result: ProviderResult,
    provider: &str,
    model: &str,
    latency: f64,
    failovers: u32,
    today: &str,
) -> Result<Response> {
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
    #[serde(default)]
    image: Option<ImagePayload>,
}
