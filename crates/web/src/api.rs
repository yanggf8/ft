//! HTTP client for `fortunet-api` — the Rust counterpart of `frontend/src/lib/api.ts`.
//!
//! Session handling matches the React client exactly: the session id lives in
//! `localStorage` under `sessionId` and rides on every request as
//! `Authorization: Bearer <id>`. Auth is magic-link: login/register only send
//! the email; a session is minted solely by `verify_email`.

use ft_schema::api::*;
use ft_schema::palm::wire::{PalmAnalyzeRequest, PalmHistoryPage, PalmReadingView};
use ft_schema::tarot::{TarotDrawRequest, TarotHistoryPage, TarotReadingView};
use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use wasm_bindgen::JsCast;

/// Canonical production API (custom domain, 2026-09); the workers.dev URL
/// keeps serving as backup and for `verify-deployment.sh`.
pub const API_URL: &str = "https://api.ahexagram.com";
/// 上線前驗證用 staging API（2026-11-22，獨立 DB／DO）。
pub const API_URL_STAGING: &str = "https://fortunet-api-staging.yanggf.workers.dev";
/// 從 staging Pages 分支開的網頁才打 staging API；同一份 dist 先上 staging、再原封上正式站。
const STAGING_HOST: &str = "staging.fortunet.pages.dev";

/// 依目前網頁的 host 選 API：staging 分支 → staging API，其他一律正式 API。
pub fn api_base() -> &'static str {
    let host = web_sys::window()
        .and_then(|w| w.location().hostname().ok())
        .unwrap_or_default();
    if host == STAGING_HOST {
        API_URL_STAGING
    } else {
        API_URL
    }
}

const SESSION_KEY: &str = "sessionId";

/// What a failed call gives back. `Api` carries the structured `{error, code}`
/// body so callers can branch on `code` (NO_BIRTH_DATA, NO_STORY, RATE_LIMIT…)
/// instead of substring-matching an error string like the React version did.
#[derive(Debug, Clone)]
pub enum ApiErr {
    /// Non-2xx with a parsed `{ error, code? }` body.
    Api { status: u16, body: ApiError },
    /// Non-2xx whose body was not the expected JSON shape.
    Status { status: u16, text: String },
    /// Transport / serialization failure.
    Network(String),
}

impl ApiErr {
    /// True when the server sent this exact `code`.
    pub fn is_code(&self, code: &str) -> bool {
        matches!(self, ApiErr::Api { body, .. } if body.is(code))
    }
    pub fn status(&self) -> Option<u16> {
        match self {
            ApiErr::Api { status, .. } | ApiErr::Status { status, .. } => Some(*status),
            ApiErr::Network(_) => None,
        }
    }
    /// Missing birth data / gender — both send the user to the profile page.
    pub fn needs_birth_data(&self) -> bool {
        self.is_code("NO_BIRTH_DATA") || self.is_code("NO_GENDER")
    }

    /// AI 端點失敗的友善中文（**不直印** `to_string()`——524／HTML 錯誤頁
    /// 會把整頁 HTML 印到畫面）。
    pub fn ai_message(&self) -> String {
        ai_error_message(
            self.code(),
            self.status(),
            matches!(self, ApiErr::Network(_)),
        )
    }

    pub fn code(&self) -> Option<&str> {
        match self {
            ApiErr::Api { body, .. } => body.code.as_deref(),
            _ => None,
        }
    }
}

/// [`ApiErr::ai_message`] 的純函式核心（便於推理；`network`＝連線失敗／逾時）。
pub fn ai_error_message(code: Option<&str>, status: Option<u16>, network: bool) -> String {
    let msg = match (code, status) {
        (Some("AI_QUOTA"), _) => "今日 AI 解讀額度已滿，明天再試",
        (Some("RATE_LIMIT"), _) | (_, Some(429)) => "請求太頻繁，請稍後再試",
        (Some("AI_QUEUE_FULL" | "AI_QUEUE_TIMEOUT"), _) => "AI 目前忙碌中，請稍後再試",
        (_, Some(524 | 504)) => "AI 回應逾時，請稍後再試",
        (_, Some(s)) if s >= 500 => "AI 服務暫時無法使用，請稍後再試",
        _ if network => "連線逾時或中斷，請檢查網路後再試",
        _ => "AI 解讀失敗，請稍後再試",
    };
    msg.to_string()
}

