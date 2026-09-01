//! Nav + footer shell — port of `Layout.tsx` with avatar dropdown.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::auth::use_auth;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    let auth = use_auth();
    let menu_open = RwSignal::new(false);

    // Close menu when navigating
    let close_menu = move || menu_open.set(false);

    view! {
        <div class="app">
            <nav class="nav">
                <A href="/" attr:class="nav-brand">"FortuneT"</A>
                <div class="nav-links">
                    <Show
                        when=move || auth.is_authed()
                        fallback=|| view! { <A href="/login">"登入"</A> }
                    >
                        <A href="/profile">"我的命盤"</A>
                        <A href="/divination/ziwei">"紫微斗數"</A>
                        <A href="/divination/western">"西洋占星"</A>
                        <A href="/story">"合盤故事"</A>
                        // Avatar dropdown
                        <div class="nav-avatar-wrap" style="position:relative">
                            <button
                                class="nav-avatar-btn"
                                on:click=move |_| menu_open.update(|o| *o = !*o)
                                aria-label="使用者選單"
                                style="background:none;border:none;cursor:pointer;padding:0;display:flex;align-items:center"
                            >
                                {move || {
                                    let user = auth.user.get();
                                    if let Some(u) = user.as_ref() {
                                        if let Some(url) = u.avatar_url.as_deref().filter(|s| !s.is_empty()) {
                                            view! {
                                                <img
                                                    src=url.to_string()
                                                    alt="avatar"
                                                    style="width:32px;height:32px;border-radius:50%;object-fit:cover;border:1.5px solid #e5e7eb"
                                                />
                                            }.into_any()
                                        } else {
                                            let initial = u
                                                .full_name
                                                .as_deref()
                                                .filter(|s| !s.is_empty())
                                                .or(Some(u.email.as_str()))
                                                .unwrap_or("?")
                                                .chars()
                                                .next()
                                                .unwrap_or('?')
                                                .to_string();
                                            view! {
                                                <span style="width:32px;height:32px;border-radius:50%;background:linear-gradient(135deg,#a78bfa,#f472b6);color:#fff;display:flex;align-items:center;justify-content:center;font-size:14px;font-weight:700">
                                                    {initial}
                                                </span>
                                            }.into_any()
                                        }
                                    } else {
                                        view! {
                                            <span style="width:32px;height:32px;border-radius:50%;background:#e5e7eb;display:flex;align-items:center;justify-content:center"> "?" </span>
                                        }.into_any()
                                    }
                                }}
                            </button>
                            <Show when=move || menu_open.get()>
                                <div
                                    class="nav-dropdown"
                                    style="position:absolute;right:0;top:calc(100% + 8px);min-width:220px;background:#fff;border:1px solid #e5e7eb;border-radius:12px;box-shadow:0 8px 24px rgba(0,0,0,0.12);padding:8px;z-index:50"
                                    on:click=move |_| close_menu()
                                >
                                    // User header
                                    <div style="padding:10px 12px;border-bottom:1px solid #f3f4f6;margin-bottom:6px">
                                        {move || {
                                            let u = auth.user.get();
                                            if let Some(user) = u {
                                                let name = user
                                                    .full_name
                                                    .clone()
                                                    .unwrap_or_else(|| user.email.clone());
                                                let email = user.email.clone();
                                                view! {
                                                    <div style="font-weight:600;font-size:14px;color:#111827;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">
                                                        {name}
                                                    </div>
                                                    <div style="font-size:12px;color:#6b7280;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">
                                                        {email}
                                                    </div>
                                                    <Show when=move || {
                                                        auth.user.get().as_ref().map(|u| u.is_admin).unwrap_or(false)
                                                    }>
                                                        <span style="display:inline-block;margin-top:4px;font-size:11px;padding:2px 6px;border-radius:999px;background:#fef3c7;color:#92400e">
                                                            "管理員"
                                                        </span>
                                                    </Show>
                                                    <Show when=move || {
                                                        !auth.user.get().as_ref().map(|u| u.is_admin).unwrap_or(false)
                                                            && (auth.user.get().as_ref().map(|u| u.billing.tier.clone()).unwrap_or_default() != "free"
                                                            || auth.user.get().as_ref().map(|u| u.billing.isTrialing).unwrap_or(false))
                                                    }>
                                                        <span style="display:inline-block;margin-top:4px;font-size:11px;padding:2px 6px;border-radius:999px;background:#eef2ff;color:#4f46e5">
                                                            {move || {
                                                                let tier = auth
                                                                    .user
                                                                    .get()
                                                                    .as_ref()
                                                                    .map(|u| u.billing.tier.clone())
                                                                    .unwrap_or_default();
                                                                if tier == "free" { "試用中".to_string() } else { tier }
                                                            }}
                                                        </span>
                                                    </Show>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }
                                        }}
                                    </div>
                                    <A
                                        href="/profile"
                                        attr:style="display:block;padding:8px 12px;border-radius:8px;font-size:14px;color:#374151;text-decoration:none"
                                        on:click=move |_| close_menu()
                                    >
                                        "基本資料"
                                    </A>
                                    <A
                                        href="/personality"
                                        attr:style="display:block;padding:8px 12px;border-radius:8px;font-size:14px;color:#374151;text-decoration:none"
                                        on:click=move |_| close_menu()
                                    >
                                        "人格測驗"
                                    </A>
                                    <Show when=move || {
                                        auth.user.get().as_ref().map(|u| u.is_admin).unwrap_or(false)
                                    }>
                                        <A
                                            href="/admin"
                                            attr:style="display:block;padding:8px 12px;border-radius:8px;font-size:14px;color:#374151;text-decoration:none"
                                            on:click=move |_| close_menu()
                                        >
                                            "邀請管理"
                                        </A>
                                    </Show>
                                    <div style="height:1px;background:#f3f4f6;margin:6px 0"></div>
                                    <button
                                        style="width:100%;text-align:left;padding:8px 12px;border-radius:8px;font-size:14px;color:#ef4444;background:none;border:none;cursor:pointer"
                                        on:click=move |_| {
                                            close_menu();
                                            auth.logout();
                                        }
                                    >
                                        "登出"
                                    </button>
                                </div>
                            </Show>
                        </div>
                        // Click-away overlay
                        <Show when=move || menu_open.get()>
                            <div
                                style="position:fixed;inset:0;z-index:40"
                                on:click=move |_| close_menu()
                            ></div>
                        </Show>
                    </Show>
                </div>
            </nav>
            <main>{children()}</main>
            <footer class="footer">"© 2025 FortuneT - AI 智能命理分析"</footer>
        </div>
    }
}
