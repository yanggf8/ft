//! Profile page — port of `ProfilePage.tsx` into the shared `UserProfile` type.

use leptos::prelude::*;
use leptos::task::spawn_local;

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
                        let year = auth.user.get().and_then(|u| u.birth_year).unwrap_or(0);
                        let gen = crate::generation::generation_story(year);
                        view! {
                            <div style="display:grid;gap:0.75rem">
                                <p style="color:var(--text);background:rgba(255,255,255,0.06);border:1px solid var(--glass-border);border-radius:8px;padding:0.75rem 1rem;font-weight:500">{text}</p>
                                {gen.map(|(title, desc)| view! {
                                    <div style="background:linear-gradient(135deg,rgba(167,139,250,0.12),rgba(244,114,182,0.10));border:1px solid rgba(167,139,250,0.25);border-radius:10px;padding:0.85rem 1rem">
                                        <div style="font-weight:700;font-size:0.9rem;color:#6d28d9;margin-bottom:0.25rem">{title}</div>
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
