//! Sprite stacking 偽 3D：把橫切片由下而上疊起、整疊一起自轉（2026-11-22）。
//!
//! 切片放在 `<base>/NN.webp`（00 = 最底層），由 `scripts/sprite-stack/*.py` 程式化體素模型產生；
//! 每層畫兩次（整格＋半格位移）補層間縫。位移量＝0.85 格，依切片格數 `grid` 換算成層寬的 %，
//! 以 `--step` 行內變數交給 CSS（`.stack-layer`）；壓扁比例、自轉與減少動態效果在 style.css。
//!
//! 結構：`.stack-stage > .stack-layer（位移＋壓扁）> .stack-spin（自轉）> img`。
//! 一層可放多張切片（`SpriteStackParts`：同一層的各零件疊在同一格），動畫只掛在 `.stack-spin`。
//! `SpriteStackSheet`：切片合成一張圖集、一個請求（吉祥物）。

use leptos::prelude::*;

/// 每層位移（層寬 %）：0.85 格。
fn layer_step(grid: usize) -> f32 {
    100.0 * 0.85 / grid as f32
}

/// 一個 `.stack-layer`（k = 半層序號；偶數整格、奇數半格位移）。
fn layer_view(k: usize, step: f32, srcs: Vec<String>) -> impl IntoView {
    let pos = (k / 2) as f32 + if k % 2 == 1 { 0.5 } else { 0.0 };
    view! {
      <div class="stack-layer" style=format!("--i:{pos};--step:{step:.3}")>
        <div class="stack-spin">
          {srcs
            .into_iter()
            .map(|src| view! { <img src=src alt="" loading="lazy" decoding="async"/> })
            .collect_view()}
        </div>
      </div>
    }
}

fn stage_view(label: String, class: String, layers: AnyView) -> impl IntoView {
    let decorative = label.is_empty();
    view! {
      <div
        class=format!("stack-stage {class}")
        role=(!decorative).then_some("img")
        aria-label=(!decorative).then_some(label)
        aria-hidden=decorative.then_some("true")
      >
        {layers}
      </div>
    }
}

#[component]
pub fn SpriteStack(
    /// 切片目錄，例如 `/assets/shop/stack/desk-plant`
    base: String,
    /// 層數（目錄內 webp 張數）
    layers: usize,
    /// 切片的平面格數（寬＝高）
    grid: usize,
    /// 無障礙名稱；空字串＝純裝飾（aria-hidden）
    #[prop(into)]
    label: String,
    /// 額外 class（尺寸、外框）
    #[prop(optional, into)]
    class: String,
) -> impl IntoView {
    let step = layer_step(grid);
    let slices = (0..layers * 2)
        .map(|k| layer_view(k, step, vec![format!("{base}/{:02}.webp", k / 2)]))
        .collect_view()
        .into_any();
    stage_view(label, class, slices)
}

/// 多零件組合疊：每個零件只有部分層有切片（`<base>/<part>/NN.webp`，NN＝全域層號）。
/// 層 z 的 `.stack-spin` 內依 `parts` 順序放所有在 z 層有切片的零件；層數＝所有零件最高層＋1。
#[component]
pub fn SpriteStackParts(
    /// 零件根目錄，例如 `/assets/stack/fs/bedroom`
    base: String,
    /// (零件 id, 該零件有切片的層號)；順序即同層內的繪製順序
    parts: Vec<(&'static str, &'static [u8])>,
    /// 切片的平面格數（寬＝高）
    grid: usize,
    /// 無障礙名稱；空字串＝純裝飾（aria-hidden）
    #[prop(into)]
    label: String,
    /// 額外 class（尺寸、外框）
    #[prop(optional, into)]
    class: String,
) -> impl IntoView {
    let step = layer_step(grid);
    let layers = parts
        .iter()
        .flat_map(|(_, ls)| ls.iter())
        .max()
        .map_or(0, |&z| z as usize + 1);
    let slices = (0..layers * 2)
        .map(|k| {
            let z = (k / 2) as u8;
            let srcs = parts
                .iter()
                .filter(|(_, ls)| ls.contains(&z))
                .map(|(id, _)| format!("{base}/{id}/{z:02}.webp"))
                .collect();
            layer_view(k, step, srcs)
        })
        .collect_view()
        .into_any();
    stage_view(label, class, slices)
}

/// 圖集版：所有切片合在一張圖（`cols` 欄、由左而右由上而下，第 0 格＝最底層），一個請求。
/// 每層是一格 `.stack-cell`，以 `background-position` 取格；圖集網址放在舞台的 `--sheet`。
/// 另放一張隱藏 `<img>` 預載：先藏起來（display:none）的疊不會載背景圖，顯示瞬間才不會閃。
#[component]
pub fn SpriteStackSheet(
    /// 圖集網址（建議帶 `?v=` 內容 hash）
    src: String,
    /// 層數
    layers: usize,
    /// 圖集欄數
    cols: usize,
    /// 切片的平面格數（寬＝高）
    grid: usize,
    /// 無障礙名稱；空字串＝純裝飾（aria-hidden）
    #[prop(into)]
    label: String,
    /// 額外 class（尺寸、外框）
    #[prop(optional, into)]
    class: String,
) -> impl IntoView {
    let step = layer_step(grid);
    let cols = cols.max(1);
    let rows = layers.div_ceil(cols).max(1);
    // 背景位置百分比：第 c 欄＝c/(cols-1)（單欄／單列時為 0）
    let pct = |i: usize, n: usize| {
        if n > 1 {
            100.0 * i as f32 / (n - 1) as f32
        } else {
            0.0
        }
    };
    let slices = (0..layers * 2)
        .map(|k| {
            let z = k / 2;
            let pos = z as f32 + if k % 2 == 1 { 0.5 } else { 0.0 };
            let bg = format!(
                "background-position:{:.4}% {:.4}%",
                pct(z % cols, cols),
                pct(z / cols, rows)
            );
            view! {
              <div class="stack-layer" style=format!("--i:{pos};--step:{step:.3}")>
                <div class="stack-spin">
                  <div class="stack-cell" style=bg></div>
                </div>
              </div>
            }
        })
        .collect_view();
    let decorative = label.is_empty();
    view! {
      <div
        class=format!("stack-stage {class}")
        style=format!("--sheet:url('{src}');--cols:{cols};--rows:{rows}")
        role=(!decorative).then_some("img")
        aria-label=(!decorative).then_some(label)
        aria-hidden=decorative.then_some("true")
      >
        <img class="stack-preload" src=src.clone() alt="" />
        {slices}
      </div>
    }
}
