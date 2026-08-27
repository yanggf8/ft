//! Nav + footer shell — port of `Layout.tsx`.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::auth::use_auth;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    let auth = use_auth();

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
                        <button class="nav-logout" on:click=move |_| auth.logout()>"登出"</button>
                    </Show>
                </div>
            </nav>
            <main>{children()}</main>
            <footer class="footer">"© 2025 FortuneT - AI 智能命理分析"</footer>
        </div>
    }
}
