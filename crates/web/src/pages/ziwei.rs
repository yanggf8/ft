//! 紫微子單元（spec docs/superpowers/specs/2026-11-21-sub-units-design.md §A.2–A.5）。
//!
//! - `/ziwei`：子單元選單（公開）；未登入時卡片連 `/login?next=/ziwei/…`。
//! - `/ziwei/chart`：命盤＋AI 解讀＝`DivinationPage chart_type="ziwei"`（lib.rs 接線）。
//! - `/ziwei/luck`：流年大運（大限時間軸、年份切換、宮格疊層、規則說明）。
//! - `/ziwei/year`：今年運勢（規則摘要 chips＋AI 解讀卡）。
//!
//! 沒填出生時辰（API `NO_BIRTH_HOUR`）→ 擋住並引導到 /profile 補填，不以預設午時排；
//! API 回 404（新 API 尚未部署）→ 顯示「服務準備中」而不是白屏。頁面文字不出現
//! 「預測」「本週」（與 F5 區隔；結尾提醒句除外，該句由 API 附加）。

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use ft_schema::ziwei_luck::{
    ZiWeiHoroscopeV1, ZiweiYearSummary, RULE_AGE, RULE_CHILDHOOD, RULE_DECADAL, RULE_FRAME,
    RULE_MUTAGEN, RULE_YEARLY,
};
use ft_schema::ZiWeiChartV3;

use crate::api::{self, ApiErr};
use crate::auth::use_auth;
use crate::components::{LuckOverlay, ZiWeiPalaceGrid};

/// 可查年份：出生農曆年到 +100（與 API `INVALID_YEAR` 範圍一致）。
const MAX_YEAR_SPAN: i32 = 100;

/// 載入失敗的分類（決定擋板文案）。
#[derive(Debug, Clone, Copy, PartialEq)]
enum Gate {
    NeedHour,
    NeedBirth,
    NotReady,
    Failed,
}

fn gate_of(e: &ApiErr) -> Gate {
    if e.is_code("NO_BIRTH_HOUR") {
        Gate::NeedHour
    } else if e.needs_birth_data() {
        Gate::NeedBirth
    } else if e.status() == Some(404) {
        // 這三個路由 API 自己不回 404：404＝新 API 尚未部署
        Gate::NotReady
    } else {
        Gate::Failed
    }
}

#[component]
fn Crumb(current: &'static str) -> impl IntoView {
    view! {
        <nav class="zw-crumb" aria-label="麵包屑">
            <A href="/">"首頁"</A>
            " › "
            <A href="/ziwei">"紫微斗數"</A>
            " › "
            <span>{current}</span>
        </nav>
    }
}

/// 擋板卡：缺時辰／缺生辰／服務準備中／載入失敗。
#[component]
fn GateCard(gate: Gate, name: &'static str) -> impl IntoView {
    match gate {
        Gate::NeedHour => view! {
            <div class="card zw-gate">
                <h3>"需要出生時辰"</h3>
                <p>{format!("{name}要用到命宮與大限，這兩個都由出生時辰決定。補上時辰後就能看。")}</p>
                <A href="/profile" attr:class="cta">"去補填出生時辰"</A>
            </div>
        }
        .into_any(),
        Gate::NeedBirth => view! {
            <div class="card zw-gate">
                <h3>"需要出生資料"</h3>
                <p>"請先填寫出生日期、時辰與性別。"</p>
                <A href="/profile" attr:class="cta">"去填寫出生資料"</A>
            </div>
        }
        .into_any(),
        Gate::NotReady => view! {
            <div class="card zw-gate">
                <h3>"服務準備中"</h3>
                <p class="muted">{format!("{name}即將開放，請稍後再來看看。")}</p>
            </div>
        }
        .into_any(),
        Gate::Failed => view! {
            <div class="card zw-gate">
                <p class="error">"載入失敗，請稍後再試。"</p>
            </div>
        }
        .into_any(),
    }
}

