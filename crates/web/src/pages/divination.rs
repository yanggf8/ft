//! Divination page — port of `DivinationPage.tsx`. Loads the chart for the URL's
//! `:type` param (ziwei | western), renders it from the shared `ft-schema` types,
//! and drives AI interpretation through `api::interpret` (retries on 409).
//!
//! The route param guards against invalid types without navigating from inside a
//! reactive effect (which re-triggers): instead we render an inline "redirect"
//! card that calls `use_navigate` only in a click-independent branch. Actually the
//! cleanest way is to check the param and, when invalid, render a `<Redirect>`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{Redirect, A};
use leptos_router::hooks::use_params_map;

use std::time::Duration;

use ft_schema::{
    element_zh, planet_zh, quality_zh, sign_class, sign_traits, sign_zh, WesternChartV3,
    ZiWeiChartV3,
};

use crate::api;
use crate::auth::use_auth;
use crate::components::{hide_broken_img, sign_art_src, sign_glyph_path, ZiWeiPalaceGrid};

#[component]
pub fn DivinationPage() -> impl IntoView {
    let auth = use_auth();
    let params = use_params_map();

    let chart = RwSignal::new(None::<serde_json::Value>);
    let ziwei = RwSignal::new(None::<ZiWeiChartV3>);
    let western = RwSignal::new(None::<WesternChartV3>);
    let interpretation = RwSignal::new(None::<String>);
    let loading = RwSignal::new(true);
    let interpreting = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    // Reactive param; `??` gives "" for missing so the guard below can react.
    let param_type = Memo::new(move |_| params.get().get("type").unwrap_or_default());

    // Load whenever the :type param changes (re-runs after log-in via auth gate).
    Effect::new(move |_| {
        let ct = param_type.get();
        if ct != "ziwei" && ct != "western" {
            // invalid — will render <Redirect> in the view
            return;
        }
        if !auth.is_authed() {
            return;
        }
        let raw = chart;
        let z = ziwei;
        let w = western;
        let interp = interpretation;
        let loading = loading;
        let err = error;
        let ct = ct.clone();
        spawn_local(async move {
            loading.set(true);
            err.set(String::new());
            match api::get_chart(&ct, false).await {
                Ok(resp) => {
                    raw.set(Some(resp.chart_data.clone()));
                    match ct.as_str() {
                        "ziwei" => z.set(resp.as_ziwei()),
                        "western" => w.set(resp.as_western()),
                        _ => {}
                    }
                    interp.set(resp.ai_interpretation.clone());
                }
                Err(e) => {
                    if e.needs_birth_data() {
                        // surface a friendly message; Profile CTA below
                        err.set("請先到「我的命格」填寫出生資料".to_string());
                    } else {
                        err.set("命盤載入失敗，請稍後再試".to_string());
                    }
                }
            }
            loading.set(false);
        });
    });

    let do_interpret = move |_| {
        let ct = param_type.get_untracked();
        let interp = interpretation;
        let err = error;
        let interpreting = interpreting;
        let z = ziwei;
        let w = western;
        spawn_local(async move {
            interpreting.set(true);
            err.set(String::new());
            match api::interpret(&ct).await {
                // stub＝離線範本：不當成功顯示，保留「產生 AI 解讀」按鈕
                Ok(resp) if resp.stub => err.set("AI 服務暫時無法使用，請稍後再試".to_string()),
                Ok(resp) => interp.set(Some(resp.interpretation)),
                Err(e) => {
                    if e.status() == Some(404) {
                        if let Ok(r) = api::get_chart(&ct, true).await {
                            if ct == "ziwei" {
                                z.set(r.as_ziwei());
                            } else {
                                w.set(r.as_western());
                            }
                            match api::interpret(&ct).await {
                                Ok(retry) if retry.stub => {
                                    err.set("AI 服務暫時無法使用，請稍後再試".to_string())
                                }
                                Ok(retry) => interp.set(Some(retry.interpretation)),
                                Err(e) => err.set(e.ai_message()),
                            }
                        } else {
                            err.set("命盤載入失敗".to_string());
                        }
                    } else {
                        err.set(e.ai_message());
                    }
                }
            }
            interpreting.set(false);
        });
    };

    let title = move || match param_type.get().as_str() {
        "ziwei" => "紫微斗數",
        "western" => "西洋占星",
        _ => "",
    };

    view! {
        // Invalid :type -> bounce to profile.
        <Show when=move || param_type.get() != "ziwei" && param_type.get() != "western">
            <Redirect path="/profile" />
        </Show>

        <div class="page">
            <button class="back-link" on:click={let nav=leptos_router::hooks::use_navigate(); move |_| nav("/profile", Default::default())}>
                "← 返回"
            </button>
            <h1 style="margin-bottom:1.5rem">{title()}</h1>

            <Show when=move || !error.get().is_empty()>
                <p class="error">{move || error.get()}</p>
            </Show>

            <Show when=move || !loading.get()>
                <div class="card">
                    <h2 style="margin-bottom:1rem">"命盤資料"</h2>
                    <Show
                        when=move || param_type.get() == "ziwei"
                        fallback=move || {
                            match western.get() {
                                Some(w) => view! { <WesternView chart=w/> }.into_any(),
                                None => view! { <pre class="prose">{raw_pretty(chart.get())}</pre> }.into_any(),
                            }
                        }
                    >
                        {move || match ziwei.get() {
                            Some(z) => view! { <ZiWeiView chart=z/> }.into_any(),
                            None => view! { <pre class="prose">{raw_pretty(chart.get())}</pre> }.into_any(),
                        }}
                    </Show>
                </div>
            </Show>

            <div class="card">
                <h2 style="margin-bottom:1rem">"AI 解讀"</h2>
                <Show
                    when=move || interpretation.get().is_some()
                    fallback=move || view! {
                        <div>
                            <p class="muted" style="margin-bottom:1rem">"尚未產生 AI 解讀"</p>
                            <button
                                class="btn-primary"
                                prop:disabled=move || interpreting.get()
                                on:click=do_interpret
                            >
                                {move || if interpreting.get() { "解讀中…（約需半分鐘）" } else { "產生 AI 解讀" }}
                            </button>
                        </div>
                    }
                >
                    <div class="ai-reading">{move || super::story::render_markdown(&interpretation.get().unwrap_or_default())}</div>
                </Show>
            </div>
        </div>
    }
}

