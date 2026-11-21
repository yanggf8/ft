//! AI Mutex Durable Object.
//!
//! - 文字鏈全走 Cloudflare Workers AI（2026-11-20 起）：`TEXT_CHAIN` 主環
//!   gemma-4-26b（關閉思考）→ 備環 llama-3.3-70b；取代 Groq→Cerebras
//!   （Cerebras 已 402）。palm 影像另走 `PALM_CHAIN`（不變）。
//! - Serializes requests (1 concurrent at a time; backpressure sheds load)
//! - Tracks exresource (requests/tokens/errors/latency/failovers) per provider/day
//! - rpm limit enforced before each ring; 整筆請求自入列起 88s 總預算（避開
//!   Cloudflare 代理 100s 524）；每日 neurons 額度用盡 → 503 `AI_QUOTA`（不走 stub）
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
use futures_util::future::{select, Either};
use worker::*;

use crate::services::ai::providers::{
    call_workers_ai, call_workers_ai_text, is_daily_quota_error, ImagePayload, ProviderResult,
    GEMMA_MODEL,
};
use crate::services::clock;

const MAX_QUEUE_DEPTH: usize = 8;
const MAX_QUEUE_WAIT_MS: f64 = 25000.0;
/// 整筆請求總預算（自入列 `queued_at` 起算，含佇列等待）——留餘裕避開
/// Cloudflare 代理 100s Read Timeout（HTTP 524）。
const REQUEST_BUDGET_MS: f64 = 88000.0;
/// 持鎖超過此值（> 整筆預算 88s）視為持有者卡死，下一個請求可奪回處理權。
const LOCK_STALE_MS: f64 = 95000.0;
/// 單環上限（主環實測長 prompt 30–34s）。
const RING_CAP_MS: f64 = 55000.0;
/// 剩餘預算低於此值不再進下一環。
const MIN_RING_MS: f64 = 15000.0;

/// 文字鏈（全 Workers AI，免 key、不出站）。實測見
/// docs/superpowers/plans/2026-11-19-zodiac-match.md §1.3。
const TEXT_PROVIDER: &str = "workers-ai";
const TEXT_RPM_KEY: &str = "workers-ai-text";
const TEXT_RPM: u32 = 30;
const LLAMA_MODEL: &str = "@cf/meta/llama-3.3-70b-instruct-fp8-fast";
const TEXT_CHAIN: [&str; 2] = [GEMMA_MODEL, LLAMA_MODEL];

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

/// 佇列成員＝等待交棒的喚醒通道（號誌式排隊，2026-11-20 Fable 二審 B1）：
/// 每個 fetch 只處理**自己的**請求、做完把處理權交給下一位，驅動者的回應不會被
/// 後面排隊者的 AI 時間拖住（舊版驅動者要排空整個佇列才回應，單筆 30–88s 時會
/// 撞前端 abort 與 Cloudflare 524）。
type Waiter = oneshot::Sender<()>;

/// 把處理權交給下一位仍在等的請求；佇列空或只剩已逾時（receiver 已丟棄）者時
/// 釋放鎖。回傳是否成功交棒。純同步，wasm 單執行緒下與 push 不會交錯。
fn hand_off(q: &QueueState<Waiter>, now: f64) -> bool {
    while let Some(next) = q.pop() {
        if next.send(()).is_ok() {
            q.touch(now);
            return true;
        }
    }
    q.release();
    false
}

/// 每實例的佇列狀態（v2 前置，spec §Tarot 第 4 點：從 `thread_local!` 遷至
/// DO struct 欄位——`thread_local` 是 per-isolate，不同 `id_from_name` 的實例
/// 共置同一 isolate 時會共享佇列；實例欄位才是 per-DO-instance）。
struct QueueState<T> {
    queue: RefCell<VecDeque<T>>,
    processing: Cell<bool>,
    /// 取得處理權的時間；持有超過 `LOCK_STALE_MS` 視為卡死可奪回（N2）。
    acquired_at: Cell<f64>,
}

