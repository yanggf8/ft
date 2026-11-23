//! 吉祥物月亮貓（sprite stacking，2026-11-23；取代 2026-11-19 的平面 WebP）。
//!
//! 切片由 `scripts/sprite-stack/mascot.py` 程式化體素產生，每個姿勢合成一張圖集
//! （`assets/mascot/stack/<pose>-<hash>.webp`，一個請求）；層數／欄數／內容 hash／舞台尺寸在產生檔 `mascot_stacks.rs`。純裝飾：整疊 aria-hidden。左右擺動、低俯角與尺寸在 style.css
//! `.mascot-stack`。

use leptos::prelude::*;

use super::mascot_stacks::{
    MASCOT_SHEET_COLS, MASCOT_STACK_GRID, MASCOT_STACK_LAYERS, MASCOT_STAGE_B, MASCOT_STAGE_H,
};
use super::SpriteStackSheet;

/// 姿勢：首頁打招呼／塔羅翻牌前期待／翻牌後開心。
#[derive(Clone, Copy)]
pub enum MascotPose {
    Wave,
    Wait,
    Joy,
}

impl MascotPose {
    fn key(self) -> &'static str {
        match self {
            MascotPose::Wave => "wave",
            MascotPose::Wait => "wait",
            MascotPose::Joy => "joy",
        }
    }
}

#[component]
pub fn Mascot(
    pose: MascotPose,
    /// 額外 class（尺寸、動畫）
    #[prop(optional, into)]
    class: String,
) -> impl IntoView {
    let key = pose.key();
    let (layers, hash) = MASCOT_STACK_LAYERS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map_or((0, ""), |(_, n, h)| (*n, *h));
    view! {
      <SpriteStackSheet
        src=format!("/assets/mascot/stack/{key}-{hash}.webp")
        layers=layers
        cols=MASCOT_SHEET_COLS
        grid=MASCOT_STACK_GRID
        label=""
        class=format!("mascot-stack {class}")
        style=format!("--mh:{MASCOT_STAGE_H};--mb:{MASCOT_STAGE_B}")
      />
    }
}
