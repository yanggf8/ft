//! Voluntary weekly practice, independent of personality and predictions.

use ft_schema::practice::{
    CreatePracticeAttemptRequest, CreatePracticePlanRequest, PracticeAttempt, PracticeOverview,
    PracticePlan, PracticePlanHistory, PracticeSummary, PRACTICE_CATALOG,
};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped_with_cancellation as spawn_local;

fn action_text(plan: &PracticePlan) -> &'static str {
    let Some(catalog) = ft_schema::practice::catalog_for_version(&plan.catalog_version) else {
        return "舊版練習說明暫時無法顯示";
    };
    catalog
        .iter()
        .find(|goal| goal.code == plan.goal_code)
        .and_then(|goal| {
            goal.actions
                .iter()
                .find(|action| action.code == plan.action_code)
        })
        .map(|action| action.instruction)
        .unwrap_or("這項練習的說明暫時無法顯示")
}

fn goal_text(plan: &PracticePlan) -> &'static str {
    let Some(catalog) = ft_schema::practice::catalog_for_version(&plan.catalog_version) else {
        return "過去選擇的目標";
    };
    catalog
        .iter()
        .find(|goal| goal.code == plan.goal_code)
        .map(|goal| goal.label)
        .unwrap_or("過去選擇的目標")
}

fn attempt_text(attempt: &PracticeAttempt) -> &'static str {
    if attempt.tried {
        match attempt.helpfulness.as_deref() {
            Some("helped") => "有嘗試・覺得有幫助",
            Some("unsure") => "有嘗試・還不確定",
            Some("not_helped") => "有嘗試・覺得沒有幫助",
            _ => "有嘗試",
        }
    } else {
        match attempt.obstacle.as_deref() {
            Some("no_chance") => "還沒嘗試・沒有合適機會",
            Some("forgot") => "還沒嘗試・忘記了",
            Some("too_big") => "還沒嘗試・步驟太大",
            _ => "還沒嘗試・其他原因",
        }
    }
}

fn request_id() -> Option<String> {
    web_sys::window()?
        .crypto()
        .ok()
        .map(|crypto| crypto.random_uuid())
}

fn refresh(
    overview: RwSignal<Option<PracticeOverview>>,
    error: RwSignal<String>,
    fetch_generation: RwSignal<u64>,
) {
    let generation = fetch_generation.get_untracked().wrapping_add(1);
    fetch_generation.set(generation);
    spawn_local(async move {
        match crate::api::get_practice().await {
            Ok(data) if fetch_generation.get_untracked() == generation => {
                overview.set(Some(data));
                error.set(String::new());
            }
            Err(_) if fetch_generation.get_untracked() == generation => {
                error.set("載入失敗，請重試".to_string())
            }
            _ => {}
        }
    });
}

fn is_stale(err: &crate::api::ApiErr) -> bool {
    err.status() == Some(409)
}

