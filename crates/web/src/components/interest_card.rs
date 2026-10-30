//! 興趣探索 v1（RIASEC 興趣剖面）——spec docs/superpowers/specs/2026-10-27-interest-explore-design.md。
//! 作答只暫存在記憶體（不存草稿，§2.3）；分數由 API 重算後寫入，畫面只顯示伺服器回傳的分數。
//! 只呈現使用者自己六型之間的相對高低，文案集中在 `ft_schema::interest::copy`（禁用詞測試把關）。

use ft_schema::interest::copy::{type_copy, CARD_INTRO, NO_PREFERENCE, RESULT_FOOTNOTE, SOURCE};
use ft_schema::interest::{
    directions, display_order, CreateInterestResultRequest, InterestOverview, InterestResult,
    RiasecType, ANSWER_ANCHORS, FORM, INTEREST_INSTRUMENT_VERSION, ITEMS, ITEM_COUNT, PAGE_SIZE,
};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped_with_cancellation as spawn_local;

const PAGES: usize = ITEM_COUNT / PAGE_SIZE;

fn request_id() -> Option<String> {
    web_sys::window()?
        .crypto()
        .ok()
        .map(|crypto| crypto.random_uuid())
}

fn scroll_to(id: &str) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
    {
        el.scroll_into_view();
    }
}

fn date_of(created_at: &str) -> String {
    created_at
        .split_once('T')
        .map(|(d, _)| d.to_string())
        .unwrap_or_else(|| created_at.to_string())
}

/// 長條長度：分數 1.0–5.0 對應 0–100%。
fn bar_percent(score: f64) -> f64 {
    ((score - 1.0) / 4.0 * 100.0).clamp(0.0, 100.0)
}

