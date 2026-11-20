//! 星座配對頁 `/zodiac`（階段 A：規則版，公開路由）。
//!
//! - 規則運算在瀏覽器（`ft_schema::zodiac_match`），**免登入路徑不打 API**。
//! - 登入且有出生資料 → `GET /api/charts/western` 自動帶入太陽／月亮／上升；
//!   未填時辰（`auth.user.birth_hour` 為空）不顯示上升列、月亮列加註。
//! - 未登入／無出生資料／星盤載入失敗 → 12 宮格手選「我的太陽星座」。
//! - 對方只點選太陽星座（不收對方生日，站長裁示 2026-11-19）。
//! - 階段 B 的 AI 解讀區塊若加入，內文一律用 `super::story::render_markdown`。

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use ft_schema::sign_class;
use ft_schema::sign_zh;
use ft_schema::zodiac_match::{
    match_rows, normalize_sign, signs, MatchBody, MatchRow, ASPECT_NOTE, MATCH_DISCLAIMER,
    MOON_HOUR_NOTE, NO_RISING_NOTE,
};

use crate::api;
use crate::auth::use_auth;
use crate::components::sign_glyph_path;

/// 自動帶入的使用者星座（標準英文名）。
#[derive(Clone, PartialEq)]
struct MySigns {
    sun: &'static str,
    moon: Option<&'static str>,
    rising: Option<&'static str>,
}

/// 登入者星盤的載入狀態。`Idle`＝未登入／無出生資料（手選）。
#[derive(Clone, PartialEq)]
enum ChartState {
    Idle,
    Loading,
    Loaded(MySigns),
    Failed,
}

