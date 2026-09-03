//! API request / response contract shared by the Worker (`ft-api`) and the
//! Leptos frontend (`ft-web`).
//!
//! This module is the whole point of Phase C: the frontend and backend stop
//! drifting because they deserialize the *same* structs. Field names here are the
//! JSON wire names — keep the `serde(rename)` annotations aligned with what
//! `ft-api` actually emits (see backend routes), not with Rust naming taste.

#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

// ── Auth ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    /// Beta invite code (spec 2026-08-30); required while INVITE_REQUIRED is on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
}

/// `POST /api/auth/register` (201) and `/api/auth/login` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub sessionId: String,
    pub userId: String,
    pub email: String,
}

// ── User / billing ──

/// The `billing` object nested in `GET /api/users/me`.
/// Emitted camelCase by `ft-api::services::billing::UserBilling`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Billing {
    /// `free` | `premium` | `professional`
    pub tier: String,
    pub isTrialing: bool,
    pub trialEndsAt: Option<String>,
    pub hasAccess: bool,
}

/// `GET /api/users/me`. Birth fields are snake_case (they come straight from the
/// D1 `users` row); `billing` / `hasBirthData` are camelCase (computed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub birth_year: Option<i64>,
    #[serde(default)]
    pub birth_month: Option<i64>,
    #[serde(default)]
    pub birth_day: Option<i64>,
    #[serde(default)]
    pub birth_hour: Option<i64>,
    #[serde(default)]
    pub birth_minute: Option<i64>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub generation_tags: Option<Vec<String>>,
    #[serde(default)]
    pub subscription_tier: Option<String>,
    #[serde(default)]
    pub trial_ends_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    pub billing: Billing,
    pub hasBirthData: bool,
    /// True when the session email matches the worker's ADMIN_EMAIL var.
    #[serde(rename = "isAdmin", default)]
    pub is_admin: bool,
}

impl UserProfile {
    /// Human-readable birth line, e.g. `1990年5月15日 14時 · 男`.
    /// Mirrors the ProfilePage rendering in the React version.
    pub fn birth_summary(&self) -> Option<String> {
        let (y, m, d) = (self.birth_year?, self.birth_month?, self.birth_day?);
        let hour = match self.birth_hour {
            Some(h) => format!(" {}時", h),
            None => " (時辰不詳)".to_string(),
        };
        let gender = match self.gender.as_deref() {
            Some("male") => " · 男",
            Some("female") => " · 女",
            _ => "",
        };
        Some(format!("{}年{}月{}日{}{}", y, m, d, hour, gender))
    }
}

/// `PUT /api/users/me/birth` body. `birth_hour` is nullable on purpose — the UI
/// lets the user mark the hour as unknown, and the backend defaults it to 12.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BirthDataRequest {
    pub birth_year: i64,
    pub birth_month: i64,
    pub birth_day: i64,
    pub birth_hour: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_minute: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_tags: Option<Vec<String>>,
}

/// `PUT /api/users/me/birth` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirthDataResponse {
    pub success: bool,
    pub birth_data_hash: String,
}

// ── Charts ──

/// `GET /api/charts/:type`. `chart_data` stays a raw `Value` because the two
/// divination types have different shapes; use `as_ziwei` / `as_western`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartResponse {
    pub id: String,
    pub user_id: String,
    pub divination_type: String,
    pub chart_data: serde_json::Value,
    #[serde(default)]
    pub ai_interpretation: Option<String>,
    #[serde(default)]
    pub birth_data_hash: Option<String>,
    #[serde(default)]
    pub fromCache: bool,
}

impl ChartResponse {
    pub fn as_ziwei(&self) -> Option<crate::ZiWeiChartV3> {
        serde_json::from_value(self.chart_data.clone()).ok()
    }
    pub fn as_western(&self) -> Option<crate::WesternChartV3> {
        serde_json::from_value(self.chart_data.clone()).ok()
    }
}

/// `POST /api/charts/:type/interpret` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpretResponse {
    pub interpretation: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub fromCache: bool,
}

/// `GET /api/charts/story` and `POST /api/charts/story/generate` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryResponse {
    pub story: String,
    #[serde(default)]
    pub fromCache: bool,
}

/// `GET /api/charts` (200).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpretationsResponse {
    #[serde(default)]
    pub interpretations: Vec<serde_json::Value>,
}

// ── Big5 personality (F1) ──

/// `POST /api/personality/quiz` body。作答：`{skip:false, answers:[15 ints 1–5], durationMs}`；
/// 主動跳過：`{skip:true}`（answers/durationMs 必須缺省，殘留任一 → 400）。
/// **skip 是顯式旗標——omitted 欄位不等於 skip**；`skip` 自身須 `#[serde(default)]`
/// （缺省 false），否則 `{}` 在 serde 就炸成 INVALID_JSON，到不了
/// SKIP_ANSWERS_CONFLICT（Grok 二審 R2-3）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizSubmission {
    #[serde(default)]
    pub skip: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub durationMs: Option<u64>,
}

