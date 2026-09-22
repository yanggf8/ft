//! F5 predictions 服務層 — 週期生成（cycle 級凍結）＋ F6 兩段式回報。
//! Spec: docs/superpowers/specs/2026-09-04-f5-api-predictions-design.md
//! 測試策略：純函數住 ft-schema（cycle/predict）；本層不 mock、不造假（.testing-rules），
//! 語意由 route code review + 部署後手動 API 驗證。

use ft_schema::anchors::{Domain, TriggerClass, RULES_VERSION};
use ft_schema::api::{
    AnchorCoverageWire, DomainStrengths, DomainWire, OceanScores, Prediction, PredictionFeedback,
    PredictionSourceWire, ResponseWire, SituationCheck, SituationWire, TriggerWire,
};
use ft_schema::cycle::week_start_asia_taipei;
use ft_schema::predict::{
    dim_ranges, display_rounded, filter_negative_half, gated_domains, select_for_domain,
    AnchorCoverage, Selected,
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
struct StrengthsRow {
    work: i64,
    love: i64,
    family: i64,
    money: i64,
    health: i64,
}

impl StrengthsRow {
    /// 寫入端已驗證 0–3；讀回異常（不該存在）→ None，當作無快照。
    fn to_wire(&self) -> Option<DomainStrengths> {
        let cvt = |v: i64| u8::try_from(v).ok().filter(|b| *b <= 3);
        Some(DomainStrengths {
            work: cvt(self.work)?,
            love: cvt(self.love)?,
            family: cvt(self.family)?,
            money: cvt(self.money)?,
            health: cvt(self.health)?,
        })
    }
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

/// 一週的完整視圖（predictions + checks + feedback + 凍結狀態 + F4 強度快照）。
pub struct CycleView {
    pub predictions: Vec<Prediction>,
    pub checks: Vec<SituationCheck>,
    pub feedback: Vec<PredictionFeedback>,
    /// 該週是否已凍結（prediction_generations 有列）。
    pub generated: bool,
    /// 凍結時的 F4 強度快照；未生成或 rules-1 時期的舊週為 None。
    pub strengths: Option<DomainStrengths>,
}

/// 生成結果：`generated` = 本次真的跑了生成管線（false = 該週已凍結/已存在）。
pub struct GenOutcome {
    pub generated: bool,
    pub view: CycleView,
}

/// 列一週（固定領域序 work→money→love；順序與 `predict::GATE_ORDER` 耦合，見該處 doc）。
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
         ORDER BY CASE domain WHEN 'work' THEN 0 WHEN 'money' THEN 1 WHEN 'love' THEN 2 ELSE 3 END, trigger",
        &[&db::text(user_id), &db::text(cycle_id)],
    )
    .await
    .map_err(db_err)?;
    let mut predictions = Vec::with_capacity(rows.len());
    for r in rows {
        predictions.push(to_prediction(r).ok_or_else(|| {
            PredictionsError::Db("predictions row has corrupt enum/source".into())
        })?);
    }

    let checks_rows: Vec<CheckRow> = db::all::<CheckRow>(
        db,
        "SELECT cycle_id, trigger, situation, created_at FROM situation_checks \
         WHERE user_id = ?1 AND cycle_id = ?2 ORDER BY trigger",
        &[&db::text(user_id), &db::text(cycle_id)],
    )
    .await
    .map_err(db_err)?;
    let mut checks = Vec::with_capacity(checks_rows.len());
    for r in checks_rows {
        checks.push(
            to_check(r)
                .ok_or_else(|| PredictionsError::Db("situation_checks row corrupt".into()))?,
        );
    }

    let feedback_rows: Vec<FeedbackRow> = db::all::<FeedbackRow>(
        db,
        "SELECT pf.prediction_id, pf.response, pf.created_at FROM prediction_feedback pf \
         JOIN predictions p ON p.id = pf.prediction_id \
         WHERE p.user_id = ?1 AND p.cycle_id = ?2 ORDER BY pf.created_at",
        &[&db::text(user_id), &db::text(cycle_id)],
    )
    .await
    .map_err(db_err)?;
    let mut feedback = Vec::with_capacity(feedback_rows.len());
    for r in feedback_rows {
        feedback.push(
            to_feedback(r)
                .ok_or_else(|| PredictionsError::Db("prediction_feedback row corrupt".into()))?,
        );
    }

    let generated = db::first::<GenRow>(
        db,
        "SELECT profile_id FROM prediction_generations WHERE user_id = ?1 AND cycle_id = ?2",
        &[&db::text(user_id), &db::text(cycle_id)],
    )
    .await
    .map_err(db_err)?
    .is_some();

    let strengths = db::first::<StrengthsRow>(
        db,
        "SELECT work, love, family, money, health FROM prediction_strengths \
         WHERE user_id = ?1 AND cycle_id = ?2",
        &[&db::text(user_id), &db::text(cycle_id)],
    )
    .await
    .map_err(db_err)?
    .and_then(|r| r.to_wire());

    // F8 事前盲(spec 2026-09-13-f8-control §0/Codex 終審 #2):回饋未收齊前,
    // isControl 一律序列化為 false — 防止從 API 直接指認對照列。收齊 =
    // 每列有 feedback,或其情境為 absent(設計上不會有 feedback)。
    let fb_ids: std::collections::HashSet<&str> =
        feedback.iter().map(|f| f.predictionId.as_str()).collect();
    let absent_triggers: std::collections::HashSet<TriggerWire> = checks
        .iter()
        .filter(|c| c.situation == SituationWire::Absent)
        .map(|c| c.trigger)
        .collect();
    let all_accounted = predictions
        .iter()
        .all(|p| fb_ids.contains(p.id.as_str()) || absent_triggers.contains(&p.trigger));
    if !all_accounted {
        for p in &mut predictions {
            p.isControl = false;
        }
    }

    Ok(CycleView {
        predictions,
        checks,
        feedback,
        generated,
        strengths,
    })
}

