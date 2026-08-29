//! /admin — invite-link management (spec 2026-08-30). Visible to the admin
//! only: the API answers 403 for anyone else, and this page renders that as a
//! plain no-permission note. Create named links, copy the full URL for
//! Messenger, watch usage, revoke.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api::{self, AdminInvite};

#[component]
pub fn AdminPage() -> impl IntoView {
    let invites = RwSignal::new(Option::<Vec<AdminInvite>>::None);
    let forbidden = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let label = RwSignal::new(String::new());
    let max_uses = RwSignal::new("20".to_string());
    let expires_at = RwSignal::new(String::new());
    let creating = RwSignal::new(false);
    // The URL of the most recently created link, shown big with a copy button.
    let new_url = RwSignal::new(Option::<String>::None);
    let copied = RwSignal::new(false);

    let reload = move |_| {
        spawn_local(async move {
            match api::list_invites().await {
                Ok(v) => invites.set(Some(v)),
                Err(e) => match e {
                    api::ApiErr::Api { status: 403, .. } => forbidden.set(true),
                    _ => error.set(e.to_string()),
                },
            }
        })
    };
    Effect::new(move |_| reload(()));

    let create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(String::new());
        creating.set(true);
        spawn_local(async move {
            let uses = max_uses.get_untracked().parse::<i64>().unwrap_or(20);
            let exp = expires_at.get_untracked();
            match api::create_invite(&label.get_untracked(), uses, Some(exp.as_str())).await {
                Ok(created) => {
                    new_url.set(Some(created.url));
                    copied.set(false);
                    label.set(String::new());
                    expires_at.set(String::new());
                    spawn_local(async move {
                        if let Ok(v) = api::list_invites().await {
                            invites.set(Some(v));
                        }
                    });
                }
                Err(e) => error.set(e.to_string()),
            }
            creating.set(false);
        });
    };

    let copy = move |_| {
        if let Some(url) = new_url.get_untracked() {
            if let Some(win) = web_sys::window() {
                // The crate's web-sys features do not expose Navigator, so the
                // clipboard rides on Reflect + js_sys::Function like the query
                // parsing in lib.rs.
                let clip = js_sys::Reflect::get(win.as_ref(), &"navigator".into())
                    .and_then(|nav| js_sys::Reflect::get(&nav, &"clipboard".into()));
                if let Ok(clip) = clip {
                    if let Ok(f) = js_sys::Reflect::get(&clip, &"writeText".into()) {
                        let write: js_sys::Function = f.into();
                        let _ = write.call1(&clip, &wasm_bindgen::JsValue::from_str(&url));
                    }
                }
            }
            copied.set(true);
        }
    };

    let status_line = move |r: &AdminInvite| {
        if r.revoked_at.is_some() {
            "已撤銷".to_string()
        } else {
            format!("{}/{}", r.used_count, r.max_uses)
        }
    };

    view! {
        <div class="page">
            <h1>"邀請管理"</h1>
            <Show
                when=move || !forbidden.get()
                fallback=move || view! { <div class="center-note">"無權限。此頁面僅供管理員使用。"</div> }
            >
                <div class="card" style="margin-bottom:1.5rem">
                    <h2 style="margin-bottom:1rem">"建立邀請連結"</h2>
                    <form on:submit=create>
                        <div class="field">
                            <label>"備註(給自己看的)"</label>
                            <input
                                type="text"
                                placeholder="例:Messenger 群 A、給小美"
                                prop:value=move || label.get()
                                on:input=move |ev| label.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="field">
                            <label>"人數上限"</label>
                            <input
                                type="number"
                                min="1"
                                max="500"
                                prop:value=move || max_uses.get()
                                on:input=move |ev| max_uses.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="field">
                            <label>"過期日(選填)"</label>
                            <input
                                type="date"
                                prop:value=move || expires_at.get()
                                on:input=move |ev| expires_at.set(event_target_value(&ev))
                            />
                        </div>
                        <Show when=move || !error.get().is_empty()>
                            <p class="error">{move || error.get()}</p>
                        </Show>
                        <button type="submit" disabled=move || creating.get() class="btn-primary">
                            {move || if creating.get() { "建立中..." } else { "建立連結" }}
                        </button>
                    </form>
                    <Show when=move || new_url.get().is_some()>
                        <div style="margin-top:1rem;padding:0.75rem;background:rgba(16,185,129,0.1);border-radius:8px">
                            <p style="font-size:0.85rem;margin-bottom:0.5rem">"連結已建立,複製後貼到 Messenger:"</p>
                            <code style="word-break:break-all;font-size:0.8rem">
                                {move || new_url.get().unwrap_or_default()}
                            </code>
                            <div style="margin-top:0.5rem">
                                <button class="btn-primary" on:click=copy>
                                    {move || if copied.get() { "已複製 ✓" } else { "複製連結" }}
                                </button>
                            </div>
                        </div>
                    </Show>
                </div>

                <div class="card">
                    <h2 style="margin-bottom:1rem">"連結列表"</h2>
                    <Show
                        when=move || invites.get().is_some()
                        fallback=move || view! { <div class="center-note">"載入中..."</div> }
                    >
                        <div style="overflow-x:auto">
                            <table style="width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums">
                                <thead>
                                    <tr style="text-align:left;border-bottom:1px solid rgba(128,128,128,0.3)">
                                        <th style="padding:0.5rem">"碼"</th>
                                        <th style="padding:0.5rem">"備註"</th>
                                        <th style="padding:0.5rem">"用量"</th>
                                        <th style="padding:0.5rem">"過期日"</th>
                                        <th style="padding:0.5rem">"狀態"</th>
                                        <th style="padding:0.5rem"></th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=move || invites.get().unwrap_or_default()
                                        key=|r| r.code.clone()
                                        children=move |r| {
                                            let code = r.code.clone();
                                            let row_revoked = r.revoked_at.is_some();
                                            view! {
                                                <tr style="border-bottom:1px solid rgba(128,128,128,0.15)">
                                                    <td style="padding:0.5rem;font-family:monospace">{r.code.clone()}</td>
                                                    <td style="padding:0.5rem">{r.label.clone()}</td>
                                                    <td style="padding:0.5rem">{status_line(&r)}</td>
                                                    <td style="padding:0.5rem">{r.expires_at.clone().unwrap_or_else(|| "—".into())}</td>
                                                    <td style="padding:0.5rem">
                                                        {if r.revoked_at.is_some() { "已撤銷" } else { "有效" }}
                                                    </td>
                                                    <td style="padding:0.5rem">
                                                        {(!row_revoked).then(|| {
                                                            let code = code.clone();
                                                            view! {
                                                                <button
                                                                    class="btn-link"
                                                                    on:click=move |_| {
                                                                        let code = code.clone();
                                                                        spawn_local(async move {
                                                                            if api::revoke_invite(&code).await.is_ok() {
                                                                                if let Ok(v) = api::list_invites().await {
                                                                                    invites.set(Some(v));
                                                                                }
                                                                            }
                                                                        })
                                                                    }
                                                                >
                                                                    "撤銷"
                                                                </button>
                                                            }
                                                        })}
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    />
                                </tbody>
                            </table>
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