/// 五維 0–100 實測。f64 實值（(raw−3)×25/3 非整數；不預先取整，避免捨入誤差
/// 進入後續門檻比較），UI 顯示時取整。PartialEq 供 wire roundtrip 測試（Grok 二審 R2-4）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OceanScores {
    pub extraversion: f64,
    pub agreeableness: f64,
    pub conscientiousness: f64,
    pub emotionalStability: f64,
    pub intellectImagination: f64,
}

/// 一筆人格側寫。status wire 值：`complete` / `carelessSuspected` / `skippedPriorOnly`
/// （D1 存 snake_case，路由層轉換）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oceanMeasured: Option<OceanScores>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answers: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub createdAt: Option<String>,
}

/// `POST /api/personality/quiz` (200)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizResponse {
    pub profile: PersonalityProfile,
}

/// `GET /api/personality/me` (200)。讀模型（Grok 審 #3）：`profile` = 最新一筆
/// `complete`（有效側寫不因後續 skip/亂答消失）；`status` = 最新一筆的狀態，
/// 前端據此切四態。無任何資料時兩欄皆 null。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityMeResponse {
    #[serde(default)]
    pub profile: Option<PersonalityProfile>,
    #[serde(default)]
    pub status: Option<String>,
}

/// `DELETE /api/personality/me` (200)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityDeleteResponse {
    pub success: bool,
}

#[cfg(test)]
mod big5_wire_tests {
    use super::*;