/// 子單元卡圖示（與首頁 `FeatureIcon` 同畫法；首頁元件為 home.rs 私有，這裡只取三種）。
#[component]
fn HubIcon(kind: &'static str) -> impl IntoView {
    let (fill, stroke) = match kind {
        "ziwei" => (
            "M11 4.5l2.1 4.3 4.7.7-3.4 3.3.8 4.7L11 15.3l-4.2 2.2.8-4.7-3.4-3.3 4.7-.7z",
            "M11 4.5l2.1 4.3 4.7.7-3.4 3.3.8 4.7L11 15.3l-4.2 2.2.8-4.7-3.4-3.3 4.7-.7zM19.5 16v3M18 17.5h3",
        ),
        "western" => (
            "M12 6.5a5.5 5.5 0 1 1 0 11 5.5 5.5 0 0 1 0-11z",
            "M12 6.5a5.5 5.5 0 1 1 0 11 5.5 5.5 0 0 1 0-11zM6.5 12.1A9.5 2.6 -25 1 0 15.6 7.9",
        ),
        _ => (
            "M5 5.5h14a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5h-7l-4.5 3.5v-3.5H5A1.5 1.5 0 0 1 3.5 15V7A1.5 1.5 0 0 1 5 5.5z",
            "M5 5.5h14a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5h-7l-4.5 3.5v-3.5H5A1.5 1.5 0 0 1 3.5 15V7A1.5 1.5 0 0 1 5 5.5zM12 8l.8 2.2L15 11l-2.2.8L12 14l-.8-2.2L9 11l2.2-.8z",
        ),
    };
    view! {
        <span class=format!("feature-icon fi-{kind}") aria-hidden="true">
            <svg viewBox="0 0 24 24" aria-hidden="true">
                <path class="fi-fill" d=fill />
                <path d=stroke />
            </svg>
        </span>
    }
}

/// `/ziwei` 子單元選單（公開）。
#[component]
pub fn ZiweiHubPage() -> impl IntoView {
    let auth = use_auth();
    let href = move |path: &'static str| {
        if auth.is_authed() {
            path.to_string()
        } else {
            format!("/login?next={path}")
        }
    };
    view! {
        <div class="page">
            <nav class="zw-crumb" aria-label="麵包屑">
                <A href="/">"首頁"</A>
                " › "
                <span>"紫微斗數"</span>
            </nav>
            <h1>"紫微斗數"</h1>
            <p class="zw-intro">
                "選一個想看的角度。都需要登入與出生資料；流年大運與今年運勢還需要出生時辰。"
            </p>
            <div class="feature-grid zw-sub">
                <A href=move || href("/ziwei/chart") attr:class="feature">
                    <h3><HubIcon kind="ziwei"/>" 命盤"</h3>
                    <p>"十二宮、主星與四化，加上 AI 整體解讀。"</p>
                    <div class="zw-go">"點進去 →"</div>
                </A>
                <A href=move || href("/ziwei/luck") attr:class="feature">
                    <h3><HubIcon kind="western"/>" 流年大運"</h3>
                    <p>"十年一個大限、每年一個流年：看關注焦點落在哪一宮。"</p>
                    <div class="zw-go">"點進去 →"</div>
                </A>
                <A href=move || href("/ziwei/year") attr:class="feature">
                    <h3><HubIcon kind="ai"/>" 今年運勢"</h3>
                    <p>"今年的流年疊在你的命盤上，AI 寫一篇年度主題。"</p>
                    <div class="zw-go">"點進去 →"</div>
                </A>
            </div>
        </div>
    }
}

/// 本命盤上某地支的宮名。
fn natal_name(chart: &ZiWeiChartV3, branch: &str) -> String {
    chart
        .palaces
        .iter()
        .find(|p| p.branch == branch)
        .map(|p| p.name.clone())
        .unwrap_or_default()
}

/// 本命盤上某顆星所在的宮名。
fn star_palace(chart: &ZiWeiChartV3, star: &str) -> Option<String> {
    chart
        .palaces
        .iter()
        .find(|p| p.stars.iter().any(|s| s.name == star))
        .map(|p| p.name.clone())
}

/// 時間軸的一段：(標題, 副標, 點了跳到的年份, 是否高亮)。
type Seg = (String, String, Option<i32>, bool);

/// 大限時間軸（spec §A.4-1）：所選年份所在大限高亮；處於童限時前置「童限（虛歲 1–N）」
/// 一段並只高亮它。只列起始虛歲在可查範圍（≤101）內的大限。
fn timeline_segments(chart: &ZiWeiChartV3, h: &ZiWeiHoroscopeV1) -> Vec<Seg> {
    let base = i32::from(chart.birth_info.lunar.year);
    let mut segs = Vec::new();
    if h.decadal.is_childhood {
        let first = chart.major_limits.first().map(|m| m.start_age).unwrap_or(2);
        let end = first.saturating_sub(1).max(1);
        let sub = if end <= 1 {
            "虛歲 1".to_string()
        } else {
            format!("虛歲 1–{end}")
        };
        segs.push(("童限".to_string(), sub, Some(base), true));
    }
    for m in &chart.major_limits {
        if i32::from(m.start_age) > MAX_YEAR_SPAN + 1 {
            continue;
        }
        let on = !h.decadal.is_childhood && m.branch == h.decadal.branch;
        segs.push((
            format!("{}–{}", m.start_age, m.end_age),
            format!("{}{}", m.stem, m.branch),
            Some(base + i32::from(m.start_age) - 1),
            on,
        ));
    }
    segs
}

