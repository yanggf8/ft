//! Profile page — port of `ProfilePage.tsx` into the shared `UserProfile` type.

use leptos::prelude::*;
use leptos::task::spawn_local_scoped_with_cancellation as spawn_local;

use crate::auth::use_auth;
use crate::components::BirthDataForm;
use ft_schema::api::{
    CheckSituationRequest, DomainStrengths, DomainWire, FeedbackRequest,
    GeneratePredictionsRequest, ListPredictionsResponse, ResponseWire, SituationWire, TriggerWire,
};
use std::collections::HashSet;

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
        spawn_local(async move {
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
//       + 2026-09-11 F4 情境輸入切片
// 動線閘門（Grok UI 審 P0-1/P0-2）：Stage 2 綁 stage1_complete ∧ 已 refetch ∧ forecast.is_some()。
// F4（2026-09-11）：未生成週顯示五領域強度輸入，使用者按送出才 generate（不再自動生成）；
// 換週偵測改走 cycle_seen（任何狀態下都比對，NeedStrengths 中也會重置）。

#[derive(Clone)]
enum PState {
    Loading,
    Error(String),
    NoProfile,
    /// 本週尚未生成 → 顯示 F4 五領域強度輸入
    NeedStrengths,
    /// 已凍結但本週無預測；all_zero = 使用者全留 0（文案須與「無錨點命中」區分）
    Empty {
        all_zero: bool,
    },
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

/// 中性起點：先讓本週有基本感度，使用者仍可把不相關的領域拉回 0。
/// 1 = 略有感，比全 0 更像可直接微調的 EQ，而不是空白表單。
fn default_strengths() -> DomainStrengths {
    DomainStrengths {
        work: 1,
        love: 1,
        family: 1,
        money: 1,
        health: 1,
    }
}

fn is_all_zero(s: &DomainStrengths) -> bool {
    s.work == 0 && s.love == 0 && s.family == 0 && s.money == 0 && s.health == 0
}

/// F4 五領域 0–3 列（沿用人格測驗的 quiz-choice radio 體例；預設 1，可再調整）。
/// `get`/`set` 為欄位存取器 — Leptos view 無法動態索引結構體欄位。
fn strength_level(v: u8) -> &'static str {
    match v {
        0 => "無感",
        1 => "略有感",
        2 => "有感",
        _ => "很有感",
    }
}

/// F4 五領域 EQ 調整列。未生成時可拖曳；週期凍結後保留快照但鎖定，
/// 避免使用者誤以為改動會回寫已產生、可驗證的本週預測。
fn strength_row(
    label: &'static str,
    strengths: RwSignal<DomainStrengths>,
    pending_gen: RwSignal<bool>,
    state: RwSignal<PState>,
    get: fn(&DomainStrengths) -> u8,
    set: fn(&mut DomainStrengths, u8),
) -> impl IntoView {
    let aria_label = format!("{label}感知強度");
    view! {
        <label class="strength-slider-row">
            <span class="strength-slider-label">{label}</span>
            <input
                type="range"
                min="0"
                max="3"
                step="1"
                aria-label=aria_label
                prop:value=move || get(&strengths.get()).to_string()
                prop:disabled=move || {
                    pending_gen.get()
                        || !matches!(
                            state.get(),
                            PState::NeedStrengths | PState::Ready(_) | PState::Empty { .. }
                        )
                }
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse::<u8>() {
                        strengths.update(|s| set(s, v.min(3)));
                    }
                }
            />
            <output class="strength-slider-value">{move || strength_level(get(&strengths.get()))}</output>
        </label>
    }
}

fn strength_tuner(
    state: RwSignal<PState>,
    strengths: RwSignal<DomainStrengths>,
    pending_gen: RwSignal<bool>,
    notice: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <Show when=move || matches!(state.get(), PState::NeedStrengths | PState::Ready(_) | PState::Empty { .. })>
            <div class="prediction-tuner">
                <div class="prediction-tuner-head">
                    <div>
                        <strong>"本週感知調整器"</strong>
                        <span class="prediction-tuner-caption">"像 EQ 一樣，拖曳到最貼近你本週的程度"</span>
                    </div>
                    <span class="prediction-tuner-scale">"預設 1 略有感 · 0 無感 · 3 很有感"</span>
                </div>
                <div class="prediction-tuner-rows">
                    {strength_row("工作", strengths, pending_gen, state, |s| s.work, |s, v| s.work = v)}
                    {strength_row("感情", strengths, pending_gen, state, |s| s.love, |s, v| s.love = v)}
                    {strength_row("家庭", strengths, pending_gen, state, |s| s.family, |s, v| s.family = v)}
                    {strength_row("金錢", strengths, pending_gen, state, |s| s.money, |s, v| s.money = v)}
                    {strength_row("健康", strengths, pending_gen, state, |s| s.health, |s, v| s.health = v)}
                </div>
                <Show when=move || matches!(state.get(), PState::NeedStrengths | PState::Ready(_) | PState::Empty { .. })>
                    <p class="prediction-tuner-hint">
                        {move || match state.get() {
                            PState::Ready(_) => "想換一組角度可以再預測；既有預測與回饋會保留。",
                            PState::Empty { .. } => "空週仍可繼續調整；全部為 0 代表本週先不產生預測。",
                            _ => "調整好之後再產生本週預測；全部為 0 代表本週先不產生預測。",
                        }}
                    </p>
                    <button
                        class="btn-primary"
                        disabled=move || pending_gen.get()
                        on:click=move |_| {
                            spawn_local({
                                let state = state;
                                let strengths = strengths;
                                let pending_gen = pending_gen;
                                let notice = notice;
                                async move {
                                    do_generate(&state, &strengths, &pending_gen, &notice).await;
                                }
                            });
                        }
                    >{move || match state.get() {
                        PState::NeedStrengths => "產生本週預測",
                        _ => "再預測一次",
                    }}</button>
                </Show>
            </div>
        </Show>
    }
}

