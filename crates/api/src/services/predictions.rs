//! F5 predictions 服務層 — 週期生成（cycle 級凍結）＋ F6 兩段式回報。
//! Spec: docs/superpowers/specs/2026-09-04-f5-api-predictions-design.md
//! 測試策略：純函數住 ft-schema（cycle/predict）；本層不 mock、不造假（.testing-rules），
//! 語意由 route code review + 部署後手動 API 驗證。

use ft_schema::anchors::{Domain, TriggerClass, RULES_VERSION};
use ft_schema::api::{
    AnchorCoverageWire, DomainWire, OceanScores, Prediction, PredictionFeedback,
    PredictionSourceWire, ResponseWire, SituationCheck, SituationWire, TriggerWire,
};
use ft_schema::cycle::week_start_asia_taipei;
use ft_schema::predict::{
    dim_ranges, display_rounded, filter_negative_half, select_for_domain, AnchorCoverage, Selected,
};

use super::{clock, db, uuid};
use db::Turso;

/// 服務錯誤 → route 層對映 error_code。
#[derive(Debug)]
pub enum PredictionsError {
    /// 無 complete 側寫（含未施測/全 skip）
    ProfileIncomplete,
    /// prediction id 不存在或屬於他人（不洩漏存在性）
    NotFound,
    /// 寫入對象非當週
    StaleCycle,
    /// 第 1 段（situation check）未答
    SituationRequired,
    /// 第 1 段 = absent，不得進第 2 段
    SituationAbsent,
    /// 該 trigger 已有 feedback，第 1 段鎖定
    SituationLocked,
    /// feedback 一次性
    FeedbackExists,
    /// 當週無此 trigger 的預測
    UnknownTrigger,
    Db(String),
}

fn db_err(e: worker::Error) -> PredictionsError {
    PredictionsError::Db(e.to_string())
}

/// 當週 cycle_id（Asia/Taipei 週一）。clock 失效/不可解析 → Db 錯誤（fail-closed）。
pub fn current_cycle_id() -> Result<String, PredictionsError> {
    let iso = clock::now_iso();
    if iso.is_empty() {
        return Err(PredictionsError::Db("clock unavailable".into()));
    }
    week_start_asia_taipei(&iso).ok_or_else(|| PredictionsError::Db("cycle parse failed".into()))
}

// ── wire 轉換（DB 小寫字串 → 強型別；壞值 → None，route 已擋在輸入端）──

fn trigger_from_str(s: &str) -> Option<TriggerWire> {
    Some(match s {
        "t1" => TriggerWire::T1,
        "t2" => TriggerWire::T2,
        "t3" => TriggerWire::T3,
        "t4" => TriggerWire::T4,
        "t5" => TriggerWire::T5,
        "t6" => TriggerWire::T6,
        _ => return None,
    })
}

fn trigger_class_to_str(t: TriggerClass) -> &'static str {
    match t {
        TriggerClass::T1 => "t1",
        TriggerClass::T2 => "t2",
        TriggerClass::T3 => "t3",
        TriggerClass::T4 => "t4",
        TriggerClass::T5 => "t5",
        TriggerClass::T6 => "t6",
    }
}

fn trigger_to_str(t: TriggerWire) -> &'static str {
    match t {
        TriggerWire::T1 => "t1",
        TriggerWire::T2 => "t2",
        TriggerWire::T3 => "t3",
        TriggerWire::T4 => "t4",
        TriggerWire::T5 => "t5",
        TriggerWire::T6 => "t6",
    }
}

fn domain_from_str(s: &str) -> Option<DomainWire> {
    Some(match s {
        "work" => DomainWire::Work,
        "love" => DomainWire::Love,
        "family" => DomainWire::Family,
        "money" => DomainWire::Money,
        "health" => DomainWire::Health,
        _ => return None,
    })
}

fn domain_to_str(d: Domain) -> &'static str {
    match d {
        Domain::Work => "work",
        Domain::Love => "love",
        Domain::Family => "family",
        Domain::Money => "money",
        Domain::Health => "health",
    }
}