/// `/ziwei/luck` 流年大運。
#[component]
pub fn ZiweiLuckPage() -> impl IntoView {
    let auth = use_auth();
    let chart = RwSignal::new(None::<ZiWeiChartV3>);
    let horo = RwSignal::new(None::<ZiWeiHoroscopeV1>);
    let year = RwSignal::new(None::<i32>);
    let gate = RwSignal::new(None::<Gate>);
    let loading = RwSignal::new(true);
    let year_loading = RwSignal::new(false);
    let year_error = RwSignal::new(String::new());
    let req_gen = RwSignal::new(0u32);

    Effect::new(move |_| {
        if !auth.is_authed() {
            return;
        }
        spawn_local(async move {
            let _ = loading.try_set(true);
            // 先問流年（含時辰擋板）；預設年＝API 回的「今天」luckYear（春節前即去年）
            match api::ziwei_horoscope(None).await {
                Ok(r) => {
                    let _ = year.try_set(Some(r.horoscope.luck_year));
                    let _ = horo.try_set(Some(r.horoscope));
                }
                Err(e) => {
                    let _ = gate.try_set(Some(gate_of(&e)));
                    let _ = loading.try_set(false);
                    return;
                }
            }
            match api::get_chart("ziwei", false).await {
                Ok(c) => match c.as_ziwei() {
                    Some(z) => {
                        let _ = chart.try_set(Some(z));
                    }
                    None => {
                        let _ = gate.try_set(Some(Gate::Failed));
                    }
                },
                Err(e) => {
                    let _ = gate.try_set(Some(gate_of(&e)));
                }
            }
            let _ = loading.try_set(false);
        });
    });

    let bounds = move || {
        chart.with(|c| {
            c.as_ref().map(|c| {
                let lo = i32::from(c.birth_info.lunar.year);
                (lo, lo + MAX_YEAR_SPAN)
            })
        })
    };

    let select_year = move |y: i32| {
        let Some((lo, hi)) = bounds() else {
            return;
        };
        let y = y.clamp(lo, hi);
        if year.get_untracked() == Some(y) {
            return;
        }
        year.set(Some(y));
        year_error.set(String::new());
        let my_gen = req_gen.get_untracked() + 1;
        req_gen.set(my_gen);
        year_loading.set(true);
        spawn_local(async move {
            let res = api::ziwei_horoscope(Some(y)).await;
            // 期間又換了年份 → 舊回應整包丟棄
            if req_gen.try_get_untracked() != Some(my_gen) {
                return;
            }
            match res {
                Ok(r) => {
                    let _ = horo.try_set(Some(r.horoscope));
                }
                Err(e) => {
                    let msg = if e.is_code("INVALID_YEAR") {
                        "這一年不在可查範圍內。"
                    } else {
                        "這一年的流年載入失敗，請稍後再試。"
                    };
                    let _ = year_error.try_set(msg.to_string());
                }
            }
            let _ = year_loading.try_set(false);
        });
    };

    let ready = move || chart.with(|c| c.is_some()) && horo.with(|h| h.is_some());

    view! {
        <div class="page">
            <Crumb current="流年大運"/>
            <h1>"流年大運"</h1>
            <Show when=move || loading.get() && gate.get().is_none()>
                <div class="center-note">"載入中…"</div>
            </Show>
            {move || gate.get().map(|g| view! { <GateCard gate=g name="流年大運"/> })}
            <Show when=move || gate.get().is_none() && ready()>
                <div class="card">
                    <h3>"大限（十年一運）"</h3>
                    <p class="muted zw-note">"點一段大限，跳到那十年的第一年。"</p>
                    <div class="zw-timeline">
                        {move || {
                            let (Some(c), Some(h)) = (chart.get(), horo.get()) else {
                                return Vec::new();
                            };
                            timeline_segments(&c, &h)
                                .into_iter()
                                .map(|(title, sub, jump, on)| {
                                    view! {
                                        <button
                                            type="button"
                                            class="zw-seg"
                                            class:on=on
                                            aria-pressed=on.to_string()
                                            on:click=move |_| {
                                                if let Some(y) = jump {
                                                    select_year(y);
                                                }
                                            }
                                        >
                                            <b>{title}</b>
                                            {sub}
                                        </button>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }}
                    </div>
                    <div class="zw-year">
                        <button
                            type="button"
                            aria-label="前一年"
                            prop:disabled=move || {
                                year.get().zip(bounds()).map(|(y, (lo, _))| y <= lo).unwrap_or(true)
                            }
                            on:click=move |_| {
                                if let Some(y) = year.get_untracked() {
                                    select_year(y - 1);
                                }
                            }
                        >
                            "‹"
                        </button>
                        <span class="zw-year-label" aria-live="polite">
                            {move || {
                                let y = year.get().unwrap_or_default();
                                match horo.get() {
                                    Some(h) if h.luck_year == y && !year_loading.get() => {
                                        format!("{y} {}{}年", h.yearly.stem, h.yearly.branch)
                                    }
                                    _ => format!("{y} …"),
                                }
                            }}
                        </span>
                        <button
                            type="button"
                            aria-label="後一年"
                            prop:disabled=move || {
                                year.get().zip(bounds()).map(|(y, (_, hi))| y >= hi).unwrap_or(true)
                            }
                            on:click=move |_| {
                                if let Some(y) = year.get_untracked() {
                                    select_year(y + 1);
                                }
                            }
                        >
                            "›"
                        </button>
                    </div>
                    <Show when=move || !year_error.get().is_empty()>
                        <p class="error">{move || year_error.get()}</p>
                    </Show>
                    {move || {
                        let (Some(c), Some(h)) = (chart.get(), horo.get()) else {
                            return None;
                        };
                        Some(view! {
                            <ZiWeiPalaceGrid palaces=c.palaces overlay=LuckOverlay { horoscope: h }/>
                        })
                    }}
                </div>
                <div class="card">
                    <h3>"怎麼看這張圖"</h3>
                    <div class="zw-legend">
                        <div><span class="zw-badge dec">"大限命宮"</span>{RULE_DECADAL}</div>
                        <div><span class="zw-badge dec">"童限"</span>{RULE_CHILDHOOD}</div>
                        <div><span class="zw-badge">"流年命宮"</span>{RULE_YEARLY}</div>
                        <div><span class="zw-badge age">"小限"</span>{RULE_AGE}</div>
                        <div><span class="zw-flow">"流祿／流權／流科／流忌"</span>" "{RULE_MUTAGEN}</div>
                        <p class="muted zw-note">{RULE_FRAME}</p>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// 今年運勢的規則摘要 chips。
fn summary_chips(s: &ZiweiYearSummary, chart: Option<&ZiWeiChartV3>) -> Vec<(String, String)> {
    let h = &s.horoscope;
    let natal = |branch: &str| {
        chart
            .map(|c| natal_name(c, branch))
            .filter(|n| !n.is_empty())
            .map(|n| format!("（本命{n}）"))
            .unwrap_or_default()
    };
    let mut chips = Vec::new();
    let decadal = if h.decadal.is_childhood {
        format!("童限{}", natal(&h.decadal.branch))
    } else {
        let range = match (h.decadal.start_age, h.decadal.end_age) {
            (Some(a), Some(b)) => format!("{a}–{b} "),
            _ => String::new(),
        };
        format!(
            "{range}{}{}{}",
            h.decadal.stem,
            h.decadal.branch,
            natal(&h.decadal.branch)
        )
    };
    chips.push(("所在大限".to_string(), decadal));
    chips.push((
        "流年命宮".to_string(),
        format!("{}{}", h.yearly.branch, natal(&h.yearly.branch)),
    ));
    for (label, star) in h.yearly.mutagen.labeled() {
        let at = chart
            .and_then(|c| star_palace(c, star))
            .map(|p| format!("（在本命{p}）"))
            .unwrap_or_default();
        chips.push((label.to_string(), format!("{star}{at}")));
    }
    chips
}

/// AI 錯誤的友善中文：`AI_DAILY_LIMIT`＝本功能 24h 3 次上限。
fn ai_error_text(e: &ApiErr) -> String {
    if e.is_code("AI_DAILY_LIMIT") {
        return "今天的今年運勢 AI 解讀次數已用完（24 小時內最多 3 次），明天再試。".to_string();
    }
    if e.is_code("NO_BIRTH_HOUR") {
        return "需要先補填出生時辰，才能產生解讀。".to_string();
    }
    if e.needs_birth_data() {
        return "需要先填寫出生資料，才能產生解讀。".to_string();
    }
    e.ai_message()
}

/// `/ziwei/year` 今年運勢。
#[component]
pub fn ZiweiYearPage() -> impl IntoView {
    let auth = use_auth();
    let summary = RwSignal::new(None::<ZiweiYearSummary>);
    let chart = RwSignal::new(None::<ZiWeiChartV3>);
    let reading = RwSignal::new(None::<String>);
    let gate = RwSignal::new(None::<Gate>);
    let loading = RwSignal::new(true);
    let ai_loading = RwSignal::new(false);
    let ai_error = RwSignal::new(String::new());

    Effect::new(move |_| {
        if !auth.is_authed() {
            return;
        }
        spawn_local(async move {
            let _ = loading.try_set(true);
            match api::ziwei_year().await {
                Ok(r) => {
                    let _ = reading.try_set(r.reading.map(|x| x.interpretation));
                    let _ = summary.try_set(Some(r.summary));
                }
                Err(e) => {
                    let _ = gate.try_set(Some(gate_of(&e)));
                    let _ = loading.try_set(false);
                    return;
                }
            }
            // 本命宮名只用來註記 chips；失敗不擋頁面
            if let Ok(c) = api::get_chart("ziwei", false).await {
                let _ = chart.try_set(c.as_ziwei());
            }
            let _ = loading.try_set(false);
        });
    });

    let do_interpret = move |_| {
        if ai_loading.get_untracked() {
            return;
        }
        ai_loading.set(true);
        ai_error.set(String::new());
        spawn_local(async move {
            let res = api::ziwei_year_interpret().await;
            match res {
                Ok(r) => {
                    let _ = reading.try_set(Some(r.interpretation));
                }
                Err(e) => {
                    let _ = ai_error.try_set(ai_error_text(&e));
                }
            }
            let _ = ai_loading.try_set(false);
        });
    };

    view! {
        <div class="page">
            <Crumb current="今年運勢"/>
            <h1>"今年運勢"</h1>
            <Show when=move || loading.get() && gate.get().is_none()>
                <div class="center-note">"載入中…"</div>
            </Show>
            {move || gate.get().map(|g| view! { <GateCard gate=g name="今年運勢"/> })}
            {move || {
                if gate.get().is_some() {
                    return None;
                }
                let s = summary.get()?;
                let h = &s.horoscope;
                let title = format!(
                    "{} {}{}年・虛歲 {}",
                    h.luck_year, h.yearly.stem, h.yearly.branch, h.nominal_age
                );
                let chips = chart.with(|c| summary_chips(&s, c.as_ref()));
                Some(view! {
                    <div class="card">
                        <h3>{title}</h3>
                        <div class="zw-sum">
                            {chips
                                .into_iter()
                                .map(|(k, v)| view! { <span class="zw-chip">{k}" "<b>{v}</b></span> })
                                .collect_view()}
                        </div>
                        <A href="/ziwei/luck" attr:class="btn-link">"在流年大運看這一年的宮位圖 →"</A>
                    </div>
                })
            }}
            <Show when=move || gate.get().is_none() && summary.with(|s| s.is_some())>
                <div class="card">
                    <h2>"今年的主題"</h2>
                    <Show
                        when=move || reading.with(|r| r.is_some())
                        fallback=move || view! {
                            <div>
                                <p class="muted zw-note">
                                    "AI 依你的本命盤、目前的大限與今年的流年寫一篇年度主題。每人 24 小時內 3 次。"
                                </p>
                                <button
                                    class="btn-primary"
                                    prop:disabled=move || ai_loading.get()
                                    on:click=do_interpret
                                >
                                    {move || if ai_loading.get() { "解讀中…（約需半分鐘）" } else { "產生今年運勢解讀" }}
                                </button>
                            </div>
                        }
                    >
                        <div class="ai-reading">
                            {move || super::story::render_markdown(&reading.get().unwrap_or_default())}
                        </div>
                    </Show>
                    <Show when=move || !ai_error.get().is_empty()>
                        <p class="error">{move || ai_error.get()}</p>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
