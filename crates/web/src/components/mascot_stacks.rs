//! 吉祥物月亮貓 sprite stack 的圖集常數——由 `scripts/sprite-stack/mascot.py` 產生，**勿手改**。
//!
//! 每個姿勢一張圖集 `crates/web/assets/mascot/stack/<pose>.webp`：8 欄、由左而右由上而下，第 0 格＝最底層。

/// 切片的平面格數（寬＝高）。
pub const MASCOT_STACK_GRID: usize = 32;

/// 圖集欄數。
pub const MASCOT_SHEET_COLS: usize = 8;

/// 各姿勢：(姿勢, 層數, 圖集內容 hash——網址帶 `?v=`，zone 的瀏覽器快取 4h 才不會配到舊圖)。
pub const MASCOT_STACK_LAYERS: [(&str, usize, &str); 3] = [
    ("wave", 48, "a5741e5f5f00"),
    ("wait", 48, "185c99dd3ef6"),
    ("joy", 48, "56f6a1f2735e"),
];