#[component]
pub fn ZodiacMatchPage() -> impl IntoView {
    let auth = use_auth();

    let chart = RwSignal::new(ChartState::Idle);
    // 已為哪個 user 發出載入（避免 auth.user 每次變動都重打）
    let fetched_for = RwSignal::new(None::<String>);
    let manual_sun = RwSignal::new(None::<&'static str>);
    let target = RwSignal::new(None::<&'static str>);
    // 翻牌動畫重播：每次結果改變切換 a/b 兩個同內容的 keyframes
    let flip_b = RwSignal::new(false);

    let has_birth = Memo::new(move |_| auth.user.get().map(|u| u.hasBirthData).unwrap_or(false));
    let has_hour = Memo::new(move |_| {
        auth.user
            .get()
            .map(|u| u.birth_hour.is_some())
            .unwrap_or(false)
    });

    // 公開頁 bootstrap 為非同步：auth.user 初始 None，登入後才帶入星盤。
    Effect::new(move |_| {
        let Some(user) = auth.user.get() else {
            chart.set(ChartState::Idle);
            fetched_for.set(None);
            return;
        };
        if !user.hasBirthData {
            chart.set(ChartState::Idle);
            fetched_for.set(None);
            return;
        }
        if fetched_for.get_untracked().as_deref() == Some(user.id.as_str()) {
            return;
        }
        let uid = user.id.clone();
        fetched_for.set(Some(uid.clone()));
        chart.set(ChartState::Loading);
        spawn_local(async move {
            // no-cache：走 ETag 重新驗證，補填時辰後不會沿用一小時內的舊星盤
            let got = match api::get_chart("western", true).await {
                Ok(resp) => resp.as_western().and_then(|w| {
                    Some(MySigns {
                        sun: normalize_sign(&w.sun_sign.name)?,
                        moon: normalize_sign(&w.moon_sign.name),
                        rising: normalize_sign(&w.ascendant.sign),
                    })
                }),
                Err(_) => None,
            };
            // 登出／換帳號競態：await 期間使用者已變 → 丟棄（狀態由 Effect 重設）
            // 不可再讀本元件的 signal：元件可能已卸載（disposed → wasm panic）。
            if auth.user.get_untracked().map(|u| u.id) != Some(uid) {
                return;
            }
            chart.set(match got {
                Some(s) => ChartState::Loaded(s),
                None => ChartState::Failed,
            });
        });
    });

    // 目前用來配對的「我的」各列（上升只在有時辰時列入）
    let my_rows = Memo::new(move |_| -> Vec<(MatchBody, &'static str)> {
        match chart.get() {
            ChartState::Loaded(s) => {
                let mut v = vec![(MatchBody::Sun, s.sun)];
                if let Some(m) = s.moon {
                    v.push((MatchBody::Moon, m));
                }
                if has_hour.get() {
                    if let Some(r) = s.rising {
                        v.push((MatchBody::Rising, r));
                    }
                }
                v
            }
            ChartState::Loading => Vec::new(),
            _ => manual_sun
                .get()
                .map(|s| vec![(MatchBody::Sun, s)])
                .unwrap_or_default(),
        }
    });

    let results = Memo::new(move |_| -> Vec<MatchRow> {
        match target.get() {
            Some(t) => match_rows(&my_rows.get(), t),
            None => Vec::new(),
        }
    });

    // 結果改變（不論是點對方、改選自己、或星盤晚到）→ 切換動畫 class 重播翻牌
    // 並捲到結果，避免結果出現在視窗外看似沒反應。
    Effect::new(move |prev: Option<Vec<MatchRow>>| {
        let rows = results.get();
        if !rows.is_empty() && prev.as_ref() != Some(&rows) {
            flip_b.update(|b| *b = !*b);
            request_animation_frame(scroll_to_result);
        }
        rows
    });

    let loaded = move || matches!(chart.get(), ChartState::Loaded(_));

    view! {
      <div class="page zm-page">
        <h1 style="margin-bottom:0.5rem">"星座配對"</h1>
        <p class="muted" style="margin-bottom:1.25rem">
          "點選對方的太陽星座，看看你們的互動傾向。不需要對方的生日，也不會上傳任何資料。"
        </p>

        <section class="card zm-section" aria-labelledby="zm-me-title">
          <h2 id="zm-me-title" class="zm-title">"我的星座"</h2>
          {move || {
            if auth.loading.get() {
              return view! { <p class="muted">"載入中…"</p> }.into_any();
            }
            match chart.get() {
              ChartState::Loading => view! {
                <p class="muted">"星盤讀取中，稍候自動顯示"</p>
              }.into_any(),
              ChartState::Loaded(s) => {
                let hour = has_hour.get();
                let rising = if hour { s.rising } else { None };
                let mut got = vec!["太陽"];
                if s.moon.is_some() { got.push("月亮"); }
                if rising.is_some() { got.push("上升"); }
                let note = format!("已依你的出生資料帶入{}。", got.join("、"));
                view! {
                  <div class="zm-mine">
                    <MineChip body=MatchBody::Sun sign=Some(s.sun) />
                    <MineChip body=MatchBody::Moon sign=s.moon />
                    <MineChip body=MatchBody::Rising sign=rising />
                  </div>
                  <p class="zm-note">{note}</p>
                  {(!hour).then(|| view! { <p class="zm-note">{NO_RISING_NOTE}</p> })}
                }.into_any()
              }
              state => {
                let failed = state == ChartState::Failed;
                view! {
                  <p class="zm-sub">"選你的太陽星座"</p>
                  <SignGrid selected=manual_sun label="我的太陽星座" />
                  {move || {
                    if failed {
                      view! { <p class="zm-note">"星盤暫時讀不到，先手選太陽星座。"</p> }.into_any()
                    } else if !auth.is_authed() {
                      view! {
                        <p class="zm-note">
                          <A href="/login?next=/zodiac">"登入"</A>
                          "並填寫出生資料，就能自動帶入太陽、月亮、上升。"
                        </p>
                      }.into_any()
                    } else if !has_birth.get() {
                      view! {
                        <p class="zm-note">
                          "到"<A href="/profile">"我的命格"</A>"填寫出生資料，就能自動帶入太陽、月亮、上升。"
                        </p>
                      }.into_any()
                    } else {
                      view! { <span></span> }.into_any()
                    }
                  }}
                }.into_any()
              }
            }
          }}
        </section>

        <section class="card zm-section" aria-labelledby="zm-them-title">
          <h2 id="zm-them-title" class="zm-title">"對方的太陽星座"</h2>
          <SignGrid selected=target label="對方的太陽星座" />
        </section>

        // 報讀器只讀一句摘要（role=status）；結果區本身不設 live
        <p class="zc-sr" role="status">{move || summary(&results.get())}</p>

        <section class="zm-results" id="zm-results" aria-label="配對結果">
          {move || {
            let rows = results.get();
            if rows.is_empty() {
              let hint = if matches!(chart.get(), ChartState::Loading) || auth.loading.get() {
                ""
              } else if my_rows.get().is_empty() {
                "先選你的太陽星座，再點對方的星座。"
              } else {
                "點對方的星座，翻開你們的配對。"
              };
              return view! { <p class="zm-hint">{hint}</p> }.into_any();
            }
            let their = rows[0].their_sign;
            let their_cls = format!("zm-reveal zc-sign-{}", sign_class(their).unwrap_or("none"));
            let moon_hint = loaded() && !has_hour.get();
            view! {
              <div class=their_cls class:zm-anim-b=move || flip_b.get() id="zm-reveal">
                <div class="zm-reveal-head">
                  {sign_class(their).and_then(sign_glyph_path).map(|d| view! {
                    <svg class="zm-reveal-glyph" viewBox="0 0 24 24" aria-hidden="true"><path d=d /></svg>
                  })}
                  <h2 class="zm-reveal-name">{format!("對方：{}", sign_zh(their))}</h2>
                </div>
                {rows.into_iter().map(|r| {
                  let moon_note = moon_hint && r.has(MatchBody::Moon);
                  view! { <ResultRow row=r moon_note=moon_note /> }
                }).collect_view()}
              </div>
              <p class="zm-foot">{MATCH_DISCLAIMER}</p>
              <p class="zm-foot">{ASPECT_NOTE}</p>
            }
            .into_any()
          }}
        </section>
      </div>
    }
}

/// 報讀摘要：「對方天蠍座：太陽 正對面（對分相）互補也拉鋸；月亮 隔 3 個星座（四分相）拉扯」。
fn summary(rows: &[MatchRow]) -> String {
    let Some(first) = rows.first() else {
        return String::new();
    };
    let parts: Vec<String> = rows
        .iter()
        .map(|r| {
            let bodies: Vec<&str> = r.bodies.iter().map(|b| b.zh()).collect();
            format!(
                "{} {} {}",
                bodies.join("＋"),
                r.aspect.plain(),
                r.label.zh()
            )
        })
        .collect();
    format!("對方{}：{}", sign_zh(first.their_sign), parts.join("；"))
}

fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .map(|m| m.matches())
        .unwrap_or(false)
}

/// `#zm-reveal.scrollIntoView({block:"nearest", behavior})`（以 Reflect 呼叫，免加 web-sys feature）。
fn scroll_to_result() {
    use wasm_bindgen::JsCast;
    let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("zm-reveal"))
    else {
        return;
    };
    let opts = js_sys::Object::new();
    let behavior = if prefers_reduced_motion() {
        "auto"
    } else {
        "smooth"
    };
    let _ = js_sys::Reflect::set(&opts, &"block".into(), &"nearest".into());
    let _ = js_sys::Reflect::set(&opts, &"behavior".into(), &behavior.into());
    if let Ok(f) = js_sys::Reflect::get(&el, &"scrollIntoView".into()) {
        if let Some(f) = f.dyn_ref::<js_sys::Function>() {
            let _ = f.call1(&el, &opts);
        }
    }
}

