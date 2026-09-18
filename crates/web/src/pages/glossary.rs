//! 名詞解釋頁 — 靜態詞條（免登入）。資料於 crate::glossary（spec §5 鎖定）。

use leptos::prelude::*;
use leptos_router::components::A;

use crate::glossary::{Category, GLOSSARY_VERSION, TERMS};

fn category_section(cat: Category) -> impl IntoView {
    let terms: Vec<_> = TERMS.iter().filter(|t| t.category == cat).collect();
    view! {
        <section style="margin-bottom:2rem">
            <h2 style="margin-bottom:0.75rem">{cat.label()}</h2>
            {terms
                .into_iter()
                .map(|t| {
                    view! {
                        <details style="margin-bottom:0.5rem">
                            <summary style="cursor:pointer;padding:0.5rem 0">{t.term}</summary>
                            <p class="muted" style="margin:0.25rem 0 0.75rem 0.25rem">{t.def}</p>
                        </details>
                    }
                })
                .collect_view()}
        </section>
    }
}

#[component]
pub fn GlossaryPage() -> impl IntoView {
    view! {
        <div class="page">
            <A href="/" attr:class="back-link">"← 返回"</A>
            <h1 style="margin-bottom:0.5rem">"名詞解釋"</h1>
            <p class="muted" style="margin-bottom:1.5rem">
                {format!("紫微、西洋占星與姓名學的常見名詞一次看懂（初稿 {}，內容審訂中）。", GLOSSARY_VERSION)}
            </p>
            {category_section(Category::Ziwei)}
            {category_section(Category::ZiweiPattern)}
            {category_section(Category::Western)}
            {category_section(Category::Naming)}
        </div>
    }
}
