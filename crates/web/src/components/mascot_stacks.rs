//! 吉祥物月亮貓 sprite stack 的切片層數——由 `scripts/sprite-stack/mascot.py` 產生，**勿手改**。
//!
//! 切片在 `crates/web/assets/mascot/stack/<pose>/NN.webp`。

/// 切片的平面格數（寬＝高）。
pub const MASCOT_STACK_GRID: usize = 32;

/// 各姿勢的層數。
pub const MASCOT_STACK_LAYERS: [(&str, usize); 3] = [("wave", 48), ("wait", 48), ("joy", 48)];