#[component]
fn MineChip(body: MatchBody, sign: Option<&'static str>) -> impl IntoView {
    let cls = match sign.and_then(sign_class) {
        Some(k) => format!("zm-mine-chip zc-sign-{k}"),
        None => "zm-mine-chip zm-mine-none".to_string(),
    };
    let text = format!(
        "{}：{}",
        body.zh(),
        sign.map(sign_zh).unwrap_or_else(|| "—".to_string())
    );
    view! { <span class=cls>{text}</span> }
}

/// 12 宮格：button＋aria-pressed；再點一次已選的不取消（保持結果可讀）。
#[component]
fn SignGrid(selected: RwSignal<Option<&'static str>>, label: &'static str) -> impl IntoView {
    view! {
      <div class="zm-grid" role="group" aria-label=label>
        {signs().into_iter().map(|s| {
          let key = sign_class(s).unwrap_or("none");
          let cls = format!("zm-sign zc-sign-{key}");
          let glyph = sign_glyph_path(key);
          let name = sign_zh(s);
          view! {
            <button
              type="button"
              class=cls
              class:zm-selected=move || selected.get() == Some(s)
              aria-pressed=move || (selected.get() == Some(s)).to_string()
              on:click=move |_| selected.set(Some(s))
            >
              {glyph.map(|d| view! {
                <svg class="zm-sign-glyph" viewBox="0 0 24 24" aria-hidden="true"><path d=d /></svg>
              })}
              <span class="zm-sign-name">{name}</span>
            </button>
          }
        }).collect_view()}
      </div>
    }
}

#[component]
fn ResultRow(row: MatchRow, moon_note: bool) -> impl IntoView {
    let label_cls = format!("zm-label zm-label-{}", row.label.key());
    view! {
      <article class="zm-row">
        <p class="zm-row-theme">{row.theme()}</p>
        <h3 class="zm-row-frame">
          <span class="zm-nw">{row.my_part()}</span>
          "× "
          <span class="zm-nw">{row.their_part()}</span>
        </h3>
        <p class="zm-tags">
          <span class="zm-aspect">{row.aspect.plain()}</span>
          <span class=label_cls>{row.label.zh()}</span>
        </p>
        <p class="zm-line"><span class="zm-line-k">"距離"</span>{row.label.line()}</p>
        <p class="zm-line"><span class="zm-line-k">"元素"</span>{row.element_line}</p>
        <p class="zm-line"><span class="zm-line-k">"步調"</span>{row.modality_line}</p>
        {moon_note.then(|| view! { <p class="zm-note">{MOON_HOUR_NOTE}</p> })}
      </article>
    }
}
