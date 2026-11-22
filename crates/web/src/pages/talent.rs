//! 上手小實驗 `/try`（spec docs/superpowers/specs/2026-11-22-talent-tryout-design.md，§13 優先）。
//!
//! 同一頁以狀態切換（不開子路由）：未登入說明＋登入連結；登入後依序是進行中的實驗
//! （鎖定的猜測、15 分鐘倒數、對答案）、剛對完的結果卡、抽題／全部題目＋先猜、我的紀錄、
//! 「比你以為的順」與猜測傾向。
//!
//! 所有功能文案取自 `ft_schema::talent` 常數（禁字測試守門）；本檔只寫一般操作字
//! （按鈕、狀態名、錯誤提示）。抽題只吃「試過的領域」（`draw_three`），不看任何結果。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ft_schema::api::{TalentTrial, TalentTrialCreate, TalentTrialReport, TalentTrialsResponse};
use ft_schema::talent::{
    self, step_label, Domain, TendencyKind, TrialItem, CATALOG, CONDITION_NOTE, DELETE_NOTE,
    LOGIN_PROMPT, PAGE_INTRO, PAGE_TITLE, PREDICT_LOCK_NOTE, PREDICT_QUESTION, REPORT_QUESTION,
    SELF_NOTE, SKIP_LABEL, SMOOTHER_NOTE, SMOOTHER_TITLE, STEPS, STUCK_QUESTION, STUCK_REASONS,
    TENDENCY_FOOTNOTE, TENDENCY_TITLE, TIMER_NOTE,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::api::{self, ApiErr};
use crate::auth::use_auth;

/// 倒數長度（秒）：來自 schema `TIMER_MINUTES`（測試確保與文案一致）。純前端、不存。
const TIMER_SECS: i32 = ft_schema::talent::TIMER_MINUTES * 60;
/// 已有進行中實驗時，先猜區的說明（API 409 `OPEN_TRIAL_EXISTS` 同義）。
const OPEN_EXISTS_MSG: &str = "你還有一個進行中的小實驗，先對完答案或等它過期。";
/// 已下架題目（item_id 不在目前 CATALOG）的標題。
const RETIRED_TITLE: &str = "已下架的題目";
/// 兩段式刪除「全部」的上膛鍵。
const ARM_ALL: &str = "__all__";

// ═══ 小工具 ═══

/// `0..n` 的亂數（`draw_three` 的 pick）。
fn rand_index(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    ((js_sys::Math::random() * n as f64) as usize).min(n - 1)
}

/// 每次使用者動作一個 requestId（重試同一動作沿用）：`crypto.randomUUID()`（36 字，
/// 符合 8–64 `[A-Za-z0-9_-]`）；crypto 不可用時 32 hex（同 shop `new_anon_id`）。
fn new_request_id() -> String {
    if let Some(crypto) = web_sys::window().and_then(|w| w.crypto().ok()) {
        return crypto.random_uuid();
    }
    let hex = "0123456789abcdef".as_bytes();
    (0..32)
        .map(|_| hex[(js_sys::Math::random() * 16.0) as usize % 16] as char)
        .collect()
}

/// `created_at`（RFC3339 UTC）→ 瀏覽器本地時區的 `YYYY-MM-DD`（spec §13.1.4）。
fn local_date(iso: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return iso.split('T').next().unwrap_or(iso).to_string();
    }
    format!(
        "{}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

fn title_of(item_id: &str) -> &'static str {
    talent::find(item_id).map_or(RETIRED_TITLE, |i| i.title)
}

fn status_label(status: &str) -> &'static str {
    match status {
        "open" => "進行中",
        "reported" => "已對答案",
        "skipped" => "沒做",
        _ => "過期",
    }
}

/// 「你猜：X → 實際：Y」（只有已對答案才有實際）。
fn guess_line(t: &TalentTrial) -> String {
    match (t.status.as_str(), t.reached) {
        ("reported", Some(r)) => format!(
            "你猜：{} → 實際：{}",
            step_label(t.predicted),
            step_label(r)
        ),
        _ => format!("你猜：{}", step_label(t.predicted)),
    }
}

