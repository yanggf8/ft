//! Profile page — port of `ProfilePage.tsx` into the shared `UserProfile` type.

use leptos::prelude::*;
use leptos::task::spawn_local;

use ft_schema::api::{
    CheckSituationRequest, DomainWire, FeedbackRequest, ListPredictionsResponse, ResponseWire,
    SituationWire, TriggerWire,
};
use std::collections::HashSet;
use wasm_bindgen::prelude::*;

use crate::auth::use_auth;
use crate::components::BirthDataForm;

#[component]
pub fn ProfilePage() -> impl IntoView {
    let auth = use_auth();
    let show_form = RwSignal::new(false);

    // If the user has no birth data yet, show the form once on first render.
    Effect::new(move |_| {
        let needs = !auth
            .user
            .get_untracked()
            .map(|u| u.hasBirthData)
            .unwrap_or(true);
        if needs && !show_form.get_untracked() {
            show_form.set(true);
        }
    });

    let saved = Callback::new(move |_| {
        let auth = auth;
        show_form.set(false);
        spawn_local(async move {
            auth.refresh(true).await;
        });
    });

    view! {
        <div class="page-narrow">
            <AccountCard auth=auth />
            <BirthCard
                auth=auth
                show_form=show_form
                on_saved=saved
            />
            <PersonalityCard />
            <PredictionsCard />
        </div>
    }
}

#[component]
fn AccountCard(auth: crate::auth::AuthCtx) -> impl IntoView {
    view! {
        <div class="card">
            <h2 style="margin-bottom:1rem">"帳號資訊"</h2>
            <p><strong>"Email:"</strong> {move || auth.user.get().map(|u| u.email).unwrap_or_default()}</p>
            <p>
                <strong>"方案:"</strong>
                {move || {
                    match auth.user.get().map(|u| u.billing.tier.clone()) {
                        Some(t) if t == "free" => "免費".to_string(),
                        Some(t) => t,
                        None => "-".to_string(),
                    }
                }}
                {move || {
                    if auth.user.get().map(|u| u.billing.isTrialing).unwrap_or(false) {
                        " ✓ 試用中".to_string()
                    } else {
                        String::new()
                    }
                }}
            </p>
        </div>
    }
}

#[component]
fn BirthCard(
    auth: crate::auth::AuthCtx,
    show_form: RwSignal<bool>,
    on_saved: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="card">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:1rem">
                <h2>"出生資料"</h2>
                <Show
                    when=move || auth.user.get().map(|u| u.hasBirthData).unwrap_or(false) && !show_form.get()
                >
                    <button class="btn-link" on:click=move |_| show_form.set(true)>"編輯"</button>
                </Show>
            </div>

            <Show
                when=move || show_form.get()
                fallback=move || {
                    let has = auth.user.get().map(|u| u.hasBirthData).unwrap_or(false);
                    if has {
                        let text: String = auth.user.get().and_then(|u| u.birth_summary()).unwrap_or_default();
                        let gen = {
                            let tags = auth.user.get().and_then(|u| u.generation_tags.clone()).unwrap_or_default();
                            if tags.len() > 1 {
                                crate::generation::combined_generation_story(&tags)
                                    .map(|(t,d)| (t, d))
                            } else if tags.len() == 1 {
                                crate::generation::combined_generation_story(&tags)
                                    .map(|(t,d)| (t, d))
                            } else {
                                let year = auth.user.get().and_then(|u| u.birth_year).unwrap_or(0);
                                crate::generation::generation_story(year).map(|(t,d)| (t.to_string(), d.to_string()))
                            }
                        };
                        view! {
                            <div style="display:grid;gap:0.75rem">
                                <p style="color:var(--text);background:rgba(255,255,255,0.06);border:1px solid var(--glass-border);border-radius:8px;padding:0.75rem 1rem;font-weight:500">{text}</p>
                                {gen.map(|(title, desc)| view! {
                                    <div style="background:linear-gradient(135deg,rgba(167,139,250,0.12),rgba(244,114,182,0.10));border:1px solid rgba(167,139,250,0.25);border-radius:10px;padding:0.85rem 1rem">
                                        <div style="font-weight:700;font-size:0.9rem;color:var(--gen-title);margin-bottom:0.25rem">{title}</div>
                                        <p style="font-size:0.85rem;line-height:1.6;color:var(--silver-dim);margin:0">{desc}</p>
                                    </div>
                                }.into_any()).unwrap_or_else(|| view! { <span></span> }.into_any())}
                            </div>
                        }.into_any()
                    } else {
                        view! { <p class="muted">"請先填寫出生資料以開始算命"</p> }.into_any()
                    }
                }
            >
                <BirthDataForm
                    initial=auth.user.get_untracked()
                    on_saved=on_saved
                />
            </Show>
        </div>
    }
}