#[component]
pub fn PracticeCard() -> impl IntoView {
    let overview = RwSignal::new(None::<PracticeOverview>);
    let fetch_generation = RwSignal::new(0_u64);
    let loading = RwSignal::new(true);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let choosing = RwSignal::new(false);
    let goal = RwSignal::new(String::new());
    let action = RwSignal::new(String::new());
    let pending_plan = RwSignal::new(None::<CreatePracticePlanRequest>);
    let reporting = RwSignal::new(false);
    let tried = RwSignal::new(None::<bool>);
    let feedback = RwSignal::new(String::new());
    let pending_attempt = RwSignal::new(None::<(String, CreatePracticeAttemptRequest)>);
    let show_history = RwSignal::new(false);
    let history = RwSignal::new(Vec::<PracticePlanHistory>::new());
    let history_cursor = RwSignal::new(None::<String>);
    let history_loaded = RwSignal::new(false);
    let selected_history = RwSignal::new(None::<PracticePlan>);
    let history_attempts = RwSignal::new(Vec::<PracticeAttempt>::new());
    let attempts_cursor = RwSignal::new(None::<String>);
    let deleting = RwSignal::new(false);

    spawn_local(async move {
        match crate::api::get_practice().await {
            Ok(data) if fetch_generation.get_untracked() == 0 => overview.set(Some(data)),
            Err(_) if fetch_generation.get_untracked() == 0 => {
                error.set("載入失敗，請重試".to_string())
            }
            _ => {}
        }
        loading.set(false);
    });

    let submit_plan = move |_| {
        if busy.get_untracked() {
            return;
        }
        let Some(state) = overview.get_untracked() else {
            return;
        };
        let selected_goal = goal.get_untracked();
        let selected_action = action.get_untracked();
        if !ft_schema::practice::valid_goal_action(&selected_goal, &selected_action) {
            error.set("請先選一個目標和行動".to_string());
            return;
        }
        let Some(req) = pending_plan
            .get_untracked()
            .filter(|req| {
                req.goal_code == selected_goal
                    && req.action_code == selected_action
                    && req.expected_epoch == state.practice_epoch
                    && req.replaces_plan_id == state.active_plan.as_ref().map(|p| p.id.clone())
            })
            .or_else(|| {
                Some(CreatePracticePlanRequest {
                    goal_code: selected_goal,
                    action_code: selected_action,
                    replace_active: state.active_plan.is_some(),
                    replaces_plan_id: state.active_plan.as_ref().map(|p| p.id.clone()),
                    expected_epoch: state.practice_epoch,
                    request_id: request_id()?,
                })
            })
        else {
            error.set("無法建立提交編號，請重新整理後再試".to_string());
            return;
        };
        pending_plan.set(Some(req.clone()));
        busy.set(true);
        error.set(String::new());
        spawn_local(async move {
            match crate::api::create_practice_plan(&req).await {
                Ok(_) => {
                    pending_plan.set(None);
                    choosing.set(false);
                    history_loaded.set(false);
                    show_history.set(false);
                    selected_history.set(None);
                    note.set("已更新練習。".to_string());
                    refresh(overview, error, fetch_generation);
                }
                Err(err) if is_stale(&err) => {
                    pending_plan.set(None);
                    note.set("練習已在其他分頁變更，資料已重新載入。".to_string());
                    refresh(overview, error, fetch_generation);
                }
                Err(_) => error.set("儲存失敗，請重試".to_string()),
            }
            busy.set(false);
        });
    };

    let submit_attempt = move |_| {
        if busy.get_untracked() {
            return;
        }
        let Some(plan) = overview.get_untracked().and_then(|v| v.active_plan) else {
            return;
        };
        let Some(did_try) = tried.get_untracked() else {
            error.set("請先選擇是否已嘗試".to_string());
            return;
        };
        let response = feedback.get_untracked();
        if response.is_empty() {
            error.set("請選擇這次的感受或原因".to_string());
            return;
        }
        let Some((_, req)) = pending_attempt
            .get_untracked()
            .filter(|(id, req)| {
                id == &plan.id
                    && req.tried == did_try
                    && (if did_try {
                        req.helpfulness.as_deref()
                    } else {
                        req.obstacle.as_deref()
                    }) == Some(response.as_str())
            })
            .or_else(|| {
                Some((
                    plan.id.clone(),
                    CreatePracticeAttemptRequest {
                        tried: did_try,
                        helpfulness: did_try.then_some(response),
                        obstacle: (!did_try).then_some(feedback.get_untracked()),
                        request_id: request_id()?,
                    },
                ))
            })
        else {
            error.set("無法建立提交編號，請重新整理後再試".to_string());
            return;
        };
        pending_attempt.set(Some((plan.id.clone(), req.clone())));
        busy.set(true);
        error.set(String::new());
        spawn_local(async move {
            match crate::api::create_practice_attempt(&plan.id, &req).await {
                Ok(_) => {
                    pending_attempt.set(None);
                    reporting.set(false);
                    tried.set(None);
                    feedback.set(String::new());
                    let next_step = match (did_try, req.helpfulness.as_deref()) {
                        (true, Some("not_helped"))
                            if can_shrink(&plan.goal_code, &plan.action_code) =>
                        {
                            "這個做法對你可能不合適。下次可以保留、縮小步驟或換一種方法。"
                        }
                        (true, Some("not_helped")) => {
                            "這個做法對你可能不合適。下次可以保留或換一種方法。"
                        }
                        (true, _) => "記下來了。下次可以沿用或換個做法。",
                        (false, _) => "沒關係。你可以等下次機會或換個做法。",
                    };
                    note.set(next_step.to_string());
                    refresh(overview, error, fetch_generation);
                }
                Err(err) if is_stale(&err) => {
                    pending_attempt.set(None);
                    note.set("練習已在其他分頁變更，資料已重新載入。".to_string());
                    refresh(overview, error, fetch_generation);
                }
                Err(_) => error.set("儲存失敗，請重試".to_string()),
            }
            busy.set(false);
        });
    };

    let end_plan = move |_| {
        if busy.get_untracked() {
            return;
        }
        let Some(plan) = overview.get_untracked().and_then(|v| v.active_plan) else {
            return;
        };
        busy.set(true);
        error.set(String::new());
        spawn_local(async move {
            match crate::api::end_practice_plan(&plan.id).await {
                Ok(_) => {
                    choosing.set(false);
                    history_loaded.set(false);
                    show_history.set(false);
                    selected_history.set(None);
                    note.set("這項練習已結束，紀錄仍可回顧。".to_string());
                    refresh(overview, error, fetch_generation);
                }
                Err(err) if is_stale(&err) => {
                    note.set("練習已在其他分頁變更，資料已重新載入。".to_string());
                    refresh(overview, error, fetch_generation);
                }
                Err(_) => error.set("儲存失敗，請重試".to_string()),
            }
            busy.set(false);
        });
    };

    let load_history = move |append: bool| {
        if busy.get_untracked() {
            return;
        }
        let cursor = if append {
            history_cursor.get_untracked()
        } else {
            None
        };
        busy.set(true);
        spawn_local(async move {
            match crate::api::get_practice_history(cursor.as_deref()).await {
                Ok(page) => {
                    if append {
                        history.update(|items| items.extend(page.plans));
                    } else {
                        history.set(page.plans);
                    }
                    history_cursor.set(page.next_cursor);
                    history_loaded.set(true);
                    show_history.set(true);
                }
                Err(_) => error.set("歷史紀錄載入失敗，請重試".to_string()),
            }
            busy.set(false);
        });
    };

    let load_attempts = move |plan: PracticePlan, append: bool| {
        if busy.get_untracked() {
            return;
        }
        let cursor = if append {
            attempts_cursor.get_untracked()
        } else {
            None
        };
        busy.set(true);
        spawn_local(async move {
            match crate::api::get_practice_attempts(&plan.id, cursor.as_deref()).await {
                Ok(page) => {
                    if append {
                        history_attempts.update(|items| items.extend(page.attempts));
                    } else {
                        history_attempts.set(page.attempts);
                    }
                    attempts_cursor.set(page.next_cursor);
                    selected_history.set(Some(plan));
                }
                Err(_) => error.set("回報紀錄載入失敗，請重試".to_string()),
            }
            busy.set(false);
        });
    };

    view! {
        <section class="card practice-card" aria-labelledby="practice-title">
            <h2 id="practice-title">"本週小練習"</h2>
            <p class="muted practice-intro">"選一件小事試試看。紀錄只供你回顧，儲存在本站帳號，可隨時刪除。"</p>
            <Show when=move || !error.get().is_empty() fallback=|| ()>
                <p class="practice-message practice-error" role="alert">{move || error.get()}</p>
                <button class="btn-link" type="button" on:click=move |_| refresh(overview, error, fetch_generation)>"重新載入"</button>
            </Show>
            <Show when=move || !note.get().is_empty() fallback=|| ()>
                <p class="practice-message" role="status">{move || note.get()}</p>
            </Show>
            <Show when=move || loading.get() fallback=|| ()>
                <p class="muted">"載入中…"</p>
            </Show>
            <Show when=move || overview.get().is_some() fallback=|| ()>
                <Show when=move || overview.get().and_then(|s| s.active_plan).is_some() fallback=|| ()>
                    <div class="practice-active">
                        <p class="practice-kicker">"目前在試"</p>
                        <h3>{move || overview.get().and_then(|s| s.active_plan).map(|p| goal_text(&p).to_string()).unwrap_or_default()}</h3>
                        <p>{move || overview.get().and_then(|s| s.active_plan).map(|p| action_text(&p).to_string()).unwrap_or_default()}</p>
                        <div class="practice-actions">
                            <button type="button" class="btn-primary" disabled=move || busy.get() on:click=move |_| {
                                reporting.set(true); choosing.set(false); note.set(String::new());
                            }>"記一次嘗試"</button>
                            <button type="button" class="btn-link" disabled=move || busy.get() on:click=move |_| {
                                if let Some(plan) = overview.get_untracked().and_then(|s| s.active_plan) {
                                    goal.set(plan.goal_code); action.set(plan.action_code);
                                    choosing.set(true); reporting.set(false);
                                }
                            }>"換個做法"</button>
                            <button type="button" class="btn-link" disabled=move || busy.get() on:click=end_plan>"結束練習"</button>
                        </div>
                        <Show when=move || overview.get().is_some_and(|s| !s.recent_attempts.is_empty()) fallback=|| ()>
                            <details class="practice-recent">
                                <summary>"最近的回報"</summary>
                                <ul class="practice-attempt-list">
                                    <For each=move || overview.get().map(|s| s.recent_attempts).unwrap_or_default() key=|item| item.id.clone() children=move |item| {
                                        view! { <li>{item.created_at.clone()} "　" {attempt_text(&item)}</li> }
                                    } />
                                </ul>
                            </details>
                        </Show>
                        <button type="button" class="btn-link" disabled=move || busy.get() on:click=move |_| {
                            if let Some(plan) = overview.get_untracked().and_then(|s| s.active_plan) {
                                show_history.set(true);
                                load_attempts(plan, false);
                            }
                        }>"查看全部回報"</button>
                    </div>
                </Show>
                <Show when=move || overview.get().and_then(|s| s.active_plan).is_none() fallback=|| ()>
                    <p>"這週想試哪一件小事？選一個就好。"</p>
                    <Show when=move || !choosing.get() fallback=|| ()>
                        <button type="button" class="btn-primary" on:click=move |_| choosing.set(true)>"選一件小事"</button>
                    </Show>
                </Show>

                <Show when=move || choosing.get() fallback=|| ()>
                    <div class="practice-form">
                        <fieldset>
                            <legend>"想練習哪件事？"</legend>
                            {PRACTICE_CATALOG.iter().map(|item| {
                                let code = item.code.to_string();
                                let checked_code = code.clone();
                                view! {
                                    <label class="practice-option">
                                        <input type="radio" name="practice-goal" checked=move || goal.get() == checked_code on:change=move |_| {
                                            goal.set(code.clone()); action.set(String::new()); pending_plan.set(None);
                                        } />
                                        <span>{item.label}</span>
                                    </label>
                                }
                            }).collect_view()}
                        </fieldset>
                        <Show when=move || !goal.get().is_empty() fallback=|| ()>
                            <fieldset>
                                <legend>"選一個行動"</legend>
                                {move || PRACTICE_CATALOG.iter().find(|item| item.code == goal.get()).map(|item| item.actions.iter().map(|choice| {
                                    let code = choice.code.to_string();
                                    let checked_code = code.clone();
                                    view! {
                                        <label class="practice-option">
                                            <input type="radio" name="practice-action" checked=move || action.get() == checked_code on:change=move |_| { action.set(code.clone()); pending_plan.set(None); } />
                                            <span><strong>{choice.label}</strong><small>{choice.instruction}</small></span>
                                        </label>
                                    }
                                }).collect_view())}
                            </fieldset>
                        </Show>
                        <div class="practice-actions">
                            <button type="button" class="btn-primary" disabled=move || busy.get() || action.get().is_empty() on:click=submit_plan>
                                {move || if busy.get() { "儲存中…" } else if overview.get().and_then(|s| s.active_plan).is_some() { "換成這項練習" } else { "開始這項練習" }}
                            </button>
                            <button type="button" class="btn-link" on:click=move |_| choosing.set(false)>"取消"</button>
                        </div>
                    </div>
                </Show>

                <Show when=move || reporting.get() && overview.get().and_then(|s| s.active_plan).is_some() fallback=|| ()>
                    <div class="practice-form">
                        <fieldset>
                            <legend>"這次有嘗試嗎？"</legend>
                            <label class="practice-option"><input type="radio" name="practice-tried" checked=move || tried.get() == Some(true) on:change=move |_| { tried.set(Some(true)); feedback.set(String::new()); pending_attempt.set(None); } />"有嘗試"</label>
                            <label class="practice-option"><input type="radio" name="practice-tried" checked=move || tried.get() == Some(false) on:change=move |_| { tried.set(Some(false)); feedback.set(String::new()); pending_attempt.set(None); } />"還沒嘗試"</label>
                        </fieldset>
                        <Show when=move || tried.get().is_some() fallback=|| ()>
                            <fieldset>
                                <legend>{move || if tried.get() == Some(true) { "這次覺得有幫助嗎？" } else { "這次沒嘗試的主要原因？" }}</legend>
                                {move || {
                                    let choices: &[(&str, &str)] = if tried.get() == Some(true) {
                                        &[("helped", "有幫助"), ("unsure", "還不確定"), ("not_helped", "沒有幫助")]
                                    } else {
                                        &[("no_chance", "沒有合適機會"), ("forgot", "忘記了"), ("too_big", "步驟太大"), ("other", "其他原因")]
                                    };
                                    choices.iter().map(|(code, label)| {
                                        let code = (*code).to_string();
                                        let checked_code = code.clone();
                                        view! { <label class="practice-option"><input type="radio" name="practice-feedback" checked=move || feedback.get() == checked_code on:change=move |_| { feedback.set(code.clone()); pending_attempt.set(None); } />{*label}</label> }
                                    }).collect_view()
                                }}
                            </fieldset>
                        </Show>
                        <div class="practice-actions">
                            <button type="button" class="btn-primary" disabled=move || busy.get() || feedback.get().is_empty() on:click=submit_attempt>"儲存這次紀錄"</button>
                            <button type="button" class="btn-link" on:click=move |_| reporting.set(false)>"取消"</button>
                        </div>
                    </div>
                </Show>

                <Show when=move || overview.get().is_some_and(|s| s.active_plan.is_some() || s.current_week.attempted > 0 || s.current_week.not_tried > 0 || s.previous_week.attempted > 0 || s.previous_week.not_tried > 0) fallback=|| ()>
                <div class="practice-summary" aria-live="polite">
                    <h3>"這週回顧"</h3>
                    <p>{move || overview.get().map(|s| summary_line(&s.current_week)).unwrap_or_default()}</p>
                    <p class="muted">{move || overview.get().map(|s| format!("尚未嘗試的紀錄：{} 次", s.current_week.not_tried)).unwrap_or_default()}</p>
                    <Show when=move || overview.get().is_some_and(|s| s.previous_week.attempted > 0 || s.previous_week.not_tried > 0) fallback=|| ()>
                        <p class="muted">{move || overview.get().map(|s| format!("上週：{} 次嘗試、{} 次覺得有幫助。", s.previous_week.attempted, s.previous_week.helped)).unwrap_or_default()}</p>
                    </Show>
                    <Show when=move || overview.get().and_then(|s| s.active_plan).is_some_and(|p| can_shrink(&p.goal_code, &p.action_code)) fallback=|| ()>
                        <button type="button" class="btn-link" on:click=move |_| {
                            if let Some(plan) = overview.get_untracked().and_then(|s| s.active_plan) {
                                if let Some(smaller) = ft_schema::practice::smaller_action(&plan.goal_code, &plan.action_code) {
                                    goal.set(plan.goal_code); action.set(smaller.to_string()); choosing.set(true); reporting.set(false);
                                }
                            }
                        }>"縮小步驟"</button>
                    </Show>
                </div>
                </Show>

                <Show when=move || overview.get().is_some_and(|s| s.total_plans > 0 || s.total_attempts > 0) fallback=|| ()>
                <div class="practice-footer">
                    <button type="button" class="btn-link" on:click=move |_| {
                        if show_history.get_untracked() { show_history.set(false); }
                        else if history_loaded.get_untracked() { show_history.set(true); }
                        else { load_history(false); }
                    }>"查看過去練習"</button>
                    <button type="button" class="btn-link practice-delete" disabled=move || deleting.get() || busy.get() on:click=move |_| {
                        let message = overview.get_untracked().map(|state| format!(
                            "確定刪除全部 {} 項小練習計畫和 {} 筆回報紀錄？這無法復原；人格和預測資料不會一併刪除，興趣探索紀錄需在該卡片另行刪除。",
                            state.total_plans, state.total_attempts
                        )).unwrap_or_default();
                        let confirmed = web_sys::window().and_then(|w| w.confirm_with_message(&message).ok()).unwrap_or(false);
                        if !confirmed { return; }
                        fetch_generation.update(|generation| *generation = generation.wrapping_add(1));
                        deleting.set(true); error.set(String::new());
                        spawn_local(async move {
                            match crate::api::delete_practice().await {
                                Ok(_) => {
                                    history.set(Vec::new()); history_cursor.set(None); history_loaded.set(false);
                                    history_attempts.set(Vec::new()); selected_history.set(None); show_history.set(false);
                                    pending_plan.set(None); pending_attempt.set(None); choosing.set(false); reporting.set(false);
                                    note.set("小練習紀錄已刪除。".to_string());
                                    refresh(overview, error, fetch_generation);
                                }
                                Err(_) => error.set("刪除失敗，請重試".to_string()),
                            }
                            deleting.set(false);
                        });
                    }>"刪除全部小練習紀錄"</button>
                </div>
                </Show>
                <Show when=move || show_history.get() fallback=|| ()>
                    <div class="practice-history">
                        <h3>"練習紀錄"</h3>
                        <For each=move || history.get() key=|item| item.plan.id.clone() children=move |item| {
                            let plan = item.plan.clone();
                            view! {
                                <div class="practice-history-row">
                                    <strong>{goal_text(&plan)}</strong>
                                    <span class="muted">{format!("{} 次嘗試・{} 次覺得有幫助", item.attempted, item.helped)}</span>
                                    <button type="button" class="btn-link" on:click=move |_| load_attempts(plan.clone(), false)>"查看回報"</button>
                                </div>
                            }
                        } />
                        <Show when=move || history_cursor.get().is_some() fallback=|| ()>
                            <button type="button" class="btn-link" disabled=move || busy.get() on:click=move |_| load_history(true)>"載入更多練習"</button>
                        </Show>
                        <Show when=move || selected_history.get().is_some() fallback=|| ()>
                            <h4>{move || selected_history.get().map(|p| goal_text(&p).to_string()).unwrap_or_default()}</h4>
                            <p>{move || selected_history.get().map(|p| action_text(&p).to_string()).unwrap_or_default()}</p>
                            <ul class="practice-attempt-list">
                                <For each=move || history_attempts.get() key=|item| item.id.clone() children=move |item| {
                                    view! { <li>{item.created_at.clone()} "　" {attempt_text(&item)}</li> }
                                } />
                            </ul>
                            <Show when=move || attempts_cursor.get().is_some() fallback=|| ()>
                                <button type="button" class="btn-link" disabled=move || busy.get() on:click=move |_| {
                                    if let Some(plan) = selected_history.get_untracked() { load_attempts(plan, true); }
                                }>"載入更多回報"</button>
                            </Show>
                        </Show>
                    </div>
                </Show>
            </Show>
        </section>
    }
}

