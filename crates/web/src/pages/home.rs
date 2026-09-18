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
                        <div class="hero-actions">
                            <A href="/login" attr:class="cta">"開始使用"</A>
                            <A href="/naming" attr:class="cta-alt">"姓名學（免登入）"</A>
                        </div>
                    }
                >
                    <div class="hero-actions">
                        <A href="/divination/ziwei" attr:class="cta">"紫微斗數"</A>
                        <A href="/divination/western" attr:class="cta-alt">"西洋占星"</A>
                        <A href="/personality" attr:class="cta-alt">"人格測驗"</A>
                        <A href="/naming" attr:class="cta-alt">"姓名學"</A>
                        <A href="/profile" attr:class="cta-alt">"我的資料"</A>
                    </div>
                </Show>
            </div>
            <div class="feature-grid">
                <div class="feature">
                    <h3><span class="feature-icon">"🔮"</span>" 紫微斗數"</h3>
                    <p>"傳統中國命理學，精準分析命盤格局與人生運勢"</p>
                </div>
                <div class="feature">
                    <h3><span class="feature-icon">"⭐"</span>" 西洋占星"</h3>
                    <p>"星座與行星位置分析，探索性格與天賦"</p>
                </div>
                <A href="/naming" attr:class="feature">
                    <h3><span class="feature-icon">"✒️"</span>" 姓名學"</h3>
                    <p>"五格剖象＋三才五行，輸入姓名即時解析（免登入）"</p>
                </A>
                <div class="feature">
                    <h3><span class="feature-icon">"🤖"</span>" AI 解讀"</h3>
                    <p>"智能 AI 提供專業且易懂的命理解讀"</p>
                </div>
                <div class="feature">
                    <h3><span class="feature-icon">"🧠"</span>" 人格測驗"</h3>
                    <p>"IPIP-15 十五題，約 90 秒，了解你的行為傾向"</p>
                </div>
                <A href="/glossary" attr:class="feature">
                    <h3><span class="feature-icon">"📖"</span>" 名詞解釋"</h3>
                    <p>"紫微、占星、姓名學常見名詞一次看懂"</p>
                </A>
            </div>
        </div>
    }
}