fn raw_pretty(raw: Option<serde_json::Value>) -> String {
    raw.map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
        .unwrap_or_default()
}

#[component]
fn ZiWeiView(chart: ZiWeiChartV3) -> impl IntoView {
    let meta = chart.meta.clone();
    let four = chart.four_pillars.clone();
    view! {
        <div style="display:grid;grid-template-columns:minmax(0,1fr);gap:1rem">
            <div class="chart-meta" style="display:grid;gap:0.35rem">
                <span>
                    {move || {
                        let l = chart.birth_info.lunar.clone();
                        let leap = if l.is_leap.unwrap_or(false) { "(閏)" } else { "" };
                        format!("農曆: {}年{}月{}日{}", l.year, l.month, l.day, leap)
                    }}
                </span>
                <span><strong>"五行局:"</strong>{move || chart.five_element.clone()}</span>
                <span style="font-size:0.9rem;color:var(--silver-dim)">
                    {format!("{} {}  {} {}  {} {}  {} {}", four.year.stem, four.year.branch, four.month.stem, four.month.branch, four.day.stem, four.day.branch, four.hour.stem, four.hour.branch)}
                    <span style="margin-left:0.5rem;color:var(--silver-faint)">"八字"</span>
                </span>
                <span>
                    <strong>"大限:"</strong>
                    {move || chart.major_limits.iter().map(|m| format!("{}-{} {}{}", m.start_age, m.end_age, m.stem, m.branch)).collect::<Vec<_>>().join(" · ")}
                </span>
                <span style="color:var(--silver-faint);font-size:0.75rem">
                    {move || format!("#{} · {}", meta.chart_schema_version, meta.engine_version_ziwei)}
                </span>
            </div>
            <ZiWeiPalaceGrid palaces=chart.palaces.clone() />
        </div>
    }
}

