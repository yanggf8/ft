//! /personality — IPIP-15 questionnaire + result page (F1 slice).
//! The questionnaire deliberately contains no birth-chart information; the
//! result keeps the norm source and non-diagnostic disclaimer visible.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use ft_schema::api::{OceanScores, PersonalityMeResponse, PersonalityProfile, QuizSubmission};
use ft_schema::items::{DIMENSION_NAMES, ITEMS, NORMS, SCALE_ANCHORS, SOURCE};

use crate::api::{self, ApiErr};

#[derive(Clone)]
enum PersonalityState {
    Loading,
    Quiz { suspected: bool },
    Result(PersonalityProfile),
    NotScored { profile: Option<PersonalityProfile> },
}

/// Descriptive profile copy. Bands use the rounded display score so the text
/// and number never disagree at a boundary: low <33, middle [33, 67), high >=67.
fn profile_copy(dim: usize, score: f64) -> &'static str {
    let shown = score.round();
    let band = if shown >= 67.0 {
        2
    } else if shown < 33.0 {
        0
    } else {
        1
    };
    match (dim, band) {
        (0, 2) => "你在人群裡容易感到自在，主動開啟對話對你來說不算負擔，能量常從互動中來。",
        (0, 1) => "你在需要互動的場合能自然應對，但獨處與社交對你都是舒服的選項。",
        (0, 0) => "你傾向安靜的相處方式，獨處時反而更容易把事情想清楚。",
        (1, 2) => "你容易注意到別人的處境，也樂意先把方便留給對方。",
        (1, 1) => "你對人保持友善，但也會視情況照顧自己的需要。",
        (1, 0) => "你偏好直接講重點，先講道理再談感受，對你來說更自然。",
        (2, 2) => "你做事傾向按部就班，事先安排比臨場發揮讓你安心。",
        (2, 1) => "你對重要的部分有要求，但也允許一些彈性空間。",
        (2, 0) => "你偏好保持彈性，計畫太細反而覺得綁手綁腳。",
        (3, 2) => "你多數時候情緒平穩，即使有波動，通常也能較快恢復平穩。",
        (3, 1) => "你的情緒有起伏，但多半能自己消化，不太會長時間卡住。",
        (3, 0) => "你的情緒起伏比較明顯，遇到壓力時感受會放大——這是傾向描述，不是缺陷。",
        (4, 2) => "你喜歡想法多一點的對話，腦中常同時轉著好幾個念頭。",
        (4, 1) => "你對有興趣的題目會想多挖一層，其他人事物則量力而為。",
        (4, 0) => "你偏好具體實在的東西，先看到再相信讓你踏實。",
        _ => "",
    }
}

fn state_from_response(response: PersonalityMeResponse) -> PersonalityState {
    match response.status.as_deref() {
        None => PersonalityState::Quiz { suspected: false },
        Some("complete") => match response.profile {
            Some(profile) => PersonalityState::Result(profile),
            None => PersonalityState::NotScored { profile: None },
        },
        Some("carelessSuspected") => PersonalityState::Quiz { suspected: true },
        Some("skippedPriorOnly") => PersonalityState::NotScored {
            profile: response.profile,
        },
        Some(_) => PersonalityState::NotScored {
            profile: response.profile,
        },
    }
}

async fn fetch_state() -> Result<PersonalityState, ApiErr> {
    api::get_personality(true).await.map(state_from_response)
}

fn reset_quiz(
    state: RwSignal<PersonalityState>,
    answers: RwSignal<[Option<u8>; 15]>,
    error: RwSignal<String>,
    quiz_started_at: RwSignal<f64>,
) {
    answers.set([None; 15]);
    error.set(String::new());
    quiz_started_at.set(js_sys::Date::now());
    state.set(PersonalityState::Quiz { suspected: false });
}

fn post_error_message(error: &ApiErr) -> &'static str {
    if error.is_code("RATE_LIMIT") || error.status() == Some(429) {
        "稍後再試"
    } else if error.is_code("VALIDATION_FAILED")
        || error.is_code("SKIP_ANSWERS_CONFLICT")
        || error.status() == Some(400)
    {
        "作答格式錯誤，請重新作答"
    } else {
        "送出失敗，請稍後再試"
    }
}