fn coverage_from_str(s: &str) -> Option<AnchorCoverageWire> {
    match s {
        "high" => Some(AnchorCoverageWire::High),
        "low" => Some(AnchorCoverageWire::Low),
        _ => None,
    }
}

fn coverage_to_str(c: AnchorCoverage) -> &'static str {
    match c {
        AnchorCoverage::High => "high",
        AnchorCoverage::Low => "low",
    }
}

// ── row 型別（snake_case 對齊 DB 欄名；is_control 由 i64 轉 bool）──

#[derive(serde::Deserialize)]
struct GenRow {
    // 保留語意：凍結快照綁定當時 profile；本切片不需讀出。
    #[allow(dead_code)]
    profile_id: String,
}

#[derive(serde::Deserialize)]
struct ProfileRow {
    id: String,
    ipip_answers: Option<String>,
    ocean_measured: Option<String>,
}

#[derive(serde::Deserialize)]
struct PredictionRow {
    id: String,
    profile_id: String,
    cycle_id: String,
    domain: String,
    trigger: String,
    tendency: Option<String>,
    forecast: Option<String>,
    experiment: Option<String>,
    anchor_ids: String,
    anchor_coverage: String,
    source: String,
    rules_version: String,
    is_control: i64,
    created_at: String,
}

#[derive(serde::Deserialize)]
struct CheckRow {
    cycle_id: String,
    trigger: String,
    situation: String,
    created_at: String,
}

#[derive(serde::Deserialize)]
struct FeedbackRow {
    prediction_id: String,
    response: String,
    created_at: String,
}

#[derive(serde::Deserialize)]
struct OwnedPredictionRow {
    user_id: String,
    cycle_id: String,
    trigger: String,
}

#[derive(serde::Deserialize)]
struct SituationRow {
    situation: String,
}

#[derive(serde::Deserialize)]
struct OneRow {
    #[allow(dead_code)]
    one: i64,
}

fn to_prediction(r: PredictionRow) -> Option<Prediction> {
    let domain = domain_from_str(&r.domain)?;
    let trigger = trigger_from_str(&r.trigger)?;
    let anchor_coverage = coverage_from_str(&r.anchor_coverage)?;
    let source = match r.source.as_str() {
        "rule_anchor" => PredictionSourceWire::RuleAnchor,
        _ => return None,
    };
    let anchor_ids: Vec<String> = serde_json::from_str(&r.anchor_ids).unwrap_or_default();
    Some(Prediction {
        id: r.id,
        profileId: r.profile_id,
        cycleId: r.cycle_id,
        domain,
        trigger,
        tendency: r.tendency,
        forecast: r.forecast,
        experiment: r.experiment,
        anchorIds: anchor_ids,
        anchorCoverage: anchor_coverage,
        source,
        rulesVersion: r.rules_version,
        isControl: r.is_control != 0,
        createdAt: r.created_at,
    })
}

fn to_check(r: CheckRow) -> Option<SituationCheck> {
    let trigger = trigger_from_str(&r.trigger)?;
    let situation = match r.situation.as_str() {
        "absent" => SituationWire::Absent,
        "occurred" => SituationWire::Occurred,
        _ => return None,
    };
    Some(SituationCheck {
        cycleId: r.cycle_id,
        trigger,
        situation,
        createdAt: r.created_at,
    })
}

fn to_feedback(r: FeedbackRow) -> Option<PredictionFeedback> {
    let response = match r.response.as_str() {
        "hit" => ResponseWire::Hit,
        "miss" => ResponseWire::Miss,
        "other" => ResponseWire::Other,
        _ => return None,
    };
    Some(PredictionFeedback {
        predictionId: r.prediction_id,
        response,
        createdAt: r.created_at,
    })
}

/// 一週的完整視圖（predictions + checks + feedback）。
pub struct CycleView {
    pub predictions: Vec<Prediction>,
    pub checks: Vec<SituationCheck>,
    pub feedback: Vec<PredictionFeedback>,
}