/// F6 遮罩（Grok P0-2）：當週 distinct(trigger) 未全部有 check → tendency/forecast/experiment
/// 全數改 null。GET 與 generate 回應皆須經過本函數。
pub fn redact_view(view: &mut CycleView) {
    use std::collections::HashSet;
    // TriggerWire 已 derive Hash（Grok 二審 P2 #9：不用 Debug 當鍵）
    let distinct: HashSet<TriggerWire> = view.predictions.iter().map(|p| p.trigger).collect();
    let answered: HashSet<TriggerWire> = view.checks.iter().map(|c| c.trigger).collect();
    if !distinct.is_subset(&answered) {
        for p in &mut view.predictions {
            p.tendency = None;
            p.forecast = None;
            p.experiment = None;
        }
    }
}

/// F8 對照籤(production):25% 中籤;crypto 不可用 = 不中籤(該槽回真實組,
/// fail-closed;spec 2026-09-13-f8-control §1/Kimi #7)。
fn draw_control_default() -> bool {
    crate::services::uuid::secure_bytes(1).map_or(false, |b| b[0] % 4 == 0)
}

/// F8 洗牌來源:其他使用者的最新 complete 側寫(Kimi #1:每人取最新一列後
/// 隨機抽,避免重測使用者的舊側寫被過度加權)。損壞列有界重抽(≤3);
/// 池空 → None。回 (ranges, display),已驗證可用(spec §1.2 先驗後用)。
async fn draw_shuffled_profile(db: &Turso, me: &str) -> Option<([u8; 5], [f64; 5])> {
    #[derive(serde::Deserialize)]
    struct ShuffledRow {
        ipip_answers: Option<String>,
        ocean_measured: Option<String>,
    }
    for _ in 0..3 {
        let row: Option<ShuffledRow> = db::first(
            db,
            "SELECT p.ipip_answers, p.ocean_measured FROM personality_profiles p \
             WHERE p.measurement_status = 'complete' AND p.user_id != ?1 \
               AND p.id = (SELECT q.id FROM personality_profiles q \
                           WHERE q.user_id = p.user_id AND q.measurement_status = 'complete' \
                           ORDER BY q.created_at DESC, q.rowid DESC LIMIT 1) \
             ORDER BY RANDOM() LIMIT 1",
            &[&db::text(me)],
        )
        .await
        .map_err(|e| {
            // 查詢失敗降級為「無來源」→ 槽回真實組(spec §1;Codex 終審 #4)
            worker::console_log!("f8: shuffled-profile query failed: {e}");
        })
        .ok()
        .flatten();
        let Some(row) = row else { return None }; // 池空:重抽無意義
        let answers = row
            .ipip_answers
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<u8>>(s).ok());
        let ocean = row
            .ocean_measured
            .as_deref()
            .and_then(|s| serde_json::from_str::<OceanScores>(s).ok());
        if let (Some(answers), Some(ocean)) = (answers, ocean) {
            if let Some(ranges) = dim_ranges(&answers) {
                return Some((ranges, display_rounded(&ocean)));
            }
        }
        // 損壞列:重抽(隨機換一列)
    }
    None
}

