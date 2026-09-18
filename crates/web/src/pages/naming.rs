//! 姓名學頁（五格剖象＋三才）— 免登入、零 API：輸入姓名，同步呼叫
//! `ft_schema::naming::analyze` 在瀏覽器內計算（輸入不會傳送或儲存）。

use ft_schema::naming::{self, luck, strokes, CharStrokes, Grid, NamingError, NamingReport};
use leptos::prelude::*;
use leptos_router::components::A;

fn error_message(e: &NamingError) -> String {
    match e {
        NamingError::EmptySurname => "請輸入姓氏".to_string(),
        NamingError::EmptyGivenName => "請輸入名字".to_string(),
        NamingError::SurnameTooLong(n) => format!("姓氏請輸入 1–2 個字（目前 {n} 字）"),
        NamingError::GivenTooLong(n) => format!("名字請輸入 1–2 個字（目前 {n} 字）"),
        NamingError::NonCjk(chars) => {
            format!(
                "僅接受中文漢字，以下字元不支援：{}",
                chars.iter().collect::<String>()
            )
        }
        NamingError::UnknownChars(chars) => format!(
            "以下字不在康熙筆畫資料中：{} — 請改用繁體常用字",
            chars.iter().collect::<String>()
        ),
    }
}

/// 即時筆畫預覽：「王(4) 小(3) 明(8)」，未知字顯示「？」。
fn preview_line(field: &str) -> String {
    let trimmed = field.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed
        .chars()
        .map(|ch| match strokes::kangxi_strokes(ch) {
            Some(v) => format!("{ch}({v})"),
            None => format!("{ch}(？)"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn stroke_text(chars: &[CharStrokes]) -> String {
    chars
        .iter()
        .map(|cs| format!("{}({})", cs.ch, cs.strokes))
        .collect::<Vec<_>>()
        .join(" ")
}

fn grid_row(g: &Grid, single_given: bool) -> impl IntoView + '_ {
    let entry = luck::luck_entry(g.luck_index);
    let note = match g.kind {
        naming::GridKind::Heaven => "（祖傳之格，不單獨論吉凶）",
        // 單名外格固定 2（虛畫 +1）；雙名第二字 1 畫（一/乙）外格也是 2，不可用筆畫判斷
        naming::GridKind::Outer if single_given => "（單名固定虛畫，意義有限）",
        _ => "",
    };
    view! {
        <tr>
            <td style="padding:0.5rem 0.75rem">{g.kind.label()}</td>
            <td style="padding:0.5rem 0.75rem">{format!("{} · {}", g.strokes, entry.name)}</td>
            <td style="padding:0.5rem 0.75rem">{g.element.label()}</td>
            <td style="padding:0.5rem 0.75rem"><span class="star">{entry.class.label()}</span></td>
            <td style="padding:0.5rem 0.75rem">{format!("{}{}", entry.text, note)}</td>
        </tr>
    }
}

#[component]
pub fn NamingPage() -> impl IntoView {
    let surname = RwSignal::new(String::new());
    let given = RwSignal::new(String::new());
    let error = RwSignal::new(String::new());
    let report = RwSignal::new(None::<NamingReport>);

    let surname_preview = Memo::new(move |_| preview_line(&surname.get()));
    let given_preview = Memo::new(move |_| preview_line(&given.get()));

    let on_submit = move |_| match naming::analyze(&surname.get(), &given.get()) {
        Ok(r) => {
            error.set(String::new());
            report.set(Some(r));
        }
        Err(e) => {
            error.set(error_message(&e));
            report.set(None);
        }
    };

    view! {
        <div class="page">
            <A href="/" attr:class="back-link">"← 返回"</A>
            <h1 style="margin-bottom:0.5rem">"姓名學"</h1>
            <p class="muted" style="margin-bottom:1.5rem">
                "五格剖象＋三才五行，輸入繁體姓名即時解析。計算全部在你的瀏覽器完成，輸入不會傳送或儲存。"
            </p>

            <div class="card">
                <form on:submit=move |ev| {
                    ev.prevent_default();
                    on_submit(());
                }>
                    <div class="field">
                        <label>"姓氏（1–2 字）"</label>
                        <input class="form-input"
                            prop:value=move || surname.get()
                            on:input=move |ev| surname.set(event_target_value(&ev))
                            placeholder="王"
                        />
                        <p class="muted" style="margin:0.35rem 0 0;font-size:0.85rem">
                            {move || surname_preview.get()}
                        </p>
                    </div>
                    <div class="field">
                        <label>"名字（1–2 字）"</label>
                        <input class="form-input"
                            prop:value=move || given.get()
                            on:input=move |ev| given.set(event_target_value(&ev))
                            placeholder="小明"
                        />
                        <p class="muted" style="margin:0.35rem 0 0;font-size:0.85rem">
                            {move || given_preview.get()}
                        </p>
                    </div>
                    <Show when=move || !error.get().is_empty()>
                        <p class="error">{move || error.get()}</p>
                    </Show>
                    <button class="btn-primary" type="submit">"開始分析"</button>
                </form>
            </div>

            {move || report.get().map(|r| {
                let single_given = r.given.len() == 1;
                view! {
                    <div class="card" style="margin-top:1.5rem">
                        <h2 style="margin-bottom:0.5rem">
                            {format!("{} {}", r.surname.iter().map(|c| c.ch).collect::<String>(), r.given.iter().map(|c| c.ch).collect::<String>())}
                        </h2>
                        <p class="muted" style="margin-bottom:1rem">
                            {format!("{} {}", stroke_text(&r.surname), stroke_text(&r.given))}
                        </p>

                        <table style="width:100%;border-collapse:collapse;font-size:0.95rem">
                            <thead>
                                <tr>
                                    <th style="padding:0.5rem 0.75rem;text-align:left">"格"</th>
                                    <th style="padding:0.5rem 0.75rem;text-align:left">"數理"</th>
                                    <th style="padding:0.5rem 0.75rem;text-align:left">"五行"</th>
                                    <th style="padding:0.5rem 0.75rem;text-align:left">"吉凶"</th>
                                    <th style="padding:0.5rem 0.75rem;text-align:left">"短評"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {r.grids.iter().map(|g| grid_row(g, single_given)).collect_view()}
                            </tbody>
                        </table>

                        <h2 style="margin:1.5rem 0 0.5rem 0">"三才配置"</h2>
                        <p style="margin-bottom:0.5rem">
                            <span style="margin-right:0.75rem">{format!("「{}」", r.sancai.pattern)}</span>
                            <span class="star">{r.sancai.rating.label()}</span>
                        </p>
                        <p class="muted">{
                            let e = r.sancai.elements;
                            format!(
                                "天格{}與人格{}{}、人格{}與地格{}{}。",
                                e[0].label(), e[1].label(), r.sancai.pairs[0].label(),
                                e[1].label(), e[2].label(), r.sancai.pairs[1].label(),
                            )
                        }</p>
                        <p class="muted" style="font-size:0.85rem">
                            "三才評級採相生相剋簡化規則，非傳統 125 組配置表，結果僅供參考。"
                        </p>

                        <Show when=move || single_given>
                            <p class="muted" style="font-size:0.85rem;margin-top:0.75rem">
                                "單名字的地格與外格採「假成一」慣例（虛畫 +1），吉凶意義有限。"
                            </p>
                        </Show>

                        <p class="muted" style="font-size:0.8rem;margin-top:1rem">
                            {format!(
                                "筆畫依《康熙字典》機械推導（資料版本 {}；數理表 {}）。",
                                strokes::STROKE_TABLE_VERSION, naming::NAMING_DATA_VERSION
                            )}
                        </p>
                        <p class="muted" style="font-size:0.8rem">
                            "姓名學為傳統民俗文化，結果僅供文化參考，不構成任何人生或醫療建議。"
                        </p>

                        <div style="margin-top:1rem">
                            <button class="btn-link" on:click=move |_| report.set(None)>"重新輸入"</button>
                        </div>
                    </div>
                }.into_any()
            })}
        </div>
    }
}
