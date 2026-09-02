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

// ── lightweight markdown helpers ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum InlinePart {
    Text(String),
    Bold(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Block {
    Paragraph(String),
    List(Vec<String>),
}

/// Split inline `**bold**` segments. Unmatched `**` is treated as plain text.
fn parse_inline(text: &str) -> Vec<InlinePart> {
    let mut parts = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("**") {
        let before = &rest[..start];
        if !before.is_empty() {
            parts.push(InlinePart::Text(before.to_string()));
        }
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find("**") {
            let bold_content = &after_open[..end];
            // Empty ** ** is not considered bold — treat as text to avoid empty <strong>.
            if bold_content.is_empty() {
                parts.push(InlinePart::Text("****".to_string()));
            } else {
                parts.push(InlinePart::Bold(bold_content.to_string()));
            }
            rest = &after_open[end + 2..];
        } else {
            // No closing ** — the opening ** and remainder are plain text.
            parts.push(InlinePart::Text(format!("**{after_open}")));
            rest = "";
            break;
        }
    }
    if !rest.is_empty() {
        parts.push(InlinePart::Text(rest.to_string()));
    }
    if parts.is_empty() {
        parts.push(InlinePart::Text(String::new()));
    }
    parts
}

/// Group body into paragraph / list blocks.
///
/// - Blank lines flush the current block.
/// - Lines starting with `- ` (or `-` without space) form a `List` block.
/// - Consecutive non-list, non-empty lines are joined with a space into one `Paragraph`.
fn parse_blocks(body: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut para_buf: Vec<String> = Vec::new();
    let mut list_buf: Vec<String> = Vec::new();

    let flush_para = |para_buf: &mut Vec<String>, blocks: &mut Vec<Block>| {
        if !para_buf.is_empty() {
            let joined = para_buf.join(" ");
            if !joined.trim().is_empty() {
                blocks.push(Block::Paragraph(joined));
            }
            para_buf.clear();
        }
    };
    let flush_list = |list_buf: &mut Vec<String>, blocks: &mut Vec<Block>| {
        if !list_buf.is_empty() {
            blocks.push(Block::List(std::mem::take(list_buf)));
        }
    };

    for raw_line in body.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            flush_list(&mut list_buf, &mut blocks);
            flush_para(&mut para_buf, &mut blocks);
            continue;
        }
        let is_list_item =
            trimmed.starts_with("- ") || (trimmed.starts_with('-') && trimmed.len() > 1);
        if is_list_item {
            flush_para(&mut para_buf, &mut blocks);
            let item = if trimmed.starts_with("- ") {
                trimmed[2..].trim().to_string()
            } else {
                trimmed[1..].trim().to_string()
            };
            list_buf.push(item);
        } else {
            flush_list(&mut list_buf, &mut blocks);
            para_buf.push(trimmed.to_string());
        }
    }
    flush_list(&mut list_buf, &mut blocks);
    flush_para(&mut para_buf, &mut blocks);
    blocks
}

/// Whether a body is long enough to warrant collapsing: > ~800 chars or >= 5 blocks.
fn is_long_body(body: &str) -> bool {
    if body.chars().count() > 800 {
        return true;
    }
    let blocks = parse_blocks(body);
    blocks.len() >= 5
}

fn render_inline_parts(parts: Vec<InlinePart>) -> AnyView {
    parts
        .into_iter()
        .map(|p| match p {
            InlinePart::Text(t) => view! { {t} }.into_any(),
            InlinePart::Bold(t) => {
                view! { <strong style="font-weight:700;color:var(--text)">{t}</strong> }.into_any()
            }
        })
        .collect_view()
        .into_any()
}

