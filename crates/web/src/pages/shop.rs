//! 開運小物 v1（spec docs/superpowers/specs/2026-11-22-shop-interest-design.md §5）。
//!
//! 公開頁：展示 `ft_schema::shop::CATALOG` 6 件候選小物，每件可按「我有興趣」登記／取消。
//! 沒有價格、沒有金流；文案一律取自 schema 常數（禁字測試守門），本檔不寫商品文字。
//!
//! 身分：localStorage `shop_interest_v1` = `{"anonId": uuid, "items": [...]}`；首次進頁產生
//! anonId。狀態以 `GET /api/shop/interest/me` 為準，失敗退回 localStorage 的 items；
//! 有 session 時 `api::with_auth` 自動帶 Bearer。

use std::collections::BTreeSet;

use ft_schema::api::ShopInterestRequest;
use ft_schema::shop::{self, ShopItem, CATALOG, PAGE_INTRO, PRIVACY_NOTE};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};

use crate::api::{self, ApiErr};
use crate::components::SpriteStack;

const STORE_KEY: &str = "shop_interest_v1";

// ═══ localStorage ═══

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    #[serde(default)]
    anon_id: String,
    #[serde(default)]
    items: Vec<String>,
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// 與 API 驗證一致（spec §4）：長度 8–64、`[A-Za-z0-9-]`。
fn valid_anon_id(s: &str) -> bool {
    (8..=64).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// 隨機代號：`crypto.randomUUID()`（36 字）；crypto 不可用時 fallback 32 hex
/// （同 tarot `generate_seed`——去重用途，非安全邊界）。
fn new_anon_id() -> String {
    if let Some(crypto) = web_sys::window().and_then(|w| w.crypto().ok()) {
        return crypto.random_uuid();
    }
    let hex = "0123456789abcdef".as_bytes();
    (0..32)
        .map(|_| hex[(js_sys::Math::random() * 16.0) as usize % 16] as char)
        .collect()
}

fn save(anon_id: &str, items: &BTreeSet<String>) {
    let st = Stored {
        anon_id: anon_id.to_string(),
        items: items.iter().cloned().collect(),
    };
    if let (Some(s), Ok(json)) = (storage(), serde_json::to_string(&st)) {
        let _ = s.set_item(STORE_KEY, &json);
    }
}

/// 讀回（壞資料丟棄、只留目前 CATALOG 內的 id）；anonId 缺或不合法就產新的並寫回。
fn load_or_init() -> (String, BTreeSet<String>) {
    let st: Stored = storage()
        .and_then(|s| s.get_item(STORE_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    let items = only_catalog(st.items);
    if valid_anon_id(&st.anon_id) {
        (st.anon_id, items)
    } else {
        let anon_id = new_anon_id();
        save(&anon_id, &items);
        (anon_id, items)
    }
}

fn only_catalog(ids: Vec<String>) -> BTreeSet<String> {
    ids.into_iter()
        .filter(|id| shop::find(id).is_some())
        .collect()
}

/// 送出失敗的卡內訊息（不直印伺服器內容）。
fn toggle_error(e: &ApiErr, cancelling: bool) -> &'static str {
    if e.status() == Some(429) || e.is_code("RATE_LIMITED") {
        "按得太頻繁了，請稍後再試。"
    } else if matches!(e, ApiErr::Network(_)) {
        "連線失敗，請檢查網路後再試。"
    } else if cancelling {
        "取消失敗，請稍後再試。"
    } else {
        "登記失敗，請稍後再試。"
    }
}

// ═══ page ═══

#[component]
pub fn ShopPage() -> impl IntoView {
    let (anon_id, items) = load_or_init();
    let registered = RwSignal::new(items);
    // 使用者已按過 → 晚到的 /me 結果不得蓋掉剛切換的狀態
    let touched = RwSignal::new(false);

    {
        let anon_id = anon_id.clone();
        spawn_local(async move {
            // 失敗（含 API 未部署）就沿用 localStorage 的 items
            if let Ok(r) = api::shop_interest_me(Some(&anon_id)).await {
                if touched.try_get_untracked() == Some(false) {
                    let set = only_catalog(r.item_ids);
                    save(&anon_id, &set);
                    registered.try_set(set);
                }
            }
        });
    }

    view! {
        <div class="page shop-page">
            <nav class="zw-crumb" aria-label="麵包屑">
                <A href="/">"首頁"</A>
                " › "
                <span>"開運小物"</span>
            </nav>
            <h1>"開運小物"</h1>
            <p class="shop-intro">{PAGE_INTRO}</p>
            <p class="shop-privacy">{PRIVACY_NOTE}</p>
            <div class="shop-grid">
                {CATALOG
                    .iter()
                    .map(|item| {
                        view! {
                            <ShopCard
                                item=item
                                anon_id=anon_id.clone()
                                registered=registered
                                touched=touched
                            />
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
fn ShopCard(
    item: &'static ShopItem,
    anon_id: String,
    registered: RwSignal<BTreeSet<String>>,
    touched: RwSignal<bool>,
) -> impl IntoView {
    let busy = RwSignal::new(false);
    let err = RwSignal::new(None::<&'static str>);
    let is_on = move || registered.with(|s| s.contains(item.id));

    let on_click = move |_| {
        if busy.get_untracked() {
            return;
        }
        let cancelling = registered.with_untracked(|s| s.contains(item.id));
        busy.set(true);
        err.set(None);
        touched.set(true);
        let anon_id = anon_id.clone();
        spawn_local(async move {
            let body = ShopInterestRequest {
                item_id: item.id.to_string(),
                anon_id: Some(anon_id.clone()),
            };
            let res = if cancelling {
                api::shop_interest_remove(&body).await
            } else {
                api::shop_interest_add(&body).await
            };
            match res {
                Ok(r) => {
                    registered.try_update(|s| {
                        if r.registered {
                            s.insert(item.id.to_string());
                        } else {
                            s.remove(item.id);
                        }
                        save(&anon_id, s);
                    });
                }
                // 失敗：狀態不動（按鈕回原態），只顯示卡內訊息
                Err(e) => {
                    err.try_set(Some(toggle_error(&e, cancelling)));
                }
            }
            busy.try_set(false);
        });
    };

    view! {
        <article class="card shop-card">
            {match stack_of(item.id) {
                Some((n, g)) => view! {
                    <SpriteStack
                        base=format!("/assets/shop/stack/{}", item.id)
                        layers=n
                        grid=g
                        label=item.name
                        class="shop-img"
                    />
                }
                    .into_any(),
                None => view! {
                    <img
                        class="shop-img"
                        src=item.image
                        alt=item.name
                        loading="lazy"
                        width="600"
                        height="600"
                    />
                }
                    .into_any(),
            }}
            <h2 class="shop-name">{item.name}</h2>
            <p class="shop-tagline">{item.tagline}</p>
            <p class="shop-body">{item.body}</p>
            <A href=item.unit_href attr:class="shop-unit">
                {format!("相關單元：{} →", item.unit_label)}
            </A>
            <div class="shop-actions">
                <button
                    type="button"
                    class="btn-primary shop-btn"
                    class:on=is_on
                    aria-pressed=move || if is_on() { "true" } else { "false" }
                    disabled=move || busy.get()
                    on:click=on_click
                >
                    {move || if is_on() { "已登記 ✓（再按取消）" } else { "我有興趣" }}
                </button>
                {move || err.get().map(|m| view! { <p class="error shop-err" role="alert">{m}</p> })}
            </div>
        </article>
    }
}

/// sprite stacking 品項：(id, 層數, 切片格數)。切片由 `scripts/sprite-stack/desk_plant.py` 產生，
/// 放在 `assets/shop/stack/<id>/NN.webp`（由下而上）；層數改了要同步這裡。
const STACKS: &[(&str, usize, usize)] = &[("desk-plant", 24, 48)];

fn stack_of(id: &str) -> Option<(usize, usize)> {
    STACKS
        .iter()
        .find(|(k, _, _)| *k == id)
        .map(|(_, n, g)| (*n, *g))
}
