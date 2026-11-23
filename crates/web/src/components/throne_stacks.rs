//! 首頁月亮王座吉祥物的圖集與疊加物常數——由 `scripts/sprite-stack/throne.py` 產生，**勿手改**。
//!
//! 圖集在 `crates/web/assets/mascot/throne/`：`body-<pose>-<hash>.webp`（底座＋王座＋貓身，靜止）、
//! `head-<variant>-<hash>.webp`（只有頭，月相五種＋sleep）；兩者層數相同、逐層交錯疊。

/// 切片的平面格數（寬＝高）。
pub const THRONE_GRID: usize = 64;
/// 圖集欄數。
pub const THRONE_COLS: usize = 8;
/// 層數（身體與頭的圖集相同）。
pub const THRONE_LAYERS: usize = 82;
/// 舞台高（層寬倍數，CSS `--mh`）與層底位移（CSS `--mb`）。
pub const THRONE_STAGE_H: f32 = 1.483;
pub const THRONE_STAGE_B: f32 = 0.256;
/// 頭的轉軸（層框內 x%, y%）＝ CSS `transform-origin`。
pub const THRONE_HEAD_ORIGIN: (f32, f32) = (50.0, 43.75);

/// 身體圖集：(姿勢, hash)。
pub const THRONE_BODY: [(&str, &str); 2] = [("wave", "df591262c374"), ("sleep", "617d3b80bc43")];
/// 頭圖集：(月相 wire 值或 "sleep", hash)。
pub const THRONE_HEAD: [(&str, &str); 6] = [
    ("new", "f91afa9b64cc"),
    ("crescent", "35e4f3102e6b"),
    ("quarter", "c54205325efc"),
    ("gibbous", "58f8bdd0b6f5"),
    ("full", "1b8a9ad21532"),
    ("sleep", "24376ceb4341"),
];

/// 疊加物檔名 hash：月亮、太陽、星星。
pub const THRONE_MOON: &str = "eee4ed1b96f6";
pub const THRONE_SUN: &str = "ed09bdbf13c8";
pub const THRONE_STAR: &str = "7fadd2823f0c";
/// 月亮／太陽在舞台上的位置：(left%, bottom%, width%, height%)（%＝舞台寬／高）。
pub const THRONE_SKY_RECT: (f32, f32, f32, f32) = (30.469, 77.668, 32.812, 18.807);
/// 椅背上的星星位置（同上）；產生器已檢查不會蓋到前方體素。
pub const THRONE_STARS: [(f32, f32, f32, f32); 5] = [
    (16.406, 73.19, 14.062, 8.06),
    (66.406, 79.459, 14.062, 8.06),
    (11.719, 64.234, 14.062, 8.06),
    (72.656, 68.712, 14.062, 8.06),
    (57.031, 89.31, 14.062, 8.06),
];
