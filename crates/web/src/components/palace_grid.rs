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
    // Traditional 4x4 layout, center 2x2 empty. Map branch to grid position.
    // Order: 寅(0,3) 卯(0,2) 辰(0,1) 巳(0,0) 午(1,0) 未(2,0) 申(3,0) 酉(3,1) 戌(3,2) 亥(3,3) 子(2,3) 丑(1,3)
    // For CSS grid 4x4, positions 5,6,9,10 are center.
    let order = ["巳", "午", "未", "申", "酉", "戌", "亥", "子", "丑", "寅", "卯", "辰"];
    let mut sorted: Vec<Option<ZiWeiPalaceV3>> = vec![None; 12];
    for p in palaces {
        if let Some(idx) = order.iter().position(|b| *b == p.branch) {
            sorted[idx] = Some(p);
        }
    }
    // Grid positions for 12 palaces in 4x4 (row-major, skipping center 4)
    let grid_positions = [
        (1, 1), // 巳
        (1, 2), // 午
        (1, 3), // 未
        (1, 4), // 申
        (2, 4), // 酉
        (3, 4), // 戌
        (4, 4), // 亥
        (4, 3), // 子
        (4, 2), // 丑
        (4, 1), // 寅
        (3, 1), // 卯
        (2, 1), // 辰
    ];

    view! {
        <div class="palace-grid" style="display:grid;grid-template-columns:repeat(4,1fr);grid-template-rows:repeat(4,1fr);gap:6px;aspect-ratio:1">
            {sorted
                .into_iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    let palace = p?;
                    let (r, c) = grid_positions[i];
                    let is_life = palace.is_life_palace.unwrap_or(false);
                    let is_body = palace.is_body_palace.unwrap_or(false);
                    let class = if is_life { "palace life" } else { "palace" };
                    let head = format!(
                        "{} {} · {}{}{}",
                        palace.branch,
                        palace.stem,
                        palace.name,
                        if is_life { " ★命宮" } else { "" },
                        if is_body { " ·身宮" } else { "" },
                    );
                    Some(view! {
                        <div class=class style=format!("grid-row:{};grid-column:{};display:flex;flex-direction:column", r, c)>
                            <div class="palace-head">{head}</div>
                            <div class="palace-stars">
                                {palace.stars
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
                    })
                })
                .collect_view()}
            // Center block for overall info (could show birth summary)
            <div style="grid-row:2/4;grid-column:2/4;background:rgba(255,255,255,0.04);border:1px solid var(--glass-border);border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--silver-dim);font-size:0.75rem">
                "命盤中心"
            </div>
        </div>
    }
}