/// 初始載入動線：GET → 非空 Ready / 未生成 NeedStrengths / 凍結空 Empty。
/// `initing` 重入鎖：mount/重試/focus 並發時只跑一趟（Grok UI 二審 P2-3）。
async fn card_init(
    state: &RwSignal<PState>,
    initing: &RwSignal<bool>,
    strengths: &RwSignal<DomainStrengths>,
    cycle_seen: &RwSignal<Option<String>>,
) {
    if initing.get_untracked() {
        return;
    }
    initing.set(true);
    card_init_inner(state, strengths, cycle_seen).await;
    initing.set(false);
}

async fn card_init_inner(
    state: &RwSignal<PState>,
    strengths: &RwSignal<DomainStrengths>,
    cycle_seen: &RwSignal<Option<String>>,
) {
    state.set(PState::Loading);
    match crate::api::get_predictions(true).await {
        Ok(resp) => {
            // 換週偵測：跨週一長駐時任何狀態下 cycleId 變了就重置本週輸入
            let rollover = cycle_seen
                .get_untracked()
                .map(|c| c != resp.cycleId)
                .unwrap_or(false);
            cycle_seen.set(Some(resp.cycleId.clone()));
            if rollover {
                strengths.set(default_strengths());
            }
            if let Some(snapshot) = resp.strengths {
                strengths.set(snapshot);
            }
            if !resp.predictions.is_empty() {
                state.set(PState::Ready(Box::new(resp)));
            } else if !resp.generated {
                // 尚未生成 → F4 情境輸入（使用者送出才 generate）
                state.set(PState::NeedStrengths);
            } else {
                let all_zero = resp.strengths.map(|s| is_all_zero(&s)).unwrap_or(false);
                state.set(PState::Empty { all_zero });
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
        Ok(resp) => {
            let all_zero = resp.strengths.map(|s| is_all_zero(&s)).unwrap_or(false);
            state.set(PState::Empty { all_zero });
        }
        Err(e) => state.set(PState::Error(friendly(&e))),
    }
}

/// F4 情境輸入送出：generate（帶五領域強度）→ refetch（伺服器為真相）。
async fn do_generate(
    state: &RwSignal<PState>,
    strengths: &RwSignal<DomainStrengths>,
    pending_gen: &RwSignal<bool>,
    notice: &RwSignal<Option<String>>,
) {
    pending_gen.set(true);
    let body = GeneratePredictionsRequest {
        strengths: Some(strengths.get_untracked()),
    };
    match crate::api::generate_predictions(&body).await {
        Ok(_) => {
            // 全 0 → 凍結誠實空週 → Empty{all_zero}; 有命中 → Ready
            card_refresh(state).await;
            notice.set(None);
        }
        Err(e) if e.is_code("PROFILE_INCOMPLETE") => state.set(PState::NoProfile),
        Err(e) => notice.set(Some(friendly(&e))),
    }
    pending_gen.set(false);
}

/// F6 第 1 段提交：成功後若收齊 → refetch 全文；否則 patch local；鎖定類錯誤 → 同步。
async fn do_check(
    state: &RwSignal<PState>,
    pending: &RwSignal<Option<TriggerWire>>,
    initing: &RwSignal<bool>,
    strengths: &RwSignal<DomainStrengths>,
    cycle_seen: &RwSignal<Option<String>>,
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
            card_init(state, initing, strengths, cycle_seen).await;
        }
        Err(e) => notice.set(Some(friendly(&e))),
    }
    pending.set(None);
}

/// F6 第 2 段提交：一次性；鎖定類錯誤 → 同步；換週 → 重跑初始動線。
async fn do_feedback(
    state: &RwSignal<PState>,
    pending: &RwSignal<Option<String>>,
    initing: &RwSignal<bool>,
    strengths: &RwSignal<DomainStrengths>,
    cycle_seen: &RwSignal<Option<String>>,
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
            card_init(state, initing, strengths, cycle_seen).await;
        }
        Err(e) => notice.set(Some(friendly(&e))),
    }
    pending.set(None);
}