fn summary_line(summary: &PracticeSummary) -> String {
    if summary.attempted == 0 {
        "本週尚無嘗試紀錄".to_string()
    } else {
        format!(
            "本週你記了 {} 次嘗試；其中 {} 次覺得有幫助。",
            summary.attempted, summary.helped
        )
    }
}

fn can_shrink(goal_code: &str, action_code: &str) -> bool {
    ft_schema::practice::smaller_action(goal_code, action_code).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_attempts_and_keeps_missed_chances_separate() {
        let s = PracticeSummary {
            cycle_id: "2026-10-19".to_string(),
            attempted: 2,
            helped: 1,
            unsure: 1,
            not_helped: 0,
            not_tried: 3,
        };
        assert_eq!(
            summary_line(&s),
            "本週你記了 2 次嘗試；其中 1 次覺得有幫助。"
        );
        assert_eq!(s.not_tried, 3);
    }

    #[test]
    fn empty_summary_does_not_judge_the_user() {
        let s = PracticeSummary {
            cycle_id: "2026-10-19".to_string(),
            attempted: 0,
            helped: 0,
            unsure: 0,
            not_helped: 0,
            not_tried: 1,
        };
        assert_eq!(summary_line(&s), "本週尚無嘗試紀錄");
    }

    #[test]
    fn shrink_is_offered_only_for_larger_steps() {
        assert!(can_shrink("express_need", "say_one_sentence"));
        assert!(!can_shrink("express_need", "write_one_sentence"));
    }

    #[test]
    fn archived_plan_labels_use_the_stored_catalog_version() {
        let mut plan = PracticePlan {
            id: "plan".to_string(),
            goal_code: "express_need".to_string(),
            action_code: "say_one_sentence".to_string(),
            catalog_version: "1".to_string(),
            created_at: "2026-10-20T00:00:00Z".to_string(),
            ended_at: None,
        };
        assert_eq!(goal_text(&plan), "說清楚自己的需求");
        assert_eq!(action_text(&plan), "下次討論時，說出一句自己想表達的需求。");
        plan.catalog_version = "unknown".to_string();
        assert_eq!(goal_text(&plan), "過去選擇的目標");
        assert_eq!(action_text(&plan), "舊版練習說明暫時無法顯示");
    }
}
