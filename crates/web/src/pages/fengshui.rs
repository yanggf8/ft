//! 空間風水 v1 Batch 2 — `/fengshui`（公開、純前端、零 API）。
//! spec：docs/superpowers/specs/2026-11-17-fengshui-room-v1-design.md §2.5、§3、§5。
//!
//! 流程：開場（選房型）→ 問答（一題一頁、俯視示意圖可點）→ 結果（flag／mild 蓋牌、
//! 翻卡先猜 → ok／unknown 折疊 → 說明卡）。
//! - 答案只存 localStorage `fengshui_room_v1_{room}`；猜的結果只存在頁面狀態。
//! - 判定與文案全部來自 `ft_schema::fengshui`（`assess`、corpus 固定文案、evidence）；
//!   本檔不寫任何卡片內容，只負責呈現。
//! - 「可以試試看」只顯示文字，不放按鈕（§5；按鈕隨 Batch 3 觀察功能上線）。
//! - 示意圖以 `view!` 內嵌 SVG 繪製（CSP：不引外部 JS、無 inline script）；
//!   線條一律 `currentColor`，色彩由 style.css 既有 token 決定。

use std::collections::{BTreeMap, HashMap, HashSet};

use ft_schema::fengshui::corpus::{
    reveal_miss_line, reveal_score_line, EXPLAINER, EXPLAINER_TITLE, LABEL_ADJUSTMENTS,
    LABEL_CLEAR_ANSWERS, LABEL_FENGSHUI, LABEL_LOCAL_ONLY, LABEL_OK_TAG, LABEL_RESEARCH,
    LABEL_SEE_RESEARCH, LABEL_UNKNOWN_GROUP, OPENING, REVEAL_ALL_OK, REVEAL_CHANCE, REVEAL_FLIP,
    REVEAL_HIT, REVEAL_HOWTO, REVEAL_MISS, REVEAL_PROMPT, REVEAL_SKIP_ALL,
};
use ft_schema::fengshui::{
    assess, evidence_by_key, question, questions, Card, Evidence, QuestionDef, Room, Stance,
    Status, FENGSHUI_ROOM_CORPUS_VERSION, FENGSHUI_ROOM_RULESET_VERSION,
};
use leptos::prelude::*;
use leptos_router::components::A;

// ═══ localStorage ═══

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn storage_key(room: Room) -> String {
    format!("fengshui_room_v1_{}", room.wire())
}

/// 讀回答案；只留下本房型存在的題目與選項（壞資料、舊規則集殘留一律丟棄）。
fn load_answers(room: Room) -> BTreeMap<String, String> {
    let raw = storage().and_then(|s| s.get_item(&storage_key(room)).ok().flatten());
    let parsed: BTreeMap<String, String> = raw
        .and_then(|r| serde_json::from_str(&r).ok())
        .unwrap_or_default();
    parsed
        .into_iter()
        .filter(|(qid, oid)| {
            question(qid).is_some_and(|q| q.room == room && q.options.iter().any(|o| o.id == oid))
        })
        .collect()
}

fn save_answers(room: Room, answers: &BTreeMap<String, String>) {
    if let (Some(s), Ok(json)) = (storage(), serde_json::to_string(answers)) {
        let _ = s.set_item(&storage_key(room), &json);
    }
}

fn clear_all_answers() {
    if let Some(s) = storage() {
        for r in Room::ALL {
            let _ = s.remove_item(&storage_key(r));
        }
    }
}