/// 生成結果：`generated` = 本次真的跑了生成管線（false = 該週已凍結/已存在）。
pub struct GenOutcome {
    pub generated: bool,
    pub view: CycleView,
}

/// 列一週（固定領域序 work→money）。
pub async fn list_cycle(
    db: &Turso,
    user_id: &str,
    cycle_id: &str,
) -> Result<CycleView, PredictionsError> {
    let rows: Vec<PredictionRow> = db::all(
        db,
        "SELECT id, profile_id, cycle_id, domain, trigger, tendency, forecast, experiment, \
                anchor_ids, anchor_coverage, source, rules_version, is_control, created_at \
         FROM predictions WHERE user_id = ?1 AND cycle_id = ?2 \
         ORDER BY CASE domain WHEN 'work' THEN 0 WHEN 'money' THEN 1 ELSE 2 END, trigger",
        &[&db::text(user_id), &db::text(cycle_id)],
    )
    .await
    .map_err(db_err)?;
    let predictions = rows.into_iter().filter_map(to_prediction).collect();

    let checks: Vec<SituationCheck> = db::all::<CheckRow>(
        db,
        "SELECT cycle_id, trigger, situation, created_at FROM situation_checks \
         WHERE user_id = ?1 AND cycle_id = ?2 ORDER BY trigger",
        &[&db::text(user_id), &db::text(cycle_id)],
    )
    .await
    .map_err(db_err)?
    .into_iter()
    .filter_map(to_check)
    .collect();

    let feedback: Vec<PredictionFeedback> = db::all::<FeedbackRow>(
        db,
        "SELECT pf.prediction_id, pf.response, pf.created_at FROM prediction_feedback pf \
         JOIN predictions p ON p.id = pf.prediction_id \
         WHERE p.user_id = ?1 AND p.cycle_id = ?2 ORDER BY pf.created_at",
        &[&db::text(user_id), &db::text(cycle_id)],
    )
    .await
    .map_err(db_err)?
    .into_iter()
    .filter_map(to_feedback)
    .collect();

    Ok(CycleView {
        predictions,
        checks,
        feedback,
    })
}

/// F6 遮罩（Grok P0-2）：當週 distinct(trigger) 未全部有 check → tendency/forecast/experiment
/// 全數改 null。GET 與 generate 回應皆須經過本函數。
pub fn redact_view(view: &mut CycleView) {
    use std::collections::HashSet;
    let distinct: HashSet<String> = view
        .predictions
        .iter()
        .map(|p| format!("{:?}", p.trigger))
        .collect();
    let answered: HashSet<String> = view
        .checks
        .iter()
        .map(|c| format!("{:?}", c.trigger))
        .collect();
    if !distinct.is_subset(&answered) {
        for p in &mut view.predictions {
            p.tendency = None;
            p.forecast = None;
            p.experiment = None;
        }
    }
}

