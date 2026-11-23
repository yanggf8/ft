//! 吉祥物月亮貓（sprite stacking，2026-11-23；取代 2026-11-19 的平面 WebP）。
//!
//! 切片由 `scripts/sprite-stack/mascot.py` 程式化體素產生（`assets/mascot/stack/<pose>/NN.webp`），
//! 層數在產生檔 `mascot_stacks.rs`。純裝飾：整疊 aria-hidden。左右擺動、低俯角與尺寸在 style.css
//! `.mascot-stack`。

use leptos::prelude::*;

use super::mascot_stacks::{MASCOT_STACK_GRID, MASCOT_STACK_LAYERS};
use super::SpriteStack;

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
    /// 先藏起來、之後才顯示時設 true，避免顯示瞬間才開始載切片
    #[prop(optional)]
    eager: bool,
) -> impl IntoView {
    let key = pose.key();
    let layers = MASCOT_STACK_LAYERS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(0, |(_, n)| *n);
    view! {
      <SpriteStack
        base=format!("/assets/mascot/stack/{key}")
        layers=layers
        grid=MASCOT_STACK_GRID
        label=""
        class=format!("mascot-stack {class}")
        eager=eager
      />
    }
}