fn render_blocks(blocks: &[Block]) -> AnyView {
    blocks
    .iter()
    .map(|b| match b {
      Block::Paragraph(text) => {
        let parts = parse_inline(text);
        view! {
          <p style="margin:0 0 0.85rem;line-height:1.8;color:var(--text)">
            {render_inline_parts(parts)}
          </p>
        }
        .into_any()
      }
      Block::List(items) => {
        view! {
          <ul style="margin:0 0 0.85rem 1.25rem;line-height:1.8;color:var(--text)">
            {items.iter().map(|it| {
              let parts = parse_inline(it);
              view! { <li style="margin-bottom:0.3rem">{render_inline_parts(parts)}</li> }.into_any()
            }).collect_view()}
          </ul>
        }
        .into_any()
      }
    })
    .collect_view()
    .into_any()
}

#[component]
fn ChapterCard(heading: String, body: String) -> impl IntoView {
    let is_long = is_long_body(&body);
    let expanded = RwSignal::new(false);
    let blocks = parse_blocks(&body);
    // Preview: first 3 blocks or enough to ~600 chars, whichever is smaller.
    let preview_len = {
        if blocks.len() <= 3 {
            blocks.len()
        } else {
            let mut acc = 0usize;
            let mut n = 0usize;
            for b in &blocks {
                let len = match b {
                    Block::Paragraph(t) => t.chars().count(),
                    Block::List(items) => {
                        items.iter().map(|s| s.chars().count()).sum::<usize>() + items.len() * 2
                    }
                };
                acc += len;
                n += 1;
                if n >= 3 || acc >= 600 {
                    break;
                }
            }
            n.max(1)
        }
    };

    view! {
      <div class="card">
        <h2 style="margin-bottom:1rem">{heading}</h2>
        <div class="prose" style="white-space:normal">
          {move || {
            if is_long && !expanded.get() {
              render_blocks(&blocks[..preview_len])
            } else {
              render_blocks(&blocks)
            }
          }}
        </div>
        {move || {
          if is_long {
            let label = if expanded.get() { "收起" } else { "展開全文" };
            view! {
              <button
                on:click=move |_| expanded.update(|v| *v = !*v)
                style="margin-top:0.75rem;background:none;border:none;padding:0;font-size:0.875rem;cursor:pointer;color:var(--silver-dim);text-decoration:underline;text-underline-offset:3px"
              >
                {label}
              </button>
            }
              .into_any()
          } else {
            view! { <span/> }.into_any()
          }
        }}
      </div>
    }
}

#[component]
fn FallbackCard(body: String) -> impl IntoView {
    let is_long = is_long_body(&body);
    let expanded = RwSignal::new(false);
    let blocks = parse_blocks(&body);
    let preview_len = {
        if blocks.len() <= 3 {
            blocks.len()
        } else {
            let mut acc = 0usize;
            let mut n = 0usize;
            for b in &blocks {
                let len = match b {
                    Block::Paragraph(t) => t.chars().count(),
                    Block::List(items) => {
                        items.iter().map(|s| s.chars().count()).sum::<usize>() + items.len() * 2
                    }
                };
                acc += len;
                n += 1;
                if n >= 3 || acc >= 600 {
                    break;
                }
            }
            n.max(1)
        }
    };

    view! {
      <div class="card">
        <div class="prose" style="white-space:normal">
          {move || {
            if is_long && !expanded.get() {
              render_blocks(&blocks[..preview_len])
            } else {
              render_blocks(&blocks)
            }
          }}
        </div>
        {move || {
          if is_long {
            let label = if expanded.get() { "收起" } else { "展開全文" };
            view! {
              <button
                on:click=move |_| expanded.update(|v| *v = !*v)
                style="margin-top:0.75rem;background:none;border:none;padding:0;font-size:0.875rem;cursor:pointer;color:var(--silver-dim);text-decoration:underline;text-underline-offset:3px"
              >
                {label}
              </button>
            }
              .into_any()
          } else {
            view! { <span/> }.into_any()
          }
        }}
      </div>
    }
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
            view! { <FallbackCard body=s /> }.into_any()
        } else {
            view! {
        {chapters.into_iter().map(|c| view! { <ChapterCard heading=c.heading body=c.body /> }).collect_view()}
      }
        .into_any()
        }
    }
}
