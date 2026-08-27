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
                        view! { <p style="color:#374151">{text}</p> }.into_any()
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
