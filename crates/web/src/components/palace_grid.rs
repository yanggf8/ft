//! 十二宮格 — port of `ZiWeiPalaceGrid.tsx`.
//!
//! 選用 `overlay`（流年大運，spec 2026-11-21 §A.4）：大限命宮／童限、流年命宮、小限徽章
//! 以**地支**定位；流年四化在對應星曜旁標「流祿／流權／流科／流忌」（與生年「化X」分開）；
//! 各宮上方加「流年○○」宮名；中心格顯示該年干支、虛歲、大限範圍、流年四化。
//! 不傳 overlay＝原本的命盤頁行為。

use ft_schema::ziwei_luck::ZiWeiHoroscopeV1;
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

/// 流年大運疊層資料。
#[derive(Debug, Clone, PartialEq)]
pub struct LuckOverlay {
    pub horoscope: ZiWeiHoroscopeV1,
}

impl LuckOverlay {
    /// 某地支宮位上的徽章：`(css class, 文字)`。
    fn badges(&self, branch: &str) -> Vec<(&'static str, &'static str)> {
        let h = &self.horoscope;
        let mut out = Vec::new();
        if h.decadal.branch == branch {
            out.push((
                "zw-badge dec",
                if h.decadal.is_childhood {
                    "童限"
                } else {
                    "大限命宮"
                },
            ));
        }
        if h.yearly.branch == branch {
            out.push(("zw-badge", "流年命宮"));
        }
        if h.age.branch == branch {
            out.push(("zw-badge age", "小限"));
        }
        out
    }

    fn yearly_name(&self, branch: &str) -> Option<String> {
        self.horoscope
            .yearly
            .palace_names
            .iter()
            .find(|b| b.branch == branch)
            .map(|b| format!("流年{}", b.name))
    }

    fn flow_tag(&self, star: &str) -> Option<&'static str> {
        self.horoscope
            .yearly
            .mutagen
            .labeled()
            .into_iter()
            .find(|(_, s)| *s == star)
            .map(|(label, _)| label)
    }
}

#[component]
pub fn ZiWeiPalaceGrid(
    palaces: Vec<ZiWeiPalaceV3>,
    #[prop(optional)] overlay: Option<LuckOverlay>,
) -> impl IntoView {
    // Traditional 4x4 layout, center 2x2 empty. Map branch to grid position.
    // Order: 寅(0,3) 卯(0,2) 辰(0,1) 巳(0,0) 午(1,0) 未(2,0) 申(3,0) 酉(3,1) 戌(3,2) 亥(3,3) 子(2,3) 丑(1,3)
    // For CSS grid 4x4, positions 5,6,9,10 are center.
    let order = [
        "巳", "午", "未", "申", "酉", "戌", "亥", "子", "丑", "寅", "卯", "辰",
    ];
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
    let grid_class = if overlay.is_some() {
        "palace-grid zw-luck"
    } else {
        "palace-grid"
    };
    let center = match &overlay {
        Some(o) => {
            let h = &o.horoscope;
            let decadal = if h.decadal.is_childhood {
                "童限".to_string()
            } else {
                match (h.decadal.start_age, h.decadal.end_age) {
                    (Some(s), Some(e)) => {
                        format!("大限 {s}–{e} {}{}", h.decadal.stem, h.decadal.branch)
                    }
                    _ => format!("大限 {}{}", h.decadal.stem, h.decadal.branch),
                }
            };
            let flows = h
                .yearly
                .mutagen
                .labeled()
                .iter()
                .map(|(label, star)| format!("{star}{}", label.trim_start_matches('流')))
                .collect::<Vec<_>>()
                .join("・");
            view! {
                <div class="zw-center">
                    <div class="zw-center-big">
                        {format!("{} {}{}年", h.luck_year, h.yearly.stem, h.yearly.branch)}
                    </div>
                    <div>{format!("虛歲 {}・{}", h.nominal_age, decadal)}</div>
                    <div>{format!("流年四化：{flows}")}</div>
                </div>
            }
            .into_any()
        }
        // Center block for overall info (could show birth summary)
        None => view! {
            <div style="grid-row:2/4;grid-column:2/4;background:var(--surface-tint);border:1px solid var(--glass-border);border-radius:8px;display:flex;align-items:center;justify-content:center;color:var(--silver-dim);font-size:0.75rem">
                "命盤中心"
            </div>
        }
        .into_any(),
    };

    view! {
        <div class=grid_class>
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
                    let yname = overlay.as_ref().and_then(|o| o.yearly_name(&palace.branch));
                    let badges = overlay
                        .as_ref()
                        .map(|o| o.badges(&palace.branch))
                        .unwrap_or_default();
                    let ov = overlay.clone();
                    Some(view! {
                        <div class=class style=format!("grid-row:{};grid-column:{};display:flex;flex-direction:column", r, c)>
                            <div class="palace-head">{head}</div>
                            {yname.map(|n| view! { <div class="zw-yname">{n}</div> })}
                            {(!badges.is_empty()).then(|| view! {
                                <div class="zw-badges">
                                    {badges
                                        .into_iter()
                                        .map(|(cls, text)| view! { <span class=cls>{text}</span> })
                                        .collect_view()}
                                </div>
                            })}
                            <div class="palace-stars">
                                {palace.stars
                                    .into_iter()
                                    .map(|s| {
                                        let star_class = match s.star_type.as_str() {
                                            "main" => "star main",
                                            "transformation" => "star transformation",
                                            _ => "star",
                                        };
                                        let flow = ov.as_ref().and_then(|o| o.flow_tag(&s.name));
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
                                        view! {
                                            <span class=star_class>{label}</span>
                                            {flow.map(|f| view! { <span class="zw-flow">{f}</span> })}
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    })
                })
                .collect_view()}
            {center}
        </div>
    }
}