fn scores(scores: &OceanScores) -> [f64; 5] {
    [
        scores.extraversion,
        scores.agreeableness,
        scores.conscientiousness,
        scores.emotionalStability,
        scores.intellectImagination,
    ]
}

#[component]
pub fn PersonalityPage() -> impl IntoView {
    let state = RwSignal::new(PersonalityState::Loading);
    let answers = RwSignal::new([None::<u8>; 15]);
    let error = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);
    let deleting = RwSignal::new(false);
    let quiz_started_at = RwSignal::new(js_sys::Date::now());
    // ── F3 疊圖(spec 2026-09-07-f2-f3 §2.3):一律 opt-in,不自動顯示 ──
    let overlay = RwSignal::new(Option::<ft_schema::symbolic::OverlayResponse>::None);
    let overlay_error = RwSignal::new(String::new());
    let overlay_stage = RwSignal::new(0u8); // 0=未展開 1=說明頁 2=疊圖
                                            // 世代計數:重測/刪除後遲到的 overlay 回應不得覆寫新狀態(Codex 二輪 #3)
    let overlay_request_gen = RwSignal::new(0u32);
    let reset_overlay = move || {
        overlay_request_gen.set(overlay_request_gen.get() + 1);
        overlay.set(None);
        overlay_error.set(String::new());
        overlay_stage.set(0);
    };
    let open_overlay = move |_| {
        // 進入前固定說明(此文案為禁用詞白名單,經 owner 核可)
        overlay_stage.set(1);
    };
    let load_overlay = move |_| {
        let gen = overlay_request_gen.get() + 1;
        overlay_request_gen.set(gen);
        spawn_local(async move {
            match api::fetch_overlay().await {
                Ok(v) => {
                    overlay_error.set(String::new()); // 重試成功要清掉舊錯誤(Codex 二輪 #4)
                    if overlay_request_gen.get() == gen {
                        overlay.set(Some(v));
                        overlay_stage.set(2);
                    }
                }
                Err(e) => {
                    if overlay_request_gen.get() != gen {
                        return; // 遲到的舊回應,不覆寫(Codex 二輪 #3)
                    }
                    overlay_error.set(if e.is_code("F3_DISABLED") {
                        "你目前選擇僅使用命盤象徵、未完成人格測驗，對照功能未開啟。".to_string()
                    } else if e.is_code("MEASUREMENT_PENDING") {
                        "測驗結果尚待確認，完成有效測驗後即可對照。".to_string()
                    } else if e.is_code("NO_MEASUREMENT") {
                        "完成人格測驗後即可對照。".to_string()
                    } else {
                        // 500/網路錯：固定中文文案，不外洩伺服器訊息(Kimi 終審 #4)
                        "暫時無法載入，請稍後再試。".to_string()
                    });
                    overlay_stage.set(2);
                }
            }
        });
    };

    Effect::new(move |_| {
        spawn_local(async move {
            match fetch_state().await {
                Ok(next) => state.set(next),
                Err(_) => {
                    error.set("人格資料載入失敗，請稍後再試".to_string());
                    state.set(PersonalityState::Quiz { suspected: false });
                }
            }
        });
    });

    let submit_answers = move |_| {
        if submitting.get_untracked() {
            return;
        }
        let current = answers.get_untracked();
        let Some(values) = current.into_iter().collect::<Option<Vec<u8>>>() else {
            return;
        };

        submitting.set(true);
        error.set(String::new());
        let elapsed = (js_sys::Date::now() - quiz_started_at.get_untracked()).max(0.0) as u64;
        let body = QuizSubmission {
            skip: false,
            answers: Some(values),
            durationMs: Some(elapsed),
        };

        spawn_local(async move {
            match api::submit_quiz(&body).await {
                Err(err) if err.is_code("CARELESS_SUSPECTED") => {
                    answers.set([None; 15]);
                    quiz_started_at.set(js_sys::Date::now());
                    error.set(String::new());
                    state.set(PersonalityState::Quiz { suspected: true });
                }
                result => {
                    let post_error = result.err().map(|err| post_error_message(&err));
                    match fetch_state().await {
                        Ok(next) => state.set(next),
                        Err(_) if post_error.is_none() => {
                            error.set("人格資料載入失敗，請稍後再試".to_string());
                        }
                        Err(_) => {}
                    }
                    if let Some(message) = post_error {
                        error.set(message.to_string());
                    }
                }
            }
            submitting.set(false);
        });
    };

    let skip_quiz = move |_| {
        if submitting.get_untracked() {
            return;
        }
        submitting.set(true);
        error.set(String::new());
        let body = QuizSubmission {
            skip: true,
            answers: None,
            durationMs: None,
        };

        spawn_local(async move {
            match api::submit_quiz(&body).await {
                Err(err) if err.is_code("CARELESS_SUSPECTED") => {
                    answers.set([None; 15]);
                    quiz_started_at.set(js_sys::Date::now());
                    error.set(String::new());
                    state.set(PersonalityState::Quiz { suspected: true });
                }
                result => {
                    let post_error = result.err().map(|err| post_error_message(&err));
                    match fetch_state().await {
                        Ok(next) => state.set(next),
                        Err(_) if post_error.is_none() => {
                            error.set("人格資料載入失敗，請稍後再試".to_string());
                        }
                        Err(_) => {}
                    }
                    if let Some(message) = post_error {
                        error.set(message.to_string());
                    }
                }
            }
            submitting.set(false);
        });
    };

    view! {
        <div class="page">
            <A href="/" attr:class="back-link">"← 返回"</A>
            <h1 style="margin-bottom:1.5rem">"人格測驗"</h1>

            <Show when=move || !error.get().is_empty()>
                <p class="error">{move || error.get()}</p>
            </Show>

            {move || match state.get() {
                PersonalityState::Loading => view! {
                    <div class="center-note">"載入中..."</div>
                }.into_any(),
                PersonalityState::Quiz { suspected } => view! {
                    <div class="card">
                        <div class="quiz-head">
                            <div>
                                <h2>"IPIP-15"</h2>
                                <p class="muted">"十五題，約 90 秒。請依你平常的情況作答。"</p>
                            </div>
                            <button
                                class="btn-link"
                                prop:disabled=move || submitting.get()
                                on:click=skip_quiz
                            >
                                "先不測"
                            </button>
                        </div>
                        <p class="quiz-consent muted">
                            "作答視為同意僅用於本站人格分析；原始作答僅儲存於本站，不對外提供。"
                        </p>
                        <Show when=move || suspected>
                            <p class="error">"這份作答與常見模式差異較大，結果可能不具參考性，請再試一次"</p>
                        </Show>
                        {move || {
                            let n = answers.get().iter().filter(|a| a.is_some()).count();
                            view! {
                                <p class="quiz-progress"><strong>{format!("{n}")}</strong>" / 15"</p>
                                <div class="quiz-progress-bar" aria-hidden="true">
                                    <span style=format!("width:{:.0}%", n as f32 / 15.0 * 100.0)></span>
                                </div>
                            }
                        }}
                        <div class="quiz-key" aria-hidden="true">
                            {SCALE_ANCHORS.iter().map(|anchor| view! { <span>{*anchor}</span> }).collect_view()}
                        </div>

                        <div>
                            {ITEMS.iter().enumerate().map(|(index, item)| {
                                view! {
                                    <fieldset class="quiz-item">
                                        <legend>
                                            <span class="quiz-no">{item.no}</span>
                                            {item.text}
                                        </legend>
                                        <div class="quiz-choices">
                                            {SCALE_ANCHORS.iter().enumerate().map(|(anchor_index, anchor)| {
                                                let value = (anchor_index + 1) as u8;
                                                let input_name = format!("personality-item-{}", item.no);
                                                view! {
                                                    <label class="quiz-choice">
                                                        <input
                                                            type="radio"
                                                            name=input_name
                                                            value=value
                                                            aria-label=format!("{} {}", value, anchor)
                                                            prop:checked=move || answers.get()[index] == Some(value)
                                                            prop:disabled=move || submitting.get()
                                                            on:change=move |_| answers.update(|all| all[index] = Some(value))
                                                        />
                                                        <span class="quiz-choice-n" aria-hidden="true">{value}</span>
                                                    </label>
                                                }
                                            }).collect_view()}
                                        </div>
                                    </fieldset>
                                }
                            }).collect_view()}
                        </div>

                        <div class="quiz-submit">
                            <button
                                class="btn-primary"
                                prop:disabled=move || submitting.get() || answers.get().iter().any(Option::is_none)
                                on:click=submit_answers
                            >
                                {move || if submitting.get() { "送出中..." } else { "查看結果" }}
                            </button>
                        </div>
                    </div>
                }.into_any(),
                PersonalityState::Result(profile) => {
                    let measured = profile.oceanMeasured.clone();
                    let raw_answers = profile.answers.clone();
                    view! {
                        <div class="card">
                            <h2 style="margin-bottom:0.5rem">"你的行為傾向"</h2>
                            <p class="muted" style="margin-bottom:1.25rem">"分數呈現五個面向的相對傾向。"</p>

                            {measured.map(|ocean| {
                                scores(&ocean).into_iter().enumerate().map(|(index, score)| {
                                    let shown = score.round();
                                    let mean = NORMS[index].mean.clamp(0.0, 100.0);
                                    let band_left = (NORMS[index].mean - NORMS[index].sd).clamp(0.0, 100.0);
                                    let band_right = (NORMS[index].mean + NORMS[index].sd).clamp(0.0, 100.0);
                                    view! {
                                        <section class="ocean-dim">
                                            <div class="ocean-dim-head">
                                                <strong>{DIMENSION_NAMES[index]}</strong>
                                                <span class="ocean-score">{format!("{shown:.0}")}</span>
                                            </div>
                                            <div class="ocean-track-wrap">
                                                <div class="ocean-track">
                                                    <div class="ocean-band" style=format!("left:{band_left:.2}%;width:{:.2}%", band_right - band_left)></div>
                                                    <div
                                                        class="ocean-mean"
                                                        title="臺灣中老年立意取樣常模平均"
                                                        style=format!("left:{mean:.2}%")
                                                    ></div>
                                                    <div class="ocean-marker" style=format!("left:{:.2}%", score.clamp(0.0, 100.0))></div>
                                                </div>
                                            </div>
                                            <p class="ocean-caption">
                                                {format!("常模平均 {:.0}；淡色區為平均正負一個標準差", NORMS[index].mean)}
                                            </p>
                                            <p class="ocean-copy">{profile_copy(index, score)}</p>
                                        </section>
                                    }
                                }).collect_view()
                            })}

                            <div class="ocean-foot">
                                <strong>"趨勢參考，非心理診斷、非醫療建議"</strong>
                                <p class="muted">{SOURCE}</p>

                                <details>
                                    <summary>"查看原始 15 題作答"</summary>
                                <ol style="margin:1rem 0 0 1.25rem;display:grid;gap:0.5rem">
                                    {raw_answers.unwrap_or_default().into_iter().enumerate().map(|(index, answer)| {
                                        let item = &ITEMS[index];
                                        let anchor = SCALE_ANCHORS.get(answer.saturating_sub(1) as usize).copied().unwrap_or("");
                                        view! {
                                            <li>{format!("{}：{}（{}）", item.text, answer, anchor)}</li>
                                        }
                                    }).collect_view()}
                                </ol>
                            </details>

                                <div class="actions">
                                    <button
                                        class="btn-primary"
                                        on:click=move |_| {
                                            reset_overlay();
                                            reset_quiz(state, answers, error, quiz_started_at);
                                        }
                                    >
                                    "重測"
                                </button>
                                <button
                                    class="btn-link"
                                    prop:disabled=move || deleting.get()
                                    on:click=move |_| {
                                        if deleting.get_untracked() {
                                            return;
                                        }
                                        let confirmed = web_sys::window()
                                            .and_then(|window| window.confirm_with_message("確定刪除全部人格資料？").ok())
                                            .unwrap_or(false);
                                        if !confirmed {
                                            return;
                                        }
                                        deleting.set(true);
                                        error.set(String::new());
                                        spawn_local(async move {
                                            match api::delete_personality().await {
                                                Ok(_) => {
                                                    reset_overlay();
                                                    reset_quiz(state, answers, error, quiz_started_at);
                                                }
                                                Err(_) => error.set("刪除失敗，請稍後再試".to_string()),
                                            }
                                            deleting.set(false);
                                        });
                                    }
                                >
                                    {move || if deleting.get() { "刪除中..." } else { "刪除人格資料" }}
                                </button>
                                </div>
                            </div>

                            <div class="card" style="margin-top:1.5rem">
                                <h2 style="margin-bottom:0.5rem">"命盤象徵對照"</h2>
                                <Show when=move || overlay_stage.get() == 0 fallback=|| ()>
                                    <button class="btn-link" on:click=open_overlay>
                                        "查看命盤象徵對照"
                                    </button>
                                </Show>
                                <Show when=move || overlay_stage.get() == 1 fallback=|| ()>
                                    <p class="muted" style="line-height:1.9">
                                        "「命盤象徵傾向」來自出生盤的傳統星性對照，"
                                        "「實測人格」來自你剛完成的人格測驗。兩條線來源不同，"
                                        "不一致很常見；不一致不代表你有缺陷，也不代表你需要改變。"
                                        "只有兩者差距夠大時，我們才會描述那個落差。"
                                    </p>
                                    <button class="btn-primary" on:click=load_overlay>
                                        "我知道了，看對照"
                                    </button>
                                </Show>
                                <Show when=move || overlay_stage.get() == 2 fallback=|| ()>
                                    {move || {
                                        let err = overlay_error.get();
                                        if !err.is_empty() {
                                            return view! {
                                                <div>
                                                    <p class="error">{err}</p>
                                                    <button class="btn-link" on:click=load_overlay>
                                                        "重試"
                                                    </button>
                                                </div>
                                            }.into_any();
                                        }
                                        let Some(v) = overlay.get() else {
                                            return view! { <div/> }.into_any();
                                        };
                                        let missing_note = if !v.birth_known {
                                            Some("填寫生辰後即可和命盤對照。".to_string())
                                        } else if v.prior_source.is_none() {
                                            Some("命盤暫時無法計算，請稍後再試。".to_string())
                                        } else {
                                            None
                                        };
                                        let note_view = missing_note.map(|n| view! {
                                            <p class="muted">{n}</p>
                                        });
                                        // 雷達:五軸 E,A,C,S,O;實線=實測、虛線=命盤象徵;
                                        // 軸上不標裸分;0 分就在圓心,不外推
                                        let pts = |vals: Vec<f64>| -> String {
                                            vals.iter().enumerate().map(|(i, s)| {
                                                let ang = -90.0f64 + 72.0 * i as f64;
                                                let r = 80.0 * (s / 100.0).clamp(0.0, 1.0);
                                                let (x, y) = (
                                                    100.0 + r * ang.to_radians().cos(),
                                                    100.0 + r * ang.to_radians().sin(),
                                                );
                                                format!("{x:.1},{y:.1}")
                                            }).collect::<Vec<_>>().join(" ")
                                        };
                                        let measured_pts =
                                            pts(v.dims.iter().map(|d| d.measured).collect());
                                        let prior_pts = pts(
                                            v.dims.iter().map(|d| d.prior.unwrap_or(50.0)).collect(),
                                        );
                                        let axis_labels = [
                                            "E 外向",
                                            "A 友善",
                                            "C 嚴謹",
                                            "S 情緒穩定(高=穩)",
                                            "O 智性開放",
                                        ];
                                        let dim_label = |code: &str| -> &'static str {
                                            let i = ft_schema::symbolic::DIM_CODES
                                                .iter()
                                                .position(|&c| c == code)
                                                .unwrap_or(0);
                                            ft_schema::symbolic::DIM_LABELS[i]
                                        };
                                        let band_label =
                                            |code: &str| ft_schema::symbolic::band_label(code);
                                        let basis_label = |b: Option<&str>| match b {
                                            Some("classical") => "依據:古典星性",
                                            Some("designer") => "依據:設計裁量",
                                            Some("mixed") => "依據:古典星性 + 設計裁量",
                                            _ => "—",
                                        };
                                        let radar = if v.prior_source.is_some() {
                                            view! {
                                                <svg viewBox="0 0 200 200"
                                                    style="width:min(320px,80%);display:block;margin:0 auto"
                                                >
                                                    <polygon
                                                        points={pts(vec![100.0; 5])}
                                                        fill="none"
                                                        stroke="#e5e7eb"
                                                    />
                                                    <polygon
                                                        points={measured_pts.clone()}
                                                        fill="rgba(59,130,246,0.12)"
                                                        stroke="#3b82f6"
                                                        stroke-width="2"
                                                    />
                                                    <polygon
                                                        points={prior_pts.clone()}
                                                        fill="none"
                                                        stroke="#9aa3b2"
                                                        stroke-width="2"
                                                        stroke-dasharray="5,4"
                                                    />
                                                    {axis_labels.iter().enumerate().map(|(i, label)| {
                                                        let ang = -90.0f64 + 72.0 * i as f64;
                                                        let (x, y) = (
                                                            100.0 + 92.0 * ang.to_radians().cos(),
                                                            100.0 + 92.0 * ang.to_radians().sin(),
                                                        );
                                                        view! {
                                                            <text
                                                                x={x.to_string()}
                                                                y={y.to_string()}
                                                                text-anchor="middle"
                                                                style="font-size:7px;fill:#6b7280"
                                                            >{*label}</text>
                                                        }
                                                    }).collect_view()}
                                                </svg>
                                                <p class="muted" style="text-align:center">
                                                    "實線 = 你的實測;虛線 = 命盤象徵傾向"
                                                </p>
                                            }.into_any()
                                        } else {
                                            view! { <div/> }.into_any()
                                        };
                                        view! {
                                            {note_view}
                                            {radar}
                                            {v.dims.iter().map(|d| view! {
                                                <div style="padding:0.4rem 0;border-top:1px solid #f3f4f6">
                                                    <div style="display:flex;justify-content:space-between">
                                                        <span>{dim_label(&d.dim)}</span>
                                                        <span class="muted">
                                                            {format!(
                                                                "命盤象徵 {} · 你的實測 {}",
                                                                d.prior_band.as_deref().map(band_label).unwrap_or("—"),
                                                                band_label(&d.measured_band),
                                                            )}
                                                        </span>
                                                    </div>
                                                    <details>
                                                        <summary
                                                            class="muted"
                                                            style="font-size:0.75rem"
                                                        >"來源"</summary>
                                                        <span
                                                            class="muted"
                                                            style="font-size:0.75rem"
                                                        >
                                                            {format!(
                                                                "{};與人格測驗分數無關。",
                                                                basis_label(d.basis.as_deref()),
                                                            )}
                                                        </span>
                                                    </details>
                                                </div>
                                            }).collect_view()}
                                            {v.dims.iter().filter(|d| d.narrative_ok).map(|d| view! {
                                                <p style="line-height:1.9;margin-top:0.8rem">
                                                    {d.text.clone().unwrap_or_default()}
                                                </p>
                                            }).collect_view()}
                                        }.into_any()
                                        }}
                                    </Show>
                            </div>
                        </div>
                    }.into_any()
                }
                PersonalityState::NotScored { profile } => view! {
                    <div class="card">
                        <h2 style="margin-bottom:0.75rem">"本次無法計分"</h2>
                        <p class="muted">"你可以在準備好時重新作答。"</p>
                        <div style="display:flex;gap:0.75rem;flex-wrap:wrap;margin-top:1.5rem">
                            <button
                                class="btn-primary"
                                on:click=move |_| {
                                            reset_overlay();
                                            reset_quiz(state, answers, error, quiz_started_at);
                                        }
                            >
                                "重測"
                            </button>
                            {profile.map(|previous| view! {
                                <button
                                    class="btn-link"
                                    on:click=move |_| state.set(PersonalityState::Result(previous.clone()))
                                >
                                    "查看上一次結果"
                                </button>
                            })}
                        </div>
                    </div>
                }.into_any(),
            }}
        </div>
    }
}
