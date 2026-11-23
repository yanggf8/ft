//! 首頁月亮王座吉祥物的圖集與疊加物常數——由 `scripts/sprite-stack/throne.py` 產生，**勿手改**。
//!
//! 圖集在 `crates/web/assets/mascot/throne/`：`body-<pose>-<hash>.webp`（底座＋王座＋貓身，靜止）、
//! `head-<crescent|sleep>-<hash>.webp`（只有頭）；兩者層數相同、逐層交錯疊。

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
pub const THRONE_BODY: [(&str, &str); 2] = [("wave", "da30f805ed6f"), ("sleep", "eb06f6e8403e")];
/// 頭圖集：("crescent" 或 "sleep", hash)。
pub const THRONE_HEAD: [(&str, &str); 2] =
    [("crescent", "35e4f3102e6b"), ("sleep", "24376ceb4341")];
/// 椅背月亮（當晚月相，鍵＝`MoonPhase::key()`）：(月相, hash)。
pub const THRONE_MOONS: [(&str, &str); 5] = [
    ("new", "ad282a4fc8b0"),
    ("crescent", "329bde42d55c"),
    ("quarter", "c9d6700b4110"),
    ("gibbous", "bdca8b7b11a0"),
    ("full", "f1361db79217"),
];
/// 太陽（核心／射線）、星星、星星與雲層遮罩的檔名 hash。
pub const THRONE_SUN_CORE: &str = "f10ba6eb2379";
pub const THRONE_SUN_RAYS: &str = "5d495f0a9e9a";
/// 太陽的正方形框：(left%, 中心 bottom%, width%)——月亮對轉軸的對稱點（初始在地平線下）。
pub const THRONE_SUN_BOX: (f32, f32, f32) = (36.719, -7.071, 32.812);
/// 雲圖：(形狀, hash)。
pub const THRONE_CLOUD_IMGS: [(&str, &str); 2] = [("0", "1f7f228264bb"), ("1", "a1a150cb07d4")];
/// 椅背上飄的雲：(形狀, 雲底 bottom%, 寬%)；飄動時間在 style.css `.throne-cloud`。
pub const THRONE_CLOUDS: [(&str, f32, f32); 2] = [("0", 76.319, 39.062), ("1", 68.154, 40.625)];
pub const THRONE_STAR: &str = "7fadd2823f0c";
pub const THRONE_ORBIT_MASK: &str = "578f6a610b42";
/// 月亮／太陽在舞台上的位置：(left%, bottom%, width%, height%)（%＝舞台寬／高）。
pub const THRONE_SKY_RECT: (f32, f32, f32, f32) = (30.469, 77.668, 32.812, 18.807);
/// 夜空的星星中心：(left%, bottom%)；星星寬（%＝舞台寬）。
pub const THRONE_STARS: [(f32, f32); 6] = [
    (23.438, 79.011),
    (75.0, 83.489),
    (18.75, 65.578),
    (79.688, 70.055),
    (65.625, 94.236),
    (31.25, 93.34),
];
pub const THRONE_STAR_W: f32 = 14.062;
/// 天空轉輪的轉軸：(left%, bottom%)；夜空組在上、太陽在對面，24 秒內轉 180°×2（style.css `.throne-wheel`）。
pub const THRONE_WHEEL: (f32, f32) = (50.0, 40.0);