/// 抽題的「試過的領域」＝已對答案且題目仍在 CATALOG（spec §13.3.17）。
fn tried_domains(trials: &[TalentTrial]) -> Vec<Domain> {
    let mut v: Vec<Domain> = trials
        .iter()
        .filter(|t| t.status == "reported")
        .filter_map(|t| talent::find(&t.item_id).map(|i| i.domain))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// 30 位數字每 5 位空一格，好讀好背。
fn grouped_digits(trial_id: &str) -> String {
    talent::digits_for(trial_id)
        .as_bytes()
        .chunks(5)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn common_error(e: &ApiErr) -> Option<&'static str> {
    if e.status() == Some(429) || e.is_code("RATE_LIMITED") {
        Some("按得太頻繁了，請稍後再試。")
    } else if e.status() == Some(401) {
        Some("登入已過期，請重新登入。")
    } else if matches!(e, ApiErr::Network(_)) {
        Some("連線失敗，請檢查網路後再試。")
    } else {
        None
    }
}

fn load_error(e: &ApiErr) -> &'static str {
    if e.status() == Some(404) {
        // 這條路由 API 自己不回 404：404＝新 API 尚未部署
        "服務準備中，請稍後再來看看。"
    } else {
        common_error(e).unwrap_or("載入失敗，請稍後再試。")
    }
}

fn scroll_to(selector: &str) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(selector).ok().flatten())
    {
        el.scroll_into_view();
    }
}

// ═══ 頁面狀態 ═══

#[derive(Clone, Copy)]
struct TryCtx {
    data: RwSignal<Option<TalentTrialsResponse>>,
    load_err: RwSignal<Option<&'static str>>,
    /// 剛對完答案的那一筆（結果卡）
    result: RwSignal<Option<TalentTrial>>,
}

impl TryCtx {
    async fn load(self) {
        match api::talent_trials().await {
            Ok(r) => {
                self.load_err.try_set(None);
                self.data.try_set(Some(r));
            }
            Err(e) => {
                self.load_err.try_set(Some(load_error(&e)));
            }
        }
    }

    fn reload(self) {
        spawn_local(async move { self.load().await });
    }
}

// ═══ page ═══