#[component]
fn PersonalityCard() -> impl IntoView {
    let data = RwSignal::new(None::<ft_schema::api::PersonalityMeResponse>);
    let loading = RwSignal::new(true);

    {
        let data = data;
        let loading = loading;
        leptos::task::spawn_local(async move {
            if let Ok(resp) = crate::api::get_personality(false).await {
                data.set(Some(resp));
            }
            loading.set(false);
        });
    }

    view! {
        <div class="card">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:1rem">
                <h2>"人格資料"</h2>
                <a href="/personality" class="btn-link" style="text-decoration:none">"前往測驗 →"</a>
            </div>
            <Show when=move || loading.get() fallback=move || {
                let resp = data.get();
                match resp.as_ref().and_then(|r| r.profile.as_ref()) {
                    Some(p) => {
                        let status = p.status.clone();
                        let ocean = p.oceanMeasured.clone();
                        view! {
                            <div style="display:grid;gap:0.5rem">
                                <p style="font-size:0.85rem;color:var(--silver-dim)">
                                    "狀態: " {status.clone()}
                                </p>
                                {ocean.map(|o| view! {
                                    <div style="display:grid;grid-template-columns:1fr 1fr;gap:0.4rem;font-size:0.85rem">
                                        <span>"外向: " {format!("{:.0}", o.extraversion)}</span>
                                        <span>"友善: " {format!("{:.0}", o.agreeableness)}</span>
                                        <span>"自律: " {format!("{:.0}", o.conscientiousness)}</span>
                                        <span>"情緒穩定: " {format!("{:.0}", o.emotionalStability)}</span>
                                        <span>"開放: " {format!("{:.0}", o.intellectImagination)}</span>
                                    </div>
                                }.into_any()).unwrap_or_else(|| view! { <p class="muted">"尚無量測數據"</p> }.into_any())}
                            </div>
                        }.into_any()
                    }
                    None => view! { <p class="muted">"尚未完成人格測驗，前往測驗可補充命格參考"</p> }.into_any(),
                }
            }>
                <p class="muted">"載入中..."</p>
            </Show>
        </div>
    }
}

// ── F5 本週預測卡 ──
// Spec: docs/superpowers/specs/2026-09-04-f5-web-predictions-ui-design.md §3
// 動線閘門（Grok UI 審 P0-1/P0-2）：Stage 2 綁 stage1_complete ∧ 已 refetch ∧ forecast.is_some()；
// generate 每 mount 一次（latch）；STALE_CYCLE → 清 latch 重跑。

#[derive(Clone)]
enum PState {
    Loading,
    Error(String),
    NoProfile,
    Empty,
    Ready(Box<ListPredictionsResponse>),
}

fn friendly(e: &crate::api::ApiErr) -> String {
    if e.is_code("RATE_LIMIT") {
        "動作太頻繁，請稍後再試".to_string()
    } else {
        format!("載入失敗：{e}")
    }
}

fn domain_label(d: DomainWire) -> &'static str {
    match d {
        DomainWire::Work => "工作",
        DomainWire::Money => "金錢",
        DomainWire::Love => "感情",
        DomainWire::Family => "家庭",
        DomainWire::Health => "健康",
    }
}

/// 初始載入動線：GET → 空且未 generate → POST 一次 → 再 GET。
/// `initing` 重入鎖：mount/重試/focus 並發時只跑一趟（Grok UI 二審 P2-3）。
async fn card_init(state: &RwSignal<PState>, latch: &RwSignal<bool>, initing: &RwSignal<bool>) {
    if initing.get_untracked() {
        return;
    }
    initing.set(true);
    card_init_inner(state, latch).await;
    initing.set(false);
}