impl std::fmt::Display for ApiErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiErr::Api { body, .. } => write!(f, "{}", body),
            ApiErr::Status { status, text } => write!(f, "HTTP {}: {}", status, text),
            ApiErr::Network(e) => write!(f, "{}", e),
        }
    }
}

// ── session storage ──

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn get_session() -> Option<String> {
    storage()?
        .get_item(SESSION_KEY)
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
}

pub fn set_session(session_id: Option<&str>) {
    let Some(s) = storage() else { return };
    match session_id {
        Some(id) => {
            let _ = s.set_item(SESSION_KEY, id);
        }
        None => {
            let _ = s.remove_item(SESSION_KEY);
        }
    }
}

// ── request plumbing ──

fn url(path: &str) -> String {
    format!("{}{}", api_base(), path)
}

/// Turn a finished response into `T`, or into a structured `ApiErr`.
async fn decode<T: DeserializeOwned>(resp: gloo_net::http::Response) -> Result<T, ApiErr> {
    let status = resp.status();
    if (200..300).contains(&status) {
        return resp
            .json::<T>()
            .await
            .map_err(|e| ApiErr::Network(format!("decode failed: {}", e)));
    }
    let text = resp.text().await.unwrap_or_default();
    match serde_json::from_str::<ApiError>(&text) {
        Ok(body) => Err(ApiErr::Api { status, body }),
        Err(_) => Err(ApiErr::Status { status, text }),
    }
}

fn with_auth(mut builder: gloo_net::http::RequestBuilder) -> gloo_net::http::RequestBuilder {
    if let Some(sid) = get_session() {
        builder = builder.header("Authorization", &format!("Bearer {}", sid));
    }
    builder
}

