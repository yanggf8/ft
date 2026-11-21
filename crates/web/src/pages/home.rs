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
                // 吉祥物月亮貓（裝飾；alt="" ＋ aria-hidden，不進報讀）
                <img class="mascot mascot-hero" src="/assets/mascot/wave.webp" alt="" aria-hidden="true" width="120" height="185" />
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
                        <A href="/ziwei" attr:class="cta">"紫微斗數"</A>
                        <A href="/divination/western" attr:class="cta-alt">"西洋占星"</A>
                        <A href="/personality" attr:class="cta-alt">"人格測驗"</A>
                        <A href="/naming" attr:class="cta-alt">"姓名學"</A>
                        <A href="/profile" attr:class="cta-alt">"我的資料"</A>
                    </div>
                </Show>
            </div>
            <div class="feature-grid">
                <A href="/tarot" attr:class="feature">
                    <h3>
                        <FeatureIcon kind="tarot"/>" 塔羅"
                    </h3>
                    <p>"今日抽牌免登入；綜合、感情、事業三張牌陣需登入，當下問題的反思工具"</p>
                </A>
                <A href="/palm" attr:class="feature">
                    <h3>
                        <FeatureIcon kind="palm"/>" 手相"
                        <span class="feature-new">"NEW"</span>
                    </h3>
                    <p>"拍一張掌心照，AI 抽取掌紋特徵（免出生資料）；教學示意免登入"</p>
                </A>
                <A href="/fengshui" attr:class="feature">
                    <h3>
                        <FeatureIcon kind="fengshui"/>" 空間風水"
                        <span class="feature-new">"NEW"</span>
                    </h3>
                    <p>"臥室、書房或辦公室座位的擺設，並列風水的說法與相關研究；先猜研究站在哪一邊再翻卡（免登入）"</p>
                </A>
                <A href="/zodiac" attr:class="feature">
                    <h3>
                        <FeatureIcon kind="match"/>" 星座配對"
                        <span class="feature-new">"NEW"</span>
                    </h3>
                    <p>"點選對方的太陽星座，看你們的互動傾向（免登入）；登入後再加入你的月亮、上升"</p>
                </A>
                <A href="/ziwei" attr:class="feature">
                    <h3><FeatureIcon kind="ziwei"/>" 紫微斗數"</h3>
                    <p>"命盤、流年大運、今年運勢三個角度，看命盤格局與這幾年的關注主題"</p>
                </A>
                <div class="feature">
                    <h3><FeatureIcon kind="western"/>" 西洋占星"</h3>
                    <p>"星座與行星位置分析，探索性格傾向與成長方向"</p>
                </div>
                <A href="/naming" attr:class="feature">
                    <h3><FeatureIcon kind="naming"/>" 姓名學"</h3>
                    <p>"五格剖象＋三才五行，輸入姓名即時解析（免登入）"</p>
                </A>
                <div class="feature">
                    <h3><FeatureIcon kind="ai"/>" AI 解讀"</h3>
                    <p>"智能 AI 提供專業且易懂的命理解讀"</p>
                </div>
                <A href="/personality" attr:class="feature">
                    <h3><FeatureIcon kind="personality"/>" 人格測驗"</h3>
                    <p>"IPIP-15 十五題，約 90 秒，了解你的行為傾向"</p>
                </A>
                <A href="/glossary" attr:class="feature">
                    <h3><FeatureIcon kind="glossary"/>" 名詞解釋"</h3>
                    <p>"紫微、占星、塔羅、手相、姓名學、風水常見名詞一次看懂"</p>
                </A>
            </div>
        </div>
    }
}

/// 首頁功能卡圖示（取代 emoji：不依賴系統 emoji 字型，色彩全走 token）。
/// 每個圖示＝一條描邊 path（必要時加一塊白色填色 path），24×24 viewBox；
/// 圓底色由 `.fi-<kind>` 決定（style.css），描邊 `--primary-deep`。
#[component]
fn FeatureIcon(kind: &'static str) -> impl IntoView {
    let (fill, stroke) = icon_paths(kind);
    view! {
      <span class=format!("feature-icon fi-{kind}") aria-hidden="true">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          {(!fill.is_empty()).then(|| view! { <path class="fi-fill" d=fill /> })}
          <path d=stroke />
        </svg>
      </span>
    }
}

