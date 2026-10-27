//! 今日抽牌頁（v1a）— 免登入、零 API：決定性抽牌在瀏覽器內完成，不連線。
//! 種子 = 台北日期（`ft_schema::tarot::taipei_today`）＋ localStorage 裝置種子
//! （32 hex）；同裝置同日同牌、換日自動換牌。明文不防重擲——算命娛樂站定位，
//! 抽牌是體驗不是契約（清儲存重抽屬使用者自由）。

use ft_schema::tarot::{self, DrawnCard, TAROT_CORPUS_VERSION, TAROT_DRAW_ALGO_VERSION};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

const SEED_KEY: &str = "tarot_seed";

/// 16 bytes 等效的裝置種子（32 hex 字）。crypto 不可用時 fallback
/// `Math.random`——種子為體驗用途非安全邊界，與伺服端 CSPRNG 紀律不衝突。
fn generate_seed() -> String {
    if let Some(crypto) = web_sys::window().and_then(|w| w.crypto().ok()) {
        // random_uuid() = 36 字（含 4 個連字號）→ 去 '-' 後恰為 32 hex
        return crypto.random_uuid().replace('-', "");
    }
    let hex = "0123456789abcdef".as_bytes();
    let mut out = String::with_capacity(32);
    for _ in 0..32 {
        let i = (js_sys::Math::random() * 16.0) as usize % 16;
        out.push(hex[i] as char);
    }
    out
}

fn read_stored_seed() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let v = storage.get_item(SEED_KEY).ok()??;
    (v.len() == 32 && v.chars().all(|c| c.is_ascii_hexdigit())).then_some(v)
}

fn store_seed(seed: &str) {
    let _ = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .and_then(|s| s)
        .map(|s| s.set_item(SEED_KEY, seed));
}

/// 本次工作階段的種子：優先 localStorage；隱私模式等存不進去時退回
/// in-memory（重載後換牌——設計明文的降級取捨）。
fn session_seed() -> String {
    thread_local! {
        static SEED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    }
    SEED.with(|s| {
        if let Some(v) = s.borrow().clone() {
            return v;
        }
        let v = read_stored_seed().unwrap_or_else(|| {
            let g = generate_seed();
            store_seed(&g);
            g
        });
        *s.borrow_mut() = Some(v.clone());
        v
    })
}

fn taipei_date_now() -> Option<String> {
    let iso = js_sys::Date::new_0().to_iso_string().as_string()?;
    tarot::taipei_today(&iso)
}

/// 今日牌（決定性）：seed = `"{YYYY-MM-DD}|{device_seed}"`（43 bytes ≥ 32）。
fn draw_today() -> Option<DrawnCard> {
    let date = taipei_date_now()?;
    let seed = format!("{date}|{}", session_seed());
    tarot::daily_draw(seed.as_bytes())
}

