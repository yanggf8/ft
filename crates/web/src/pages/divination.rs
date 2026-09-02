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
use leptos_router::components::Redirect;
use leptos_router::hooks::use_params_map;

use ft_schema::{WesternChartV3, ZiWeiChartV3};

use crate::api;
use crate::auth::use_auth;
use crate::components::ZiWeiPalaceGrid;

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
                        err.set(e.to_string());
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
                Ok(resp) => interp.set(Some(resp.interpretation)),
                Err(e) => {
                    if e.status() == Some(404) {
                        if let Ok(r) = api::get_chart(&ct, true).await {
                            if ct == "ziwei" {
                                z.set(r.as_ziwei());
                            } else {
                                w.set(r.as_western());
                            }
                            if let Ok(retry) = api::interpret(&ct).await {
                                interp.set(Some(retry.interpretation));
                            } else {
                                err.set("解讀失敗，請稍後再試".to_string());
                            }
                        } else {
                            err.set("命盤載入失敗".to_string());
                        }
                    } else {
                        err.set(e.to_string());
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
                                {move || if interpreting.get() { "解讀中..." } else { "產生 AI 解讀" }}
                            </button>
                        </div>
                    }
                >
                    <div class="prose">{move || interpretation.get().unwrap_or_default()}</div>
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
        <div style="display:grid;gap:1rem">
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
                <span style="color:#9aa3b2;font-size:0.75rem">
                    {move || format!("#{} · {}", meta.chart_schema_version, meta.engine_version_ziwei)}
                </span>
            </div>
            <ZiWeiPalaceGrid palaces=chart.palaces.clone() />
        </div>
    }
}

#[component]
fn WesternView(chart: WesternChartV3) -> impl IntoView {
    let sun = chart.sun_sign.clone();
    let moon = chart.moon_sign.clone();
    let asc = chart.ascendant.clone();
    view! {
        <div style="display:grid;gap:1rem">
            <div style="text-align:center;padding:0.75rem 1rem;background:linear-gradient(135deg,#fdf2f8,#f0f9ff);border-radius:12px;border:1px solid #fce7f3">
                <div style="font-size:1.1rem;font-weight:700;color:#be185d">"‧₊˚✧ 西洋星盤 ✧˚₊‧"</div>
                <div style="font-size:0.75rem;color:#4b5563;margin-top:2px">"太陽 · 月亮 · 上升 · 行星"</div>
            </div>
            <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:0.5rem">
                <div style="background:#fefce8;border:1px solid #fde68a;border-radius:10px;padding:0.6rem;text-align:center">
                    <div style="font-size:0.7rem;color:#a16207;letter-spacing:0.05em">"太陽星座"</div>
                    <div style="font-size:1.3rem;margin:2px 0">{sun.symbol.clone()}</div>
                    <div style="font-size:0.85rem;font-weight:600;color:#92400e">{format!("{} {}", sun.name, sun.symbol)}</div>
                </div>
                <div style="background:#eff6ff;border:1px solid #bfdbfe;border-radius:10px;padding:0.6rem;text-align:center">
                    <div style="font-size:0.7rem;color:#1e40af;letter-spacing:0.05em">"月亮星座 (真實)"</div>
                    <div style="font-size:1.3rem;margin:2px 0">{moon.symbol.clone()}</div>
                    <div style="font-size:0.85rem;font-weight:600;color:#1e3a8a">{format!("{} {}", moon.name, moon.symbol)}</div>
                </div>
                <div style="background:#f5f3ff;border:1px solid #ddd6fe;border-radius:10px;padding:0.6rem;text-align:center">
                    <div style="font-size:0.7rem;color:#6d28d9;letter-spacing:0.05em">"上升星座"</div>
                    <div style="font-size:1.3rem;margin:2px 0">{asc_sign_symbol(&asc.sign)}</div>
                    <div style="font-size:0.85rem;font-weight:600;color:#4c1d95">{format!("{} {:.1}°", asc.sign, asc.degree)}</div>
                </div>
            </div>
            <div>
                <div style="font-weight:600;margin-bottom:0.5rem;color:#374151">"行星落座 ♡"</div>
                <div class="palace-stars" style="display:grid;grid-template-columns:repeat(auto-fill,minmax(140px,1fr));gap:0.4rem">
                    {chart.planets.into_iter().map(|p| {
                        let sym = planet_symbol(&p.name);
                        let sign_sym = sign_symbol(&p.sign);
                        let label = format!("{} {} {} {:.1}°", sym, p.name, sign_sym, p.degree);
                        view! { <span class="star main" style="background:#fff;border:1px solid #fce7f3;border-radius:8px;padding:0.4rem 0.6rem;font-size:0.8rem;display:flex;align-items:center;gap:0.3rem;justify-content:center"><span style="font-size:1rem">{sym}</span>{format!("{} {} {:.1}°", p.name, sign_sym, p.degree)}</span> }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}

fn planet_symbol(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "sun" => "☉",
        "moon" => "☽",
        "mercury" => "☿",
        "venus" => "♀",
        "mars" => "♂",
        "jupiter" => "♃",
        "saturn" => "♄",
        "uranus" => "♅",
        "neptune" => "♆",
        "pluto" => "♇",
        _ => "‧",
    }
}

fn sign_symbol(sign: &str) -> &'static str {
    match sign.to_lowercase().as_str() {
        "aries" => "♈",
        "taurus" => "♉",
        "gemini" => "♊",
        "cancer" => "♋",
        "leo" => "♌",
        "virgo" => "♍",
        "libra" => "♎",
        "scorpio" => "♏",
        "sagittarius" => "♐",
        "capricorn" => "♑",
        "aquarius" => "♒",
        "pisces" => "♓",
        _ => "",
    }
}

fn asc_sign_symbol(sign: &str) -> String {
    sign_symbol(sign).to_string()
}
