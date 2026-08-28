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
                        <div style="display:flex;justify-content:space-between;gap:1rem;align-items:start;margin-bottom:1rem">
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
                        <p style="margin-bottom:1.5rem">
                            "作答視為同意僅用於本站人格分析；原始作答僅儲存於本站，不對外提供。"
                        </p>
                        <Show when=move || suspected>
                            <p class="error">"這份作答與常見模式差異較大，結果可能不具參考性，請再試一次"</p>
                        </Show>

                        <div style="display:grid;gap:1.5rem">
                            {ITEMS.iter().enumerate().map(|(index, item)| {
                                view! {
                                    <fieldset style="border:0;border-bottom:1px solid #e5e7eb;padding:0 0 1.25rem">
                                        <legend style="font-weight:600;margin-bottom:0.75rem">
                                            {format!("{}. {}", item.no, item.text)}
                                        </legend>
                                        <div style="display:grid;grid-template-columns:repeat(5,minmax(0,1fr));gap:0.5rem">
                                            {SCALE_ANCHORS.iter().enumerate().map(|(anchor_index, anchor)| {
                                                let value = (anchor_index + 1) as u8;
                                                let input_name = format!("personality-item-{}", item.no);
                                                view! {
                                                    <label style="display:flex;flex-direction:column;align-items:center;gap:0.35rem;text-align:center;font-size:0.8rem">
                                                        <input
                                                            type="radio"
                                                            name=input_name
                                                            value=value
                                                            prop:checked=move || answers.get()[index] == Some(value)
                                                            prop:disabled=move || submitting.get()
                                                            on:change=move |_| answers.update(|all| all[index] = Some(value))
                                                        />
                                                        <span>{format!("{} {}", value, anchor)}</span>
                                                    </label>
                                                }
                                            }).collect_view()}
                                        </div>
                                    </fieldset>
                                }
                            }).collect_view()}
                        </div>

                        <button
                            class="btn-primary"
                            style="width:100%;margin-top:1.5rem"
                            prop:disabled=move || submitting.get() || answers.get().iter().any(Option::is_none)
                            on:click=submit_answers
                        >
                            {move || if submitting.get() { "送出中..." } else { "查看結果" }}
                        </button>
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
                                        on:click=move |_| reset_quiz(state, answers, error, quiz_started_at)
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
                                                Ok(_) => reset_quiz(state, answers, error, quiz_started_at),
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
                                on:click=move |_| reset_quiz(state, answers, error, quiz_started_at)
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