#[component]
pub fn TalentPage() -> impl IntoView {
    let auth = use_auth();
    view! {
        <div class="page try-page">
            <nav class="zw-crumb" aria-label="麵包屑">
                <A href="/">"首頁"</A>
                " › "
                <span>{PAGE_TITLE}</span>
            </nav>
            <h1>{PAGE_TITLE}</h1>
            <p class="try-intro">{PAGE_INTRO}</p>
            <p class="try-self">{SELF_NOTE}</p>
            {move || {
                if auth.loading.get() {
                    view! { <div class="center-note">"Loading..."</div> }.into_any()
                } else if auth.is_authed() {
                    view! { <TryApp/> }.into_any()
                } else {
                    view! {
                        <div class="card try-login">
                            <p>{LOGIN_PROMPT}</p>
                            <a href="/login?next=/try" class="cta" data-testid="try-login">"登入"</a>
                        </div>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}

#[component]
fn TryApp() -> impl IntoView {
    let ctx = TryCtx {
        data: RwSignal::new(None),
        load_err: RwSignal::new(None),
        result: RwSignal::new(None),
    };
    ctx.reload();

    // 進行中＝status open；併發雙寫時以最舊的為準（列表新到舊 → 取最後一筆）。
    // Memo：只有那一筆變了才重繪，倒數與對答案表單不會被其他重載洗掉。
    let open = Memo::new(move |_| {
        ctx.data.with(|d| {
            d.as_ref()
                .and_then(|d| d.trials.iter().rev().find(|t| t.status == "open").cloned())
        })
    });
    let shown = RwSignal::new(Vec::<&'static TrialItem>::new());
    let selected = RwSignal::new(None::<&'static TrialItem>);

    let draw = move |_| {
        let tried = ctx.data.with_untracked(|d| {
            d.as_ref()
                .map(|d| tried_domains(&d.trials))
                .unwrap_or_default()
        });
        shown.set(talent::draw_three(&tried, rand_index));
        selected.set(None);
    };
    let show_all = move |_| {
        shown.set(CATALOG.iter().collect());
        selected.set(None);
    };

    view! {
        {move || ctx.load_err.get().map(|m| view! { <p class="error" role="alert">{m}</p> })}
        {move || open.get().map(|t| view! { <OpenTrial trial=t ctx=ctx/> })}
        {move || ctx.result.get().map(|t| view! { <ResultCard trial=t/> })}

        <section class="card try-pick">
            <div class="try-pick-actions">
                <button type="button" class="btn-primary try-draw" data-testid="try-draw" on:click=draw>
                    "抽三個"
                </button>
                <button type="button" class="cta-alt try-all" on:click=show_all>
                    "看全部題目"
                </button>
            </div>
            <div class="try-cards">
                {move || {
                    shown
                        .get()
                        .into_iter()
                        .map(|item| {
                            let on = move || selected.get().is_some_and(|s| s.id == item.id);
                            view! {
                                <button
                                    type="button"
                                    class="try-card"
                                    class:on=on
                                    aria-pressed=move || on().to_string()
                                    data-item=item.id
                                    on:click=move |_| selected.set(Some(item))
                                >
                                    <span class="try-card-domain">{item.domain.label()}</span>
                                    <span class="try-card-title">{item.title}</span>
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
            {move || {
                selected
                    .get()
                    .map(|item| {
                        view! {
                            <div class="try-detail">
                                <ItemDetail item=item/>
                                <p class="try-timer-note">{TIMER_NOTE}</p>
                                <PredictForm item=item ctx=ctx open=open selected=selected shown=shown/>
                            </div>
                        }
                    })
            }}
        </section>

        {move || ctx.data.get().map(|d| view! { <Records data=d ctx=ctx/> })}
    }
}

/// 題目內容：標題、領域、準備、三步。
#[component]
fn ItemDetail(item: &'static TrialItem) -> impl IntoView {
    view! {
        <h2 class="try-title">{item.title}</h2>
        <p class="try-domain">{item.domain.label()}</p>
        <p class="try-prep">{format!("準備：{}", item.prep)}</p>
        <ol class="try-steps">
            {item.steps.iter().map(|s| view! { <li>{*s}</li> }).collect_view()}
        </ol>
    }
}

/// 四選一步數鈕（`還沒到第 1 步`…`第 3 步`）。
#[component]
fn StepChoices(
    value: Signal<Option<u8>>,
    disabled: Signal<bool>,
    on_pick: Callback<u8>,
) -> impl IntoView {
    view! {
        <div class="try-steps-pick" role="group">
            {(0..=STEPS)
                .map(|n| {
                    let on = move || value.get() == Some(n);
                    view! {
                        <button
                            type="button"
                            class="try-step"
                            class:on=on
                            aria-pressed=move || on().to_string()
                            data-step=n.to_string()
                            disabled=move || disabled.get()
                            on:click=move |_| on_pick.run(n)
                        >
                            {step_label(n)}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn PredictForm(
    item: &'static TrialItem,
    ctx: TryCtx,
    open: Memo<Option<TalentTrial>>,
    selected: RwSignal<Option<&'static TrialItem>>,
    shown: RwSignal<Vec<&'static TrialItem>>,
) -> impl IntoView {
    let guess = RwSignal::new(None::<u8>);
    let rid = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let err = RwSignal::new(None::<&'static str>);
    let has_open = move || open.with(|o| o.is_some());
    let locked = Signal::derive(move || has_open() || busy.get());

    let submit = move |_| {
        let Some(predicted) = guess.get_untracked() else {
            return;
        };
        if locked.get_untracked() {
            return;
        }
        let request_id = rid.get_untracked().unwrap_or_else(new_request_id);
        rid.set(Some(request_id.clone()));
        busy.set(true);
        err.set(None);
        spawn_local(async move {
            let body = TalentTrialCreate {
                item_id: item.id.to_string(),
                predicted,
                request_id,
            };
            match api::talent_create(&body).await {
                Ok(_) => {
                    ctx.result.try_set(None);
                    shown.try_set(Vec::new());
                    selected.try_set(None);
                    ctx.load().await;
                    scroll_to(".try-open");
                }
                Err(e) => {
                    let msg = if e.is_code("OPEN_TRIAL_EXISTS") {
                        ctx.reload();
                        OPEN_EXISTS_MSG
                    } else {
                        common_error(&e).unwrap_or("送出失敗，請稍後再試。")
                    };
                    err.try_set(Some(msg));
                    busy.try_set(false);
                }
            }
        });
    };

    view! {
        <div class="try-predict" data-testid="try-predict">
            <h3 class="try-q">{PREDICT_QUESTION}</h3>
            <StepChoices
                value=guess.into()
                disabled=locked
                on_pick=Callback::new(move |n| {
                    guess.set(Some(n));
                    rid.set(None);
                })
            />
            <p class="try-note">{PREDICT_LOCK_NOTE}</p>
            <Show when=has_open>
                <p class="try-locked-note">{OPEN_EXISTS_MSG}</p>
            </Show>
            <button
                type="button"
                class="btn-primary try-submit"
                data-testid="try-predict-submit"
                disabled=move || guess.get().is_none() || locked.get()
                on:click=submit
            >
                "送出"
            </button>
            {move || err.get().map(|m| view! { <p class="error try-err" role="alert">{m}</p> })}
        </div>
    }
}

/// 進行中的那一筆：題目、鎖定的猜測、倒數、（digits）數字、對答案。
#[component]
fn OpenTrial(trial: TalentTrial, ctx: TryCtx) -> impl IntoView {
    let item = talent::find(&trial.item_id);
    let digits = item
        .filter(|i| i.id == "digits")
        .map(|_| grouped_digits(&trial.id));
    view! {
        <section class="card try-open" data-testid="try-open">
            <p class="try-badge">"進行中"</p>
            {match item {
                Some(item) => view! { <ItemDetail item=item/> }.into_any(),
                None => view! { <h2 class="try-title">{RETIRED_TITLE}</h2> }.into_any(),
            }}
            <p class="try-guess">{guess_line(&trial)}</p>
            {digits.map(|d| view! { <p class="try-digits" data-testid="try-digits">{d}</p> })}
            <p class="try-timer-note">{TIMER_NOTE}</p>
            <Countdown/>
            <ReportForm trial_id=trial.id.clone() ctx=ctx/>
        </section>
    }
}

/// 15 分鐘倒數：可不用；`setInterval`，離開頁面即停（不存）。
#[component]
fn Countdown() -> impl IntoView {
    // None＝未開始；Some(0)＝時間到
    let left = RwSignal::new(None::<i32>);
    let handle: Arc<Mutex<Option<IntervalHandle>>> = Arc::new(Mutex::new(None));
    let stop = {
        let handle = handle.clone();
        move || {
            if let Some(h) = handle.lock().ok().and_then(|mut g| g.take()) {
                h.clear();
            }
        }
    };
    on_cleanup({
        let stop = stop.clone();
        move || stop()
    });

    let toggle = move |_| {
        stop();
        if left.get_untracked().is_some_and(|s| s > 0) {
            left.set(None);
            return;
        }
        left.set(Some(TIMER_SECS));
        let tick_stop = stop.clone();
        let h = set_interval_with_handle(
            move || {
                let done = left
                    .try_update(|l| match l {
                        Some(s) if *s > 1 => {
                            *s -= 1;
                            false
                        }
                        _ => {
                            *l = Some(0);
                            true
                        }
                    })
                    .unwrap_or(true);
                if done {
                    tick_stop();
                }
            },
            Duration::from_secs(1),
        )
        .ok();
        if let Ok(mut g) = handle.lock() {
            *g = h;
        }
    };

    let running = move || left.get().is_some_and(|s| s > 0);
    view! {
        <div class="try-timer">
            <span class="try-clock" role="timer">
                {move || match left.get() {
                    None => String::new(),
                    Some(0) => "時間到".to_string(),
                    Some(s) => format!("{:02}:{:02}", s / 60, s % 60),
                }}
            </span>
            <button type="button" class="cta-alt try-timer-btn" on:click=toggle>
                {move || if running() { "停止計時" } else { "開始計時" }}
            </button>
        </div>
    }
}

#[component]
fn ReportForm(trial_id: String, ctx: TryCtx) -> impl IntoView {
    // None＝還沒選；Some(None)＝這次沒做；Some(Some(n))＝第 n 步
    let choice = RwSignal::new(None::<Option<u8>>);
    let stuck = RwSignal::new(None::<&'static str>);
    let rid = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let err = RwSignal::new(None::<&'static str>);
    let trial_id = StoredValue::new(trial_id);

    let reached = Signal::derive(move || choice.get().flatten());
    let skipping = move || choice.get() == Some(None);

    let submit = move |_| {
        let Some(reached) = choice.get_untracked() else {
            return;
        };
        if busy.get_untracked() {
            return;
        }
        let request_id = rid.get_untracked().unwrap_or_else(new_request_id);
        rid.set(Some(request_id.clone()));
        busy.set(true);
        err.set(None);
        let id = trial_id.get_value();
        spawn_local(async move {
            let body = TalentTrialReport {
                reached,
                stuck: reached.and(stuck.get_untracked()).map(|s| s.to_string()),
                request_id,
            };
            match api::talent_report(&id, &body).await {
                Ok(t) => {
                    let show = t.status == "reported";
                    ctx.result.try_set(show.then_some(t));
                    ctx.load().await;
                    if show {
                        scroll_to(".try-result");
                    }
                }
                Err(e) => {
                    let msg = if e.is_code("EXPIRED") || e.status() == Some(410) {
                        ctx.reload();
                        "這一筆已經超過 7 天，不能再對答案了。"
                    } else if e.is_code("ALREADY_REPORTED") {
                        ctx.reload();
                        "這一筆已經對過答案了。"
                    } else if e.status() == Some(404) {
                        ctx.reload();
                        "找不到這一筆紀錄。"
                    } else {
                        common_error(&e).unwrap_or("送出失敗，請稍後再試。")
                    };
                    err.try_set(Some(msg));
                    busy.try_set(false);
                }
            }
        });
    };

    view! {
        <div class="try-report" data-testid="try-report">
            <h3 class="try-q">{REPORT_QUESTION}</h3>
            <StepChoices
                value=reached
                disabled=busy.into()
                on_pick=Callback::new(move |n| {
                    choice.set(Some(Some(n)));
                    rid.set(None);
                })
            />
            <button
                type="button"
                class="try-skip"
                class:on=skipping
                aria-pressed=move || skipping().to_string()
                disabled=move || busy.get()
                on:click=move |_| {
                    choice.set(Some(None));
                    stuck.set(None);
                    rid.set(None);
                }
            >
                {SKIP_LABEL}
            </button>
            <Show when=move || reached.get().is_some()>
                <h3 class="try-q try-q-sub">{STUCK_QUESTION}</h3>
                <div class="try-stuck-pick" role="group">
                    {STUCK_REASONS
                        .iter()
                        .map(|(wire, label)| {
                            let wire: &'static str = wire;
                            let on = move || stuck.get() == Some(wire);
                            view! {
                                <button
                                    type="button"
                                    class="try-stuck"
                                    class:on=on
                                    aria-pressed=move || on().to_string()
                                    disabled=move || busy.get()
                                    on:click=move |_| {
                                        // 再點一次取消（可不選）
                                        stuck.update(|s| {
                                            *s = if *s == Some(wire) { None } else { Some(wire) }
                                        });
                                        rid.set(None);
                                    }
                                >
                                    {*label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </Show>
            <button
                type="button"
                class="btn-primary try-submit"
                data-testid="try-report-submit"
                disabled=move || choice.get().is_none() || busy.get()
                on:click=submit
            >
                "送出"
            </button>
            {move || err.get().map(|m| view! { <p class="error try-err" role="alert">{m}</p> })}
        </div>
    }
}

/// 剛對完答案的結果卡。
#[component]
fn ResultCard(trial: TalentTrial) -> impl IntoView {
    let gap = trial.gap.map(talent::gap_sentence);
    view! {
        <section class="card try-result" data-testid="try-result" aria-live="polite">
            <h2 class="try-title">{title_of(&trial.item_id)}</h2>
            <p class="try-result-line">{guess_line(&trial)}</p>
            {gap.map(|g| view! { <p class="try-result-gap">{g}</p> })}
            <p class="try-note">{CONDITION_NOTE}</p>
            <p class="try-date">{local_date(&trial.created_at)}</p>
        </section>
    }
}

/// 我的紀錄＋比你以為的順＋猜測傾向。
#[component]
fn Records(data: TalentTrialsResponse, ctx: TryCtx) -> impl IntoView {
    // 兩段式刪除（無 timer、無 window.confirm）：Some(id)＝已上膛；ARM_ALL＝全部
    let armed = RwSignal::new(None::<String>);
    let del_err = RwSignal::new(None::<&'static str>);

    let delete = move |id: Option<String>| {
        del_err.set(None);
        spawn_local(async move {
            let res = match &id {
                Some(id) => api::talent_delete(id).await,
                None => api::talent_delete_all().await,
            };
            match res {
                // 404＝他處已刪／重刪 → 照樣重載
                Ok(_) | Err(ApiErr::Api { status: 404, .. }) => {
                    let gone = |t: &TalentTrial| id.as_ref().is_none_or(|i| *i == t.id);
                    if ctx.result.with_untracked(|r| r.as_ref().is_some_and(gone)) {
                        ctx.result.try_set(None);
                    }
                    ctx.reload();
                }
                Err(e) => {
                    del_err.try_set(Some(common_error(&e).unwrap_or("刪除失敗，請稍後再試。")));
                }
            }
        });
    };

    let trials = data.trials;
    let smoother: Vec<TalentTrial> = trials
        .iter()
        .filter(|t| t.status == "reported" && t.gap.is_some_and(|g| g > 0))
        .cloned()
        .collect();
    let tendency = data.tendency;
    let has_rows = !trials.is_empty();

    let rows = trials
        .into_iter()
        .map(|t| {
            let id = t.id.clone();
            let id_cmp = t.id.clone();
            let is_armed = move || armed.with(|a| a.as_deref() == Some(id_cmp.as_str()));
            let is_armed2 = is_armed.clone();
            let gap = t.gap.filter(|_| t.status == "reported");
            view! {
                <li class="try-history-row" data-status=t.status.clone()>
                    <div class="try-row-head">
                        <span class="try-row-title">{title_of(&t.item_id)}</span>
                        <span class=format!("try-status st-{}", t.status)>{status_label(&t.status)}</span>
                    </div>
                    <p class="try-row-meta">
                        {format!("{} ・ {}", local_date(&t.created_at), guess_line(&t))}
                    </p>
                    {gap.map(|g| view! { <p class="try-row-gap">{format!("差距：{}", talent::gap_sentence(g))}</p> })}
                    <div class="try-row-actions">
                        <button
                            type="button"
                            class="btn-link try-del"
                            on:click=move |_| {
                                if is_armed() {
                                    armed.set(None);
                                    delete(Some(id.clone()));
                                } else {
                                    armed.set(Some(id.clone()));
                                }
                            }
                        >
                            {
                                let is_armed = is_armed2.clone();
                                move || if is_armed() { "確定刪除？再點一次" } else { "刪除" }
                            }
                        </button>
                        <Show when=is_armed2.clone()>
                            <span class="try-del-note">{DELETE_NOTE}</span>
                        </Show>
                    </div>
                </li>
            }
        })
        .collect_view();

    let all_armed = move || armed.with(|a| a.as_deref() == Some(ARM_ALL));
    let tendency_view = if tendency.need > 0 {
        view! { <p>{talent::tendency_need_sentence(tendency.need)}</p> }.into_any()
    } else {
        let kind = tendency
            .kind
            .as_deref()
            .and_then(TendencyKind::from_wire)
            .map(|k| k.sentence());
        view! {
            <p>{talent::tendency_counts_sentence(tendency.under, tendency.exact, tendency.over)}</p>
            {kind.map(|k| view! { <p class="try-tendency-kind">{k}</p> })}
            <p class="try-note">{TENDENCY_FOOTNOTE}</p>
        }
        .into_any()
    };

    view! {
        <section class="card try-history" data-testid="try-history">
            <h2>"我的紀錄"</h2>
            {move || del_err.get().map(|m| view! { <p class="error" role="alert">{m}</p> })}
            {if has_rows {
                view! {
                    <ul class="try-history-list">{rows}</ul>
                    <div class="try-row-actions try-del-all-row">
                        <button
                            type="button"
                            class="btn-link try-del-all"
                            on:click=move |_| {
                                if all_armed() {
                                    armed.set(None);
                                    delete(None);
                                } else {
                                    armed.set(Some(ARM_ALL.to_string()));
                                }
                            }
                        >
                            {move || if all_armed() { "確定刪除全部紀錄？再點一次" } else { "刪除全部紀錄" }}
                        </button>
                        <Show when=all_armed>
                            <span class="try-del-note">{DELETE_NOTE}</span>
                        </Show>
                    </div>
                }
                    .into_any()
            } else {
                view! { <p class="muted try-history-empty" data-testid="try-history-empty">"還沒有紀錄。"</p> }
                    .into_any()
            }}
        </section>

        <section class="card try-smoother">
            <h2>{SMOOTHER_TITLE}</h2>
            <ul class="try-smoother-list">
                {smoother
                    .into_iter()
                    .map(|t| {
                        view! {
                            <li>
                                <span class="try-row-title">{title_of(&t.item_id)}</span>
                                <p class="try-row-meta">
                                    {format!("{} ・ {}", local_date(&t.created_at), guess_line(&t))}
                                </p>
                                <p class="try-note">{CONDITION_NOTE}</p>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            <p class="try-note">{SMOOTHER_NOTE}</p>
        </section>

        <section class="card try-tendency">
            <h2>{TENDENCY_TITLE}</h2>
            {tendency_view}
        </section>
    }
}