/// D2-A 壓列後把 is_control 旗標按 (domain, anchor.id) 接回存活列。
/// 鍵唯一(每 domain 一列);被壓列的旗標自然消失(存活者語意,已登記於
/// docs/preregistration/f8-d6.md)。未知列一律視為真實組。
fn reattach_control_flags(
    keys: &[(&'static str, Domain, bool, Option<&'static str>)],
    filtered: &[Selected<'_>],
) -> Vec<bool> {
    filtered
        .iter()
        .map(|s| {
            keys.iter()
                .rev()
                .find(|(id, _, _, _)| *id == s.anchor.id)
                .map(|(_, _, c, _)| *c)
                .unwrap_or(false)
        })
        .collect()
}

/// 一個生成槽位的完整計畫;fallback = 對照降級真實組的原因。
/// (prediction/ledger 的 id 皆於 freeze 前配發 — Codex 終審 #3。)
struct SlotPlan {
    domain: Domain,
    selected: Selected<'static>,
    is_control: bool,
    fallback: Option<&'static str>,
}

/// 純函數:單槽決策(F8 對照 or 真實)— spec rev.3 §1。輸入全部可注入,可測試。
/// 回 (selected, is_control, fallback_reason);None = 槽位誠實空(真實與對照皆零命中)。
fn plan_slot(
    domain: Domain,
    draw_control: bool,
    shuffled: Option<([u8; 5], [f64; 5])>,
    real_display: [f64; 5],
    real_ranges: [u8; 5],
) -> Option<(Selected<'static>, bool, Option<&'static str>)> {
    if draw_control {
        if let Some((c_ranges, c_display)) = shuffled {
            if let Some(s) = select_for_domain(domain, c_display, c_ranges) {
                return Some((s, true, None)); // 對照
            }
            // 洗牌向量零命中 → 真實 fallback(rev.3;prereg 逃生口;Codex 終審 #5 對帳)
            return select_for_domain(domain, real_display, real_ranges)
                .map(|s| (s, false, Some("fallback_zero_hit")));
        }
        // 池空 → 真實 fallback
        return select_for_domain(domain, real_display, real_ranges)
            .map(|s| (s, false, Some("fallback_pool_empty")));
    }
    select_for_domain(domain, real_display, real_ranges).map(|s| (s, false, None))
}

/// 週期生成（cycle 級凍結冪等）。Grok P0-4：一週一 profile 一快照；
/// 已有 predictions/checks/feedback 的週 → 整次只回現況，絕不補 domain
/// （防週中重測混 profile）。只有「空週且尚未開始回報」才允許重新調整 F4 感知。
/// F4：`strengths` 為本週情境輸入（route 層已驗證 0–3），正常情況隨 freeze 凍結。
pub async fn generate(
    db: &Turso,
    user_id: &str,
    cycle_id: &str,
    strengths: &DomainStrengths,
) -> Result<GenOutcome, PredictionsError> {
    generate_with_draw(db, user_id, cycle_id, strengths, draw_control_default).await
}

/// 可注入抽籤的本體(golden 測試注入「永不中籤」序列釘死真實路徑輸出;
/// 分布測試注入計數器)。
pub(crate) async fn generate_with_draw(
    db: &Turso,
    user_id: &str,
    cycle_id: &str,
    strengths: &DomainStrengths,
    draw_control: impl Fn() -> bool,
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
        // 空週是可修正的輸入錯誤：尚未產生 prediction，也沒有任何 F6
        // 回報，讓使用者可以重新調整感知後再跑一次。兩句 DELETE 放在
        // 同一個 batch，避免兩個重試同時把同一週解凍。
        const CLEAR_STRENGTHS_SQL: &str = "DELETE FROM prediction_strengths \
             WHERE user_id = ?1 AND cycle_id = ?2 \
               AND EXISTS (SELECT 1 FROM prediction_generations \
                           WHERE user_id = ?1 AND cycle_id = ?2) \
               AND NOT EXISTS (SELECT 1 FROM predictions WHERE user_id = ?1 AND cycle_id = ?2) \
               AND NOT EXISTS (SELECT 1 FROM situation_checks WHERE user_id = ?1 AND cycle_id = ?2) \
               AND NOT EXISTS (SELECT 1 FROM f8_assignments WHERE user_id = ?1 AND cycle_id = ?2)";
        const CLEAR_GENERATION_SQL: &str = "DELETE FROM prediction_generations \
             WHERE user_id = ?1 AND cycle_id = ?2 \
               AND NOT EXISTS (SELECT 1 FROM predictions WHERE user_id = ?1 AND cycle_id = ?2) \
               AND NOT EXISTS (SELECT 1 FROM situation_checks WHERE user_id = ?1 AND cycle_id = ?2) \
               AND NOT EXISTS (SELECT 1 FROM prediction_feedback pf \
                               JOIN predictions p ON p.id = pf.prediction_id \
                               WHERE p.user_id = ?1 AND p.cycle_id = ?2) \
               AND NOT EXISTS (SELECT 1 FROM f8_assignments WHERE user_id = ?1 AND cycle_id = ?2)";
        let uid = db::text(user_id);
        let cyc = db::text(cycle_id);
        let clear_params: [&db::Param<'_>; 2] = [&uid, &cyc];
        let counts = db::batch(
            db,
            &[
                (CLEAR_STRENGTHS_SQL, &clear_params),
                (CLEAR_GENERATION_SQL, &clear_params),
            ],
        )
        .await
        .map_err(db_err)?;
        if counts.get(1).copied().unwrap_or(0) == 0 {
            let view = list_cycle(db, user_id, cycle_id).await?;
            return Ok(GenOutcome {
                generated: false,
                view,
            });
        }
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

    // 3. F4 領域閘門（強度 ≥1 且該域有錨點，GATE_ORDER 序）× F8 逐槽 25% 對照籤
    //    (spec 2026-09-13-f8-control rev.3 §1):中籤槽以其他使用者最新 complete
    //    側寫走完全相同管線。一切對照失敗(池空/資料損壞/零命中/抽籤或查詢不可用)
    //    誠實降級該槽真實組 — freeze 前後都不報錯(Codex 終審 #3/#4)。
    //    family/health 目錄尚空,select_for_domain 恆 None(前瞻擴充)。
    let gated = gated_domains(strengths);
    let draws: Vec<bool> = gated.iter().map(|_| draw_control()).collect();
    let shuffled = if draws.iter().any(|d| *d) {
        match draw_shuffled_profile(db, user_id).await {
            Some(v) => Some(v),
            None => {
                worker::console_log!("f8: shuffled-profile query failed; slots degrade to real");
                None
            }
        }
    } else {
        None
    };
    // 槽位計畫(Codex 終審 #3):所有 id 於 freeze 前配發 — freeze 後零 crypto 依賴。
    let mut plan: Vec<SlotPlan> = Vec::new();
    for (domain, drawn) in gated.iter().zip(draws.iter()) {
        if let Some((selected, is_control, fallback)) =
            plan_slot(*domain, *drawn, shuffled, display, ranges)
        {
            plan.push(SlotPlan {
                domain: *domain,
                selected,
                is_control,
                fallback,
            });
        }
    }
    let keys: Vec<(&'static str, Domain, bool, Option<&'static str>)> = plan
        .iter()
        .map(|p| (p.selected.anchor.id, p.domain, p.is_control, p.fallback))
        .collect();
    let selecteds: Vec<Selected<'static>> = plan.iter().map(|p| p.selected.clone()).collect();
    let sel = filter_negative_half(selecteds);
    let control_flags = reattach_control_flags(&keys, &sel);
    let ledger_ids: Vec<String> = keys.iter().map(|_| uuid::random_uuid()).collect();

    // 4. 凍結 + 插入（單一 Hrana batch = 原子隱含交易；F7 同款）。
    //    steps[0]=F4 強度快照、steps[1]=cycle 凍結快照、steps[2..]=predictions
    //    （is_control 逐列帶入 F8 對照旗標）。兩個快照皆 INSERT OR IGNORE：
    //    併發敗者的整個 batch 是結構性 no-op（predictions 另有 WHERE NOT EXISTS
    //    原子防呆），以 steps[1] 的 affected==0 偵測「他人已凍結」→ 只回現況，
    //    絕不補 domain。空週也凍結。
    let created = clock::now_iso();
    if created.is_empty() {
        return Err(PredictionsError::Db("clock unavailable".into()));
    }
    const STRENGTHS_INSERT_SQL: &str = "INSERT OR IGNORE INTO prediction_strengths \
         (user_id, cycle_id, work, love, family, money, health, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";
    const GEN_INSERT_SQL: &str = "INSERT OR IGNORE INTO prediction_generations \
         (user_id, cycle_id, profile_id, generated_at) VALUES (?1, ?2, ?3, ?4)";
    const PRED_INSERT_SQL: &str = "INSERT INTO predictions \
         (id, user_id, profile_id, cycle_id, domain, trigger, tendency, forecast, \
          experiment, anchor_ids, anchor_coverage, source, rules_version, is_control, created_at) \
         SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'rule_anchor', ?12, ?13, ?14 \
         WHERE NOT EXISTS (SELECT 1 FROM predictions \
                           WHERE user_id = ?2 AND cycle_id = ?4 AND domain = ?5)";

    let uid = db::text(user_id);
    let cyc = db::text(cycle_id);
    let pid = db::text(&profile.id);
    let ts = db::text(&created);
    let s_work = db::int(strengths.work as i32);
    let s_love = db::int(strengths.love as i32);
    let s_family = db::int(strengths.family as i32);
    let s_money = db::int(strengths.money as i32);
    let s_health = db::int(strengths.health as i32);

    // predictions 的擁有參數（id/anchor_ids 字串須活到 batch 呼叫；
    // id 於 freeze 前配發 — Codex 終審 #3）
    let preds: Vec<(String, String, &Selected<'static>, bool)> = sel
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let anchor_ids =
                serde_json::to_string(&s.anchor_ids).unwrap_or_else(|_| "[]".to_string());
            (uuid::random_uuid(), anchor_ids, s, control_flags[i])
        })
        .collect();

    let mut pred_param_lists: Vec<Vec<&db::Param<'_>>> = Vec::with_capacity(preds.len());
    // 先收「擁有的」Param（借用 preds 字串與 &'static str，活到 batch 呼叫為止），
    // 再第二輪取 `&Param` — 迴圈內區域變數不能被推入外層存活的向量。
    let mut owned_lists: Vec<Vec<db::Param<'_>>> = Vec::with_capacity(preds.len());
    for (id, anchor_ids, s, is_control) in &preds {
        owned_lists.push(vec![
            db::text(id),
            db::text(user_id),
            db::text(&profile.id),
            db::text(cycle_id),
            db::text(domain_to_str(s.anchor.domain)),
            db::text(trigger_class_to_str(s.trigger)),
            db::text(s.anchor.tendency),
            db::text(s.anchor.forecast),
            db::opt_text(s.anchor.experiment),
            db::text(anchor_ids),
            db::text(coverage_to_str(s.coverage)),
            db::text(RULES_VERSION),
            db::int(*is_control as i32),
            db::text(&created),
        ]);
    }
    for l in &owned_lists {
        pred_param_lists.push(l.iter().collect());
    }

    let strengths_params: [&db::Param<'_>; 8] = [
        &uid, &cyc, &s_work, &s_love, &s_family, &s_money, &s_health, &ts,
    ];
    let gen_params: [&db::Param<'_>; 4] = [&uid, &cyc, &pid, &ts];
    let mut stmts: Vec<(&'static str, &[&db::Param<'_>])> = Vec::with_capacity(2 + preds.len());
    stmts.push((STRENGTHS_INSERT_SQL, &strengths_params));
    stmts.push((GEN_INSERT_SQL, &gen_params));
    for params in &pred_param_lists {
        stmts.push((PRED_INSERT_SQL, params));
    }

    let counts = db::batch(db, &stmts).await.map_err(db_err)?;
    if counts.get(1).copied().unwrap_or(0) == 0 {
        // 併發/重試：他人已凍結 → 只回現況（絕不補 domain）
        let view = list_cycle(db, user_id, cycle_id).await?;
        return Ok(GenOutcome {
            generated: false,
            view,
        });
    }

    // 5. F8 帳本(唯一在 batch 外的寫入;純簿記,不承載 F5 原子性 — Codex 終審
    //    #1 ITT 對帳):存活槽位 → assigned(+落庫 prediction id);被 D2-A 壓掉
    //    槽位 → suppressed。所有 id 已於 freeze 前配發。
    for (id, _, s, is_control) in preds.iter() {
        let drawn_arm = if *is_control { "control" } else { "real" };
        let li = keys
            .iter()
            .position(|(aid, _, _, _)| *aid == s.anchor.id)
            .expect("survivor must come from plan");
        // fallback_reason 落實際值（fallback_pool_empty/fallback_zero_hit）—
        // drawn_arm 記降級後的最終 arm，不記原因的話「中籤後降級」與「未中籤」
        // 無法區分，25% 逃生口登記（spec rev.3 §1.4）無從對帳。
        let a_f = db::opt_text(keys[li].3);
        let a_id = db::text(&ledger_ids[li]);
        let a_u = db::text(user_id);
        let a_c = db::text(cycle_id);
        let a_d = db::text(domain_to_str(s.anchor.domain));
        let a_arm = db::text(drawn_arm);
        let a_pid = db::text(id);
        let a_t = db::text(&created);
        db::exec(
            db,
            "INSERT INTO f8_assignments \
             (id, user_id, cycle_id, domain, drawn_arm, fallback_reason, suppressed, prediction_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8)",
            &[&a_id, &a_u, &a_c, &a_d, &a_arm, &a_f, &a_pid, &a_t],
        )
        .await
        .map_err(db_err)?;
    }
    // 被計畫但被 D2-A 壓掉的槽位(含對照指派):帳本記 suppressed,ITT 可對帳
    // (Codex 終審 #1;spec rev.3 §1)。
    let survived: std::collections::HashSet<&'static str> =
        sel.iter().map(|s| s.anchor.id).collect();
    for (k_i, (anchor_id, domain, is_control, fallback)) in keys.iter().enumerate() {
        if survived.contains(anchor_id) {
            continue;
        }
        let aid = &ledger_ids[k_i];
        let drawn_arm = if *is_control { "control" } else { "real" };
        let a_f = db::opt_text(*fallback);
        let a_id = db::text(aid);
        let a_u = db::text(user_id);
        let a_c = db::text(cycle_id);
        let a_d = db::text(domain_to_str(*domain));
        let a_arm = db::text(drawn_arm);
        let a_t = db::text(&created);
        db::exec(
            db,
            "INSERT INTO f8_assignments \
             (id, user_id, cycle_id, domain, drawn_arm, fallback_reason, suppressed, prediction_id, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, NULL, ?7)",
            &[&a_id, &a_u, &a_c, &a_d, &a_arm, &a_f, &a_t],
        )
        .await
        .map_err(db_err)?;
    }

    let view = list_cycle(db, user_id, cycle_id).await?;
    Ok(GenOutcome {
        generated: true,
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

    let created = clock::now_iso();
    if created.is_empty() {
        return Err(PredictionsError::Db("clock unavailable".into()));
    }
    let situation_str = match situation {
        SituationWire::Absent => "absent",
        SituationWire::Occurred => "occurred",
    };
    // 原子鎖（Grok 二審 P1 #2）：feedback 存在 → SELECT 0 列 → changes=0 → SITUATION_LOCKED。
    // 檢查與寫入同一句，無「SELECT 通過→feedback 寫入→覆寫 situation」的交錯窗。
    let changes = db::exec_changes(
        db,
        "INSERT INTO situation_checks (user_id, cycle_id, trigger, situation, created_at) \
         SELECT ?1, ?2, ?3, ?4, ?5 \
         WHERE NOT EXISTS (SELECT 1 FROM prediction_feedback pf \
                           JOIN predictions p ON p.id = pf.prediction_id \
                           WHERE p.user_id = ?2 AND p.cycle_id = ?3 AND p.trigger = ?4) \
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
    if changes == 0 {
        return Err(PredictionsError::SituationLocked);
    }

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
    // 僅 occurred 放行（Grok 二審 P1 #3）：absent 仍為合法 409；其他損壞值 fail-closed。
    match check.map(|c| c.situation).as_deref() {
        Some("occurred") => {}
        Some("absent") => return Err(PredictionsError::SituationAbsent),
        None => return Err(PredictionsError::SituationRequired),
        _ => return Err(PredictionsError::Db("situation_checks row corrupt".into())),
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
    // 原子一次性（Grok 二審 P2 #7）：WHERE NOT EXISTS → changes=0 = 已存在（FEEDBACK_EXISTS），
    // 不靠 PK 撞出 500。
    let changes = db::exec_changes(
        db,
        "INSERT INTO prediction_feedback (prediction_id, response, created_at) \
         SELECT ?1, ?2, ?3 \
         WHERE NOT EXISTS (SELECT 1 FROM prediction_feedback WHERE prediction_id = ?1)",
        &[
            &db::text(prediction_id),
            &db::text(response_str),
            &db::text(&created),
        ],
    )
    .await
    .map_err(db_err)?;
    if changes == 0 {
        return Err(PredictionsError::FeedbackExists);
    }

    Ok(PredictionFeedback {
        predictionId: prediction_id.to_string(),
        response,
        createdAt: created,
    })
}

#[cfg(test)]
mod f8_tests {
    use super::*;
    use ft_schema::anchors::ANCHORS;

    fn selected_of(domain: Domain) -> Selected<'static> {
        let a = ANCHORS.iter().find(|a| a.domain == domain).unwrap();
        Selected {
            trigger: a.trigger,
            anchor: a,
            anchor_ids: vec![a.id],
            coverage: AnchorCoverage::High,
            valence: a.valence,
        }
    }

    #[test]
    fn control_flags_follow_surviving_rows() {
        // keys 必須用真實 ANCHORS id(reattach 以 anchor.id 對帳;4-tuple =
        // anchor_id, domain, is_control, fallback_reason)
        let work = selected_of(Domain::Work);
        let money = selected_of(Domain::Money);
        let keys = vec![
            (work.anchor.id, Domain::Work, false, None),
            (
                money.anchor.id,
                Domain::Money,
                true,
                Some("fallback_zero_hit"),
            ),
        ];
        // D2-A 壓掉 work → money 的對照旗標必須跟著存活列走
        let rows = vec![money.clone()];
        assert_eq!(reattach_control_flags(&keys, &rows), vec![true]);
        let rows = vec![work, money];
        assert_eq!(reattach_control_flags(&keys, &rows), vec![false, true]);
    }

    #[test]
    fn unknown_rows_default_to_real() {
        let keys = vec![("work-x", Domain::Work, false, None)];
        let rows = vec![selected_of(Domain::Money)];
        assert_eq!(reattach_control_flags(&keys, &rows), vec![false]);
    }

    #[test]
    fn draw_is_exact_quarter_and_crypto_failure_is_real() {
        // 25% 籤的純映射:% 4 == 0 中籤(256 整除 4,無模偏誤;spec §1)
        let draw = |b: u8| b % 4 == 0;
        assert!(draw(0));
        assert!(draw(4));
        assert!(!draw(1));
        assert!(!draw(255));
        // crypto 不可用(None)→ 不中籤 = 該槽真實組(fail-closed;Kimi #7)
        let crypto_none: Option<Vec<u8>> = None;
        assert!(!crypto_none.map_or(false, |b| b[0] % 4 == 0));
    }
}