async fn card_init_inner(state: &RwSignal<PState>, latch: &RwSignal<bool>) {
    state.set(PState::Loading);
    match crate::api::get_predictions(true).await {
        Ok(resp) if !resp.predictions.is_empty() => state.set(PState::Ready(Box::new(resp))),
        Ok(_) => {
            if latch.get_untracked() {
                state.set(PState::Empty);
                return;
            }
            latch.set(true);
            match crate::api::generate_predictions().await {
                Ok(_) => match crate::api::get_predictions(true).await {
                    Ok(resp) if !resp.predictions.is_empty() => {
                        state.set(PState::Ready(Box::new(resp)))
                    }
                    Ok(_) => state.set(PState::Empty),
                    Err(e) => state.set(PState::Error(friendly(&e))),
                },
                Err(e) if e.is_code("PROFILE_INCOMPLETE") => state.set(PState::NoProfile),
                Err(e) => state.set(PState::Error(friendly(&e))),
            }
        }
        Err(e) => state.set(PState::Error(friendly(&e))),
    }
}

/// 收齊後/同步用：重新 GET 全文，不觸發 generate。
async fn card_refresh(state: &RwSignal<PState>) {
    state.set(PState::Loading);
    match crate::api::get_predictions(true).await {
        Ok(resp) if !resp.predictions.is_empty() => state.set(PState::Ready(Box::new(resp))),
        Ok(_) => state.set(PState::Empty),
        Err(e) => state.set(PState::Error(friendly(&e))),
    }
}

/// F6 第 1 段提交：成功後若收齊 → refetch 全文；否則 patch local；鎖定類錯誤 → 同步。
async fn do_check(
    state: &RwSignal<PState>,
    pending: &RwSignal<Option<TriggerWire>>,
    latch: &RwSignal<bool>,
    initing: &RwSignal<bool>,
    notice: &RwSignal<Option<String>>,
    t: TriggerWire,
    s: SituationWire,
) {
    pending.set(Some(t));
    let body = CheckSituationRequest {
        cycleId: None,
        trigger: t,
        situation: s,
    };
    match crate::api::put_situation_check(&body).await {
        Ok(_) => {
            // P1-1（Grok 二審）：每次 PUT 成功都 refetch，伺服器為真相——
            // 並行最後兩題也不會有「本地收齊但 forecast 仍遮罩」的卡死分支。
            card_refresh(state).await;
            notice.set(None);
        }
        Err(e)
            if e.is_code("SITUATION_LOCKED")
                || e.is_code("FEEDBACK_EXISTS")
                || e.is_code("UNKNOWN_TRIGGER")
                || e.is_code("NOT_FOUND")
                || e.is_code("SITUATION_REQUIRED")
                || e.is_code("SITUATION_ABSENT") =>
        {
            card_refresh(state).await;
        }
        Err(e) if e.is_code("STALE_CYCLE") => {
            latch.set(false);
            card_init(state, latch, initing).await;
        }
        Err(e) => notice.set(Some(friendly(&e))),
    }
    pending.set(None);
}

/// F6 第 2 段提交：一次性；鎖定類錯誤 → 同步；換週 → 重跑初始動線。
async fn do_feedback(
    state: &RwSignal<PState>,
    pending: &RwSignal<Option<String>>,
    latch: &RwSignal<bool>,
    initing: &RwSignal<bool>,
    notice: &RwSignal<Option<String>>,
    id: String,
    r: ResponseWire,
) {
    pending.set(Some(id.clone()));
    let body = FeedbackRequest { response: r };
    match crate::api::post_prediction_feedback(&id, &body).await {
        Ok(fb) => {
            state.update(|st| {
                if let PState::Ready(x) = st {
                    x.feedback.push(fb);
                }
            });
            notice.set(None);
        }
        Err(e)
            if e.is_code("FEEDBACK_EXISTS")
                || e.is_code("SITUATION_LOCKED")
                || e.is_code("UNKNOWN_TRIGGER")
                || e.is_code("NOT_FOUND")
                || e.is_code("SITUATION_REQUIRED")
                || e.is_code("SITUATION_ABSENT") =>
        {
            card_refresh(state).await;
        }
        Err(e) if e.is_code("STALE_CYCLE") => {
            latch.set(false);
            card_init(state, latch, initing).await;
        }
        Err(e) => notice.set(Some(friendly(&e))),
    }
    pending.set(None);
}

