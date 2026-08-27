//! Landing page — port of `HomePage.tsx`.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::auth::use_auth;

#[component]
pub fn HomePage() -> impl IntoView {
    let auth = use_auth();

    view! {
        <div class="hero">
            <div class="hero-head">
                <h1>"FortuneT - AI 智能命理分析"</h1>
                <p class="hero-sub">"紫微斗數與西洋占星的專業解讀"</p>
                <Show
                    when=move || auth.is_authed()
                    fallback=|| view! {
                        <A href="/login" attr:class="cta">"開始使用"</A>
                    }
                >
                    <div class="hero-actions">
                        <A href="/divination/ziwei" attr:class="cta">"紫微斗數"</A>
                        <A href="/divination/western" attr:class="cta-alt">"西洋占星"</A>
                        <A href="/profile" attr:class="cta-alt">"我的資料"</A>
                    </div>
                </Show>
            </div>
            <div class="feature-grid">
                <div class="feature">
                    <h3>"🔮 紫微斗數"</h3>
                    <p>"傳統中國命理學，精準分析命盤格局與人生運勢"</p>
                </div>
                <div class="feature">
                    <h3>"⭐ 西洋占星"</h3>
                    <p>"星座與行星位置分析，探索性格與天賦"</p>
                </div>
                <div class="feature">
                    <h3>"🤖 AI 解讀"</h3>
                    <p>"智能 AI 提供專業且易懂的命理解讀"</p>
                </div>
            </div>
        </div>
    }
}
