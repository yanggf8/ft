//! Big5 personality routes (F1 slice) — quiz with careless-responding state
//! machine, latest profile, data-rights delete.
//! Spec: docs/superpowers/specs/2026-08-28-big5-f1-design.md

use ft_schema::api::{
    PersonalityDeleteResponse, PersonalityMeResponse, PersonalityProfile, QuizResponse,
    QuizSubmission,
};

use super::super::error;
use super::super::services::{clock, db, uuid};
use super::common::{apply_cache_headers, auth_user, client_ip, limiter, ok_json};
use super::R;

const RATE_LIMIT: u32 = 10;
const WINDOW_MS: f64 = 60000.0;

const SELECT_LATEST: &str = "SELECT ipip_answers, ocean_measured, measurement_status, created_at \
     FROM personality_profiles WHERE user_id = ?1 ORDER BY created_at DESC, rowid DESC LIMIT 1";

/// escalation 判斷用：最新一筆（**不限 status**）的狀態——若為 careless_suspected 則本次
/// 升級 skipped_prior_only（「重測一次仍觸發」＝連續次數閘，不採時間窗，Grok 對抗審 #2）。
const SELECT_LATEST_STATUS: &str = "SELECT measurement_status \
     FROM personality_profiles WHERE user_id = ?1 ORDER BY created_at DESC, rowid DESC LIMIT 1";

/// D1 status（snake_case）→ wire status（camelCase）。
fn status_to_wire(s: &str) -> String {
    match s {
        "careless_suspected" => "carelessSuspected".to_string(),
        "skipped_prior_only" => "skippedPriorOnly".to_string(),
        other => other.to_string(), // "complete"
    }
}

#[derive(serde::Deserialize)]
struct StatusRow {
    measurement_status: String, // escalation 判斷「最新一筆是否 careless」用
}

#[derive(serde::Deserialize)]
struct ProfileRow {
    ipip_answers: Option<String>,
    ocean_measured: Option<String>,
    measurement_status: String,
    created_at: Option<String>,
}

fn row_to_profile(r: ProfileRow) -> Option<PersonalityProfile> {
    Some(PersonalityProfile {
        status: status_to_wire(&r.measurement_status),
        oceanMeasured: r.ocean_measured.and_then(|s| serde_json::from_str(&s).ok()),
        answers: r.ipip_answers.and_then(|s| serde_json::from_str(&s).ok()),
        createdAt: r.created_at,
    })
}

