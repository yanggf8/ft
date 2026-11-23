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
    ("crescent", "eee4ed1b96f6"),
    ("quarter", "4aa4b5077698"),
    ("gibbous", "fa4d77fb21bf"),
    ("full", "dd6133ec8eb2"),
];
/// 太陽、星星、星星層遮罩的檔名 hash。
pub const THRONE_SUN: &str = "ed09bdbf13c8";
pub const THRONE_STAR: &str = "7fadd2823f0c";
pub const THRONE_ORBIT_MASK: &str = "578f6a610b42";
/// 月亮／太陽在舞台上的位置：(left%, bottom%, width%, height%)（%＝舞台寬／高）。
pub const THRONE_SKY_RECT: (f32, f32, f32, f32) = (30.469, 77.668, 32.812, 18.807);
/// 星星公轉：(中心 left%, 中心 bottom%, 半徑（層寬倍數）, 星星寬（層寬倍數）)。
pub const THRONE_ORBIT: (f32, f32, f32, f32) = (49.04, 59.363, 0.3, 0.1406);
/// 星星數。
pub const THRONE_ORBIT_STARS: usize = 6;
