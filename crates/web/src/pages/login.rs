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
    let invite = RwSignal::new(String::new());
    // Some(check) once the prefilled/typed code has been preflighted against
    // the public endpoint; drives the ✓ label / ✗ hint under the field.
    let invite_check = RwSignal::new(Option::<api::InviteCheck>::None);
    let error = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    // Some(email) once the API accepted step 1 — flips the card to the
    // check-your-inbox state. There is no session yet and nothing to navigate
    // to; the emailed link completes the flow.
    let sent_to = RwSignal::new(Option::<String>::None);

    // Prefill from `?invite=` (the link the admin copies in /admin).
    if let Some(code) = crate::query_param("invite") {
        invite.set(code);
        is_register.set(true);
        let check = invite_check.clone();
        let code_for_check = invite.get_untracked();
        spawn_local(async move {
            check.set(api::check_invite(&code_for_check).await.ok());
        });
    }

    let precheck_invite = move || {
        let code = invite.get_untracked();
        if code.is_empty() {
            invite_check.set(None);
            return;
        }
        let check = invite_check.clone();
        spawn_local(async move {
            check.set(api::check_invite(&code).await.ok());
        });
    };

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(String::new());
        // Beta gate: the register tab demands a code up front so the user does
        // not wait for an email that will never be sent.
        if is_register.get_untracked() && invite.get_untracked().trim().is_empty() {
            error.set("請填邀請碼".to_string());
            return;
        }
        loading.set(true);
        spawn_local(async move {
            let email_v = email.get_untracked();
            let result = if is_register.get_untracked() {
                let name = full_name.get_untracked();
                let inv = invite.get_untracked();
                api::register(&email_v, Some(name.as_str()), Some(inv.as_str())).await
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
                                    <label>"邀請碼"</label>
                                    <input
                                        type="text"
                                        placeholder="例:ABCD2345FG"
                                        prop:value=move || invite.get()
                                        on:input=move |ev| {
                                            invite.set(event_target_value(&ev));
                                            invite_check.set(None);
                                        }
                                        on:blur=move |_| precheck_invite()
                                    />
                                    <Show when=move || {
                                        invite_check.get()
                                            .as_ref()
                                            .map(|c| c.valid)
                                            .unwrap_or(false)
                                    }>
                                        <p style="font-size:0.75rem;color:#10b981;margin-top:0.25rem">
                                            {move || {
                                                format!(
                                                    "✓ 已套用邀請:{}",
                                                    invite_check
                                                        .get()
                                                        .and_then(|c| c.label)
                                                        .unwrap_or_default()
                                                )
                                            }}
                                        </p>
                                    </Show>
                                    <Show when=move || {
                                        invite_check
                                            .get()
                                            .as_ref()
                                            .map(|c| !c.valid)
                                            .unwrap_or(false)
                                    }>
                                        <p style="font-size:0.75rem;color:#ef4444;margin-top:0.25rem">
                                            "✗ 邀請碼無效或已失效"
                                        </p>
                                    </Show>
                                </div>
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