fn scroll_top() {
    if let Some(w) = web_sys::window() {
        w.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

// ═══ 俯視示意圖（viewBox 0 0 120 90；房間框 x 10–110、y 10–80）═══

fn cls(base: &str, hl: bool) -> String {
    if hl {
        format!("{base} fs-hl")
    } else {
        base.to_string()
    }
}

fn room_box() -> AnyView {
    view! { <rect class="fs-wall" x="10" y="10" width="100" height="70"/> }.into_any()
}

fn rect_at(class: &str, x: i32, y: i32, w: i32, h: i32) -> AnyView {
    view! {
        <rect class=class.to_string() x=x.to_string() y=y.to_string()
            width=w.to_string() height=h.to_string()/>
    }
    .into_any()
}

fn seg(class: &str, x1: i32, y1: i32, x2: i32, y2: i32) -> AnyView {
    view! {
        <line class=class.to_string() x1=x1.to_string() y1=y1.to_string()
            x2=x2.to_string() y2=y2.to_string()/>
    }
    .into_any()
}

/// 牆線強調（例如「床頭後面是實牆」）。
fn wall_mark(x1: i32, y1: i32, x2: i32, y2: i32) -> AnyView {
    seg("fs-wallmark fs-hl", x1, y1, x2, y2)
}

/// 上牆的門：鉸鏈 (x,10)，門扇向下開，寬 16。
fn door_top(x: i32, hl: bool) -> AnyView {
    let d = format!("M {x} 26 A 16 16 0 0 0 {} 10", x + 16);
    view! {
        <g class=cls("fs-door", hl)>
            <line x1=x.to_string() y1="10" x2=x.to_string() y2="26"/>
            <path d=d/>
        </g>
    }
    .into_any()
}

/// 下牆的門：鉸鏈 (x,80)，門扇向上開。
fn door_bottom(x: i32, hl: bool) -> AnyView {
    let d = format!("M {x} 64 A 16 16 0 0 1 {} 80", x + 16);
    view! {
        <g class=cls("fs-door", hl)>
            <line x1=x.to_string() y1="80" x2=x.to_string() y2="64"/>
            <path d=d/>
        </g>
    }
    .into_any()
}

/// 直牆的門：鉸鏈 (x,y)；`into_left` 為門扇往左側空間開。
fn door_vert(x: i32, y: i32, into_left: bool, hl: bool) -> AnyView {
    let (lx, sweep) = if into_left { (x - 16, 0) } else { (x + 16, 1) };
    let d = format!("M {lx} {y} A 16 16 0 0 {sweep} {x} {}", y + 16);
    view! {
        <g class=cls("fs-door", hl)>
            <line x1=x.to_string() y1=y.to_string() x2=lx.to_string() y2=y.to_string()/>
            <path d=d/>
        </g>
    }
    .into_any()
}

/// 橫牆上的窗（雙線）。
fn win_h(x1: i32, x2: i32, y: i32, hl: bool) -> AnyView {
    view! {
        <g class=cls("fs-win", hl)>
            <line x1=x1.to_string() y1=(y - 2).to_string() x2=x2.to_string() y2=(y - 2).to_string()/>
            <line x1=x1.to_string() y1=(y + 2).to_string() x2=x2.to_string() y2=(y + 2).to_string()/>
        </g>
    }
    .into_any()
}

/// 直牆上的窗（雙線）。
fn win_v(x: i32, y1: i32, y2: i32, hl: bool) -> AnyView {
    view! {
        <g class=cls("fs-win", hl)>
            <line x1=(x - 2).to_string() y1=y1.to_string() x2=(x - 2).to_string() y2=y2.to_string()/>
            <line x1=(x + 2).to_string() y1=y1.to_string() x2=(x + 2).to_string() y2=y2.to_string()/>
        </g>
    }
    .into_any()
}

/// 床（床頭朝上：枕頭在上緣）。
fn bed(x: i32, y: i32, w: i32, h: i32, hl: bool) -> AnyView {
    view! {
        <g class=cls("fs-furn", hl)>
            <rect x=x.to_string() y=y.to_string() width=w.to_string() height=h.to_string()/>
            <rect x=(x + 4).to_string() y=(y + 3).to_string() width=(w - 8).to_string() height="8"/>
            <line x1=x.to_string() y1=(y + h * 2 / 5).to_string()
                x2=(x + w).to_string() y2=(y + h * 2 / 5).to_string()/>
        </g>
    }
    .into_any()
}

fn desk(x: i32, y: i32, w: i32, h: i32) -> AnyView {
    rect_at("fs-furn", x, y, w, h)
}

/// 椅子（人面朝上；椅背弧線在下方）。
fn chair(cx: i32, cy: i32) -> AnyView {
    let d = format!(
        "M {} {} Q {cx} {} {} {}",
        cx - 6,
        cy + 4,
        cy + 11,
        cx + 6,
        cy + 4
    );
    view! {
        <g class="fs-furn">
            <circle cx=cx.to_string() cy=cy.to_string() r="5"/>
            <path d=d/>
        </g>
    }
    .into_any()
}

/// 視線範圍（淡色扇形）。
fn sight(points: &str) -> AnyView {
    view! { <polygon class="fs-sight" points=points.to_string()/> }.into_any()
}

/// 外露的樑（虛線帶）。
fn beam_h(y: i32, hl: bool) -> AnyView {
    rect_at(&cls("fs-band", hl), 10, y, 100, 8)
}

/// 尖角：菱形（柱角／櫃角）。
fn sharp_corner(cx: i32, cy: i32) -> AnyView {
    let pts = format!(
        "{},{cy} {cx},{} {},{cy} {cx},{}",
        cx - 7,
        cy - 7,
        cx + 7,
        cy + 7
    );
    view! { <polygon class="fs-furn fs-hl" points=pts/> }.into_any()
}

fn boxes(pts: &[(i32, i32)], class: &str) -> Vec<AnyView> {
    pts.iter()
        .map(|&(x, y)| rect_at(class, x, y, 5, 5))
        .collect()
}

fn qmark() -> Vec<AnyView> {
    vec![
        room_box(),
        view! { <text class="fs-qmark" x="60" y="56" text-anchor="middle">"?"</text> }.into_any(),
    ]
}

/// 每題每個選項的示意圖。題目／選項 id 見 `ft_schema::fengshui::ruleset`。
fn figure(qid: &str, oid: &str) -> Vec<AnyView> {
    if oid == "unknown" {
        return qmark();
    }
    let mut v = vec![room_box()];
    match (qid, oid) {
        // ── 臥室 ──
        ("bed_back", "wall") => {
            v.push(bed(40, 12, 40, 46, false));
            v.push(wall_mark(30, 10, 90, 10));
            v.push(door_bottom(86, false));
        }
        ("bed_back", "window") => {
            v.push(bed(40, 14, 40, 46, false));
            v.push(win_h(38, 82, 10, true));
            v.push(door_bottom(86, false));
        }
        ("bed_back", "passage") => {
            v.push(bed(40, 28, 40, 46, false));
            v.push(door_top(50, true));
        }
        ("bed_back", "floating") => {
            v.push(bed(40, 26, 40, 40, true));
            v.push(seg("fs-dash fs-hl", 40, 18, 80, 18));
            v.push(door_bottom(86, false));
        }
        ("bed_door_view", "visible") => {
            v.push(sight("33,24 104,52 104,78 40,78"));
            v.push(bed(16, 12, 34, 46, false));
            v.push(door_vert(110, 58, true, true));
        }
        ("bed_door_view", "facing") => {
            v.push(sight("33,24 18,78 50,78"));
            v.push(bed(16, 12, 34, 46, false));
            v.push(door_bottom(25, true));
        }
        ("bed_door_view", "hidden") => {
            v.push(sight("33,24 14,78 64,78"));
            v.push(bed(16, 12, 34, 46, false));
            v.push(door_top(72, true));
        }
        ("bed_overhead", "none") => {
            v.push(bed(40, 16, 40, 50, false));
        }
        ("bed_overhead", "beam") => {
            v.push(bed(40, 16, 40, 50, false));
            v.push(beam_h(32, true));
        }
        ("bed_overhead", "heavy") => {
            v.push(bed(40, 16, 40, 50, false));
            v.push(rect_at("fs-band fs-hl", 42, 12, 36, 9));
            v.push(view! { <circle class="fs-band fs-hl" cx="60" cy="44" r="5"/> }.into_any());
        }
        ("bed_mirror", "none") => {
            v.push(bed(16, 12, 34, 46, false));
            v.push(seg("fs-mirror fs-hl", 72, 12, 98, 12));
        }
        ("bed_mirror", "visible") => {
            v.push(bed(16, 12, 34, 46, false));
            v.push(seg("fs-mirror fs-hl", 108, 18, 108, 44));
            v.push(seg("fs-dash fs-hl", 106, 31, 38, 20));
        }
        ("bed_corner", "none") => {
            v.push(bed(16, 12, 34, 46, false));
            v.push(rect_at("fs-furn", 96, 12, 13, 13));
        }
        ("bed_corner", "present") => {
            v.push(bed(16, 12, 34, 46, false));
            v.push(sharp_corner(64, 34));
            v.push(seg("fs-dash fs-hl", 56, 34, 51, 34));
        }
        ("bed_door_door", _) => {
            // 兩個房間夾一條走道：左邊是你的房間
            v = vec![
                rect_at("fs-wall", 10, 10, 38, 70),
                rect_at("fs-wall", 72, 10, 38, 70),
                bed(16, 14, 22, 30, false),
            ];
            if oid == "facing" {
                v.push(door_vert(48, 34, true, true));
                v.push(door_vert(72, 34, false, true));
                v.push(seg("fs-dash fs-hl", 48, 42, 72, 42));
            } else {
                v.push(door_vert(48, 20, true, true));
                v.push(door_vert(72, 56, false, false));
            }
        }
        ("bed_light", "dark") => {
            v.push(bed(16, 12, 34, 46, false));
            v.push(win_v(110, 20, 52, false));
            v.push(
                view! {
                    <polyline class="fs-curtain fs-hl"
                        points="104,18 100,23 104,28 100,33 104,38 100,43 104,48 100,53"/>
                }
                .into_any(),
            );
        }
        ("bed_light", "leak") => {
            v.push(bed(16, 12, 34, 46, false));
            v.push(win_v(110, 20, 52, true));
            v.push(seg("fs-dash fs-hl", 104, 24, 56, 28));
            v.push(seg("fs-dash fs-hl", 104, 36, 56, 40));
            v.push(seg("fs-dash fs-hl", 104, 48, 56, 52));
        }
        ("bed_clutter", lvl) => {
            v.push(bed(40, 12, 40, 46, false));
            let pts: &[(i32, i32)] = match lvl {
                "little" => &[(20, 66)],
                "some" => &[(18, 64), (28, 70), (90, 66), (60, 66)],
                _ => &[
                    (16, 62),
                    (24, 70),
                    (32, 64),
                    (18, 40),
                    (88, 20),
                    (94, 42),
                    (86, 62),
                    (98, 70),
                    (60, 66),
                    (50, 72),
                ],
            };
            v.extend(boxes(pts, "fs-box fs-hl"));
            if lvl == "lots" {
                // 床底下的雜物（虛線）
                v.extend(boxes(&[(50, 38), (64, 28), (62, 46)], "fs-band fs-hl"));
            }
        }
        // ── 書房／工作位 ──
        ("seat_back", "wall") => {
            v.push(desk(38, 40, 44, 14));
            v.push(chair(60, 64));
            v.push(wall_mark(36, 80, 84, 80));
        }
        ("seat_back", "window") => {
            v.push(desk(38, 30, 44, 14));
            v.push(chair(60, 56));
            v.push(win_h(38, 82, 80, true));
        }
        ("seat_back", "passage") => {
            v.push(desk(38, 26, 44, 14));
            v.push(chair(60, 52));
            v.push(door_bottom(66, true));
        }
        ("seat_door_view", "visible") => {
            v.push(sight("60,46 26,12 108,12 108,30"));
            v.push(desk(38, 26, 44, 14));
            v.push(chair(60, 52));
            v.push(door_vert(110, 18, true, true));
        }
        ("seat_door_view", "facing") => {
            v.push(sight("60,50 40,12 80,12"));
            v.push(desk(38, 32, 44, 12));
            v.push(chair(60, 56));
            v.push(door_top(52, true));
        }
        ("seat_door_view", "hidden") => {
            v.push(sight("60,46 30,12 90,12"));
            v.push(desk(38, 26, 44, 14));
            v.push(chair(60, 50));
            v.push(door_bottom(68, true));
        }
        ("seat_overhead", "none") => {
            v.push(desk(38, 26, 44, 14));
            v.push(chair(60, 52));
        }
        ("seat_overhead", "beam") => {
            v.push(desk(38, 26, 44, 14));
            v.push(chair(60, 52));
            v.push(beam_h(46, true));
        }
        ("seat_front", "open") => {
            v.push(sight("60,62 36,12 84,12"));
            v.push(desk(38, 44, 44, 12));
            v.push(chair(60, 68));
            v.push(win_h(38, 82, 10, true));
        }
        ("seat_front", "wall") => {
            v.push(desk(38, 11, 44, 12));
            v.push(chair(60, 32));
            v.push(wall_mark(34, 10, 86, 10));
        }
        ("seat_corner", "none") => {
            v.push(desk(38, 26, 44, 14));
            v.push(chair(60, 52));
            v.push(rect_at("fs-furn", 96, 12, 13, 13));
        }
        ("seat_corner", "present") => {
            v.push(desk(38, 26, 44, 14));
            v.push(chair(60, 52));
            v.push(sharp_corner(82, 56));
            v.push(seg("fs-dash fs-hl", 74, 56, 67, 54));
        }
        ("seat_light", "enough") => {
            v.push(desk(60, 30, 36, 14));
            v.push(chair(78, 56));
            v.push(win_v(110, 16, 60, true));
            v.push(seg("fs-dash fs-hl", 104, 20, 84, 26));
            v.push(seg("fs-dash fs-hl", 104, 50, 84, 48));
        }
        ("seat_light", "dim") => {
            v.push(desk(60, 30, 36, 14));
            v.push(chair(78, 56));
            v.push(win_v(10, 62, 76, false));
            v.push(rect_at("fs-band fs-hl", 56, 22, 44, 46));
        }
        ("seat_clutter", lvl) => {
            v.push(desk(38, 26, 44, 14));
            v.push(chair(60, 52));
            let pts: &[(i32, i32)] = match lvl {
                "little" => &[(42, 29)],
                "some" => &[(42, 29), (70, 30), (24, 60)],
                _ => &[
                    (42, 29),
                    (50, 32),
                    (62, 29),
                    (72, 32),
                    (20, 40),
                    (24, 62),
                    (90, 60),
                    (96, 40),
                    (84, 70),
                    (40, 70),
                ],
            };
            v.extend(boxes(pts, "fs-box fs-hl"));
        }
        _ => {}
    }
    v
}

// ═══ 共用小元件 ═══

fn evidence_item(e: &'static Evidence) -> impl IntoView {
    let mut meta = format!("設計：{}；", e.design);
    if let Some(n) = e.n {
        meta.push_str(&format!("人數：{n}；"));
    }
    meta.push_str(&format!("測的結果：{}；限制：{}", e.outcome, e.limitation));
    if !e.full_text_read {
        meta.push_str("（本站尚未讀到全文）");
    }
    view! {
        <li>
            <a href=e.url target="_blank" rel="noopener noreferrer">{e.citation}</a>
            <span class="fs-ref-meta">{meta}</span>
        </li>
    }
}

/// 「看研究」展開區：`refs` 依據文獻；`further` 另列延伸閱讀。
fn evidence_list(refs: &'static [&'static str], further: &'static [&'static str]) -> AnyView {
    let list = |keys: &'static [&'static str]| {
        keys.iter()
            .filter_map(|k| evidence_by_key(k))
            .map(evidence_item)
            .collect_view()
    };
    view! {
        {if refs.is_empty() {
            view! { <p class="muted fs-ref-empty">"這一項沒有可以列出的研究。"</p> }.into_any()
        } else {
            view! { <ul class="fs-ref-list">{list(refs)}</ul> }.into_any()
        }}
        {(!further.is_empty()).then(|| view! {
            <p class="fs-ref-head">"延伸閱讀"</p>
            <ul class="fs-ref-list">{list(further)}</ul>
        })}
    }
    .into_any()
}