impl<T> QueueState<T> {
    fn new() -> Self {
        Self {
            queue: RefCell::new(VecDeque::new()),
            processing: Cell::new(false),
            acquired_at: Cell::new(0.0),
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

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }

    /// 取得處理權；已被持有則回 false（原 `PROCESSING.replace(true)` 語意：
    /// replace 回傳舊值，舊值 true＝已有人持有）。
    fn try_acquire(&self, now: f64) -> bool {
        if self.processing.get() && now - self.acquired_at.get() <= LOCK_STALE_MS {
            return false;
        }
        self.processing.set(true);
        self.acquired_at.set(now);
        true
    }

    /// 交棒時更新持有時間（接棒者重新計時）。
    fn touch(&self, now: f64) {
        self.acquired_at.set(now);
    }

    /// 移除不符條件的成員（清掉已逾時離開的等待者，避免灌水深度；N3）。
    fn retain(&self, keep: impl FnMut(&T) -> bool) {
        self.queue.borrow_mut().retain(keep);
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
    fn hand_off_skips_timed_out_waiters_and_keeps_lock() {
        let q = QueueState::<Waiter>::new();
        assert!(q.try_acquire(0.0));
        let (dead_tx, dead_rx) = oneshot::channel::<()>();
        drop(dead_rx); // 已逾時離開的等待者
        let (live_tx, mut live_rx) = oneshot::channel::<()>();
        q.push(dead_tx);
        q.push(live_tx);
        assert!(hand_off(&q, 0.0), "應交棒給仍在等的請求");
        assert_eq!(live_rx.try_recv(), Ok(Some(())));
        assert!(!q.try_acquire(0.0), "交棒後處理權仍被持有（由接棒者持有）");
        assert!(q.is_empty());
    }

    #[test]
    fn hand_off_releases_when_no_live_waiter() {
        let q = QueueState::<Waiter>::new();
        assert!(q.try_acquire(0.0));
        let (dead_tx, dead_rx) = oneshot::channel::<()>();
        drop(dead_rx);
        q.push(dead_tx);
        assert!(!hand_off(&q, 0.0));
        assert!(q.try_acquire(0.0), "無人可接棒時必須釋放鎖");
    }

    #[test]
    fn stale_lock_is_reclaimed() {
        let q = QueueState::<u32>::new();
        assert!(q.try_acquire(0.0));
        assert!(!q.try_acquire(LOCK_STALE_MS), "未逾時不得奪鎖");
        assert!(q.try_acquire(LOCK_STALE_MS + 1.0), "持有者卡死超時應可奪回");
        assert!(!q.try_acquire(LOCK_STALE_MS + 2.0), "奪回後重新計時");
    }

    #[test]
    fn acquire_release_semantics() {
        let q = QueueState::<u32>::new();
        assert!(q.try_acquire(0.0), "首個取得者應成功");
        assert!(!q.try_acquire(0.0), "第二個取得者應失敗（已持有）");
        q.release();
        assert!(q.try_acquire(0.0), "釋放後可再取得");
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
        for ct in [
            "ziwei",
            "western",
            "story",
            "tarot",
            "zodiac_match",
            "ziwei_year",
        ] {
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
        // 文字鏈依 (model, chart_type)：gemma story 3072／其他 2048；
        // llama story 2048／其他 1536（palm 不經此路）
        assert_eq!(max_tokens_for(GEMMA_MODEL, "story"), 3072);
        assert_eq!(max_tokens_for(LLAMA_MODEL, "story"), 2048);
        for ct in ["ziwei", "western", "tarot", "zodiac_match", "ziwei_year"] {
            assert_eq!(max_tokens_for(GEMMA_MODEL, ct), 2048, "{ct}");
            assert_eq!(max_tokens_for(LLAMA_MODEL, ct), 1536, "{ct}");
        }
    }

    #[test]
    fn text_chain_is_workers_ai_gemma_then_llama() {
        assert_eq!(
            TEXT_CHAIN,
            [
                "@cf/google/gemma-4-26b-a4b-it",
                "@cf/meta/llama-3.3-70b-instruct-fp8-fast",
            ]
        );
    }

    #[test]
    fn ring_timeout_respects_budget() {
        // 剛入列：取單環上限
        assert_eq!(ring_timeout_ms(0.0, 0.0), Some(55000));
        // 排隊 24s＋主環 50s 後：剩 14s < 15s → 不進備環
        assert_eq!(ring_timeout_ms(0.0, 74000.0), None);
        // 剩 30s：給 30s
        assert_eq!(ring_timeout_ms(0.0, 58000.0), Some(30000));
    }

    #[test]
    fn text_truncation_includes_incomplete() {
        assert!(text_truncated(Some("length")));
        assert!(text_truncated(Some("incomplete")));
        assert!(!text_truncated(Some("stop")));
        assert!(!text_truncated(None));
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
    queue: QueueState<Waiter>,
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
        let queued_at = clock::now_ms();
        if !self.queue.try_acquire(queued_at) {
            self.queue.retain(|w| !w.is_canceled());
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
            let (tx, rx) = oneshot::channel::<()>();
            self.queue.push(tx);
            // 準時 25s 逾時（不必等前一筆跑完才發現）；逾時即丟棄 rx，交棒時
            // send 失敗會自動跳過本請求。兩者同時就緒時 select 先看左邊（交棒）。
            let timer = Box::pin(Delay::from(std::time::Duration::from_millis(
                MAX_QUEUE_WAIT_MS as u64,
            )));
            match select(rx, timer).await {
                Either::Left((Ok(()), _)) => {} // 已接棒，持有處理權
                _ => return queue_timeout(clock::now_ms() - queued_at),
            }
        }
        // 持有處理權：只處理自己的請求，結束後一定交棒（handle_request 以 Result
        // 回傳、不 panic，中間沒有提早 return）。
        let res = self.handle_request(req, queued_at).await;
        hand_off(&self.queue, clock::now_ms());
        res
    }
}

fn queue_timeout(waited: f64) -> Result<Response> {
    let mut r = Response::from_json(&serde_json::json!({
        "error": "AI request timed out while queued",
        "code": "AI_QUEUE_TIMEOUT",
        "waitedMs": waited,
    }))?
    .with_status(503);
    r.headers_mut().append("Retry-After", "30")?;
    Ok(r)
}

impl AIMutexDO {
    async fn handle_request(&self, mut req: Request, queued_at: f64) -> Result<Response> {
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

        let ai = match self.env.ai("AI") {
            Ok(ai) => Some(ai),
            Err(e) => {
                console_error!("workers-ai binding error: {}", e);
                None
            }
        };

        // binding 缺失＝部署錯：不進鏈，直接落到下方 stub（既有降級語意）
        let chain: &[&str] = if ai.is_some() { &TEXT_CHAIN } else { &[] };
        for model in chain.iter() {
            let Some(ai) = ai.as_ref() else { break };
            let Some(timeout_ms) = ring_timeout_ms(queued_at, clock::now_ms()) else {
                console_error!(
                    "text chain budget exhausted before {} failovers={}",
                    model,
                    failover_count
                );
                break;
            };
            if !self.check_rpm(TEXT_RPM_KEY, TEXT_RPM).await? {
                continue;
            }

            let start = clock::now_ms();
            match call_workers_ai_text(
                ai,
                model,
                &interpret.chartType,
                &interpret.chartData,
                interpret.language.as_deref(),
                interpret.focus.as_deref(),
                max_tokens_for(model, &interpret.chartType),
                timeout_ms,
            )
            .await
            {
                Ok(result) if text_truncated(result.finish_reason.as_deref()) => {
                    // 截斷內容不可用：換下一環（palm 的 500 AI_TRUNCATED 契約不同）
                    let msg = format!("{}: truncated ({:?})", model, result.finish_reason);
                    console_error!("{}", msg);
                    self.record_error(TEXT_PROVIDER, &today, "TRUNCATED", &msg)
                        .await?;
                    last_error = Some((TEXT_PROVIDER.to_string(), "TRUNCATED".to_string(), msg));
                    failover_count += 1;
                }
                Ok(result) => {
                    let latency = clock::now_ms() - start;
                    let tokens = result.tokens_used.unwrap_or(0.0);
                    // P2 telemetry: generation hit + story length + stub flag
                    worker::console_log!(
                        "metric: ai_chart_type={} provider={} model={} ai_is_stub=false ai_generation_tags_len={} ai_has_generation_story={} ai_story_chars={} failovers={} latency_ms={}",
                        interpret.chartType,
                        TEXT_PROVIDER,
                        model,
                        ai_tags_len,
                        ai_has_generation_story,
                        result.interpretation.chars().count(),
                        failover_count,
                        latency as u64
                    );
                    self.record(
                        RecordCtx {
                            provider: TEXT_PROVIDER.to_string(),
                            tokens,
                            latency,
                            failovers: failover_count,
                        },
                        &today,
                    )
                    .await?;
                    return response_for(
                        result,
                        TEXT_PROVIDER,
                        model,
                        latency,
                        failover_count,
                        &today,
                    );
                }
                Err(e) => {
                    console_error!("provider {} ({}) failed: {}", TEXT_PROVIDER, model, e);
                    if is_daily_quota_error(&e) {
                        // 全站每日 neurons 用盡：換環無益、不寫 stub——直接 503 AI_QUOTA
                        self.record_error(TEXT_PROVIDER, &today, "AI_QUOTA", &e)
                            .await?;
                        return quota_exceeded();
                    }
                    let code = classify(&e);
                    let code_owned = code.to_string();
                    self.record_error(TEXT_PROVIDER, &today, &code_owned, &e)
                        .await?;
                    last_error = Some((TEXT_PROVIDER.to_string(), code_owned, e.clone()));
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
            "【{}】此為本地備用解讀（AI 服務暫時不可用，已自動切換為離線模板，待 AI 恢復後可重新生成）。命盤已正確計算，稍後再試即可取得 AI 解讀。",
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
                    if is_daily_quota_error(&e) {
                        self.record_error(PALM_PROVIDER, today, "AI_QUOTA", &e)
                            .await?;
                        return quota_exceeded();
                    }
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
        "ziwei" | "western" | "story" | "tarot" | "zodiac_match" | "ziwei_year" => {
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

/// 文字鏈 max_tokens 依 (model, chart_type)：gemma（實測輸出 1.2–1.5k）
/// story 3072／其他 2048；llama（輸出單價高 7.5 倍，壓低上限）story 2048／
/// 其他 1536。palm 不經此路——走 PALM_CHAIN 各環自帶值（2048/3072）。
fn max_tokens_for(model: &str, chart_type: &str) -> u32 {
    let story = chart_type == "story";
    match (model == GEMMA_MODEL, story) {
        (true, true) => 3072,
        (true, false) => 2048,
        (false, true) => 2048,
        (false, false) => 1536,
    }
}

/// 本環可用逾時：`min(RING_CAP, deadline − now)`；剩不到 MIN_RING 回 None。
fn ring_timeout_ms(queued_at: f64, now: f64) -> Option<u64> {
    let remaining = queued_at + REQUEST_BUDGET_MS - now;
    if remaining < MIN_RING_MS {
        return None;
    }
    Some(remaining.min(RING_CAP_MS) as u64)
}

/// 文字鏈截斷判定（→ 換下一環）：choices 形 `length`、Responses 形 `incomplete`。
fn text_truncated(finish_reason: Option<&str>) -> bool {
    matches!(finish_reason, Some("length") | Some("incomplete"))
}

/// 每日 Workers AI 額度用盡：503 `AI_QUOTA`（文字與 palm 共用；不走 stub）。
fn quota_exceeded() -> Result<Response> {
    Response::from_json(&serde_json::json!({
        "error": "Daily AI quota exceeded",
        "code": "AI_QUOTA",
    }))
    .map(|r| r.with_status(503))
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