/// 週期生成（cycle 級凍結冪等）。Grok P0-4：一週一 profile 一快照；
/// 已有 generations 列 → 整次只回現況，絕不補 domain（防週中重測混 profile）。
pub async fn generate(
    db: &Turso,
    user_id: &str,
    cycle_id: &str,
) -> Result<GenOutcome, PredictionsError> {
    // 1. cycle 已凍結？
    let frozen: Option<GenRow> = db::first(
        db,
        "SELECT profile_id FROM prediction_generations WHERE user_id = ?1 AND cycle_id = ?2",
        &[&db::text(user_id), &db::text(cycle_id)],
    )
    .await
    .map_err(db_err)?;
    if frozen.is_some() {
        let view = list_cycle(db, user_id, cycle_id).await?;
        return Ok(GenOutcome {
            generated: false,
            view,
        });
    }

    // 2. 最新 complete 側寫（有效側寫不因後續 skip/亂答消失 — 對齊 personality GET）
    let profile: Option<ProfileRow> = db::first(
        db,
        "SELECT id, ipip_answers, ocean_measured FROM personality_profiles \
         WHERE user_id = ?1 AND measurement_status = 'complete' \
         ORDER BY created_at DESC, rowid DESC LIMIT 1",
        &[&db::text(user_id)],
    )
    .await
    .map_err(db_err)?;
    let profile = profile.ok_or(PredictionsError::ProfileIncomplete)?;

    let answers: Vec<u8> = profile
        .ipip_answers
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .ok_or_else(|| PredictionsError::Db("ipip_answers corrupt".into()))?;
    let ocean: OceanScores = profile
        .ocean_measured
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .ok_or_else(|| PredictionsError::Db("ocean_measured corrupt".into()))?;
    let ranges = dim_ranges(&answers)
        .ok_or_else(|| PredictionsError::Db("ipip_answers length invalid".into()))?;
    let display = display_rounded(&ocean);

    // 3. 命中 → 擇一 → 負面不過半（v1：work + money）
    let mut sel: Vec<Selected<'static>> = Vec::new();
    for domain in [Domain::Work, Domain::Money] {
        if let Some(s) = select_for_domain(domain, display, ranges) {
            sel.push(s);
        }
    }
    let sel = filter_negative_half(sel);

    // 4. 落庫（每 domain 原子 WHERE NOT EXISTS，UNIQUE 當防呆；空週也寫 generations 凍結）
    let created = clock::now_iso();
    if created.is_empty() {
        return Err(PredictionsError::Db("clock unavailable".into()));
    }
    for s in &sel {
        let id = uuid::random_uuid();
        let anchor_ids = serde_json::to_string(&s.anchor_ids).unwrap_or_else(|_| "[]".to_string());
        db::exec_changes(
            db,
            "INSERT INTO predictions \
                    (id, user_id, profile_id, cycle_id, domain, trigger, tendency, forecast, \
                     experiment, anchor_ids, anchor_coverage, source, rules_version, is_control, created_at) \
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'rule_anchor', ?12, 0, ?13 \
             WHERE NOT EXISTS (SELECT 1 FROM predictions \
                               WHERE user_id = ?2 AND cycle_id = ?4 AND domain = ?5)",
            &[
                &db::text(&id),
                &db::text(user_id),
                &db::text(&profile.id),
                &db::text(cycle_id),
                &db::text(domain_to_str(s.anchor.domain)),
                &db::text(trigger_class_to_str(s.trigger)),
                &db::text(s.anchor.tendency),
                &db::text(s.anchor.forecast),
                &db::opt_text(s.anchor.experiment),
                &db::text(&anchor_ids),
                &db::text(coverage_to_str(s.coverage)),
                &db::text(RULES_VERSION),
                &db::text(&created),
            ],
        )
        .await
        .map_err(db_err)?;
    }

    // 5. cycle 凍結快照（PRIMARY KEY 防重；INSERT OR IGNORE）
    let gen_changes = db::exec_changes(
        db,
        "INSERT OR IGNORE INTO prediction_generations (user_id, cycle_id, profile_id, generated_at) \
         VALUES (?1, ?2, ?3, ?4)",
        &[
            &db::text(user_id),
            &db::text(cycle_id),
            &db::text(&profile.id),
            &db::text(&created),
        ],
    )
    .await
    .map_err(db_err)?;

    let view = list_cycle(db, user_id, cycle_id).await?;
    Ok(GenOutcome {
        generated: gen_changes > 0,
        view,
    })
}