/// 大阿爾克那顯示羅馬數字（0 為愚者）、小阿爾克那顯示花色記號＋序號。
fn card_symbol(id: &str) -> String {
    let Some((suit, num)) = id.split_once('_') else {
        return "?".to_string();
    };
    let n: u8 = num.parse().unwrap_or(0);
    match suit {
        "major" => {
            if n == 0 {
                "0".to_string()
            } else {
                const VALS: [(u32, &str); 5] =
                    [(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];
                let mut n = u32::from(n);
                let mut s = String::new();
                for (v, sym) in VALS {
                    while n >= v {
                        s.push_str(sym);
                        n -= v;
                    }
                }
                s
            }
        }
        "wands" => format!("杖 {n}"),
        "cups" => format!("杯 {n}"),
        "swords" => format!("劍 {n}"),
        _ => format!("幣 {n}"),
    }
}

#[component]
pub fn TarotPage() -> impl IntoView {
    let auth = crate::auth::use_auth();
    let draw = RwSignal::new(draw_today());
    let flipped = RwSignal::new(false);
    let date = taipei_date_now().unwrap_or_default();

    view! {
        <div class="page">
            <A href="/" attr:class="back-link">"← 返回"</A>
            <h1 style="margin-bottom:0.5rem">"今日抽牌"</h1>
            <p class="muted" style="margin-bottom:1.5rem">
                "點開牌面，看看今天帶來什麼訊息。抽牌完全在你的瀏覽器完成，不會連線、不會儲存。"
            </p>

            {move || match draw.get() {
                None => view! {
                    <div class="card">
                        <p class="error">"無法取得今日時間，請確認裝置時鐘後重新整理。"</p>
                    </div>
                }.into_any(),
                Some(d) => {
                    let meaning = if d.reversed { d.card.reversed } else { d.card.upright };
                    let orientation = if d.reversed { "逆位" } else { "正位" };
                    let aria_flipped =
                        format!("今日牌：{}，{}", d.card.name, orientation);
                    // 分支內組好（每次呼叫新建）：footer 閉包 move 分支本地值，
                    // 外層閉包只 borrow `date` —— 保持 Fn（leptos view 要求）。
                    let footer = format!(
                        "依台北時間每日更新（今天：{date}）；語料版本 {}、抽牌演算法版本 {}。",
                        TAROT_CORPUS_VERSION, TAROT_DRAW_ALGO_VERSION
                    );
                    view! {
                        <button
                            class="tarot-card"
                            class:flipped=move || flipped.get()
                            on:click=move |_| flipped.set(true)
                            aria-label=move || if flipped.get() {
                                aria_flipped.clone()
                            } else {
                                "翻開今日牌".to_string()
                            }
                        >
                            <div class="tarot-inner">
                                <div class="tarot-face tarot-back" aria-hidden="true">
                                    <span class="tarot-back-mark">"✶"</span>
                                    <span class="tarot-back-brand">"FortuneT"</span>
                                </div>
                                <div class="tarot-face tarot-front">
                                    <span class="tarot-symbol" class:reversed=d.reversed>
                                        {card_symbol(d.card.id)}
                                    </span>
                                    <span class="tarot-name">{d.card.name}</span>
                                    <span class="tarot-badge">{orientation}</span>
                                </div>
                            </div>
                        </button>

                        <Show when=move || flipped.get()>
                            <div class="card" style="margin-top:1.5rem">
                                <p style="margin:0 0 0.5rem 0">
                                    <span class="star">{d.card.name}</span>
                                    <span class="muted" style="margin-left:0.5rem">
                                        {d.card.keywords}
                                    </span>
                                </p>
                                <p style="margin:0">{meaning}</p>
                                <p class="muted" style="font-size:0.8rem;margin:1rem 0 0 0">
                                    {footer.clone()}
                                </p>
                            </div>
                        </Show>
                    }.into_any()
                }
            }}

            <div style="margin-top:1.5rem">
                <Show when=move || auth.is_authed()>
                    <A href="/tarot/spread" attr:class="cta-alt">"前往三張牌陣・完整解牌（存紀錄）"</A>
                </Show>
                <Show when=move || !auth.is_authed()>
                    <A href="/login?next=/tarot/spread" attr:class="cta-alt">"登入後體驗三張牌陣解牌"</A>
                </Show>
            </div>
        </div>
    }
}

// ═══ v1b：三張牌陣（登入）═══

const SPREAD_LABELS: [&str; 3] = ["現況", "阻礙", "建議"];

fn tarot_request_id() -> Option<String> {
    web_sys::window()?
        .crypto()
        .ok()
        .map(|crypto| crypto.random_uuid())
}

fn draw_error_message(e: &crate::api::ApiErr) -> String {
    match e {
        crate::api::ApiErr::Api { status: 429, .. } => {
            "抽牌次數暫時達到上限，請稍後再試。".to_string()
        }
        crate::api::ApiErr::Api { status: 409, .. } => {
            "偵測到重複的抽牌請求，請再抽一次。".to_string()
        }
        _ => "抽牌失敗，請稍後再試。".to_string(),
    }
}

#[component]
pub fn TarotSpreadPage() -> impl IntoView {
    use crate::api;
    use ft_schema::tarot::{TarotDrawRequest, TarotReadingView};

    let question = RwSignal::new(String::new());
    let reading = RwSignal::new(None::<TarotReadingView>);
    // 每張牌獨立翻（on:click），全翻開才顯示整體解讀
    let revealed = RwSignal::new([false, false, false]);
    let drawing = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    let items = RwSignal::new(Vec::<TarotReadingView>::new());
    let next_cursor = RwSignal::new(None::<String>);
    let history_error = RwSignal::new(String::new());
    let loading_history = RwSignal::new(false);
    // 兩段式刪除確認（無 timer）：Some(id)＝已上膛、再點才執行；"__all__"＝整批
    let armed_delete = RwSignal::new(None::<String>);

    let refresh_history = move || {
        let items = items;
        let next_cursor = next_cursor;
        let history_error = history_error;
        spawn_local(async move {
            match api::tarot_history(None).await {
                Ok(page) => {
                    items.set(page.items);
                    next_cursor.set(page.next_cursor);
                    history_error.set(String::new());
                }
                Err(_) => history_error.set("無法載入歷史紀錄。".to_string()),
            }
        });
    };

    Effect::new(move |_| {
        let _ = refresh_history;
        // 掛載即載第一頁
        spawn_local(async move {
            match api::tarot_history(None).await {
                Ok(page) => {
                    items.set(page.items);
                    next_cursor.set(page.next_cursor);
                }
                Err(_) => history_error.set("無法載入歷史紀錄。".to_string()),
            }
        });
    });

    let on_draw = move |_| {
        if drawing.get_untracked() {
            return;
        }
        let Some(request_id) = tarot_request_id() else {
            error.set("無法產生請求識別碼，請重新整理頁面。".to_string());
            return;
        };
        let trimmed = question.get_untracked().trim().to_string();
        let body = TarotDrawRequest {
            spread: "three".to_string(),
            question: (!trimmed.is_empty()).then_some(trimmed),
            request_id,
        };
        drawing.set(true);
        error.set(String::new());
        spawn_local(async move {
            let result = api::tarot_draw(&body).await;
            drawing.set(false);
            match result {
                Ok(view) => {
                    reading.set(Some(view));
                    revealed.set([false, false, false]);
                    refresh_history();
                }
                Err(e) => error.set(draw_error_message(&e)),
            }
        });
    };

    let question_count = Memo::new(move |_| question.get().chars().count());
    let all_revealed = Memo::new(move |_| revealed.get().iter().all(|b| *b));

    view! {
        <div class="page">
            <A href="/tarot" attr:class="back-link">"← 今日抽牌"</A>
            <h1 style="margin-bottom:0.5rem">"三張牌陣"</h1>
            <p class="muted" style="margin-bottom:1.5rem">
                "輸入想問的事（可留空），抽「現況・阻礙・建議」三張牌，逐一翻開後閱讀整體解牌。抽牌由伺服器進行並存入你的紀錄。"
            </p>

            <div class="card">
                <div class="field">
                    <label>"想問什麼？（選填）"</label>
                    <input class="form-input"
                        prop:value=move || question.get()
                        on:input=move |ev| question.set(event_target_value(&ev))
                        maxlength="200"
                        placeholder="例如：最近的職涯選擇"
                    />
                    <p class="muted" style="margin:0.35rem 0 0;font-size:0.85rem">
                        {move || format!("{}/200", question_count.get())}
                    </p>
                </div>
                <button class="btn-primary" on:click=on_draw>
                    {move || if drawing.get() { "洗牌中…" } else { "抽三張牌" }}
                </button>
                <Show when=move || !error.get().is_empty()>
                    <p class="error" style="margin-top:0.75rem">{move || error.get()}</p>
                </Show>
            </div>

            {move || reading.get().map(|r| {
                let question_note = r.question.clone();
                let interpretation = r.interpretation.clone();
                let corpus_version = r.corpus_version.clone();
                let cards = r.cards.clone();
                view! {
                    <div class="tarot-trio" role="group" aria-label="三張牌陣結果">
                        {cards.into_iter().enumerate().map(|(i, c)| {
                            let reveal_i = move |_| {
                                let mut v = revealed.get_untracked();
                                if i < 3 { v[i] = true; }
                                revealed.set(v);
                            };
                            let is_revealed = move || revealed.get()[i.min(2)];
                            view! {
                                <div class="tarot-slot">
                                    <span class="tarot-slot-label">{SPREAD_LABELS[i]}</span>
                                    <button
                                        class="tarot-card tarot-card--sm"
                                        class:flipped=is_revealed
                                        on:click=reveal_i
                                        aria-label=move || if is_revealed() {
                                            format!("{}，{}", c.name, if c.reversed { "逆位" } else { "正位" })
                                        } else {
                                            format!("翻開{}", SPREAD_LABELS[i])
                                        }
                                    >
                                        <div class="tarot-inner">
                                            <div class="tarot-face tarot-back" aria-hidden="true">
                                                <span class="tarot-back-mark">"✶"</span>
                                            </div>
                                            <div class="tarot-face tarot-front">
                                                <span class="tarot-symbol" class:reversed=c.reversed>
                                                    {card_symbol(&c.id.clone())}
                                                </span>
                                                <span class="tarot-name">{c.name.clone()}</span>
                                                <span class="tarot-badge">
                                                    {if c.reversed { "逆位" } else { "正位" }}
                                                </span>
                                            </div>
                                        </div>
                                    </button>
                                </div>
                            }.into_any()
                        }).collect_view()}
                    </div>

                    {move || all_revealed.get().then(|| {
                        let q_note = question_note.clone();
                        view! {
                            <div class="card" style="margin-top:1.5rem">
                                <h2 style="margin-bottom:0.75rem">"解牌"</h2>
                                {q_note.map(|q| view! {
                                    <p class="muted" style="margin:0 0 0.75rem 0">{format!("「{q}」")}</p>
                                }.into_any())}
                                <p style="white-space:pre-line;margin:0">{interpretation.clone()}</p>
                                <p class="muted" style="font-size:0.8rem;margin:1rem 0 0 0">
                                    {format!("語料版本 {}", corpus_version.clone())}
                                </p>
                            </div>
                        }.into_any()
                    })}
                }.into_any()
            })}

            <h2 style="margin:2rem 0 0.75rem 0">"歷史紀錄"</h2>
            <Show when=move || !history_error.get().is_empty()>
                <p class="error">{move || history_error.get()}</p>
            </Show>
            <Show
                when=move || !items.get().is_empty()
                fallback=|| view! { <p class="muted">"還沒有紀錄。抽一次牌吧。"</p> }
            >
                <div style="display:flex;justify-content:flex-end">
                    {move || {
                        let armed = armed_delete;
                        let is_armed = move || armed.get_untracked().as_deref() == Some("__all__");
                        view! {
                            <button
                                class="btn-link"
                                on:click=move |_| {
                                    if is_armed() {
                                        armed_delete.set(None);
                                        spawn_local(async move {
                                            if api::tarot_delete_all().await.is_ok() {
                                                items.set(Vec::new());
                                                next_cursor.set(None);
                                            } else {
                                                history_error.set("刪除失敗，請稍後再試。".to_string());
                                            }
                                        });
                                    } else {
                                        armed_delete.set(Some("__all__".to_string()));
                                    }
                                }
                            >
                                {move || if armed.get_untracked().as_deref() == Some("__all__") {
                                    "確定刪除全部紀錄？再點一次執行".to_string()
                                } else {
                                    "刪除全部紀錄".to_string()
                                }}
                            </button>
                        }
                    }}
                </div>
                {move || items.get().into_iter().map(|item| {
                    let id = item.id.clone();
                    let id_label = id.clone();
                    let mut chips = Vec::new();
                    for (i, c) in item.cards.iter().enumerate() {
                        chips.push(format!("{}：{}（{}）", SPREAD_LABELS[i], c.name, if c.reversed { "逆位" } else { "正位" }));
                    }
                    let interpretation = item.interpretation.clone();
                    let question_line = item.question.clone()
                        .map(|q| format!("「{q}」"))
                        .unwrap_or_else(|| "無特定問題".to_string());
                    let date = item.created_at.split_once('T').map(|(d, _)| d.to_string()).unwrap_or_else(|| item.created_at.clone());
                    let armed_delete = armed_delete;
                    view! {
                        <div class="card" style="margin-top:1rem">
                            <p class="muted" style="margin:0 0 0.25rem 0;font-size:0.85rem">
                                {format!("{} ・ {}", date, question_line)}
                            </p>
                            <p style="margin:0 0 0.5rem 0">{chips.join("　")}</p>
                            <p class="muted" style="white-space:pre-line;margin:0">{interpretation}</p>
                            <div style="margin-top:0.5rem">
                                <button class="btn-link" on:click=move |_| {
                                    let this_id = id.clone();
                                    if armed_delete.get_untracked().as_deref() == Some(this_id.as_str()) {
                                        armed_delete.set(None);
                                        let id_for_del = id.clone();
                                        spawn_local(async move {
                                            if api::tarot_delete(&id_for_del).await.is_ok() {
                                                items.update(|v| v.retain(|r| r.id != id_for_del));
                                            } else {
                                                history_error.set("刪除失敗，請稍後再試。".to_string());
                                            }
                                        });
                                    } else {
                                        armed_delete.set(Some(id.clone()));
                                    }
                                }>
                                    {move || if armed_delete.get_untracked().as_deref() == Some(id_label.as_str()) {
                                        "確定刪除？再點一次".to_string()
                                    } else {
                                        "刪除".to_string()
                                    }}
                                </button>
                            </div>
                        </div>
                    }.into_any()
                }).collect_view()}
                {move || next_cursor.get().map(|cursor| {
                    let next_cursor = next_cursor;
                    view! {
                        <div style="margin-top:1rem">
                            <button class="btn-link" on:click=move |_| {
                                if loading_history.get_untracked() { return; }
                                loading_history.set(true);
                                let cursor = cursor.clone();
                                let items = items;
                                let next_cursor = next_cursor;
                                let history_error = history_error;
                                let loading_history = loading_history;
                                spawn_local(async move {
                                    match api::tarot_history(Some(&cursor)).await {
                                        Ok(page) => {
                                            items.update(|v| v.extend(page.items));
                                            next_cursor.set(page.next_cursor);
                                        }
                                        Err(_) => history_error.set("載入更多失敗。".to_string()),
                                    }
                                    loading_history.set(false);
                                });
                            }>"載入更多"</button>
                        </div>
                    }.into_any()
                })}
            </Show>
        </div>
    }
}
