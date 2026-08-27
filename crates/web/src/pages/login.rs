//! Passwordless login / register — port of `LoginPage.tsx`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::api;
use crate::auth::use_auth;

#[component]
pub fn LoginPage() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();

    let is_register = RwSignal::new(false);
    let email = RwSignal::new(String::new());
    let full_name = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let loading = RwSignal::new(false);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(String::new());
        loading.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            let email_v = email.get_untracked();
            let result = if is_register.get_untracked() {
                let name = full_name.get_untracked();
                api::register(&email_v, Some(name.as_str())).await.map(|_| ())
            } else {
                api::login(&email_v).await.map(|_| ())
            };
            match result {
                Ok(()) => {
                    auth.refresh(false).await;
                    navigate("/profile", Default::default());
                }
                Err(e) => error.set(e.to_string()),
            }
            loading.set(false);
        });
    };

    view! {
        <div class="auth-page">
            <div class="card">
                <h2 style="margin-bottom:1.5rem;text-align:center">
                    {move || if is_register.get() { "註冊帳號" } else { "登入" }}
                </h2>
                <form on:submit=submit>
                    <div class="field">
                        <label>"Email"</label>
                        <input
                            type="email"
                            placeholder="your@email.com"
                            required
                            prop:value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))
                        />
                    </div>
                    <Show when=move || is_register.get()>
                        <div class="field">
                            <label>"姓名 (選填)"</label>
                            <input
                                type="text"
                                placeholder="您的姓名"
                                prop:value=move || full_name.get()
                                on:input=move |ev| full_name.set(event_target_value(&ev))
                            />
                        </div>
                    </Show>
                    <Show when=move || !error.get().is_empty()>
                        <p class="error">{move || error.get()}</p>
                    </Show>
                    <button
                        type="submit"
                        disabled=move || loading.get()
                        class="btn-primary"
                        style="width:100%"
                    >
                        {move || {
                            if loading.get() { "處理中...".to_string() }
                            else if is_register.get() { "註冊".to_string() }
                            else { "登入".to_string() }
                        }}
                    </button>
                </form>
                <div style="margin-top:1.5rem;text-align:center">
                    <button
                        class="btn-link"
                        on:click=move |_| is_register.update(|v| *v = !*v)
                    >
                        {move || if is_register.get() { "已有帳號？登入" } else { "沒有帳號？註冊" }}
                    </button>
                </div>
                <Show when=move || !is_register.get()>
                    <p style="margin-top:1rem;font-size:0.75rem;color:#6b7280;text-align:center">
                        "註：本系統使用無密碼登入，僅需 Email"
                    </p>
                </Show>
            </div>
        </div>
    }
}
