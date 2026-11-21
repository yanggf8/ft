//! Sprite stacking 偽 3D：把橫切片由下而上疊起、整疊一起自轉（2026-11-22）。
//!
//! 切片放在 `<base>/NN.webp`（00 = 最底層），由 `scripts/sprite-stack/*.py` 程式化體素模型產生；
//! 每層畫兩次（整格＋半格位移）補層間縫。位移量＝0.85 格，依切片格數 `grid` 換算成層寬的 %，
//! 以 `--step` 行內變數交給 CSS（`.stack-layer`）；壓扁比例、自轉與減少動態效果在 style.css。

use leptos::prelude::*;

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
    let step = 100.0 * 0.85 / grid as f32;
    let slices = (0..layers * 2)
        .map(|k| {
            let z = k / 2;
            let pos = z as f32 + if k % 2 == 1 { 0.5 } else { 0.0 };
            let src = format!("{base}/{z:02}.webp");
            view! {
              <div class="stack-layer" style=format!("--i:{pos};--step:{step:.3}")>
                <img src=src alt="" loading="lazy" decoding="async"/>
              </div>
            }
        })
        .collect_view();
    let decorative = label.is_empty();
    view! {
      <div
        class=format!("stack-stage {class}")
        role=(!decorative).then_some("img")
        aria-label=(!decorative).then_some(label)
        aria-hidden=decorative.then_some("true")
      >
        {slices}
      </div>
    }
}