    /// 契約鎖定：wire 形狀（camelCase、Option 語意、status 值）不得漂移。
    #[test]
    fn quiz_submission_roundtrip_with_skip() {
        let skip: QuizSubmission = serde_json::from_str(r#"{"skip":true}"#).unwrap();
        assert!(skip.skip && skip.answers.is_none() && skip.durationMs.is_none());
        let normal: QuizSubmission = serde_json::from_str(
            r#"{"skip":false,"answers":[3,4,5,1,2,3,4,5,1,2,3,4,5,1,2],"durationMs":42000}"#,
        )
        .unwrap();
        assert!(!normal.skip);
        assert_eq!(normal.answers.as_ref().unwrap().len(), 15);
        assert_eq!(normal.durationMs, Some(42000));
    }

    /// `{}`（欄位全缺）反序列化成功 → match 落 (false, None) → 400 SKIP_ANSWERS_CONFLICT
    /// （skip 須 serde default，否則 serde 直接炸 INVALID_JSON——Grok 二審 R2-3）。
    #[test]
    fn empty_body_deserializes_with_skip_false() {
        let empty: QuizSubmission = serde_json::from_str(r#"{}"#).unwrap();
        assert!(!empty.skip && empty.answers.is_none() && empty.durationMs.is_none());
        let skip_only: QuizSubmission = serde_json::from_str(r#"{"skip":false}"#).unwrap();
        assert!(!skip_only.skip && skip_only.answers.is_none());
    }

    /// `{skip:true}` 序列化後**恰好只有 skip 一個鍵**（baicodex F15——
    /// Option 無 skip_serializing_if 會輸出 null，與「必須缺省」契約文字不符）。
    #[test]
    fn skip_true_serializes_to_single_key() {
        let j = serde_json::to_value(QuizSubmission {
            skip: true,
            answers: None,
            durationMs: None,
        })
        .unwrap();
        let obj = j.as_object().unwrap();
        assert_eq!(obj.len(), 1, "skip payload must carry exactly one key");
        assert_eq!(obj["skip"], true);
    }

    /// GET 讀模型 wire 鎖（Grok 二審 R2-9）：profile 內 status 與頂層 status 可不同；
    /// 無資料兩欄皆 null 且 serialize 都要出現（DELETE 後 verify 斷言依賴）。
    #[test]
    fn me_response_read_model_lock() {
        let r: PersonalityMeResponse = serde_json::from_str(
            r#"{"profile":{"status":"complete","oceanMeasured":{"extraversion":0.0,"agreeableness":0.0,"conscientiousness":0.0,"emotionalStability":0.0,"intellectImagination":0.0}},"status":"skippedPriorOnly"}"#,
        )
        .unwrap();
        assert_eq!(r.profile.as_ref().unwrap().status, "complete");
        assert_eq!(r.status.as_deref(), Some("skippedPriorOnly"));

        let j = serde_json::to_value(PersonalityMeResponse {
            profile: None,
            status: None,
        })
        .unwrap();
        assert!(j.get("profile").and_then(|v| v.as_null()).is_some());
        assert!(j.get("status").and_then(|v| v.as_null()).is_some());
    }

    /// OceanScores 欄位名（emotionalStability / intellectImagination）鎖 camelCase（Grok #19）。
    #[test]
    fn ocean_scores_wire_lock() {
        let o = OceanScores {
            extraversion: 100.0,
            agreeableness: 75.0,
            conscientiousness: 50.0,
            emotionalStability: 75.0,
            intellectImagination: 0.0,
        };
        let j = serde_json::to_value(&o).unwrap();
        assert_eq!(j["extraversion"], 100.0);
        assert_eq!(j["emotionalStability"], 75.0);
        assert_eq!(j["intellectImagination"], 0.0);
        let back: OceanScores = serde_json::from_value(j).unwrap();
        assert_eq!(back, o);
    }

    #[test]
    fn profile_serializes_camel_wire() {
        let p = PersonalityProfile {
            status: "carelessSuspected".into(),
            oceanMeasured: None,
            answers: Some(vec![3; 15]),
            createdAt: Some("2026-08-28T00:00:00.000Z".into()),
        };
        let j = serde_json::to_value(&p).unwrap();
        assert_eq!(j["status"], "carelessSuspected");
        assert!(j.get("oceanMeasured").is_none()); // skip_serializing_if
        let back: PersonalityProfile = serde_json::from_value(j).unwrap();
        assert_eq!(back.status, "carelessSuspected");
    }

    #[test]
    fn me_response_profile_null() {
        let r: PersonalityMeResponse = serde_json::from_str(r#"{"profile":null}"#).unwrap();
        assert!(r.profile.is_none());
    }
}

// ── F5 predictions / situation_checks / prediction_feedback ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainWire {
    Work,
    Love,
    Family,
    Money,
    Health,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerWire {
    #[serde(rename = "t1")]
    T1,
    #[serde(rename = "t2")]
    T2,
    #[serde(rename = "t3")]
    T3,
    #[serde(rename = "t4")]
    T4,
    #[serde(rename = "t5")]
    T5,
    #[serde(rename = "t6")]
    T6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnchorCoverageWire {
    High,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SituationWire {
    Absent,
    Occurred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseWire {
    Hit,
    Miss,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionSourceWire {
    #[serde(rename = "rule_anchor")]
    RuleAnchor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: String,
    pub profileId: String,
    pub cycleId: String,
    pub domain: DomainWire,
    pub trigger: TriggerWire,
    pub tendency: String,
    pub forecast: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment: Option<String>,
    pub anchorIds: Vec<String>,
    pub anchorCoverage: AnchorCoverageWire,
    pub source: PredictionSourceWire,
    pub rulesVersion: String,
    #[serde(default)]
    pub isControl: bool,
    pub createdAt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SituationCheck {
    pub cycleId: String,
    pub trigger: TriggerWire,
    pub situation: SituationWire,
    pub createdAt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionFeedback {
    pub predictionId: String,
    pub response: ResponseWire,
    pub createdAt: String,
}

#[cfg(test)]
mod f5_wire_tests {
    use super::*;

    #[test]
    fn trigger_wire_is_lowercase_t1() {
        let j = serde_json::to_value(TriggerWire::T1).unwrap();
        assert_eq!(j, "t1");
        let back: TriggerWire = serde_json::from_value(j).unwrap();
        assert_eq!(back, TriggerWire::T1);
        let t6 = serde_json::to_value(TriggerWire::T6).unwrap();
        assert_eq!(t6, "t6");
    }

    #[test]
    fn domain_wire_is_lowercase() {
        let j = serde_json::to_value(DomainWire::Work).unwrap();
        assert_eq!(j, "work");
    }

    #[test]
    fn prediction_roundtrip() {
        let p = Prediction {
            id: "p1".into(),
            profileId: "prof1".into(),
            cycleId: "2026-09-01".into(),
            domain: DomainWire::Work,
            trigger: TriggerWire::T1,
            tendency: "t".into(),
            forecast: "f".into(),
            experiment: Some("e".into()),
            anchorIds: vec!["work-t1-agr-lo-1".into()],
            anchorCoverage: AnchorCoverageWire::High,
            source: PredictionSourceWire::RuleAnchor,
            rulesVersion: "rules-1".into(),
            isControl: false,
            createdAt: "2026-09-01T00:00:00Z".into(),
        };
        let j = serde_json::to_value(&p).unwrap();
        assert_eq!(j["domain"], "work");
        assert_eq!(j["trigger"], "t1");
        assert_eq!(j["anchorCoverage"], "high");
        assert_eq!(j["source"], "rule_anchor");
        let back: Prediction = serde_json::from_value(j).unwrap();
        assert_eq!(back.domain, DomainWire::Work);
    }
}

// ── Errors ──

/// Every non-2xx body from `ft-api` is `{ error, code? }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    #[serde(default)]
    pub code: Option<String>,
}

impl ApiError {
    pub fn is(&self, code: &str) -> bool {
        self.code.as_deref() == Some(code)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.code {
            Some(c) => write!(f, "{} ({})", self.error, c),
            None => write!(f, "{}", self.error),
        }
    }
}