/// 情境回報（F6 第 1 段）：(user, cycle, trigger) 去重；有預測才可報（D7）；
/// 該 trigger 已有 feedback → 鎖定（P0-3）。
pub async fn upsert_check(
    db: &Turso,
    user_id: &str,
    cycle_id: &str,
    trigger: TriggerWire,
    situation: SituationWire,
) -> Result<SituationCheck, PredictionsError> {
    let trigger_str = trigger_to_str(trigger);

    let exists: Option<OneRow> = db::first(
        db,
        "SELECT 1 AS one FROM predictions WHERE user_id = ?1 AND cycle_id = ?2 AND trigger = ?3 LIMIT 1",
        &[
            &db::text(user_id),
            &db::text(cycle_id),
            &db::text(trigger_str),
        ],
    )
    .await
    .map_err(db_err)?;
    if exists.is_none() {
        return Err(PredictionsError::UnknownTrigger);
    }

    let locked: Option<OneRow> = db::first(
        db,
        "SELECT 1 AS one FROM prediction_feedback pf \
         JOIN predictions p ON p.id = pf.prediction_id \
         WHERE p.user_id = ?1 AND p.cycle_id = ?2 AND p.trigger = ?3 LIMIT 1",
        &[
            &db::text(user_id),
            &db::text(cycle_id),
            &db::text(trigger_str),
        ],
    )
    .await
    .map_err(db_err)?;
    if locked.is_some() {
        return Err(PredictionsError::SituationLocked);
    }

    let created = clock::now_iso();
    if created.is_empty() {
        return Err(PredictionsError::Db("clock unavailable".into()));
    }
    let situation_str = match situation {
        SituationWire::Absent => "absent",
        SituationWire::Occurred => "occurred",
    };
    db::exec(
        db,
        "INSERT INTO situation_checks (user_id, cycle_id, trigger, situation, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(user_id, cycle_id, trigger) DO UPDATE \
           SET situation = excluded.situation, created_at = excluded.created_at",
        &[
            &db::text(user_id),
            &db::text(cycle_id),
            &db::text(trigger_str),
            &db::text(situation_str),
            &db::text(&created),
        ],
    )
    .await
    .map_err(db_err)?;

    Ok(SituationCheck {
        cycleId: cycle_id.to_string(),
        trigger,
        situation,
        createdAt: created,
    })
}

/// 反應回報（F6 第 2 段）：僅 occurred 後可交、一次性。
pub async fn record_feedback(
    db: &Turso,
    user_id: &str,
    prediction_id: &str,
    current_cycle: &str,
    response: ResponseWire,
) -> Result<PredictionFeedback, PredictionsError> {
    let pred: Option<OwnedPredictionRow> = db::first(
        db,
        "SELECT user_id, cycle_id, trigger FROM predictions WHERE id = ?1",
        &[&db::text(prediction_id)],
    )
    .await
    .map_err(db_err)?;
    let pred = pred
        .filter(|p| p.user_id == user_id)
        .ok_or(PredictionsError::NotFound)?;
    if pred.cycle_id != current_cycle {
        return Err(PredictionsError::StaleCycle);
    }

    let check: Option<SituationRow> = db::first(
        db,
        "SELECT situation FROM situation_checks \
         WHERE user_id = ?1 AND cycle_id = ?2 AND trigger = ?3",
        &[
            &db::text(user_id),
            &db::text(&pred.cycle_id),
            &db::text(&pred.trigger),
        ],
    )
    .await
    .map_err(db_err)?;
    match check.map(|c| c.situation).as_deref() {
        None => return Err(PredictionsError::SituationRequired),
        Some("absent") => return Err(PredictionsError::SituationAbsent),
        _ => {}
    }

    let exists: Option<OneRow> = db::first(
        db,
        "SELECT 1 AS one FROM prediction_feedback WHERE prediction_id = ?1",
        &[&db::text(prediction_id)],
    )
    .await
    .map_err(db_err)?;
    if exists.is_some() {
        return Err(PredictionsError::FeedbackExists);
    }

    let created = clock::now_iso();
    if created.is_empty() {
        return Err(PredictionsError::Db("clock unavailable".into()));
    }
    let response_str = match response {
        ResponseWire::Hit => "hit",
        ResponseWire::Miss => "miss",
        ResponseWire::Other => "other",
    };
    db::exec(
        db,
        "INSERT INTO prediction_feedback (prediction_id, response, created_at) VALUES (?1, ?2, ?3)",
        &[
            &db::text(prediction_id),
            &db::text(response_str),
            &db::text(&created),
        ],
    )
    .await
    .map_err(db_err)?;

    Ok(PredictionFeedback {
        predictionId: prediction_id.to_string(),
        response,
        createdAt: created,
    })
}