#[component]
fn PredictionsCard() -> impl IntoView {
    let state = RwSignal::new(PState::Loading);
    let latch = RwSignal::new(false);
    let initing = RwSignal::new(false);
    let pending_check = RwSignal::new(None::<TriggerWire>);
    let pending_feedback = RwSignal::new(None::<String>);
    let notice = RwSignal::new(None::<String>);

    {
        let state = state;
        let latch = latch;
        let initing = initing;
        spawn_local(async move {
            card_init(&state, &latch, &initing).await;
        });
    }

    // P1-2（Grok 二審）：跨週一長駐 /profile — window focus 時重比 cycleId，
    // 變了就清 latch 重跑初始動線（STALE_CYCLE 對 checks 走不到，不能只靠它）。
    {
        let state = state;
        let latch = latch;
        let initing = initing;
        Effect::new(move |_| {
            if let Some(win) = web_sys::window() {
                let cb = Closure::<dyn FnMut()>::new(move || {
                    let state = state;
                    let latch = latch;
                    let initing = initing;
                    spawn_local(async move {
                        if let Ok(resp) = crate::api::get_predictions(true).await {
                            let changed = match state.get_untracked() {
                                PState::Ready(r) => r.cycleId != resp.cycleId,
                                _ => false,
                            };
                            if changed {
                                latch.set(false);
                                card_init(&state, &latch, &initing).await;
                            }
                        }
                    });
                });
                let _ = win.add_event_listener_with_callback("focus", cb.as_ref().unchecked_ref());
                cb.forget();
            }
        });
    }

    view! {
        <div class="card">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:1rem">
                <h2>"本週預測"</h2>
                {move || {
                    if let PState::Ready(r) = state.get() {
                        view! { <span style="font-size:0.8rem;color:var(--silver-dim)">{r.cycleId.clone()}</span> }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}
            </div>

            <Show when=move || notice.get().is_some()>
                <p class="error">{move || notice.get().clone().unwrap_or_default()}</p>
            </Show>

            <Show
                when=move || matches!(state.get(), PState::Ready(_))
                fallback=move || {
                    match state.get() {
                        PState::Loading => view! { <p class="muted">"載入中..."</p> }.into_any(),
                        PState::NoProfile => view! {
                            <p class="muted">"完成人格測驗後，這裡會產生每週可驗證的情境預測。"</p>
                            <a href="/personality" class="btn-link" style="text-decoration:none">"前往測驗 →"</a>
                        }.into_any(),
                        PState::Empty => view! {
                            <p class="muted">"本週沒有明顯傾向可寫成可驗證的預測。"</p>
                        }.into_any(),
                        PState::Error(msg) => {
                            let msg = msg;
                            let busy = move || initing.get();
                            view! {
                                <p class="error">{msg}</p>
                                <button
                                    class="btn-link"
                                    disabled=busy
                                    on:click=move |_| {
                                        latch.set(false);
                                        spawn_local({
                                            let state = state;
                                            let latch = latch;
                                            let initing = initing;
                                            async move {
                                                card_init(&state, &latch, &initing).await;
                                            }
                                        });
                                    }
                                >"重試"</button>
                            }.into_any()
                        }
                        PState::Ready(_) => view! { <span></span> }.into_any(),
                    }
                }
            >
                {move || {
                    let PState::Ready(resp) = state.get() else {
                        return view! { <span></span> }.into_any();
                    };
                    // 區域 clone：For 的 each/children 都是 move closure，不能共享 Box
                    let preds = resp.predictions.clone();
                    let checks = resp.checks.clone();
                    let fbs = resp.feedback.clone();

                    // ── 派生狀態（每次 state 變動重算）──
                    let mut seen = HashSet::new();
                    let mut stage1: Vec<(TriggerWire, bool)> = Vec::new();
                    let answered: HashSet<TriggerWire> =
                        checks.iter().map(|c| c.trigger).collect();
                    for p in &preds {
                        if seen.insert(p.trigger) {
                            stage1.push((p.trigger, answered.contains(&p.trigger)));
                        }
                    }
                    stage1.sort_by_key(|(t, _)| *t as u8);
                    let stage1_complete = stage1.iter().all(|(_, a)| *a);
                    let revealed = stage1_complete && preds.iter().all(|p| p.forecast.is_some());

                    if !stage1_complete {
                        let q_of = |t: TriggerWire| {
                            ft_schema::anchors::TriggerClass::from(t).question().to_string()
                        };
                        view! {
                            <p style="font-size:0.85rem;color:var(--silver-dim);margin-bottom:0.75rem">
                                {format!("本週有 {} 則可驗證預測 — 先回答情境問題", preds.len())}
                            </p>
                            <div style="display:grid;gap:0.75rem">
                                <For
                                    each=move || stage1.clone()
                                    key=|(t, _)| *t as u8
                                    children=move |(t, is_answered)| {
                                        let checks = checks.clone();
                                        let q = q_of(t);
                                        let curr = {
                                            let mut c = "沒有".to_string();
                                            for chk in &checks {
                                                if chk.trigger == t {
                                                    c = match chk.situation {
                                                        SituationWire::Absent => "沒有".to_string(),
                                                        SituationWire::Occurred => "有".to_string(),
                                                    };
                                                }
                                            }
                                            c
                                        };
                                        view! {
                                            <div style="border:1px solid var(--glass-border);border-radius:8px;padding:0.6rem 0.8rem;background:rgba(255,255,255,0.04)">
                                                <div style="font-size:0.9rem;font-weight:600;margin-bottom:0.4rem">{q.clone()}</div>
                                                {if is_answered {
                                                    view! {
                                                        <div style="font-size:0.8rem;color:var(--silver-dim);margin-bottom:0.35rem">
                                                            {format!("已答：{}（可改答）", curr)}
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                                <div style="display:flex;gap:0.6rem;align-items:center">
                                                    <button
                                                        class="btn-link"
                                                        disabled=move || pending_check.get() == Some(t)
                                                        on:click=move |_| {
                                                            spawn_local({
                                                                let state = state;
                                                                let pending_check = pending_check;
                                                                let latch = latch;
                                                                let initing = initing;
                                                                let notice = notice;
                                                                async move {
                                                                    do_check(&state, &pending_check, &latch, &initing, &notice, t, SituationWire::Absent).await;
                                                                }
                                                            });
                                                        }
                                                    >"沒有"</button>
                                                    <button
                                                        class="btn-link"
                                                        disabled=move || pending_check.get() == Some(t)
                                                        on:click=move |_| {
                                                            spawn_local({
                                                                let state = state;
                                                                let pending_check = pending_check;
                                                                let latch = latch;
                                                                let initing = initing;
                                                                let notice = notice;
                                                                async move {
                                                                    do_check(&state, &pending_check, &latch, &initing, &notice, t, SituationWire::Occurred).await;
                                                                }
                                                            });
                                                        }
                                                    >"有"</button>
                                                    {move || {
                                                        (pending_check.get() == Some(t)).then(|| {
                                                            view! { <span style="font-size:0.8rem;color:var(--silver-dim)">"送出中..."</span> }
                                                        })
                                                    }}
                                                </div>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        }.into_any()
                    } else if revealed {
                        view! {
                            <p style="font-size:0.85rem;color:var(--silver-dim);margin-bottom:0.75rem">
                                {format!("已收齊 {} 則預測 — 依你的反應回饋", preds.len())}
                            </p>
                            <div style="display:grid;gap:0.75rem">
                                <For
                                    each=move || preds.clone()
                                    key=|p| p.id.clone()
                                    children=move |p| {
                                        let checks = checks.clone();
                                        let fbs = fbs.clone();
                                        let pid = p.id.clone();
                                        let t = p.trigger;
                                        let tc = ft_schema::anchors::TriggerClass::from(t);
                                        let occurred = checks.iter().any(|c| {
                                            c.trigger == t && c.situation == SituationWire::Occurred
                                        });
                                        let absent = checks.iter().any(|c| {
                                            c.trigger == t && c.situation == SituationWire::Absent
                                        });
                                        let fb_sent = fbs.iter().any(|f| f.predictionId == pid);
                                        let pid_d1 = pid.clone();
                                        let pid_c1 = pid.clone();
                                        let pid_d2 = pid.clone();
                                        let pid_c2 = pid.clone();
                                        let pid_d3 = pid.clone();
                                        let pid_c3 = pid.clone();
                                        view! {
                                            <div style="border:1px solid var(--glass-border);border-radius:8px;padding:0.6rem 0.8rem;background:rgba(255,255,255,0.04)">
                                                <div style="font-size:0.85rem;font-weight:700;color:var(--gen-title);margin-bottom:0.2rem">
                                                    {format!("{} · {}", domain_label(p.domain), tc.label())}
                                                </div>
                                                <div style="font-size:0.8rem;color:var(--silver-dim);margin-bottom:0.5rem">
                                                    {tc.question().to_string()}
                                                </div>
                                                {if absent {
                                                    view! { <span></span> }.into_any()
                                                } else {
                                                    view! {
                                                        <div>
                                                            {p.tendency.clone().map(|x| {
                                                                view! { <p style="font-size:0.9rem;margin:0.2rem 0">{x}</p> }.into_any()
                                                            }).unwrap_or_else(|| view! { <span></span> }.into_any())}
                                                            {p.forecast.clone().map(|x| {
                                                                view! { <p style="font-size:0.9rem;margin:0.2rem 0;color:var(--starlight)">{x}</p> }.into_any()
                                                            }).unwrap_or_else(|| view! { <span></span> }.into_any())}
                                                        </div>
                                                    }.into_any()
                                                }}
                                                {if absent {
                                                    view! { <p style="font-size:0.8rem;color:var(--silver-dim)">"情境未發生（不計入）"</p> }.into_any()
                                                } else if fb_sent {
                                                    view! { <p style="font-size:0.8rem;color:var(--silver-dim)">"已回饋"</p> }.into_any()
                                                } else if occurred && p.forecast.is_some() {
                                                    view! {
                                                        <div style="display:flex;gap:0.6rem;flex-wrap:wrap;align-items:center;margin-top:0.4rem">
                                                            <span style="font-size:0.8rem;color:var(--silver-dim)">"你的反應比較接近哪一邊？"</span>
                                                            <button
                                                                class="btn-link"
                                                                disabled=move || pending_feedback.get() == Some(pid_d1.clone())
                                                                on:click=move |_| {
                                                                    spawn_local({
                                                                        let state = state;
                                                                        let pending_feedback = pending_feedback;
                                                                        let latch = latch;
                                                                        let pid = pid_c1.clone();
                                                                        let initing = initing;
                                                                        let notice = notice;
                                                                        async move {
                                                                            do_feedback(&state, &pending_feedback, &latch, &initing, &notice, pid, ResponseWire::Hit).await;
                                                                        }
                                                                    });
                                                                }
                                                            >"接近預測的描述"</button>
                                                            <button
                                                                class="btn-link"
                                                                disabled=move || pending_feedback.get() == Some(pid_d2.clone())
                                                                on:click=move |_| {
                                                                    spawn_local({
                                                                        let state = state;
                                                                        let pending_feedback = pending_feedback;
                                                                        let latch = latch;
                                                                        let pid = pid_c2.clone();
                                                                        let initing = initing;
                                                                        let notice = notice;
                                                                        async move {
                                                                            do_feedback(&state, &pending_feedback, &latch, &initing, &notice, pid, ResponseWire::Miss).await;
                                                                        }
                                                                    });
                                                                }
                                                            >"接近相反的那一邊"</button>
                                                            <button
                                                                class="btn-link"
                                                                disabled=move || pending_feedback.get() == Some(pid_d3.clone())
                                                                on:click=move |_| {
                                                                    spawn_local({
                                                                        let state = state;
                                                                        let pending_feedback = pending_feedback;
                                                                        let latch = latch;
                                                                        let pid = pid_c3.clone();
                                                                        let initing = initing;
                                                                        let notice = notice;
                                                                        async move {
                                                                            do_feedback(&state, &pending_feedback, &latch, &initing, &notice, pid, ResponseWire::Other).await;
                                                                        }
                                                                    });
                                                                }
                                                            >"兩者都不太像"</button>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div>
                                <p class="muted">"載入全文..."</p>
                                <button class="btn-link" on:click=move |_| {
                                    spawn_local({
                                        let state = state;
                                        async move {
                                            card_refresh(&state).await;
                                        }
                                    });
                                }>"重試"</button>
                            </div>
                        }.into_any()
                    }
                }}
            </Show>
        </div>
    }
}