/// Which of the three headline cards this is.
#[derive(Clone, Copy, PartialEq)]
enum Body {
    Sun,
    Moon,
    Ascendant,
}

impl Body {
    fn label(self) -> &'static str {
        match self {
            Body::Sun => "太陽",
            Body::Moon => "月亮",
            Body::Ascendant => "上升",
        }
    }

    /// Inline-SVG path for the card front (24×24 viewBox, stroked).
    fn icon_path(self) -> &'static str {
        match self {
            Body::Sun => {
                "M8 12a4 4 0 1 0 8 0a4 4 0 1 0 -8 0 M12 2.5v2.5 M12 19v2.5 M2.5 12h2.5 M19 12h2.5 \
                 M5.3 5.3l1.8 1.8 M16.9 16.9l1.8 1.8 M5.3 18.7l1.8-1.8 M16.9 7.1l1.8-1.8"
            }
            Body::Moon => "M19.5 14.5A8 8 0 1 1 9.5 4.5a6.3 6.3 0 0 0 10 10z",
            Body::Ascendant => "M12 18V5 M6.5 10.5L12 5l5.5 5.5 M3.5 21h17",
        }
    }
}

/// One headline card's data, resolved from the wire chart.
struct SignCard {
    body: Body,
    sign: String,
    degree: Option<f64>,
    element: String,
    quality: String,
}

impl SignCard {
    fn new(body: Body, sign: &str, degree: Option<f64>, element: &str, quality: &str) -> Self {
        // The wire carries element/quality for sun/moon only; the ascendant (or an
        // empty wire value) falls back to the sign's static traits.
        let traits = sign_traits(sign);
        let pick = |wire: &str, fallback: Option<&'static str>| {
            if wire.trim().is_empty() {
                fallback.unwrap_or_default().to_string()
            } else {
                wire.to_string()
            }
        };
        SignCard {
            body,
            sign: sign.to_string(),
            degree,
            element: pick(element, traits.map(|t| t.0)),
            quality: pick(quality, traits.map(|t| t.1)),
        }
    }

    fn degree_text(&self) -> Option<String> {
        self.degree.map(|d| format!("{d:.1}°"))
    }

    /// Fixed accessible name, e.g. 「太陽星座：獅子座 23.4°」.
    fn a11y_name(&self) -> String {
        let mut name = format!("{}星座：{}", self.body.label(), sign_zh(&self.sign));
        if let Some(d) = self.degree_text() {
            name.push(' ');
            name.push_str(&d);
        }
        name
    }

    fn traits_text(&self) -> String {
        [element_zh(&self.element), quality_zh(&self.quality)]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("・")
    }
}

/// Delay before the first card turns (lets the front paint so the turn is seen),
/// then the gap between consecutive cards.
const FLIP_START_MS: u64 = 350;
const FLIP_STEP_MS: u64 = 150;

fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .map(|m| m.matches())
        .unwrap_or(false)
}

fn planet_degree(chart: &WesternChartV3, name: &str) -> Option<f64> {
    chart
        .planets
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .map(|p| p.degree)
}