fn refresh(
    overview: RwSignal<Option<InterestOverview>>,
    error: RwSignal<String>,
    fetch_generation: RwSignal<u64>,
) {
    let generation = fetch_generation.get_untracked().wrapping_add(1);
    fetch_generation.set(generation);
    spawn_local(async move {
        match crate::api::get_interest().await {
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

/// 六條長條，固定 R-I-A-S-E-C 順序（spec §2.6）。
fn bars(result: &InterestResult) -> impl IntoView {
    let scores = result.scores.to_array();
    view! {
        <div class="interest-bars">
            {RiasecType::ALL.iter().zip(scores).map(|(t, s)| {
                let name = type_copy(*t).name;
                view! {
                    <div class="interest-bar-row">
                        <span class="interest-bar-label">{name}</span>
                        <div class="interest-bar-track" role="img" aria-label=format!("{name} {s:.2}（1 到 5）")>
                            <span class="interest-bar-fill" style=format!("width:{:.1}%", bar_percent(s))></span>
                        </div>
                        <span class="interest-bar-score">{format!("{s:.2}")}</span>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

/// 較明顯的方向；最高分 < 3.0 時改顯示「沒有特別突出的偏好」（spec §2.6）。
fn directions_view(result: &InterestResult) -> impl IntoView {
    let picked = directions(&result.scores.to_array());
    if picked.is_empty() {
        view! { <p class="interest-directions">{NO_PREFERENCE}</p> }.into_any()
    } else {
        view! {
            <div class="interest-directions">
                <h3>"較明顯的方向"</h3>
                <ul>
                    {picked.into_iter().map(|t| {
                        let c = type_copy(t);
                        view! { <li><strong>{c.name}</strong>"："{c.description}</li> }
                    }).collect_view()}
                </ul>
            </div>
        }
        .into_any()
    }
}

fn direction_summary(result: &InterestResult) -> String {
    let picked = directions(&result.scores.to_array());
    if picked.is_empty() {
        "沒有特別突出的偏好".to_string()
    } else {
        let names: Vec<&str> = picked.into_iter().map(|t| type_copy(t).name).collect();
        format!("較明顯：{}", names.join("、"))
    }
}

#[component]
pub fn InterestCard() -> impl IntoView {
    let overview = RwSignal::new(None::<InterestOverview>);
    let fetch_generation = RwSignal::new(0_u64);
    let loading = RwSignal::new(true);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let note = RwSignal::new(String::new());
    let answering = RwSignal::new(false);
    let page = RwSignal::new(0_usize);
    let answers = RwSignal::new([None::<u8>; ITEM_COUNT]);
    let pending = RwSignal::new(None::<CreateInterestResultRequest>);
    let show_history = RwSignal::new(false);
    let history = RwSignal::new(Vec::<InterestResult>::new());
    let history_cursor = RwSignal::new(None::<String>);
    let history_loaded = RwSignal::new(false);
    let deleting = RwSignal::new(false);
    let order = display_order();
    // 首頁卡與人格結果頁連到 /profile#interest-title；SPA 導覽不會自動捲到錨點
    let hash = leptos_router::hooks::use_location().hash;

    spawn_local(async move {
        match crate::api::get_interest().await {
            Ok(data) if fetch_generation.get_untracked() == 0 => overview.set(Some(data)),
            Err(_) if fetch_generation.get_untracked() == 0 => {
                error.set("載入失敗，請重試".to_string())
            }
            _ => {}
        }
        loading.set(false);
        if hash.get_untracked() == "#interest-title" {
            scroll_to("interest-title");
        }
    });

    let start_quiz = move || {
        answers.set([None; ITEM_COUNT]);
        page.set(0);
        pending.set(None);
        note.set(String::new());
        error.set(String::new());
        show_history.set(false);
        answering.set(true);
        scroll_to("interest-title");
    };

    let cancel_quiz = move |_| {
        let answered = answers.get_untracked().iter().any(Option::is_some);
        if answered {
            let confirmed = web_sys::window()
                .and_then(|w| {
                    w.confirm_with_message("離開後這次的作答不會保留，確定離開？")
                        .ok()
                })
                .unwrap_or(false);
            if !confirmed {
                return;
            }
        }
        answering.set(false);
        pending.set(None);
    };

    let page_complete = move || {
        let p = page.get();
        let all = answers.get();
        order[p * PAGE_SIZE..(p + 1) * PAGE_SIZE]
            .iter()
            .all(|&i| all[i].is_some())
    };

    let go_page = move |next: usize| {
        page.set(next);
        scroll_to("interest-title");
    };

    let submit = move |_| {
        if busy.get_untracked() {
            return;
        }
        let Some(state) = overview.get_untracked() else {
            return;
        };
        let Some(values) = answers
            .get_untracked()
            .iter()
            .copied()
            .collect::<Option<Vec<u8>>>()
        else {
            error.set("還有題目沒作答".to_string());
            return;
        };
        // 重試沿用同一個 requestId（冪等）；作答或 epoch 變了才換新鍵
        let Some(req) = pending
            .get_untracked()
            .filter(|req| req.answers == values && req.expected_epoch == state.interest_epoch)
            .or_else(|| {
                Some(CreateInterestResultRequest {
                    instrument_version: INTEREST_INSTRUMENT_VERSION.to_string(),
                    form: FORM.to_string(),
                    answers: values,
                    expected_epoch: state.interest_epoch,
                    request_id: request_id()?,
                })
            })
        else {
            error.set("無法建立提交編號，請重新整理後再試".to_string());
            return;
        };
        pending.set(Some(req.clone()));
        busy.set(true);
        error.set(String::new());
        spawn_local(async move {
            match crate::api::create_interest_result(&req).await {
                Ok(result) => {
                    pending.set(None);
                    answering.set(false);
                    history_loaded.set(false);
                    show_history.set(false);
                    note.set(String::new());
                    overview.update(|o| {
                        if let Some(o) = o {
                            // 冪等重放回的是已計入的同一筆，不重複加筆數
                            if o.latest.as_ref().map(|l| &l.id) != Some(&result.id) {
                                o.total_results += 1;
                            }
                            o.latest = Some(result);
                        }
                    });
                    refresh(overview, error, fetch_generation);
                    scroll_to("interest-title");
                }
                Err(err) if err.is_code("STALE_INTEREST_STATE") => {
                    pending.set(None);
                    // 在同一個 task 內重新載入：拿到新 epoch 之前送出鈕保持停用（busy）
                    fetch_generation.update(|generation| *generation = generation.wrapping_add(1));
                    match crate::api::get_interest().await {
                        Ok(data) => {
                            overview.set(Some(data));
                            note.set(
                                "興趣探索紀錄已在其他分頁變更，資料已重新載入；你的作答還在，請再送出一次。"
                                    .to_string(),
                            );
                        }
                        Err(_) => error.set(
                            "興趣探索紀錄已在其他分頁變更，重新載入失敗，請重新整理頁面。"
                                .to_string(),
                        ),
                    }
                }
                Err(err) if err.is_code("STALE_INSTRUMENT") => {
                    pending.set(None);
                    error.set("題目已更新，請重新整理頁面後再作答。".to_string());
                }
                Err(err) if err.is_code("IDEMPOTENCY_CONFLICT") => {
                    pending.set(None);
                    error.set("送出失敗，請再試一次".to_string());
                }
                Err(_) => error.set("送出失敗，請重試".to_string()),
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
            match crate::api::get_interest_history(cursor.as_deref()).await {
                Ok(page) => {
                    if append {
                        history.update(|items| items.extend(page.results));
                    } else {
                        history.set(page.results);
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

    let delete_all = move |_| {
        let message = overview
            .get_untracked()
            .map(|state| {
                format!(
                    "確定刪除全部 {} 筆興趣探索紀錄（含原始作答）？這無法復原；人格、小練習和預測資料不會一併刪除。",
                    state.total_results
                )
            })
            .unwrap_or_default();
        let confirmed = web_sys::window()
            .and_then(|w| w.confirm_with_message(&message).ok())
            .unwrap_or(false);
        if !confirmed {
            return;
        }
        fetch_generation.update(|generation| *generation = generation.wrapping_add(1));
        deleting.set(true);
        error.set(String::new());
        spawn_local(async move {
            match crate::api::delete_interest().await {
                Ok(epoch) => {
                    // 立即清掉畫面上的舊結果；新 epoch 由回應帶回，refresh 失敗也不會用舊 epoch 送出
                    overview.set(Some(InterestOverview {
                        interest_epoch: epoch,
                        total_results: 0,
                        latest: None,
                    }));
                    history.set(Vec::new());
                    history_cursor.set(None);
                    history_loaded.set(false);
                    show_history.set(false);
                    pending.set(None);
                    answering.set(false);
                    note.set("興趣探索紀錄已刪除。".to_string());
                    refresh(overview, error, fetch_generation);
                }
                Err(_) => error.set("刪除失敗，請重試".to_string()),
            }
            deleting.set(false);
        });
    };

    view! {
        <section class="card interest-card" aria-labelledby="interest-title">
            <h2 id="interest-title">"興趣探索"</h2>
            <p class="muted practice-intro">{CARD_INTRO}</p>
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
                <Show when=move || answering.get() fallback=|| ()>
                    <div class="interest-quiz">
                        {move || {
                            let n = answers.get().iter().filter(|a| a.is_some()).count();
                            view! {
                                <p class="quiz-progress"><strong>{format!("{n}")}</strong>{format!(" / {ITEM_COUNT}　第 {} / {PAGES} 頁", page.get() + 1)}</p>
                                <div class="quiz-progress-bar" aria-hidden="true">
                                    <span style=format!("width:{:.0}%", n as f32 / ITEM_COUNT as f32 * 100.0)></span>
                                </div>
                            }
                        }}
                        <p class="muted interest-quiz-hint">"想像自己做這件事，不用考慮會不會做、薪水或需要的學歷。"</p>
                        <div class="quiz-key" aria-hidden="true">
                            {ANSWER_ANCHORS.iter().map(|anchor| view! { <span>{*anchor}</span> }).collect_view()}
                        </div>
                        {move || {
                            let p = page.get();
                            (p * PAGE_SIZE..(p + 1) * PAGE_SIZE).map(|k| {
                                let index = order[k];
                                let item = &ITEMS[index];
                                view! {
                                    <fieldset class="quiz-item">
                                        <legend>
                                            <span class="quiz-no">{k + 1}</span>
                                            {item.zh}
                                        </legend>
                                        <div class="quiz-choices">
                                            {ANSWER_ANCHORS.iter().enumerate().map(|(anchor_index, anchor)| {
                                                let value = (anchor_index + 1) as u8;
                                                view! {
                                                    <label class="quiz-choice">
                                                        <input
                                                            type="radio"
                                                            name=format!("interest-item-{}", item.no)
                                                            value=value
                                                            aria-label=format!("{value} {anchor}")
                                                            prop:checked=move || answers.get()[index] == Some(value)
                                                            prop:disabled=move || busy.get()
                                                            on:change=move |_| { answers.update(|all| all[index] = Some(value)); pending.set(None); }
                                                        />
                                                        <span class="quiz-choice-n" aria-hidden="true">{value}</span>
                                                    </label>
                                                }
                                            }).collect_view()}
                                        </div>
                                    </fieldset>
                                }
                            }).collect_view()
                        }}
                        <div class="practice-actions">
                            <button type="button" class="btn-link" disabled=move || page.get() == 0 || busy.get() on:click=move |_| go_page(page.get_untracked() - 1)>"上一頁"</button>
                            <Show when=move || page.get() + 1 < PAGES fallback=move || view! {
                                <button type="button" class="btn-primary" disabled=move || busy.get() || answers.get().iter().any(Option::is_none) on:click=submit>
                                    {move || if busy.get() { "送出中…" } else { "查看結果" }}
                                </button>
                            }>
                                <button type="button" class="btn-primary" disabled=move || !page_complete() on:click=move |_| go_page(page.get_untracked() + 1)>"下一頁"</button>
                            </Show>
                            <button type="button" class="btn-link" disabled=move || busy.get() on:click=cancel_quiz>"取消作答"</button>
                        </div>
                    </div>
                </Show>

                <Show when=move || !answering.get() fallback=|| ()>
                    {move || match overview.get().and_then(|s| s.latest) {
                        None => view! {
                            <button type="button" class="btn-primary" disabled=move || deleting.get() on:click=move |_| start_quiz()>"開始（約 5 分鐘，48 題）"</button>
                        }.into_any(),
                        Some(latest) => view! {
                            <div class="interest-result">
                                <p class="practice-kicker">{format!("作答日期：{}", date_of(&latest.created_at))}</p>
                                {bars(&latest)}
                                {directions_view(&latest)}
                                <details class="interest-reflect">
                                    <summary>"六型說明與反思提問"</summary>
                                    <ul>
                                        {RiasecType::ALL.iter().map(|t| {
                                            let c = type_copy(*t);
                                            view! { <li><strong>{c.name}</strong>"："{c.description}<br/><span class="muted">{c.reflection}</span></li> }
                                        }).collect_view()}
                                    </ul>
                                </details>
                                <p class="muted interest-foot">{RESULT_FOOTNOTE}</p>
                                <p class="muted interest-source">{SOURCE}</p>
                                <div class="practice-actions">
                                    <button type="button" class="btn-primary" on:click=move |_| scroll_to("practice-title")>"去本週小練習選一件小事"</button>
                                    <button type="button" class="btn-link" disabled=move || busy.get() || deleting.get() on:click=move |_| start_quiz()>"重新作答"</button>
                                    <button type="button" class="btn-link" disabled=move || busy.get() || deleting.get() on:click=move |_| {
                                        if show_history.get_untracked() { show_history.set(false); }
                                        else if history_loaded.get_untracked() { show_history.set(true); }
                                        else { load_history(false); }
                                    }>"查看過去結果"</button>
                                </div>
                            </div>
                        }.into_any(),
                    }}
                </Show>

                <Show when=move || show_history.get() && !answering.get() fallback=|| ()>
                    <div class="practice-history">
                        <h3>"過去結果"</h3>
                        <For each=move || history.get() key=|item| item.id.clone() children=move |item| {
                            view! {
                                <details class="practice-history-row">
                                    <summary>{format!("{}　{}", date_of(&item.created_at), direction_summary(&item))}</summary>
                                    {bars(&item)}
                                </details>
                            }
                        } />
                        <Show when=move || history_cursor.get().is_some() fallback=|| ()>
                            <button type="button" class="btn-link" disabled=move || busy.get() on:click=move |_| load_history(true)>"載入更多結果"</button>
                        </Show>
                    </div>
                </Show>

                <Show when=move || overview.get().is_some_and(|s| s.total_results > 0) && !answering.get() fallback=|| ()>
                    <div class="practice-footer">
                        <span class="muted">{move || overview.get().map(|s| format!("共 {} 筆紀錄", s.total_results)).unwrap_or_default()}</span>
                        <button type="button" class="btn-link practice-delete" disabled=move || deleting.get() || busy.get() on:click=delete_all>"刪除全部興趣探索紀錄"</button>
                    </div>
                </Show>
            </Show>
        </section>
    }
}