/// (填色 path, 描邊 path)。純資料，畫風：圓角粗線、少量細節。
fn icon_paths(kind: &str) -> (&'static str, &'static str) {
    match kind {
    // 兩張交疊的牌＋前牌上的小星
    "tarot" => (
      "M9.5 5.5h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2h-7a2 2 0 0 1-2-2v-10a2 2 0 0 1 2-2z",
      "M7.5 17.5l-1.6.4a2 2 0 0 1-2.4-1.5L1.9 10.9a2 2 0 0 1 1.5-2.4l4.1-1M9.5 5.5h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2h-7a2 2 0 0 1-2-2v-10a2 2 0 0 1 2-2zM13 9.5l.9 1.8 1.9.3-1.4 1.3.3 1.9-1.7-.9-1.7.9.3-1.9-1.4-1.3 1.9-.3z",
    ),
    // 張開的手掌
    "palm" => (
      "",
      "M8 12.5V6a1.5 1.5 0 0 1 3 0v5.5M11 11V4.5a1.5 1.5 0 0 1 3 0V11M14 11.5V6a1.5 1.5 0 0 1 3 0v8M8 12.5V11a1.5 1.5 0 0 0-3 0v4a6.5 6.5 0 0 0 6.5 6.5h.5a5 5 0 0 0 5-5v-2.5M10.5 17.5c1 .7 2.3.7 3.3 0",
    ),
    // 小屋＋窗
    "fengshui" => (
      "M5.5 10.5L12 5.5l6.5 5V19.5h-13z",
      "M3.5 12L12 5.5 20.5 12M5.5 10.5V19.5h13v-9M10 19.5v-4.5h4v4.5M15.5 8.2V5h2v4.7",
    ),
    // 星座配對：兩個交疊的圓＋上方小閃光
    "match" => (
      "M9 7a5 5 0 1 1 0 10 5 5 0 0 1 0-10z",
      "M9 7a5 5 0 1 1 0 10 5 5 0 0 1 0-10zM15 7a5 5 0 1 1 0 10 5 5 0 0 1 0-10zM12 2.5v2.5M10.75 3.75h2.5",
    ),
    // 紫微：大星＋小閃光
    "ziwei" => (
      "M11 4.5l2.1 4.3 4.7.7-3.4 3.3.8 4.7L11 15.3l-4.2 2.2.8-4.7-3.4-3.3 4.7-.7z",
      "M11 4.5l2.1 4.3 4.7.7-3.4 3.3.8 4.7L11 15.3l-4.2 2.2.8-4.7-3.4-3.3 4.7-.7zM19.5 16v3M18 17.5h3",
    ),
    // 西洋：帶環的行星
    "western" => (
      "M12 6.5a5.5 5.5 0 1 1 0 11 5.5 5.5 0 0 1 0-11z",
      "M12 6.5a5.5 5.5 0 1 1 0 11 5.5 5.5 0 0 1 0-11zM6.5 12.1A9.5 2.6 -25 1 0 15.6 7.9",
    ),
    // 姓名學：毛筆
    "naming" => (
      "M14.5 5.5l4 4-8 8-4-4z",
      "M14.5 5.5l2-2a1.4 1.4 0 0 1 2 0l2 2a1.4 1.4 0 0 1 0 2l-2 2M14.5 5.5l4 4-8 8-4-4zM6.5 13.5l-2.5 6.5 6.5-2.5",
    ),
    // AI：對話泡泡＋閃光
    "ai" => (
      "M5 5.5h14a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5h-7l-4.5 3.5v-3.5H5A1.5 1.5 0 0 1 3.5 15V7A1.5 1.5 0 0 1 5 5.5z",
      "M5 5.5h14a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5h-7l-4.5 3.5v-3.5H5A1.5 1.5 0 0 1 3.5 15V7A1.5 1.5 0 0 1 5 5.5zM12 8l.8 2.2L15 11l-2.2.8L12 14l-.8-2.2L9 11l2.2-.8z",
    ),
    // 人格：愛心
    "personality" => (
      "M12 19.5s-7-4.3-7-9.6A3.9 3.9 0 0 1 12 7.6a3.9 3.9 0 0 1 7 2.3c0 5.3-7 9.6-7 9.6z",
      "M12 19.5s-7-4.3-7-9.6A3.9 3.9 0 0 1 12 7.6a3.9 3.9 0 0 1 7 2.3c0 5.3-7 9.6-7 9.6zM8.5 10.5a1.8 1.8 0 0 1 1.8-1.8",
    ),
    // 名詞解釋：翻開的書
    "glossary" => (
      "M12 7.2C10 5.8 7 5.3 4 5.7v12.5c3-.4 6 .1 8 1.5 2-1.4 5-1.9 8-1.5V5.7c-3-.4-6 .1-8 1.5z",
      "M12 7.2C10 5.8 7 5.3 4 5.7v12.5c3-.4 6 .1 8 1.5 2-1.4 5-1.9 8-1.5V5.7c-3-.4-6 .1-8 1.5zM12 7.2v12.5",
    ),
    _ => ("", ""),
  }
}
