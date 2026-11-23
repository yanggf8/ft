//! 吉祥物月亮貓（sprite stacking，2026-11-23；取代 2026-11-19 的平面 WebP）。
//!
//! 切片由 `scripts/sprite-stack/mascot.py` 程式化體素產生，每個（姿勢, 頭形）合成一張圖集
//! （`assets/mascot/stack/<pose>-<crescent|new>-<hash>.webp`，一個請求）；層數／欄數／內容 hash／舞台尺寸在產生檔
//! `mascot_stacks.rs`。純裝飾：整疊 aria-hidden。左右擺動、低俯角與尺寸在 style.css `.mascot-stack`。
//!
//! 「引路貓」（2026-11-23 設計稿）：頭永遠是月牙（站長：招牌造型）；月相改由首頁王座椅背的月亮表現、
//! 台詞跟月相（`ft_schema::moon`）。`MoonNow` 給頁面用：
//! 月相、是否台北夜晚、今晚的日期鍵（滿月變身每晚只演一次）。

use ft_schema::moon::{self, MoonPhase};
use leptos::prelude::*;

use super::mascot_stacks::{
    MASCOT_SHEET_COLS, MASCOT_STACK_GRID, MASCOT_STACK_LAYERS, MASCOT_STAGE_B, MASCOT_STAGE_H,
};
use super::SpriteStackSheet;

/// 姿勢：首頁打招呼／塔羅翻牌前期待／翻牌後開心／新月夜睡覺（暗色月牙頭）。
#[derive(Clone, Copy)]
pub enum MascotPose {
    Wave,
    Wait,
    Joy,
    Sleep,
}

impl MascotPose {
    fn key(self) -> &'static str {
        match self {
            MascotPose::Wave => "wave",
            MascotPose::Wait => "wait",
            MascotPose::Joy => "joy",
            MascotPose::Sleep => "sleep",
        }
    }
}

/// 頁面載入當下的月相資訊（瀏覽器時鐘）。
#[derive(Clone)]
pub struct MoonNow {
    pub phase: MoonPhase,
    pub night: bool,
    pub night_key: String,
}

impl MoonNow {
    pub fn read() -> Self {
        let now = js_sys::Date::now();
        MoonNow {
            phase: moon::phase_at(now),
            night: moon::is_night_taipei(now),
            night_key: moon::taipei_night_key(now),
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
    // 頭永遠是月牙（招牌造型，2026-11-23 站長）；只有新月夜睡覺用暗色月牙頭
    let phase_key = match pose {
        MascotPose::Sleep => MoonPhase::New.key(),
        _ => MoonPhase::Crescent.key(),
    };
    let (layers, hash) = MASCOT_STACK_LAYERS
        .iter()
        .find(|(k, ph, _, _)| *k == key && *ph == phase_key)
        .map_or((0, ""), |(_, _, n, h)| (*n, *h));
    view! {
      <SpriteStackSheet
        src=format!("/assets/mascot/stack/{key}-{phase_key}-{hash}.webp")
        layers=layers
        cols=MASCOT_SHEET_COLS
        grid=MASCOT_STACK_GRID
        label=""
        class=format!("mascot-stack {class}")
        style=format!("--mh:{MASCOT_STAGE_H};--mb:{MASCOT_STAGE_B}")
      />
    }
}

/// 滿月變身時灑出的星點（裝飾，aria-hidden；位置與動畫在 style.css `.mascot-spark`）。
#[component]
pub fn MascotSparks() -> impl IntoView {
    view! {
      <span class="mascot-sparks" aria-hidden="true">
        {(0..6).map(|_| view! { <span class="mascot-spark"></span> }).collect_view()}
      </span>
    }
}
