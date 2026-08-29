//! Magic-link login / register — step 1 only. The form asks for the address,
//! the API answers 202, and the session is created exclusively by the emailed
//! link, which lands on the `/auth/verify` route (see `lib.rs`).

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;

#[component]
pub fn LoginPage() -> impl IntoView {
    let is_register = RwSignal::new(false);
    let email = RwSignal::new(String::new());
    let full_name = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    // Some(email) once the API accepted step 1 — flips the card to the
    // check-your-inbox state. There is no session yet and nothing to navigate
    // to; the emailed link completes the flow.
    let sent_to = RwSignal::new(Option::<String>::None);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(String::new());
        loading.set(true);
        spawn_local(async move {
            let email_v = email.get_untracked();
            let result = if is_register.get_untracked() {
                let name = full_name.get_untracked();
                api::register(&email_v, Some(name.as_str())).await
            } else {
                api::login(&email_v).await
            };
            match result {
                Ok(()) => sent_to.set(Some(email_v)),
                Err(e) => error.set(e.to_string()),
            }
            loading.set(false);
        });
    };

    let back_to_form = move |_| {
        sent_to.set(None);
        error.set(String::new());
    };

    view! {
        <div class="auth-page">
            <div class="card">
                <Show
                    when=move || sent_to.get().is_some()
                    fallback=move || view! {
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
                                    else if is_register.get() { "寄出註冊信".to_string() }
                                    else { "寄出登入信".to_string() }
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
                        <p style="margin-top:1rem;font-size:0.75rem;color:#6b7280;text-align:center">
                            "送出後我們會寄一封登入信到這個 Email；點信中的連結才會完成登入。"
                        </p>
                    }
                >
                    <h2 style="margin-bottom:1.5rem;text-align:center">"請到信箱點擊登入連結"</h2>
                    <p style="text-align:center;line-height:1.8">
                        {move || {
                            let addr = sent_to.get().unwrap_or_default();
                            if is_register.get() {
                                format!("我們已寄出一封驗證信到 {addr}，請到信箱點擊連結完成註冊。")
                            } else {
                                format!("若此 Email 已註冊，登入信已寄出到 {addr}，請點信中的連結完成登入。")
                            }
                        }}
                    </p>
                    <p style="font-size:0.8rem;color:#6b7280;line-height:1.8;text-align:center">
                        "連結會在數分鐘後失效，且只能使用一次。沒收到信請先檢查垃圾郵件資料夾；"
                        "登入時若一直沒收到，可能表示這個 Email 尚未註冊（系統不會另外通知）。"
                    </p>
                    <div style="margin-top:1.5rem;text-align:center">
                        <button class="btn-link" on:click=back_to_form>
                            "沒收到信？重新填寫"
                        </button>
                    </div>
                </Show>
            </div>
        </div>
    }
}
