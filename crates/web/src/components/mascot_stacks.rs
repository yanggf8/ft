//! 吉祥物月亮貓 sprite stack 的圖集常數——由 `scripts/sprite-stack/mascot.py` 產生，**勿手改**。
//!
//! 每個（姿勢, 月相）一張圖集 `crates/web/assets/mascot/stack/<pose>-<phase>-<hash>.webp`：8 欄、由左而右由上而下，第 0 格＝最底層。

/// 切片的平面格數（寬＝高）。
pub const MASCOT_STACK_GRID: usize = 48;

/// 圖集欄數。
pub const MASCOT_SHEET_COLS: usize = 8;

/// 舞台高（層寬的倍數，CSS `--mh`）與 `.stack-layer` 底部位移（`bottom: -mb × 層寬`，CSS `--mb`）：
/// 由 `stage_box` 依所有姿勢、±24° 擺動的外框算出，最低點貼舞台底、最高點貼舞台頂。
pub const MASCOT_STAGE_H: f32 = 1.077;
pub const MASCOT_STAGE_B: f32 = 0.432;

/// (姿勢, 月相 wire 值, 層數, 圖集內容 hash——在檔名裡，舊程式拿不到新圖集、zone 4h 快取也不會配到舊圖)。
pub const MASCOT_STACK_LAYERS: [(&str, &str, usize, &str); 4] = [
    ("wave", "crescent", 51, "8fd9225b9f7f"),
    ("wait", "crescent", 51, "9db5e11a72e9"),
    ("joy", "crescent", 53, "b1f15d85c90f"),
    ("sleep", "new", 51, "fc29e9e91f83"),
];