fn answer_line(c: &Card) -> String {
    format!("你的答案：{}", c.answer_label.unwrap_or("還沒回答"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Intro,
    Quiz,
    Result,
}

/// 翻卡先猜的頁面狀態（不落 localStorage，spec §3-5）。key = `CardText::key()`。
#[derive(Clone, Copy)]
struct RevealState {
    pending: RwSignal<HashMap<String, Stance>>,
    guessed: RwSignal<HashMap<String, Stance>>,
    flipped: RwSignal<HashSet<String>>,
}

impl RevealState {
    fn new() -> Self {
        Self {
            pending: RwSignal::new(HashMap::new()),
            guessed: RwSignal::new(HashMap::new()),
            flipped: RwSignal::new(HashSet::new()),
        }
    }
    fn reset(&self) {
        self.pending.set(HashMap::new());
        self.guessed.set(HashMap::new());
        self.flipped.set(HashSet::new());
    }
}

/// 每張作答過的卡（flag／mild／ok，Batch 2.1）：蓋牌 → 先猜研究站在哪一邊 → 翻開。
fn flip_card(c: Card, rs: RevealState) -> Option<AnyView> {
    let t = c.text?;
    let badge = t.badge;
    let actual = badge.stance;
    let is_ok = c.status == Status::Ok;
    let key = t.key();
    let answer = answer_line(&c);

    let k_flip = key.clone();
    let is_flipped = move || rs.flipped.with(|f| f.contains(&k_flip));
    let is_flipped2 = is_flipped.clone();

    let k_pending = key.clone();
    let pending_of = move || rs.pending.with(|p| p.get(&k_pending).copied());

    let k_guessed = key.clone();
    let guess_line = move || {
        rs.guessed.with(|g| g.get(&k_guessed).copied()).map(|g| {
            if g == actual {
                view! { <p class="fs-verdict">{format!("✓ {REVEAL_HIT}")}</p> }.into_any()
            } else {
                view! {
                    <p class="fs-verdict">
                        {format!("✗ {REVEAL_MISS}：")}{reveal_miss_line(g, actual)}
                    </p>
                }
                .into_any()
            }
        })
    };

    let covered = {
        let key = key.clone();
        move || {
            let key = key.clone();
            let key_flip = key.clone();
            let pending_of = pending_of.clone();
            let pending_of2 = pending_of.clone();
            view! {
                <p class="fs-guess-prompt">{REVEAL_PROMPT}</p>
                <div class="fs-guess-row" role="group" aria-label=REVEAL_PROMPT>
                    {Stance::ALL.into_iter().map(|r| {
                        let key = key.clone();
                        let pending_of = pending_of.clone();
                        let sel = move || pending_of() == Some(r);
                        let sel2 = sel.clone();
                        view! {
                            <button
                                class="fs-guess"
                                class:selected=sel
                                aria-pressed=move || sel2().to_string()
                                on:click=move |_| {
                                    rs.pending.update(|p| { p.insert(key.clone(), r); });
                                }
                            >
                                {r.label()}
                            </button>
                        }
                    }).collect_view()}
                </div>
                <button
                    class="btn-primary fs-flip"
                    disabled=move || pending_of2().is_none()
                    on:click=move |_| {
                        if let Some(g) = rs.pending.with_untracked(|p| p.get(&key_flip).copied()) {
                            rs.guessed.update(|m| { m.insert(key_flip.clone(), g); });
                            rs.flipped.update(|f| { f.insert(key_flip.clone()); });
                        }
                    }
                >
                    {REVEAL_FLIP}
                </button>
            }
        }
    };

    let revealed = move || {
        view! {
            {guess_line.clone()}
            <div class="fs-badge-wrap">
                <span class="fs-badge">{actual.label()}</span>
                // 沒人測過：reach 與 limitation 只會重複同一句，研究欄第一句已說明
                {(actual != Stance::Untested).then(|| view! {
                    <span class="fs-badge fs-badge-reach">{badge.reach.badge_label()}</span>
                    <p class="fs-limit">{badge.limitation}</p>
                })}
            </div>
            <p class="fs-text">{t.research}</p>
            <details class="fs-refs">
                <summary>{LABEL_SEE_RESEARCH}</summary>
                {evidence_list(t.refs, t.further)}
            </details>
        }
    };

    Some(
        view! {
            <article class="card fs-card">
                <p class="fs-card-q">
                    <strong>{c.prompt}</strong>
                    <span class="fs-answer">{answer}</span>
                    {is_ok.then(|| view! { <span class="fs-ok-tag">{LABEL_OK_TAG}</span> })}
                </p>
                <div class="fs-cols">
                    <section class="fs-col">
                        <h3 class="fs-col-title">{LABEL_FENGSHUI}</h3>
                        <p class="fs-text">{t.fengshui}</p>
                    </section>
                    <section class="fs-col fs-col-research" class:fs-covered=move || !is_flipped2()>
                        <h3 class="fs-col-title">{LABEL_RESEARCH}</h3>
                        {move || if is_flipped() {
                            revealed.clone()().into_any()
                        } else {
                            covered.clone()().into_any()
                        }}
                    </section>
                </div>
                {(!t.adjustments.is_empty()).then(|| view! {
                    <div class="fs-adj">
                        <h4 class="fs-adj-title">{LABEL_ADJUSTMENTS}</h4>
                        <ul>
                            {t.adjustments.iter().map(|a| view! { <li>{a.text}</li> }).collect_view()}
                        </ul>
                    </div>
                })}
            </article>
        }
        .into_any(),
    )
}

// ═══ 頁面 ═══

#[component]
pub fn FengshuiPage() -> impl IntoView {
    let stage = RwSignal::new(Stage::Intro);
    let room = RwSignal::new(Room::Bedroom);
    let answers = RwSignal::new(BTreeMap::<String, String>::new());
    let idx = RwSignal::new(0usize);
    let rs = RevealState::new();
    let armed_clear = RwSignal::new(false);
    // 清除後讓開場的「已回答 n 題」重讀 localStorage
    let store_tick = RwSignal::new(0u32);

    Effect::new(move |_| {
        stage.track();
        idx.track();
        scroll_top();
    });

    let start_room = move |r: Room| {
        let a = load_answers(r);
        let first_open = questions(r).position(|q| !a.contains_key(q.id));
        room.set(r);
        answers.set(a);
        rs.reset();
        armed_clear.set(false);
        match first_open {
            Some(i) => {
                idx.set(i);
                stage.set(Stage::Quiz);
            }
            None => stage.set(Stage::Result),
        }
    };

    let do_clear = move || {
        if armed_clear.get_untracked() {
            clear_all_answers();
            answers.set(BTreeMap::new());
            rs.reset();
            armed_clear.set(false);
            store_tick.update(|n| *n += 1);
            stage.set(Stage::Intro);
        } else {
            armed_clear.set(true);
        }
    };
    let clear_label = move || {
        if armed_clear.get() {
            "確定清除兩個房型的答案？再點一次"
        } else {
            LABEL_CLEAR_ANSWERS
        }
    };

    // ── 開場 ──
    let intro = move || {
        store_tick.track();
        let any_saved = Room::ALL.into_iter().any(|r| !load_answers(r).is_empty());
        view! {
            <p class="fs-opening">{OPENING}</p>
            <p class="muted fs-local">{format!("{LABEL_LOCAL_ONLY}。")}</p>
            <h2 class="fs-h2">"選一個房間"</h2>
            <div class="fs-rooms">
                {Room::ALL.into_iter().map(|r| {
                    let total = questions(r).count();
                    let done = load_answers(r).len();
                    let note = if done == 0 {
                        format!("{total} 題")
                    } else if done < total {
                        format!("已回答 {done} / {total} 題，接著回答")
                    } else {
                        "已回答完，看結果".to_string()
                    };
                    view! {
                        <button class="fs-room" on:click=move |_| start_room(r)>
                            <span class="fs-room-name">{r.label()}</span>
                            <span class="fs-room-note">{note}</span>
                        </button>
                    }
                }).collect_view()}
            </div>
            {any_saved.then(|| view! {
                <button class="btn-link fs-clear" on:click=move |_| do_clear()>{clear_label}</button>
            })}
        }
    };

    // ── 問答 ──
    let quiz = move || {
        let r = room.get();
        let qs: Vec<&'static QuestionDef> = questions(r).collect();
        let n = qs.len();
        let i = idx.get().min(n.saturating_sub(1));
        let q = qs[i];
        let a = answers.get();
        let selected = a.get(q.id).cloned();
        let all_done = qs.iter().all(|q| a.contains_key(q.id));
        let pct = (i + 1) * 100 / n.max(1);

        let pick = move |oid: &'static str| {
            answers.update(|m| {
                m.insert(q.id.to_string(), oid.to_string());
            });
            answers.with_untracked(|m| save_answers(r, m));
            if i + 1 < n {
                idx.set(i + 1);
            } else {
                stage.set(Stage::Result);
            }
        };

        view! {
            <div class="card fs-quiz">
                <div class="fs-quiz-head">
                    <span class="muted">{r.label()}</span>
                    <p class="quiz-progress">
                        "第 "<strong>{i + 1}</strong>{format!(" / {n} 題")}
                    </p>
                </div>
                <div class="quiz-progress-bar"><span style=format!("width:{pct}%")></span></div>
                <h2 class="fs-q-prompt">{q.prompt}</h2>
                <p class="muted fs-q-hint">"點圖或文字選一個；俯視示意，不必完全一樣。"</p>
                <div class="fs-opts">
                    {q.options.iter().map(|o| {
                        let is_sel = selected.as_deref() == Some(o.id);
                        view! {
                            <button
                                class="fs-opt"
                                class:selected=is_sel
                                aria-pressed=is_sel.to_string()
                                on:click=move |_| pick(o.id)
                            >
                                <svg class="fs-fig" viewBox="0 0 120 90" aria-hidden="true">
                                    {figure(q.id, o.id)}
                                </svg>
                                <span class="fs-opt-label">{o.label}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
                <div class="fs-quiz-nav">
                    <button class="btn-link" on:click=move |_| {
                        if i == 0 { stage.set(Stage::Intro) } else { idx.set(i - 1) }
                    }>
                        {if i == 0 { "← 換房間" } else { "← 上一題" }}
                    </button>
                    {(selected.is_some() && i + 1 < n).then(|| view! {
                        <button class="btn-link" on:click=move |_| idx.set(i + 1)>"下一題 →"</button>
                    })}
                    {all_done.then(|| view! {
                        <button class="btn-link" on:click=move |_| stage.set(Stage::Result)>"看結果 →"</button>
                    })}
                </div>
            </div>
        }
    };

    // ── 結果 ──
    let result = move || {
        let r = room.get();
        let cards = answers.with(|a| assess(r, a));
        let qs: Vec<&'static str> = questions(r).map(|q| q.id).collect();

        // 作答過的每一題都能翻（flag → mild → ok，assess 已排序）
        let answered: Vec<Card> = cards
            .iter()
            .filter(|c| c.status != Status::Unknown)
            .cloned()
            .collect();
        let has_actionable = answered.iter().any(|c| c.status.actionable());
        let unknowns: Vec<Card> = cards
            .iter()
            .filter(|c| c.status == Status::Unknown)
            .cloned()
            .collect();

        // (卡片 key, 實際立場)：計分與「全部直接翻開」用
        let keys: Vec<(String, Stance)> = answered
            .iter()
            .filter_map(|c| {
                let t = c.text?;
                Some((t.key(), t.badge.stance))
            })
            .collect();
        let keys_skip = keys.clone();
        let keys_score = keys.clone();
        let has_cards = !keys.is_empty();

        let any_covered = move || {
            rs.flipped
                .with(|f| keys.iter().any(|(k, _)| !f.contains(k)))
        };
        let score = move || {
            let all_flipped = rs
                .flipped
                .with(|f| keys_score.iter().all(|(k, _)| f.contains(k)));
            if !all_flipped || keys_score.is_empty() {
                return None;
            }
            let (guessed, correct) = rs.guessed.with(|g| {
                keys_score
                    .iter()
                    .filter_map(|(k, actual)| g.get(k).map(|x| x == actual))
                    .fold((0usize, 0usize), |(n, ok), hit| (n + 1, ok + hit as usize))
            });
            let skipped = keys_score.len() - guessed;
            Some(view! {
                <div class="card fs-score">
                    {if guessed > 0 {
                        view! {
                            <p class="fs-score-line">{reveal_score_line(correct, guessed)}</p>
                            <p class="muted">{REVEAL_CHANCE}</p>
                        }.into_any()
                    } else {
                        view! { <p class="fs-score-line">"全部翻開了"</p> }.into_any()
                    }}
                    {(skipped > 0 && guessed > 0).then(|| view! {
                        <p class="muted">{format!("直接翻開的 {skipped} 張不列入計算。")}</p>
                    })}
                </div>
            })
        };

        view! {
            <div class="fs-result-head">
                <h2 class="fs-h2">{format!("{}的結果", r.label())}</h2>
                <div class="fs-result-actions">
                    <button class="btn-link" on:click=move |_| {
                        idx.set(0);
                        stage.set(Stage::Quiz);
                    }>"修改答案"</button>
                    <button class="btn-link" on:click=move |_| stage.set(Stage::Intro)>"換房間"</button>
                </div>
            </div>

            {has_cards.then(|| {
                view! {
                    {(!has_actionable).then(|| view! {
                        <div class="card"><p class="fs-text">{REVEAL_ALL_OK}</p></div>
                    })}
                    <p class="muted fs-howto">{REVEAL_HOWTO}</p>
                    <Show when=any_covered.clone()>
                        <button class="btn-link fs-skip" on:click={
                            let keys_skip = keys_skip.clone();
                            move |_| {
                                rs.flipped.update(|f| {
                                    for (k, _) in &keys_skip { f.insert(k.clone()); }
                                });
                            }
                        }>{REVEAL_SKIP_ALL}</button>
                    </Show>
                    {answered.into_iter().filter_map(|c| flip_card(c, rs)).collect_view()}
                }
            })}

            {score}

            {(!unknowns.is_empty()).then(|| {
                let n = unknowns.len();
                let qs = qs.clone();
                view! {
                    <details class="card fs-group">
                        <summary>{format!("{LABEL_UNKNOWN_GROUP}（{n} 項）")}</summary>
                        {unknowns.into_iter().map(|c| {
                            let pos = qs.iter().position(|id| *id == c.item_id).unwrap_or(0);
                            let answer = answer_line(&c);
                            view! {
                                <div class="fs-mini">
                                    <p class="fs-card-q">
                                        <strong>{c.prompt}</strong>
                                        <span class="fs-answer">{answer}</span>
                                    </p>
                                    <button class="btn-link" on:click=move |_| {
                                        idx.set(pos);
                                        stage.set(Stage::Quiz);
                                    }>"回去回答這題"</button>
                                </div>
                            }
                        }).collect_view()}
                    </details>
                }
            })}

            <section class="card fs-explainer">
                <h2 class="fs-h2">{EXPLAINER_TITLE}</h2>
                {EXPLAINER.iter().map(|p| view! { <p class="fs-text">{*p}</p> }).collect_view()}
            </section>

            <button class="btn-link fs-clear" on:click=move |_| do_clear()>{clear_label}</button>
            <p class="muted fs-version">
                {format!(
                    "規則集版本 {FENGSHUI_ROOM_RULESET_VERSION}・語料版本 {FENGSHUI_ROOM_CORPUS_VERSION}・{LABEL_LOCAL_ONLY}。"
                )}
            </p>
        }
    };

    view! {
        <div class="page fs-page">
            <A href="/" attr:class="back-link">"← 返回"</A>
            <h1 style="margin-bottom:0.5rem">"空間風水"</h1>
            {move || match stage.get() {
                Stage::Intro => intro().into_any(),
                Stage::Quiz => quiz().into_any(),
                Stage::Result => result().into_any(),
            }}
        </div>
    }
}