async fn get_json<T: DeserializeOwned>(path: &str, no_cache: bool) -> Result<T, ApiErr> {
    let mut b = with_auth(Request::get(&url(path)));
    if no_cache {
        b = b.header("Cache-Control", "no-cache");
    }
    let resp = b.send().await.map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

async fn send_json<B: Serialize, T: DeserializeOwned>(
    builder: gloo_net::http::RequestBuilder,
    body: &B,
) -> Result<T, ApiErr> {
    let req = with_auth(builder)
        .header("Content-Type", "application/json")
        .json(body)
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    let resp = req
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

/// AI 端點 client 逾時：文字解讀伺服端整筆預算 88s（自 DO 入列起）＋route 前後
/// DB／engine 與網路餘裕，取 95s（仍低於 Cloudflare 100s 524）；超過即中止（防無限轉圈）。
/// 手相走 DO 的 palm 兩環（各 45s、不受 88s 預算約束），最壞可能先被此逾時中止。
const AI_CLIENT_TIMEOUT_MS: i32 = 95_000;

/// 建一個 `AI_CLIENT_TIMEOUT_MS` 後自動 abort 的 `AbortController`
/// （不用 `AbortSignal.timeout()`：舊 iOS Safari 不支援，呼叫會 throw）。
fn ai_abort_controller() -> Option<web_sys::AbortController> {
    let ctrl = web_sys::AbortController::new().ok()?;
    let c2 = ctrl.clone();
    let cb = wasm_bindgen::closure::Closure::once_into_js(move || c2.abort());
    web_sys::window()?
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.unchecked_ref(),
            AI_CLIENT_TIMEOUT_MS,
        )
        .ok()?;
    Some(ctrl)
}

/// AI 端點用的 POST（body 可無）：掛 95s abort。
async fn post_ai<B: Serialize, T: DeserializeOwned>(
    builder: gloo_net::http::RequestBuilder,
    body: Option<&B>,
) -> Result<T, ApiErr> {
    let ctrl = ai_abort_controller();
    let signal = ctrl.as_ref().map(|c| c.signal());
    let builder = with_auth(builder)
        .header("Content-Type", "application/json")
        .abort_signal(signal.as_ref());
    let req = match body {
        Some(b) => builder.json(b),
        None => builder.build(),
    }
    .map_err(|e| ApiErr::Network(e.to_string()))?;
    let resp = req
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

/// POST with no request body (interpret / story generate / logout).
async fn post_empty<T: DeserializeOwned>(path: &str) -> Result<T, ApiErr> {
    let resp = with_auth(Request::post(&url(path)))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

// ── endpoints ──

/// `POST /api/auth/register` — magic-link step 1. The identical 202 body
/// carries no session; the account only becomes usable once the emailed token
/// is exchanged through `verify_email`.
pub async fn register(
    email: &str,
    full_name: Option<&str>,
    invite: Option<&str>,
) -> Result<(), ApiErr> {
    let body = RegisterRequest {
        email: email.to_string(),
        full_name: full_name.filter(|s| !s.is_empty()).map(|s| s.to_string()),
        invite: invite.filter(|s| !s.is_empty()).map(|s| s.to_string()),
    };
    send_ignoring_body(Request::post(&url("/api/auth/register")), &body).await
}

/// `POST /api/auth/login` — magic-link step 1, same 202 contract as register.
/// An unknown address gets the identical 202 but no email is ever sent.
pub async fn login(email: &str) -> Result<(), ApiErr> {
    let body = LoginRequest {
        email: email.to_string(),
    };
    send_ignoring_body(Request::post(&url("/api/auth/login")), &body).await
}

/// `POST /api/auth/verify` — exchange the emailed token for the session. This
/// is the only auth call that mints a session; it is stored exactly the way
/// the old direct login stored it (localStorage `sessionId`).
pub async fn verify_email(token: &str) -> Result<SessionResponse, ApiErr> {
    let body = VerifyRequest {
        token: token.to_string(),
    };
    let res: SessionResponse = send_json(Request::post(&url("/api/auth/verify")), &body).await?;
    set_session(Some(&res.sessionId));
    Ok(res)
}

/// `POST /api/auth/oauth/exchange` — exchange the one-time OAuth redirect code
/// for the session. The session id is stored here exactly the way
/// `verify_email` stores it; the plain code itself is never persisted.
pub async fn exchange_oauth_code(code: &str) -> Result<SessionResponse, ApiErr> {
    let body = serde_json::json!({ "code": code });
    let res: SessionResponse =
        send_json(Request::post(&url("/api/auth/oauth/exchange")), &body).await?;
    set_session(Some(&res.sessionId));
    Ok(res)
}

/// `GET /api/personality/overlay` — F3 疊圖(spec 2026-09-07-f2-f3)。409 三態
/// (NO_MEASUREMENT / F3_DISABLED / MEASUREMENT_PENDING)由呼叫端以 ApiErr::is_code 分流。
pub async fn fetch_overlay() -> Result<ft_schema::symbolic::OverlayResponse, ApiErr> {
    get_json("/api/personality/overlay", true).await
}

/// `GET /api/invites/:code` — public preflight for the register page. A
/// network/transport failure reads as "invalid" (the register submit will get
/// the authoritative answer from the backend anyway).
pub async fn check_invite(code: &str) -> Result<InviteCheck, ApiErr> {
    let resp = Request::get(&url(&format!(
        "/api/invites/{}",
        code.trim().to_ascii_uppercase()
    )))
    .send()
    .await
    .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

#[derive(Debug, Clone, Deserialize)]
pub struct InviteCheck {
    pub valid: bool,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminInvite {
    #[serde(rename = "code")]
    pub code: String,
    pub label: String,
    #[serde(rename = "maxUses")]
    pub max_uses: i64,
    #[serde(rename = "usedCount")]
    pub used_count: i64,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
    #[serde(rename = "revokedAt")]
    pub revoked_at: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InviteCreated {
    pub code: String,
    pub url: String,
    pub label: String,
    #[serde(rename = "maxUses")]
    pub max_uses: i64,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
}

/// `GET /api/admin/invites` — admin only; a 403 surfaces as ApiErr::Api.
pub async fn list_invites() -> Result<Vec<AdminInvite>, ApiErr> {
    let resp = with_auth(Request::get(&url("/api/admin/invites")))
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    #[derive(Deserialize)]
    struct Wrapped {
        invites: Vec<AdminInvite>,
    }
    let w: Wrapped = decode(resp).await?;
    Ok(w.invites)
}

pub async fn create_invite(
    label: &str,
    max_uses: i64,
    expires_at: Option<&str>,
) -> Result<InviteCreated, ApiErr> {
    let body = serde_json::json!({
        "label": label,
        "maxUses": max_uses,
        "expiresAt": expires_at.filter(|s| !s.is_empty()),
    });
    let res: InviteCreated = send_json(Request::post(&url("/api/admin/invites")), &body).await?;
    Ok(res)
}

pub async fn revoke_invite(code: &str) -> Result<(), ApiErr> {
    let _: serde_json::Value = send_json(
        Request::post(&url(&format!("/api/admin/invites/{}/revoke", code))),
        &serde_json::json!({}),
    )
    .await?;
    Ok(())
}

/// POST whose 202 `{ok, message}` body carries nothing the caller needs — only
/// the status does. Kept separate so the magic-link step-1 calls say that.
async fn send_ignoring_body<B: Serialize>(
    builder: gloo_net::http::RequestBuilder,
    body: &B,
) -> Result<(), ApiErr> {
    let _: serde_json::Value = send_json(builder, body).await?;
    Ok(())
}

/// Wire body of `POST /api/auth/verify`. Declared here rather than in
/// `ft-schema` because this crate cannot edit the schema crate's contract for
/// a request the Worker defines its own parser for.
#[derive(Debug, Serialize)]
struct VerifyRequest {
    token: String,
}

/// Clears the local session even when the server call fails — same forgiving
/// behavior as the React `logout()`.
pub async fn logout() {
    let _ = post_empty::<serde_json::Value>("/api/auth/logout").await;
    set_session(None);
}

pub async fn get_me(no_cache: bool) -> Result<UserProfile, ApiErr> {
    get_json("/api/users/me", no_cache).await
}

pub async fn update_birth_data(data: &BirthDataRequest) -> Result<BirthDataResponse, ApiErr> {
    send_json(Request::put(&url("/api/users/me/birth")), data).await
}

pub async fn get_chart(chart_type: &str, no_cache: bool) -> Result<ChartResponse, ApiErr> {
    get_json(&format!("/api/charts/{}", chart_type), no_cache).await
}

/// `POST /api/charts/:type/interpret`, retrying once through a fresh chart when
/// the server reports the cached chart is stale (409 RECALC_REQUIRED).
pub async fn interpret(chart_type: &str) -> Result<InterpretResponse, ApiErr> {
    let path = format!("/api/charts/{}/interpret", chart_type);
    let call = || post_ai::<(), InterpretResponse>(Request::post(&url(&path)), None);
    match call().await {
        Err(e) if e.status() == Some(409) => {
            get_chart(chart_type, true).await?;
            call().await
        }
        other => other,
    }
}

pub async fn get_story(no_cache: bool) -> Result<StoryResponse, ApiErr> {
    get_json("/api/charts/story", no_cache).await
}

pub async fn generate_story() -> Result<StoryResponse, ApiErr> {
    post_ai::<(), _>(Request::post(&url("/api/charts/story/generate")), None).await
}

// ── Big5 personality (F1) ──

/// `POST /api/personality/quiz`. A 422 `CARELESS_SUSPECTED` is handled by the
/// caller as the one post-submit path that does not re-fetch the read model.
pub async fn submit_quiz(body: &QuizSubmission) -> Result<QuizResponse, ApiErr> {
    send_json(Request::post(&url("/api/personality/quiz")), body).await
}

pub async fn get_personality(no_cache: bool) -> Result<PersonalityMeResponse, ApiErr> {
    get_json("/api/personality/me", no_cache).await
}

/// Bodyless DELETE: some intermediaries reject DELETE request bodies.
pub async fn delete_personality() -> Result<PersonalityDeleteResponse, ApiErr> {
    let resp = with_auth(Request::delete(&url("/api/personality/me")))
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

// ── 塔羅 v1b（docs/ARCHITECTURE.md §Tarot）──

/// `POST /api/tarot/draw` — 三張牌陣（request_id 冪等；同鍵同 body 重放回原結果，
/// 201／200 皆成功）。
pub async fn tarot_draw(body: &TarotDrawRequest) -> Result<TarotReadingView, ApiErr> {
    send_json(Request::post(&url("/api/tarot/draw")), body).await
}

/// `GET /api/tarot/readings` — 歷史分頁（items + nextCursor）。
pub async fn tarot_history(cursor: Option<&str>) -> Result<TarotHistoryPage, ApiErr> {
    let path = match cursor {
        Some(cursor) => format!(
            "/api/tarot/readings?cursor={}",
            js_sys::encode_uri_component(cursor)
        ),
        None => "/api/tarot/readings".to_string(),
    };
    get_json(&path, true).await
}

/// `DELETE /api/tarot/readings/:id` — 單筆刪除（不存在／他人／重刪 → 404）。
pub async fn tarot_delete(id: &str) -> Result<(), ApiErr> {
    let resp = with_auth(Request::delete(&url(&format!("/api/tarot/readings/{id}"))))
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    let _: serde_json::Value = decode(resp).await?;
    Ok(())
}

/// `POST /api/tarot/readings/:id/interpret` — AI 整合解牌（同步；已完成者
/// 冪等回原結果）。`include_chart=false`＝純塔羅（契約 #7，`?chart=0`）。
pub async fn tarot_interpret(id: &str, include_chart: bool) -> Result<TarotReadingView, ApiErr> {
    let path = if include_chart {
        format!("/api/tarot/readings/{id}/interpret")
    } else {
        format!("/api/tarot/readings/{id}/interpret?chart=0")
    };
    post_ai::<(), _>(Request::post(&url(&path)), None).await
}

/// `POST /api/zodiac/match/interpret` — 星座配對 AI 解讀（登入＋有出生資料）。
/// 伺服器自讀西洋盤，前端只送對方太陽星座標準英文名。
pub async fn zodiac_match_interpret(target: &str) -> Result<ZodiacMatchInterpretResponse, ApiErr> {
    let body = ZodiacMatchInterpretRequest {
        target: target.to_string(),
    };
    post_ai(
        Request::post(&url("/api/zodiac/match/interpret")),
        Some(&body),
    )
    .await
}

/// `GET /api/ziwei/horoscope?year=Y` — 紫微流年大運（省略 year＝台北今天；春節前屬前一年）。
pub async fn ziwei_horoscope(
    year: Option<i32>,
) -> Result<ft_schema::ziwei_luck::ZiweiHoroscopeResponse, ApiErr> {
    let path = match year {
        Some(y) => format!("/api/ziwei/horoscope?year={y}"),
        None => "/api/ziwei/horoscope".to_string(),
    };
    get_json(&path, false).await
}

/// `GET /api/ziwei/year` — 今年運勢摘要＋已有的 AI 解讀（沒有則 null）。
pub async fn ziwei_year() -> Result<ft_schema::ziwei_luck::ZiweiYearResponse, ApiErr> {
    get_json("/api/ziwei/year", true).await
}

/// `POST /api/ziwei/year/interpret`（無 body）— 今年運勢 AI 解讀；24h 每人 3 次。
pub async fn ziwei_year_interpret() -> Result<ft_schema::ziwei_luck::ZiweiYearReading, ApiErr> {
    post_ai::<(), _>(Request::post(&url("/api/ziwei/year/interpret")), None).await
}

/// `DELETE /api/tarot/readings` — 整批刪除，回刪除筆數。
pub async fn tarot_delete_all() -> Result<i64, ApiErr> {
    #[derive(Deserialize)]
    struct DeletedCount {
        deleted: i64,
    }
    let resp = with_auth(Request::delete(&url("/api/tarot/readings")))
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    let n: DeletedCount = decode(resp).await?;
    Ok(n.deleted)
}

// ── 手相 v1（docs/superpowers/specs/2026-10-27-palm-v1-design.md）──

/// `POST /api/palm/analyze` — 拍照抽取＋規則式解讀（requestId 冪等；同鍵同 body
/// 重放回原結果，201/200 皆成功；422 PALM_QUALITY_LOW＝照片品質不足請重拍）。
pub async fn palm_analyze(body: &PalmAnalyzeRequest) -> Result<PalmReadingView, ApiErr> {
    post_ai(Request::post(&url("/api/palm/analyze")), Some(body)).await
}

/// `GET /api/palm/readings` — 歷史分頁（items + nextCursor）。
pub async fn palm_history(cursor: Option<&str>) -> Result<PalmHistoryPage, ApiErr> {
    let path = match cursor {
        Some(cursor) => format!(
            "/api/palm/readings?cursor={}",
            js_sys::encode_uri_component(cursor)
        ),
        None => "/api/palm/readings".to_string(),
    };
    get_json(&path, true).await
}

/// `DELETE /api/palm/readings/:id` — 單筆刪除（不存在／他人／重刪 → 404）。
pub async fn palm_delete(id: &str) -> Result<(), ApiErr> {
    let resp = with_auth(Request::delete(&url(&format!("/api/palm/readings/{id}"))))
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    let _: serde_json::Value = decode(resp).await?;
    Ok(())
}

/// `DELETE /api/palm/readings` — 整批刪除，回刪除筆數。
pub async fn palm_delete_all() -> Result<i64, ApiErr> {
    #[derive(Deserialize)]
    struct DeletedCount {
        deleted: i64,
    }
    let resp = with_auth(Request::delete(&url("/api/palm/readings")))
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    let n: DeletedCount = decode(resp).await?;
    Ok(n.deleted)
}

// ── F5 predictions（docs/superpowers/specs/2026-09-04-f5-web-predictions-ui-design.md §2）──

/// `GET /api/predictions` — 當週列表（含 checks/feedback；forecast 可能被伺服器遮罩）。
/// 進卡片/收齊/寫入後一律 no_cache: true（伺服器已 no-store，對齊 personality 頁）。
pub async fn get_predictions(no_cache: bool) -> Result<ListPredictionsResponse, ApiErr> {
    get_json("/api/predictions", no_cache).await
}

/// `POST /api/predictions/generate` — 冪等（cycle 凍結）。
/// F4：body 必帶五領域強度（0–3），隨凍結快照落庫；缺/越界 → 400 INVALID_STRENGTHS。
pub async fn generate_predictions(
    b: &GeneratePredictionsRequest,
) -> Result<GeneratePredictionsResponse, ApiErr> {
    send_json(Request::post(&url("/api/predictions/generate")), b).await
}

/// `PUT /api/predictions/checks` — F6 第 1 段。`cycleId` 恆 None（省略 = 當週）。
pub async fn put_situation_check(b: &CheckSituationRequest) -> Result<SituationCheck, ApiErr> {
    send_json(Request::put(&url("/api/predictions/checks")), b).await
}

/// `POST /api/predictions/:id/feedback` — F6 第 2 段（僅 occurred 後、一次性）。
pub async fn post_prediction_feedback(
    id: &str,
    b: &FeedbackRequest,
) -> Result<PredictionFeedback, ApiErr> {
    send_json(
        Request::post(&url(&format!("/api/predictions/{}/feedback", id))),
        b,
    )
    .await
}

// ── 開運小物 v1（spec 2026-11-22-shop-interest-design §4）──
// 公開端點：匿名靠 `anonId`；有 session 時 `with_auth` 會一併帶 Bearer。

/// `GET /api/shop/interest/me?anonId=…` — 此身分（登入者 ∪ anonId）目前登記的品項。
/// anonId 只含 `[A-Za-z0-9-]`（前端自產），不需 percent-encode。
pub async fn shop_interest_me(anon_id: Option<&str>) -> Result<ShopInterestMeResponse, ApiErr> {
    let path = match anon_id {
        Some(a) => format!("/api/shop/interest/me?anonId={a}"),
        None => "/api/shop/interest/me".to_string(),
    };
    get_json(&path, true).await
}

/// `POST /api/shop/interest` — 登記「我有興趣」（重複登記回 `already:true`）。
pub async fn shop_interest_add(body: &ShopInterestRequest) -> Result<ShopInterestResponse, ApiErr> {
    send_json(Request::post(&url("/api/shop/interest")), body).await
}

/// `DELETE /api/shop/interest/:itemId` — 取消登記；body 帶同形 `{itemId, anonId}`。
pub async fn shop_interest_remove(
    body: &ShopInterestRequest,
) -> Result<ShopInterestResponse, ApiErr> {
    let path = format!("/api/shop/interest/{}", body.item_id);
    send_json(Request::delete(&url(&path)), body).await
}

// ── 上手小實驗 v1（spec 2026-11-22-talent-tryout-design §8／§13.3；全部需登入）──

/// `GET /api/talent/trials` — 本人所有紀錄（新到舊）＋猜測傾向。
pub async fn talent_trials() -> Result<TalentTrialsResponse, ApiErr> {
    get_json("/api/talent/trials", true).await
}

/// `POST /api/talent/trials` — 先猜並鎖定（requestId 冪等；201／200 皆成功；
/// 409 `OPEN_TRIAL_EXISTS`＝已有進行中的一筆）。
pub async fn talent_create(body: &TalentTrialCreate) -> Result<TalentTrial, ApiErr> {
    send_json(Request::post(&url("/api/talent/trials")), body).await
}

/// `POST /api/talent/trials/:id/report` — 對答案（一次性；`reached: null`＝這次沒做；
/// 409 `ALREADY_REPORTED`、410 `EXPIRED`）。
pub async fn talent_report(id: &str, body: &TalentTrialReport) -> Result<TalentTrial, ApiErr> {
    let path = format!(
        "/api/talent/trials/{}/report",
        js_sys::encode_uri_component(id)
    );
    send_json(Request::post(&url(&path)), body).await
}

/// `DELETE /api/talent/trials/:id` — 硬刪一筆（不存在／他人 → 404）。
pub async fn talent_delete(id: &str) -> Result<TalentDeleteResponse, ApiErr> {
    let path = format!("/api/talent/trials/{}", js_sys::encode_uri_component(id));
    let resp = with_auth(Request::delete(&url(&path)))
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}

/// `DELETE /api/talent/trials` — 硬刪本人全部，回刪除筆數。
pub async fn talent_delete_all() -> Result<TalentDeleteResponse, ApiErr> {
    let resp = with_auth(Request::delete(&url("/api/talent/trials")))
        .send()
        .await
        .map_err(|e| ApiErr::Network(e.to_string()))?;
    decode(resp).await
}