#[component]
fn WesternView(chart: WesternChartV3) -> impl IntoView {
    let cards = vec![
        SignCard::new(
            Body::Sun,
            &chart.sun_sign.name,
            planet_degree(&chart, "Sun"),
            &chart.sun_sign.element,
            &chart.sun_sign.quality,
        ),
        SignCard::new(
            Body::Moon,
            &chart.moon_sign.name,
            planet_degree(&chart, "Moon"),
            &chart.moon_sign.element,
            &chart.moon_sign.quality,
        ),
        SignCard::new(
            Body::Ascendant,
            &chart.ascendant.sign,
            Some(chart.ascendant.degree),
            "",
            "",
        ),
    ];

    // Cards open themselves in sequence on entry (站長裁示 §2.5-1); reduced-motion
    // users get the back side immediately. A click before a card's timer fires
    // marks it `touched` so the timer won't override the user's choice.
    let reduced = prefers_reduced_motion();
    let flipped: Vec<RwSignal<bool>> = cards.iter().map(|_| RwSignal::new(reduced)).collect();
    let touched: Vec<RwSignal<bool>> = cards.iter().map(|_| RwSignal::new(false)).collect();
    if !reduced {
        let handles: Vec<TimeoutHandle> = flipped
            .iter()
            .zip(touched.iter())
            .enumerate()
            .filter_map(|(i, (&f, &t))| {
                set_timeout_with_handle(
                    move || {
                        if !t.get_untracked() {
                            f.set(true);
                        }
                    },
                    Duration::from_millis(FLIP_START_MS + FLIP_STEP_MS * i as u64),
                )
                .ok()
            })
            .collect();
        on_cleanup(move || handles.iter().for_each(TimeoutHandle::clear));
    }

    let card_views = cards
        .into_iter()
        .zip(flipped.into_iter().zip(touched))
        .map(|(card, (flipped, touched))| {
            let sign_cls = sign_class(&card.sign);
            let class = format!("zc-card zc-sign-{}", sign_cls.unwrap_or("none"));
            let a11y = card.a11y_name();
            let traits = card.traits_text();
            let degree = card.degree_text();
            let glyph = sign_cls.and_then(sign_glyph_path);
            let art = sign_cls.and_then(sign_art_src);
            let name = sign_zh(&card.sign);
            let body = card.body;
            view! {
                <button
                    type="button"
                    class=class
                    class:zc-flipped=move || flipped.get()
                    aria-pressed=move || flipped.get().to_string()
                    on:click=move |_| {
                        touched.set(true);
                        flipped.update(|f| *f = !*f);
                    }
                >
                    <span class="zc-sr">{a11y}</span>
                    <span class="zc-inner" aria-hidden="true">
                        <span class="zc-face zc-front">
                            <svg class="zc-body-icon" viewBox="0 0 24 24" aria-hidden="true">
                                <path d=body.icon_path() />
                            </svg>
                            <span class="zc-front-label">{body.label()}</span>
                        </span>
                        <span class="zc-face zc-back">
                            <span class="zc-kind">{format!("{}星座", body.label())}</span>
                            {match art {
                                Some(src) => view! {
                                    <img class="zc-art" src=src alt="" aria-hidden="true"
                        on:error=hide_broken_img
                                        width="104" height="104" decoding="async" />
                                }.into_any(),
                                None => glyph.map(|d| view! {
                                    <svg class="zc-glyph" viewBox="0 0 24 24" aria-hidden="true">
                                        <path d=d />
                                    </svg>
                                }).into_any(),
                            }}
                            <span class="zc-name">{name}</span>
                            {degree.map(|d| view! { <span class="zc-deg">{d}</span> })}
                            {(!traits.is_empty()).then(|| view! { <span class="zc-traits">{traits}</span> })}
                        </span>
                    </span>
                </button>
            }
        })
        .collect_view();

    view! {
        <div class="zc-wrap">
            <div class="zc-banner">
                <div class="zc-banner-title">"‧₊˚✧ 西洋星盤 ✧˚₊‧"</div>
                <div class="zc-banner-sub">"太陽 · 月亮 · 上升 · 行星"</div>
            </div>
            <div class="zc-cards">{card_views}</div>
            <p class="zc-hint">"點牌可翻面"</p>
            <div>
                <div class="zc-section-title">"行星落座"</div>
                <div class="zc-planets">
                    {chart.planets.into_iter().map(|p| {
                        let class = format!("zc-pill zc-sign-{}", sign_class(&p.sign).unwrap_or("none"));
                        let label = format!("{} {} {:.1}°", planet_zh(&p.name), sign_zh(&p.sign), p.degree);
                        view! {
                            <span class=class>
                                <span class="zc-dot" aria-hidden="true"></span>
                                {label}
                            </span>
                        }
                    }).collect_view()}
                </div>
            </div>
            <p class="zm-cta-row">
                <A href="/zodiac" attr:class="zm-cta">"拿你的星座去配對 →"</A>
            </p>
        </div>
    }
}