pub fn register(router: R<'static>) -> R<'static> {
    router
        .post_async("/api/personality/quiz", |mut req, ctx| async move {
            // isolate 層單例 limiter（baicodex F2）——每請求不再重建
            if !limiter()
                .lock()
                .unwrap()
                .check(&client_ip(&req), RATE_LIMIT, WINDOW_MS)
            {
                return Ok(error::error_code("Too many requests", "RATE_LIMIT", 429));
            }
            let user_id = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let body: QuizSubmission = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error_code("Invalid JSON", "INVALID_JSON", 400)),
            };
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error_code("db unavailable", "DB_UNAVAILABLE", 500)),
            };

            // skip 顯式旗標（Grok 審 #12）：`{skip:true}`＝主動跳過（F7/D5：不產出
            // 掛其名下的分數）；`{skip:false}` 必須帶 answers；`{}`（omitted）或
            // skip 與 answers 並存 → 400，防漏傳 answers 的客戶端被默默記成跳過。
            let (status, ocean_json, answers_json, duration_ms, careless_json) =
                match (body.skip, &body.answers) {
                    (true, None) => {
                        // skip 時 answers/durationMs 皆須缺省——殘留任一 → 400
                        //（Grok 二審 R2-11）。
                        if body.durationMs.is_some() {
                            return Ok(error::error_code(
                                "skip must not carry answers or durationMs",
                                "SKIP_ANSWERS_CONFLICT",
                                400,
                            ));
                        }
                        ("skipped_prior_only".to_string(), None, None, None, None)
                    }
                    (true, Some(_)) | (false, None) => {
                        return Ok(error::error_code(
                            "skip and answers are mutually exclusive",
                            "SKIP_ANSWERS_CONFLICT",
                            400,
                        ));
                    }
                    (false, Some(a)) => {
                        if let Err(v) = ft_big5::validate(a) {
                            return Ok(error::error_code(
                                format!("Validation failed: {}", v),
                                "VALIDATION_FAILED",
                                400,
                            ));
                        }
                        let dur = body.durationMs.unwrap_or(0);
                        let flags = ft_big5::detect_careless(dur, a);
                        let dims = ft_big5::inconsistent_dims(a);
                        // per-signal/per-dim 日誌（Grok 對抗審 #5）：三訊號聯集的 1%–15%
                        // 觸發率無從校準單一旋鈕；落庫各訊號與觸發維，供上線後分條件校準。
                        let careless_json = serde_json::to_string(&serde_json::json!({
                            "too_fast": flags.too_fast,
                            "straight_lining": flags.straight_lining,
                            "inconsistent": flags.inconsistent,
                            "dims": dims,
                        }))
                        .expect("serialize careless flags");
                        if ft_big5::any_triggered(&flags) {
                            // 狀態機（K1：每次提交都落庫）：**最新一筆（不限 status）**為
                            // careless_suspected → 本次升級 skipped_prior_only；否則記
                            // careless_suspected。「重測一次仍觸發」＝**連續次數**閘，
                            // 不採時間窗——Grok 對抗審 #2：5 分鐘窗會罰慢而誠實的重測、
                            // 過期 careless 過度升級；latest=complete/skipped/無記錄 → 422
                            // （careless → complete → careless 重開，R2-1）。
                            let latest: Option<StatusRow> =
                                match db::first(&db, SELECT_LATEST_STATUS, &[&db::text(&user_id)])
                                    .await
                                {
                                    Ok(r) => r,
                                    // Grok 審 #2：讀失敗不得當「無記錄」——Err → 500。
                                    Err(e) => {
                                        return Ok(error::error_code(
                                            format!("db error: {}", e),
                                            "DB_ERROR",
                                            500,
                                        ))
                                    }
                                };
                            let escalated = latest
                                .map(|r| r.measurement_status == "careless_suspected")
                                .unwrap_or(false);
                            let s = if escalated {
                                "skipped_prior_only"
                            } else {
                                "careless_suspected"
                            };
                            (
                                s.to_string(),
                                None,
                                Some(serde_json::to_string(a).expect("serialize answers")),
                                Some(dur),
                                Some(careless_json),
                            )
                        } else {
                            let o = ft_big5::score(a);
                            (
                                "complete".to_string(),
                                Some(serde_json::to_string(&o).expect("serialize scores")),
                                Some(serde_json::to_string(a).expect("serialize answers")),
                                Some(dur),
                                Some(careless_json),
                            )
                        }
                    }
                };

            let id = uuid::random_uuid();
            let uid = db::text(&user_id);
            let ans = db::opt_text(answers_json.as_deref());
            let ocean = db::opt_text(ocean_json.as_deref());
            let st = db::text(&status);
            let dur = db::opt_int(duration_ms.map(|v| v as i64));
            let cf = db::opt_text(careless_json.as_deref());
            let created = clock::now_iso();
            // now_iso() 失敗回 ""（.unwrap_or_default）——空 created_at 會讓「最新一筆」排序
            // 與 subsequent t >= window 比較失真（Grok 對抗審 #4）；fail-closed 不落庫。
            if created.is_empty() {
                return Ok(error::error_code("clock unavailable", "DB_ERROR", 500));
            }
            let created_t = db::text(&created);
            if let Err(e) = db::exec(
                &db,
                "INSERT INTO personality_profiles \
                         (id, user_id, ipip_answers, ocean_measured, measurement_status, \
                          item_duration_ms, careless_flags, created_at) \
                          VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                &[
                    &db::text(&id),
                    &uid,
                    &ans,
                    &ocean,
                    &st,
                    &dur,
                    &cf,
                    &created_t,
                ],
            )
            .await
            {
                return Ok(error::error_code(
                    format!("db error: {}", e),
                    "DB_ERROR",
                    500,
                ));
            }

            // 首次亂答 → 422，前端提示重測一次（code 由前端分流中文訊息）。
            if status == "careless_suspected" {
                return Ok(error::error_code(
                    "Careless responding suspected — retake once",
                    "CARELESS_SUSPECTED",
                    422,
                ));
            }

            let profile = PersonalityProfile {
                status: status_to_wire(&status),
                oceanMeasured: ocean_json.and_then(|s| serde_json::from_str(&s).ok()),
                answers: body.answers,
                createdAt: Some(created),
            };
            Ok(ok_json(
                &serde_json::to_value(QuizResponse { profile }).expect("serialize quiz response"),
                200,
            ))
        })
        .get_async("/api/personality/me", |req, ctx| async move {
            let user_id = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error_code("db unavailable", "DB_UNAVAILABLE", 500)),
            };
            // 讀模型（Grok 審 #3）：profile = 最新一筆 complete（有效側寫不因後續
            // skip/亂答而從產品表面消失）；status = 最新一筆的狀態（前端四態切換用）。
            let latest: Option<ProfileRow> =
                match db::first(&db, SELECT_LATEST, &[&db::text(&user_id)]).await {
                    Ok(r) => r,
                    Err(e) => {
                        return Ok(error::error_code(
                            format!("db error: {}", e),
                            "DB_ERROR",
                            500,
                        ))
                    }
                };
            let complete: Option<ProfileRow> = match db::first(
                &db,
                "SELECT ipip_answers, ocean_measured, measurement_status, created_at \
                 FROM personality_profiles WHERE user_id = ?1 AND measurement_status = 'complete' \
                 ORDER BY created_at DESC, rowid DESC LIMIT 1",
                &[&db::text(&user_id)],
            )
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    return Ok(error::error_code(
                        format!("db error: {}", e),
                        "DB_ERROR",
                        500,
                    ))
                }
            };
            let resp = PersonalityMeResponse {
                profile: complete.and_then(row_to_profile),
                status: latest.map(|r| status_to_wire(&r.measurement_status)),
            };
            let mut res = ok_json(
                &serde_json::to_value(resp).expect("serialize me response"),
                200,
            );
            // per-user GET 的 Cache-Control/Vary 一致性（對齊 charts/users；baicodex F14）
            apply_cache_headers(&mut res, 0, true);
            Ok(res)
        })
        .delete_async("/api/personality/me", |req, ctx| async move {
            let user_id = match auth_user(&req, &ctx).await {
                Ok(u) => u,
                Err(e) => return Ok(e),
            };
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error_code("db unavailable", "DB_UNAVAILABLE", 500)),
            };
            if let Err(e) = db::exec(
                &db,
                "DELETE FROM personality_profiles WHERE user_id = ?1",
                &[&db::text(&user_id)],
            )
            .await
            {
                return Ok(error::error_code(
                    format!("db error: {}", e),
                    "DB_ERROR",
                    500,
                ));
            }
            Ok(ok_json(
                &serde_json::to_value(PersonalityDeleteResponse { success: true })
                    .expect("serialize delete response"),
                200,
            ))
        })
}
