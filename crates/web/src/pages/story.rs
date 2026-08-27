//! Story page — port of `StoryPage.tsx`. Parses the four-chapter markdown the AI
//! returns into sections, or falls back to raw text when no `## ` headings.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::api;
use crate::auth::use_auth;

#[derive(Debug, Clone, PartialEq)]
struct Chapter {
    heading: String,
    body: String,
}

/// Split a story into `## 章節` blocks. Port of `parseChapters` in StoryPage.tsx.
fn parse_chapters(story: &str) -> Vec<Chapter> {
    let mut chapters = Vec::new();
    let mut rest = story;
    while let Some(header) = rest.find("## ") {
        let after_header = &rest[header..];
        let line_end = after_header.find('\n').unwrap_or(after_header.len());
        let heading = after_header[3..line_end].trim().to_string();
        rest = &after_header[line_end..];
        let next = rest.find("## ").unwrap_or(rest.len());
        let body = rest[..next].trim().to_string();
        chapters.push(Chapter { heading, body });
        rest = &rest[next..];
    }
    chapters
}

#[component]
pub fn StoryPage() -> impl IntoView {
    let auth = use_auth();

    let story = RwSignal::new(None::<String>);
    let not_generated = RwSignal::new(false);
    let loading = RwSignal::new(true);
    let generating = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    // Load the existing story once on mount.
    Effect::new(move |_| {
        if !auth.is_authed() {
            return;
        }
        let loading = loading;
        let story = story;
        let not_generated = not_generated;
        let err = error;
        let nav = use_navigate();
        spawn_local(async move {
            match api::get_story(false).await {
                Ok(resp) => {
                    story.set(Some(resp.story));
                    not_generated.set(false);
                }
                Err(e) => {
                    if e.is_code("NO_STORY") || e.status() == Some(404) {
                        not_generated.set(true);
                    } else if e.needs_birth_data() {
                        nav("/profile", Default::default());
                    } else {
                        err.set(e.to_string());
                    }
                }
            }
            loading.set(false);
        });
    });

    let do_generate = Callback::new(move |_| {
        let generating = generating;
        let err = error;
        let story = story;
        let not_generated = not_generated;
        let nav = use_navigate();
        spawn_local(async move {
            generating.set(true);
            err.set(String::new());
            match api::generate_story().await {
                Ok(resp) => {
                    story.set(Some(resp.story));
                    not_generated.set(false);
                }
                Err(e) => {
                    if e.is_code("RATE_LIMIT") {
                        err.set("請求過於頻繁，請稍後再試".to_string());
                    } else if e.is_code("AI_UNAVAILABLE") {
                        err.set("AI 服務暫時無法使用，請稍後再試".to_string());
                    } else if e.needs_birth_data() {
                        nav("/profile", Default::default());
                    } else {
                        err.set(e.to_string());
                    }
                }
            }
            generating.set(false);
        });
    });

    let back = move |_| {
        let nav = use_navigate();
        nav("/profile", Default::default());
    };

    view! {
        <div class="page">
            <button class="back-link" on:click=back>"← 返回"</button>
            <h1 style="margin-bottom:1.5rem">"合盤故事"</h1>

            <Show when=move || !error.get().is_empty()>
                <p class="error">{move || error.get()}</p>
            </Show>

            <Show when=move || not_generated.get() && story.get().is_none()>
                <div class="card">
                    <p class="muted" style="margin-bottom:1rem">
                        "將您的紫微斗數與西洋占星命盤融合成一篇專屬的合盤故事。"
                    </p>
                    <button
                        class="btn-primary"
                        prop:disabled=move || generating.get()
                        on:click=move |_| do_generate.run(())
                    >
                        {move || {
                            if generating.get() {
                                "故事生成中…（約 10–30 秒）".to_string()
                            } else {
                                "生成合盤故事".to_string()
                            }
                        }}
                    </button>
                </div>
            </Show>

            <StoryChapters story=story />
        </div>
    }
}

#[component]
fn StoryChapters(story: RwSignal<Option<String>>) -> impl IntoView {
    move || {
        let Some(s) = story.get() else {
            return view! { <div/> }.into_any();
        };
        let chapters = parse_chapters(&s);
        if chapters.is_empty() {
            view! { <div class="card"><div class="prose">{s}</div></div> }.into_any()
        } else {
            view! {
                {chapters.into_iter().map(|c| view! {
                    <div class="card">
                        <h2 style="margin-bottom:1rem">{c.heading}</h2>
                        <div class="prose">{c.body}</div>
                    </div>
                }).collect_view()}
            }
            .into_any()
        }
    }
}
