//! 出生資料表單 — port of `BirthDataForm.tsx`.

use ft_schema::api::{BirthDataRequest, UserProfile};
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;

/// `on_saved` fires after a successful PUT so the parent can refresh the profile.
#[component]
pub fn BirthDataForm(
    /// Existing profile values used to seed the fields (None on first entry).
    initial: Option<UserProfile>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let seed_hour = initial.as_ref().and_then(|u| u.birth_hour);
    let year = RwSignal::new(initial.as_ref().and_then(|u| u.birth_year).unwrap_or(1995));
    let month = RwSignal::new(initial.as_ref().and_then(|u| u.birth_month).unwrap_or(1));
    let day = RwSignal::new(initial.as_ref().and_then(|u| u.birth_day).unwrap_or(1));
    let hour = RwSignal::new(seed_hour.unwrap_or(12));
    let unknown_hour = RwSignal::new(seed_hour.is_none());
    let gender = RwSignal::new(
        initial
            .as_ref()
            .and_then(|u| u.gender.clone())
            .unwrap_or_default(),
    );
    let generation_tags = RwSignal::new(
        initial
            .as_ref()
            .and_then(|u| u.generation_tags.clone())
            .unwrap_or_default(),
    );
    let saving = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    // Generation tags: default from birth_year, user can toggle
    let all_gens = [
        "1940s", "1950s", "1960s", "1970s", "1980s", "1990s", "2000s", "2010s",
    ];
    let default_gen = move || {
        let y = year.get();
        let d = (y / 10) * 10;
        format!("{}s", d)
    };
    // Initialize with existing tags or default
    Effect::new(move |_| {
        if generation_tags.get().is_empty() {
            let d = default_gen();
            if all_gens.contains(&d.as_str()) {
                generation_tags.set(vec![d]);
            }
        }
    });

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(String::new());
        saving.set(true);
        spawn_local(async move {
            let g = gender.get_untracked();
            let tags = {
                let t = generation_tags.get_untracked();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            };
            let body = BirthDataRequest {
                birth_year: year.get_untracked(),
                birth_month: month.get_untracked(),
                birth_day: day.get_untracked(),
                birth_hour: if unknown_hour.get_untracked() {
                    None
                } else {
                    Some(hour.get_untracked())
                },
                gender: if g.is_empty() { None } else { Some(g) },
                generation_tags: tags,
                ..Default::default()
            };
            match api::update_birth_data(&body).await {
                Ok(_) => on_saved.run(()),
                Err(e) => error.set(e.to_string()),
            }
            saving.set(false);
        });
    };

    // Parses a numeric input, leaving the previous value in place on garbage.
    let num_input = move |sig: RwSignal<i64>| {
        move |ev: leptos::ev::Event| {
            if let Ok(v) = event_target_value(&ev).parse::<i64>() {
                sig.set(v);
            }
        }
    };

    view! {
        <form on:submit=submit class="form-col">
            <div class="form-grid3">
                <div>
                    <label class="form-label">"出生年"</label>
                    <input
                        type="number" min="1900" max="2100" required class="form-input"
                        prop:value=move || year.get()
                        on:input=num_input(year)
                    />
                </div>
                <div>
                    <label class="form-label">"月"</label>
                    <input
                        type="number" min="1" max="12" required class="form-input"
                        prop:value=move || month.get()
                        on:input=num_input(month)
                    />
                </div>
                <div>
                    <label class="form-label">"日"</label>
                    <input
                        type="number" min="1" max="31" required class="form-input"
                        prop:value=move || day.get()
                        on:input=num_input(day)
                    />
                </div>
            </div>

            <div>
                <label class="form-label">"出生時辰"</label>
                <div class="hour-row">
                    <input
                        type="number" min="0" max="23" class="form-input" style="flex:1"
                        prop:value=move || hour.get()
                        prop:disabled=move || unknown_hour.get()
                        on:input=num_input(hour)
                    />
                    <label class="hour-unknown">
                        <input
                            type="checkbox"
                            prop:checked=move || unknown_hour.get()
                            on:change=move |ev| unknown_hour.set(event_target_checked(&ev))
                        />
                        "不確定"
                    </label>
                </div>
            </div>

            <div>
                <label class="form-label">"性別 (紫微斗數需要)"</label>
                <select
                    class="form-input"
                    prop:value=move || gender.get()
                    on:change=move |ev| gender.set(event_target_value(&ev))
                >
                    <option value="">"-- 選擇 --"</option>
                    <option value="male">"男"</option>
                    <option value="female">"女"</option>
                </select>
            </div>

            <div>
                <label class="form-label">"世代標籤"</label>
                <p style="font-size:0.75rem;color:var(--silver-dim);margin-bottom:0.4rem">
                    "依出生年預設，可自選多個；未來重疊時將合寫成一段世代故事"
                </p>
                <div style="display:flex;flex-wrap:wrap;gap:0.35rem">
                    {all_gens.iter().map(|tag| {
                        let tag_str = tag.to_string();
                        let tag_clone = tag_str.clone();
                        view! {
                            <button
                                type="button"
                                class="tag-chip"
                                style=move || {
                                    let selected = generation_tags.get().contains(&tag_str);
                                    if selected {
                                        "padding:0.25rem 0.6rem;border-radius:999px;font-size:0.75rem;border:1px solid #8b5cf6;background:linear-gradient(135deg,#8b5cf6,#a78bfa);color:#10141f;cursor:pointer"
                                    } else {
                                        "padding:0.25rem 0.6rem;border-radius:999px;font-size:0.75rem;border:1px solid var(--glass-border);background:var(--glass-bg);color:var(--silver-dim);cursor:pointer"
                                    }
                                }
                                on:click=move |_| {
                                    let mut tags = generation_tags.get();
                                    if tags.contains(&tag_clone) {
                                        tags.retain(|t| t != &tag_clone);
                                        if tags.is_empty() {
                                            tags.push(default_gen());
                                        }
                                    } else {
                                        tags.push(tag_clone.clone());
                                    }
                                    generation_tags.set(tags);
                                }
                            >
                                {*tag}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            <Show when=move || !error.get().is_empty()>
                <p class="error">{move || error.get()}</p>
            </Show>

            <button type="submit" class="btn-primary" prop:disabled=move || saving.get()>
                {move || if saving.get() { "儲存中..." } else { "儲存出生資料" }}
            </button>
        </form>
    }
}
