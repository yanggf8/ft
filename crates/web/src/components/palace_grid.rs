//! 十二宮格 — port of `ZiWeiPalaceGrid.tsx`.

use ft_schema::ZiWeiPalaceV3;
use leptos::prelude::*;

fn sihua_label(code: &str) -> &'static str {
    match code {
        "lu" => "祿",
        "quan" => "權",
        "ke" => "科",
        "ji" => "忌",
        _ => "",
    }
}

#[component]
pub fn ZiWeiPalaceGrid(palaces: Vec<ZiWeiPalaceV3>) -> impl IntoView {
    view! {
        <div class="palace-grid">
            {palaces
                .into_iter()
                .map(|p| {
                    let is_life = p.is_life_palace.unwrap_or(false);
                    let is_body = p.is_body_palace.unwrap_or(false);
                    let class = if is_life { "palace life" } else { "palace" };
                    let head = format!(
                        "{} {} · {}{}{}",
                        p.branch,
                        p.stem,
                        p.name,
                        if is_life { " ★命宮" } else { "" },
                        if is_body { " ·身宮" } else { "" },
                    );
                    view! {
                        <div class=class>
                            <div class="palace-head">{head}</div>
                            <div class="palace-stars">
                                {p.stars
                                    .into_iter()
                                    .map(|s| {
                                        let star_class = match s.star_type.as_str() {
                                            "main" => "star main",
                                            "transformation" => "star transformation",
                                            _ => "star",
                                        };
                                        let label = format!(
                                            "{}{}{}",
                                            s.name,
                                            s.brightness
                                                .as_deref()
                                                .map(|b| format!("({})", b))
                                                .unwrap_or_default(),
                                            s.sihua
                                                .as_deref()
                                                .map(|c| format!("化{}", sihua_label(c)))
                                                .unwrap_or_default(),
                                        );
                                        view! { <span class=star_class>{label}</span> }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}