#[component]
fn PredictionsCard() -> impl IntoView {
    let state = RwSignal::new(PState::Loading);
    let initing = RwSignal::new(false);
    let pending_check = RwSignal::new(None::<TriggerWire>);
    let pending_feedback = RwSignal::new(None::<String>);
    let notice = RwSignal::new(None::<String>);
    let strengths = RwSignal::new(default_strengths());
    let pending_gen = RwSignal::new(false);
    let cycle_seen = RwSignal::new(None::<String>);

    {
        let state = state;
        let initing = initing;
        let strengths = strengths;
        let cycle_seen = cycle_seen;
        spawn_local(async move {
            card_init(&state, &initing, &strengths, &cycle_seen).await;
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

            {strength_tuner(state, strengths, pending_gen, notice)}

            <Show
                when=move || matches!(state.get(), PState::Ready(_))
                fallback=move || {
                    match state.get() {
                        PState::Loading => view! { <p class="muted">"載入中..."</p> }.into_any(),
                        PState::NoProfile => view! {
                            <p class="muted">"完成人格測驗後，這裡會產生每週可驗證的情境預測。"</p>
                            <a href="/personality" class="btn-link" style="text-decoration:none">"前往測驗 →"</a>
                        }.into_any(),
                        PState::NeedStrengths => {
                            view! {
                                <p class="muted" style="font-size:0.85rem">
                                    "先用上面的調整器標記這週的感知，再產生預測。"
                                </p>
                            }.into_any()
                        }
                        PState::Empty { all_zero } => {
                            if all_zero {
                                view! {
                                    <p class="muted">"本週你沒有標記有感的領域，所以沒有產生預測。若剛才填得太早或想修正，現在仍可重新調整。"</p>
                                }.into_any()
                            } else {
                                view! {
                                    <p class="muted">"本週沒有明顯傾向可寫成可驗證的預測；你仍可重新調整感知再試一次。"</p>
                                }.into_any()
                            }
                        }
                        PState::Error(msg) => {
                            let msg = msg;
                            let busy = move || initing.get();
                            view! {
                                <p class="error">{msg}</p>
                                <button
                                    class="btn-link"
                                    disabled=busy
                                    on:click=move |_| {
                                        spawn_local({
                                            let state = state;
                                            let initing = initing;
                                            let strengths = strengths;
                                            let cycle_seen = cycle_seen;
                                            async move {
                                                card_init(&state, &initing, &strengths, &cycle_seen).await;
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
                                                                let initing = initing;
                                                                let strengths = strengths;
                                                                let cycle_seen = cycle_seen;
                                                                let initing = initing;
                                                                let notice = notice;
                                                                async move {
                                                                    do_check(&state, &pending_check, &initing, &strengths, &cycle_seen, &notice, t, SituationWire::Absent).await;
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
                                                                let initing = initing;
                                                                let strengths = strengths;
                                                                let cycle_seen = cycle_seen;
                                                                let initing = initing;
                                                                let notice = notice;
                                                                async move {
                                                                    do_check(&state, &pending_check, &initing, &strengths, &cycle_seen, &notice, t, SituationWire::Occurred).await;
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
                        // F8 事後解盲(spec 2026-09-13-f8-control §2):回饋兩段收齊後,
                        // 週級聚合摘要(逐條指認已裁決降級,見 spec rev.2 對帳)。
                        let control_count = preds.iter().filter(|p| p.isControl).count();
                        let all_fb = !preds.is_empty()
                            && preds.iter().all(|p| fbs.iter().any(|f| f.predictionId == p.id));
                        view! {
                            <p style="font-size:0.85rem;color:var(--silver-dim);margin-bottom:0.75rem">
                                {format!("已收齊 {} 則預測 — 依你的反應回饋", preds.len())}
                            </p>
                            {if all_fb && control_count > 0 {
                                Some(view! {
                                    <p class="muted" style="font-size:0.8rem;margin-bottom:0.75rem">
                                        {format!("本週有 {control_count} 條為對照樣本（研究用）")}
                                    </p>
                                })
                            } else {
                                None
                            }}
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
                                                                        let initing = initing;
                                                                let strengths = strengths;
                                                                let cycle_seen = cycle_seen;
                                                                        let pid = pid_c1.clone();
                                                                        let initing = initing;
                                                                        let notice = notice;
                                                                        async move {
                                                                            do_feedback(&state, &pending_feedback, &initing, &strengths, &cycle_seen, &notice, pid, ResponseWire::Hit).await;
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
                                                                        let initing = initing;
                                                                let strengths = strengths;
                                                                let cycle_seen = cycle_seen;
                                                                        let pid = pid_c2.clone();
                                                                        let initing = initing;
                                                                        let notice = notice;
                                                                        async move {
                                                                            do_feedback(&state, &pending_feedback, &initing, &strengths, &cycle_seen, &notice, pid, ResponseWire::Miss).await;
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
                                                                        let initing = initing;
                                                                let strengths = strengths;
                                                                let cycle_seen = cycle_seen;
                                                                        let pid = pid_c3.clone();
                                                                        let initing = initing;
                                                                        let notice = notice;
                                                                        async move {
                                                                            do_feedback(&state, &pending_feedback, &initing, &strengths, &cycle_seen, &notice, pid, ResponseWire::Other).await;
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
