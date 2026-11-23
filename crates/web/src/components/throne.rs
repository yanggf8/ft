//! 首頁月亮王座吉祥物（2026-11-23 站長：吉祥物不用動太多，只簡單轉頭；椅背月亮↔太陽循環；
//! 星星以吉祥物為中心公轉、只在椅背；頭永遠是月牙，月相改由椅背的月亮表現；太陽時沒有星、射線繞太陽轉；
//! 椅背加雲）。
//!
//! `scripts/sprite-stack/throne.py` 產生：身體圖集（底座＋王座＋坐姿貓身，靜止）＋頭圖集（月相五種＋睡覺），
//! 用 `SpriteStackSheetDuo` 逐層交錯疊，只有頭轉；花窗上的月亮（當晚月相）／太陽是 2D 像素疊加，位置在
//! `throne_stacks.rs`（產生器已檢查不會蓋到前方體素）。星星層套產生器算好的遮罩（椅背正面減去前方體素），
//! 繞著頭部中心公轉：繞到貓背後被擋、離開椅背就看不到。動畫與減少動態效果在 style.css `.throne*`。純裝飾。

use ft_schema::moon::MoonPhase;
use leptos::prelude::*;

use super::throne_stacks::{
    THRONE_BODY, THRONE_CLOUDS, THRONE_CLOUD_IMGS, THRONE_COLS, THRONE_GRID, THRONE_HEAD,
    THRONE_HEAD_ORIGIN, THRONE_LAYERS, THRONE_MOONS, THRONE_ORBIT, THRONE_ORBIT_MASK,
    THRONE_ORBIT_STARS, THRONE_SKY_RECT, THRONE_STAGE_B, THRONE_STAGE_H, THRONE_STAR,
    THRONE_SUN_BOX, THRONE_SUN_CORE, THRONE_SUN_RAYS,
};
use super::SpriteStackSheetDuo;

const BASE: &str = "/assets/mascot/throne";

fn find(table: &'static [(&'static str, &'static str)], key: &str) -> &'static str {
    // 表是產生檔的常數，鍵一定在；找不到時回空字串（圖 404 不顯示，不 panic）
    table
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("", |(_, h)| *h)
}

fn rect_style((l, b, w, h): (f32, f32, f32, f32)) -> String {
    format!("left:{l}%;bottom:{b}%;width:{w}%;height:{h}%")
}

#[component]
pub fn Throne(
    /// 當晚月相（椅背的月亮）
    phase: MoonPhase,
    /// 新月夜睡著：睡覺身體＋暗色閉眼月牙頭、不轉頭
    #[prop(optional)]
    asleep: bool,
) -> impl IntoView {
    // 頭永遠是月牙（招牌造型）；睡著用暗色閉眼月牙
    let (pose, head) = if asleep {
        ("sleep", "sleep")
    } else {
        ("wave", "crescent")
    };
    let moon_key = phase.key();
    let moon_src = format!(
        "{BASE}/moon-{moon_key}-{}.webp",
        find(&THRONE_MOONS, moon_key)
    );
    let (ox, oy, or, star) = THRONE_ORBIT;
    let (sl, sb, sw) = THRONE_SUN_BOX;
    let sun_style = format!("left:{sl}%;bottom:{sb}%;width:{sw}%");
    let body_src = format!("{BASE}/body-{pose}-{}.webp", find(&THRONE_BODY, pose));
    let head_src = format!("{BASE}/head-{head}-{}.webp", find(&THRONE_HEAD, head));
    view! {
      <div
        class="throne"
        class:throne-asleep=asleep
        style=format!(
            "--mh:{THRONE_STAGE_H};--mb:{THRONE_STAGE_B};--orbit-mask:url('{BASE}/orbit-mask-{THRONE_ORBIT_MASK}.webp');\
             --ox:{ox}%;--oy:{oy}%;--or:{or};--star:{star}"
        )
        aria-hidden="true"
      >
        <SpriteStackSheetDuo
          src=body_src
          head_src=head_src
          layers=THRONE_LAYERS
          cols=THRONE_COLS
          grid=THRONE_GRID
          head_class="throne-head"
          head_origin=THRONE_HEAD_ORIGIN
          class="throne-stack"
        />
        <img class="throne-sky throne-moon" src=moon_src alt="" style=rect_style(THRONE_SKY_RECT) />
        <div class="throne-sun" style=sun_style>
          <img class="throne-sun-rays" src=format!("{BASE}/sun-rays-{THRONE_SUN_RAYS}.webp") alt="" />
          <img class="throne-sun-core" src=format!("{BASE}/sun-core-{THRONE_SUN_CORE}.webp") alt="" />
        </div>
        <div class="throne-clouds">
          {THRONE_CLOUDS
            .iter()
            .map(|(v, b, w)| {
              let src = format!("{BASE}/cloud-{v}-{}.webp", find(&THRONE_CLOUD_IMGS, v));
              view! { <img class="throne-cloud" src=src alt="" style=format!("bottom:{b}%;width:{w}%") /> }
            })
            .collect_view()}
        </div>
        <div class="throne-orbit">
          <div class="throne-ring">
            {(0..THRONE_ORBIT_STARS)
              .map(|i| {
                let a = 360.0 * i as f32 / THRONE_ORBIT_STARS as f32;
                view! {
                  <span class="throne-star" style=format!("--a:{a}deg")>
                    <img src=format!("{BASE}/star-{THRONE_STAR}.webp") alt="" />
                  </span>
                }
              })
              .collect_view()}
          </div>
        </div>
      </div>
    }
}
