//! 手相 v1 — `/palm` 公開 L0（教學＋示意）＋`/palm/capture` 拍照解讀（登入）。
//! spec P4/P6/P14/P15/P17/P22（docs/superpowers/specs/2026-10-27-palm-v1-design.md）。
//!
//! 照片隱私（P5/P6）：照片只存在請求生命週期——預覽是 in-memory data URL，
//! 不寫 localStorage；送出後即棄。歷史只存特徵與規則式解讀（API 契約）。

use ft_schema::palm::corpus::line_name;
use ft_schema::palm::wire::{PalmAnalyzeRequest, PalmReadingView};
use ft_schema::palm::{
    FingerLength, HandShape, LineKey, LinePresence, LineStatus, MountKey, MountProminence,
    PalmFeatures, LINE_ORDER,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;
use wasm_bindgen::JsCast;

use crate::api;
use crate::auth::use_auth;

// ═══ 共用：掌形示意圖（RW1 一物三用——L0 教學圖＝capture 導引 overlay）═══

/// 線稿掌形（stroke: currentColor——由外層 class 決定色彩，不引入新 token）。
/// `labels`：標注三主線（僅 L0 教學用；overlay 版無字）。
#[component]
fn PalmDiagram(labels: bool) -> impl IntoView {
    view! {
        <svg class="palm-figure" viewBox="0 0 220 300" role="img"
            aria-label="掌心朝鏡頭、手指朝上的掌形示意"
        >
            // 四指（中指最長的線稿韻律）
            <line x1="72"  y1="108" x2="72"  y2="182" />
            <line x1="100" y1="72"  x2="100" y2="182" />
            <line x1="128" y1="66"  x2="128" y2="182" />
            <line x1="156" y1="98"  x2="156" y2="182" />
            // 拇指
            <line x1="68" y1="208" x2="40" y2="156" />
            // 掌腹輪廓
            <path d="M 62 150 C 44 186 56 244 110 252 C 164 244 176 186 158 150" />
            // 主線：生命／智慧／感情（教學與導引的同一組曲線）
            <path class="palm-line" d="M 90 158 C 98 190 110 214 130 236" />
            <path class="palm-line" d="M 86 190 C 106 183 126 185 144 196" />
            <path class="palm-line" d="M 88 208 C 108 200 128 202 148 214" />
            <Show when=move || labels>
                <text x="132" y="238">"生命線"</text>
                <text x="118" y="180">"智慧線"</text>
                <text x="120" y="222">"感情線"</text>
            </Show>
        </svg>
    }
}

// ═══ L0：公開教學頁（P15）═══

#[component]
pub fn PalmL0Page() -> impl IntoView {
    let auth = use_auth();

    view! {
        <div class="page">
            <A href="/" attr:class="back-link">"← 返回"</A>
            <h1 style="margin-bottom:0.5rem">"手相"</h1>
            <p class="muted" style="margin-bottom:1.5rem">
                "拍一張掌心照，看看掌紋說什麼。不需要出生資料，也不用背任何規則。"
            </p>

            <div class="card">
                <div class="palm-demo">
                    <PalmDiagram labels=true />
                    <div>
                        <h2 style="margin:0 0 0.5rem 0">"怎麼拍"</h2>
                        <ul class="palm-steps">
                            <li>"掌心朝鏡頭、手指朝上，整隻手入鏡"</li>
                            <li>"光線充足處拍攝，掌紋越清楚越好"</li>
                            <li>"放輕鬆自然張開，不用刻意繃緊"</li>
                        </ul>
                        <p class="muted palm-figure-note">"示意圖（非真人手掌）"</p>
                    </div>
                </div>
                <p style="margin:0.75rem 0 0 0">
                    "上圖標出的三條主線，只是起點：生命線談節奏與韌性，智慧線談思考風格，感情線談情感的表達方式。"
                    "拍完照，我們會從照片抽取特徵，再用提問式的解讀陪你對照自己——"
                    "它描繪傾向，不預言結局，是娛樂與自我反思的工具。"
                </p>
                <p class="muted" style="font-size:0.9rem;margin:0.75rem 0 0 0">
                    "示範：解讀不會說「你的感情線代表什麼」，而會問——「感情線的弧度，"
                    "是否讓你在關係裡既能投入，也保有自己的節奏？」"
                </p>
            </div>

            <div class="card" style="margin-top:1rem">
                <h2 style="margin:0 0 0.5rem 0">"照片的隱私"</h2>
                <p style="margin:0">
                    "照片只用於當次分析：送至第三方 AI 服務解析後即棄，我們不保存照片，"
                    "只保存解析出的手相特徵（不含照片）供你查看歷史紀錄，你可以隨時刪除。"
                </p>
            </div>

            <div style="margin-top:1.5rem">
                <Show when=move || auth.is_authed()>
                    <A href="/palm/capture" attr:class="cta">"開始拍攝手掌"</A>
                </Show>
                <Show when=move || !auth.is_authed()>
                    <A href="/login?next=/palm/capture" attr:class="cta">"登入後開始拍攝"</A>
                    <p class="muted" style="font-size:0.85rem;margin:0.5rem 0 0 0">
                        "教學與示意完全公開；拍攝分析需要登入（免費）。"
                    </p>
                </Show>
            </div>
        </div>
    }
}

// ═══ capture：拍照解讀（P4/P6/P22）═══

/// 擷取結果（只存在記憶體；不落 localStorage——P5/P6）。
#[derive(Clone)]
struct Captured {
    data_url: String,
    mime: String,
    base64: String,
}

#[derive(Clone, Copy, PartialEq)]
enum CamState {
    Off,
    On,
    Failed,
}

async fn get_environment_stream() -> Result<web_sys::MediaStream, wasm_bindgen::JsValue> {
    let window = web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("no window"))?;
    let devices = window.navigator().media_devices()?;
    // facingMode:'environment'（P14 後鏡頭優先）——字典 union 用 Reflect 組，穩定於 web-sys 版本
    let video = js_sys::Object::new();
    js_sys::Reflect::set(
        &video,
        &wasm_bindgen::JsValue::from("facingMode"),
        &wasm_bindgen::JsValue::from("environment"),
    )?;
    let constraints = js_sys::Object::new();
    js_sys::Reflect::set(
        &constraints,
        &wasm_bindgen::JsValue::from("video"),
        &wasm_bindgen::JsValue::from(video),
    )?;
    // 字典型別沒有 From<Object>——unchecked_into（web-sys 慣例）
    let cs: web_sys::MediaStreamConstraints = constraints.unchecked_into();
    let promise = devices.get_user_media_with_constraints(&cs)?;
    let stream = wasm_bindgen_futures::JsFuture::from(promise).await?;
    Ok(web_sys::MediaStream::from(stream))
}

fn stop_stream(stream: &web_sys::MediaStream) {
    for track in stream.get_tracks().iter() {
        if let Ok(t) = track.dyn_into::<web_sys::MediaStreamTrack>() {
            t.stop();
        }
    }
}

/// 縮圖畫布（P4：≤768px 長邊；等比縮小、不放大小圖）。
fn make_canvas(vw: u32, vh: u32) -> Option<web_sys::HtmlCanvasElement> {
    let scale = (768.0 / u32::max(vw, vh) as f64).min(1.0);
    let w = (vw as f64 * scale).round() as u32;
    let h = (vh as f64 * scale).round() as u32;
    let doc = web_sys::window()?.document()?;
    let c = doc
        .create_element("canvas")
        .ok()?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .ok()?;
    c.set_width(w);
    c.set_height(h);
    Some(c)
}

/// `canvas.toDataURL(type, quality)`——web-sys 未綁 quality 變體，走 js_sys。
fn canvas_to_data_url(
    canvas: &web_sys::HtmlCanvasElement,
    mime: &str,
    quality: f64,
) -> Option<String> {
    let f = js_sys::Reflect::get(canvas.as_ref(), &wasm_bindgen::JsValue::from("toDataURL"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    f.call2(
        canvas.as_ref(),
        &wasm_bindgen::JsValue::from(mime),
        &wasm_bindgen::JsValue::from_f64(quality),
    )
    .ok()?
    .as_string()
}

/// 編碼：WebP q80，瀏覽器不支援退 jpeg（P4/RW5）。canvas.toDataURL 與 toBlob
/// 同一編碼器——同步版本省 BlobCallback 接線。
fn canvas_to_captured(canvas: &web_sys::HtmlCanvasElement) -> Option<Captured> {
    let mut data_url = canvas_to_data_url(canvas, "image/webp", 0.8)?;
    let mut mime = "image/webp";
    if !data_url.starts_with("data:image/webp") {
        data_url = canvas_to_data_url(canvas, "image/jpeg", 0.85)?;
        if !data_url.starts_with("data:image/jpeg") {
            return None;
        }
        mime = "image/jpeg";
    }
    let base64 = data_url.split_once(',')?.1.to_string();
    Some(Captured {
        data_url,
        mime: mime.to_string(),
        base64,
    })
}

fn capture_from_video(video: &web_sys::HtmlVideoElement) -> Option<Captured> {
    let (vw, vh) = (video.video_width(), video.video_height());
    if vw == 0 || vh == 0 {
        return None;
    }
    let canvas = make_canvas(vw, vh)?;
    let ctx = canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .ok()?;
    ctx.draw_image_with_html_video_element_and_dw_and_dh(
        video,
        0.0,
        0.0,
        canvas.width() as f64,
        canvas.height() as f64,
    )
    .ok()?;
    canvas_to_captured(&canvas)
}

fn capture_from_img(img: &web_sys::HtmlImageElement) -> Option<Captured> {
    let (vw, vh) = (img.natural_width(), img.natural_height());
    if vw == 0 || vh == 0 {
        return None;
    }
    let canvas = make_canvas(vw, vh)?;
    let ctx = canvas
        .get_context("2d")
        .ok()??
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .ok()?;
    // <img> 顯示即套用 EXIF 方向（瀏覽器預設 image-orientation: from-image），
    // 畫到 canvas 的就是轉正後的畫面——RW4：全程同一座標。
    ctx.draw_image_with_html_image_element_and_dw_and_dh(
        img,
        0.0,
        0.0,
        canvas.width() as f64,
        canvas.height() as f64,
    )
    .ok()?;
    canvas_to_captured(&canvas)
}

/// 首次知情（P6 文案逐字；二段式——首次點擊先明示，再點才開相機）。
const NOTICE_KEY: &str = "palm_ai_notice";
const NOTICE_TEXT: &str = "照片將送至第三方 AI 服務解析，解析後即棄，我們不保存照片。我們會保存解析出的手相特徵（不含照片），供你查看歷史紀錄，你可以隨時刪除。";

fn palm_error_message(e: &api::ApiErr) -> String {
    if e.is_code("PALM_QUALITY_LOW") {
        "照片品質不足，請在光線充足處、掌心朝鏡頭重拍一張。".to_string()
    } else if e.is_code("INVALID_IMAGE") || e.is_code("INVALID_PALM") {
        "照片格式無法解析，請重拍一張。".to_string()
    } else if e.status() == Some(413) {
        "照片檔案太大，請重拍一張。".to_string()
    } else if e.status() == Some(429) {
        "分析次數已達上限（24 小時內 3 次），請稍後再試。".to_string()
    } else if e.status() == Some(409) {
        "偵測到重複的分析請求，請再試一次。".to_string()
    } else {
        "分析失敗，請稍後再試。".to_string()
    }
}

// ── P22 特徵區標籤（短語；不出現 schema 英文術語——Codex S5）──

fn shape_label(s: HandShape) -> &'static str {
    match s {
        HandShape::Fire => "火型掌",
        HandShape::Earth => "土型掌",
        HandShape::Air => "風型掌",
        HandShape::Water => "水型掌",
        HandShape::Unknown => "未能判讀手型",
    }
}

fn finger_len_label(l: FingerLength) -> &'static str {
    match l {
        FingerLength::Short => "偏短",
        FingerLength::Medium => "適中",
        FingerLength::Long => "偏長",
        FingerLength::Unknown => "未判讀",
    }
}

fn mount_label(m: MountProminence) -> &'static str {
    match m {
        MountProminence::None => "平坦",
        MountProminence::Slight => "略隆起",
        MountProminence::Prominent => "隆起",
        MountProminence::Unknown => "未判讀",
    }
}

fn mount_name(k: MountKey) -> &'static str {
    match k {
        MountKey::Venus => "金星丘",
        MountKey::Jupiter => "木星丘",
        MountKey::Saturn => "土星丘",
        MountKey::Sun => "太陽丘",
        MountKey::Mercury => "水星丘",
    }
}

const FINGERS: [(&str, &str); 5] = [
    ("thumb", "拇指"),
    ("index", "食指"),
    ("middle", "中指"),
    ("ring", "無名指"),
    ("pinky", "小指"),
];
const MOUNT_ORDER: [MountKey; 5] = [
    MountKey::Venus,
    MountKey::Jupiter,
    MountKey::Saturn,
    MountKey::Sun,
    MountKey::Mercury,
];

/// 單線標籤（P22）：present→屬性列表；absent→此線不明顯；unknown/缺→未能判讀。
fn line_label(key: LineKey, status: &LineStatus) -> String {
    let name = line_name(key);
    if status.presence != LinePresence::Present {
        return match status.presence {
            LinePresence::Absent => format!("{name}：此線不明顯"),
            _ => format!("{name}：照片中未能判讀"),
        };
    }
    let Some(t) = &status.traits else {
        return format!("{name}：清晰可見");
    };
    let mut parts: Vec<&str> = Vec::new();
    match t.depth {
        ft_schema::palm::LineDepth::Deep => parts.push("紋路深刻"),
        ft_schema::palm::LineDepth::Normal => parts.push("深淺適中"),
        ft_schema::palm::LineDepth::Shallow => parts.push("紋路偏淺"),
        _ => {}
    }
    match t.length {
        ft_schema::palm::LineLength::Long => parts.push("偏長"),
        ft_schema::palm::LineLength::Medium => parts.push("長度適中"),
        ft_schema::palm::LineLength::Short => parts.push("偏短"),
        _ => {}
    }
    match t.curve {
        ft_schema::palm::LineCurve::Deep => parts.push("弧度明顯"),
        ft_schema::palm::LineCurve::Slight => parts.push("弧度柔和"),
        ft_schema::palm::LineCurve::Straight => parts.push("走勢平直"),
        _ => {}
    }
    if t.visible_breaks == Some(true) {
        parts.push("可見斷點");
    }
    if parts.is_empty() {
        format!("{name}：清晰可見")
    } else {
        format!("{name}：{}", parts.join("、"))
    }
}

/// 特徵標籤全列表（P22 順序：手型→主線→手指→掌丘）。
fn feature_tags(f: &PalmFeatures) -> Vec<String> {
    let mut out = vec![shape_label(f.hand_shape).to_string()];
    for key in LINE_ORDER {
        match f.lines.get(&key) {
            Some(s) => out.push(line_label(key, s)),
            None => out.push(format!("{}：照片中未能判讀", line_name(key))),
        }
    }
    for (field, zh) in FINGERS {
        let len = match field {
            "thumb" => f.finger_lengths.thumb,
            "index" => f.finger_lengths.index,
            "middle" => f.finger_lengths.middle,
            "ring" => f.finger_lengths.ring,
            _ => f.finger_lengths.pinky,
        };
        out.push(format!("{zh}：{}", finger_len_label(len)));
    }
    for m in MOUNT_ORDER {
        let label = f
            .mounts
            .get(&m)
            .map(|v| mount_label(*v))
            .unwrap_or("未判讀");
        out.push(format!("{}：{label}", mount_name(m)));
    }
    out
}

#[component]
pub fn PalmCapturePage() -> impl IntoView {
    let hand = RwSignal::new("left");
    let notice_pending = RwSignal::new(
        web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .and_then(|s| s)
            .map(|s| s.get_item(NOTICE_KEY).ok().flatten().is_none())
            // 儲存不可用時 fail-closed：寧可再示知一次（P6 同意閘；Codex 審）
            .unwrap_or(true),
    );
    let store_notice = move || {
        let _ = web_sys::window()
            .and_then(|w| w.local_storage().ok())
            .and_then(|s| s)
            .map(|s| s.set_item(NOTICE_KEY, "1"));
    };

    let cam_state = RwSignal::new(CamState::Off);
    let video_ref = NodeRef::<leptos::html::Video>::new();
    let stream = RwSignal::new(None::<web_sys::MediaStream>);
    // fallback 檔案的現役 object URL（換檔/失敗/離開都要 revoke——Codex 審）
    let file_url = RwSignal::new(None::<String>);
    let disposed = RwSignal::new(false);
    // 相機開啟世代：路徑切換（關閉/改檔案/重複點擊）即失效，遲到的許可
    // 回應一律 stop 不留（Codex 複審：pending 期間的競態）
    let camera_open_gen = RwSignal::new(0u32);
    let preview = RwSignal::new(None::<Captured>);
    let hidden_img_ref = NodeRef::<leptos::html::Img>::new();
    let submitting = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let result = RwSignal::new(None::<PalmReadingView>);

    let items = RwSignal::new(Vec::<PalmReadingView>::new());
    let next_cursor = RwSignal::new(None::<String>);
    let history_error = RwSignal::new(String::new());
    let history_note = RwSignal::new(String::new());
    let loading_history = RwSignal::new(false);
    // 兩段式刪除確認（tarot 先例）：Some(id)＝已上膛；"__all__"＝整批
    let armed_delete = RwSignal::new(None::<String>);

    let refresh_history = move || {
        spawn_local(async move {
            match api::palm_history(None).await {
                Ok(page) => {
                    items.set(page.items);
                    next_cursor.set(page.next_cursor);
                    history_error.set(String::new());
                }
                Err(_) => history_error.set("無法載入歷史紀錄。".to_string()),
            }
        });
    };

    Effect::new(move |_| {
        spawn_local(async move {
            match api::palm_history(None).await {
                Ok(page) => {
                    items.set(page.items);
                    next_cursor.set(page.next_cursor);
                }
                Err(_) => history_error.set("無法載入歷史紀錄。".to_string()),
            }
        });
    });

    // Critical（Codex 審）：<video> 節點在 cam_state=On 之後才渲染——串流綁定
    // 放 Effect，節點與串流都就位才接 srcObject＋play（先後到齊皆可觸發）
    Effect::new(move |_| {
        if let (Some(s), Some(v)) = (stream.get(), video_ref.get()) {
            let _ = v.set_src_object(Some(&s));
            let _ = v.play();
        }
    });

    // 離開頁面清場：停串流、revoke object URL（Codex 審 Important）
    on_cleanup(move || {
        disposed.set(true);
        if let Some(s) = stream.get_untracked() {
            stop_stream(&s);
        }
        if let Some(u) = file_url.get_untracked() {
            let _ = web_sys::Url::revoke_object_url(&u);
        }
    });

    let stop_camera = move || {
        camera_open_gen.update(|g| *g += 1);
        if let Some(s) = stream.get_untracked() {
            stop_stream(&s);
        }
        stream.set(None);
        cam_state.set(CamState::Off);
    };

    let start_camera = move |_| {
        if cam_state.get_untracked() == CamState::On {
            return;
        }
        error.set(String::new());
        let gen = camera_open_gen.get_untracked() + 1;
        camera_open_gen.set(gen);
        spawn_local(async move {
            match get_environment_stream().await {
                Ok(s) => {
                    // 世代/離開守衛：路徑已切換或已離開 → 停掉不留（Codex 複審）
                    if disposed.get_untracked() || gen != camera_open_gen.get_untracked() {
                        stop_stream(&s);
                        return;
                    }
                    // 防禦性：覆蓋前先停舊流（不同世代的殘留）
                    if let Some(old) = stream.get_untracked() {
                        stop_stream(&old);
                    }
                    stream.set(Some(s));
                    cam_state.set(CamState::On);
                    // srcObject 綁定交給下方 Effect：video 節點要等 On 才存在
                }
                Err(_) => {
                    // 過期世代的失敗不標記（使用者已另起／已切路徑）
                    if gen == camera_open_gen.get_untracked() {
                        cam_state.set(CamState::Failed);
                    }
                }
            }
        });
    };

    let on_shoot = move |_| {
        let Some(v) = video_ref.get_untracked() else {
            return;
        };
        match capture_from_video(&v) {
            Some(c) => {
                preview.set(Some(c));
                stop_camera();
                error.set(String::new());
            }
            None => error.set("擷取畫面失敗，請再試一次。".to_string()),
        }
    };

    // fallback：檔案/相簿（相機拒絕或無法開啟時的主要路徑，也隨時可用）
    let on_file = move |ev: leptos::ev::Event| {
        let Some(input) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(file) = input
            .files()
            .and_then(|f| {
                let first = js_sys::Array::from(f.as_ref()).get(0);
                first.dyn_into::<web_sys::File>().ok()
            })
            .and_then(|f| f.dyn_into::<web_sys::File>().ok())
        else {
            return;
        };
        input.set_value("");
        let Ok(url) = web_sys::Url::create_object_url_with_blob(&file) else {
            error.set("無法讀取照片，請重拍一張。".to_string());
            return;
        };
        error.set(String::new());
        // 改走檔案路徑＝活串流不再需要（Codex 審 Important）
        stop_camera();
        // 追蹤現役 object URL：換檔先 revoke 舊的（Codex 審 Important）
        if let Some(old) = file_url.get_untracked() {
            let _ = web_sys::Url::revoke_object_url(&old);
        }
        file_url.set(Some(url.clone()));
        if let Some(img) = hidden_img_ref.get() {
            let _ = img.set_attribute("src", &url);
        }
    };
    let on_hidden_img_load = move |_| {
        let Some(img) = hidden_img_ref.get_untracked() else {
            return;
        };
        let src = img.get_attribute("src").unwrap_or_default();
        if src.is_empty() {
            return;
        }
        match capture_from_img(&img) {
            Some(c) => {
                preview.set(Some(c));
                error.set(String::new());
            }
            None => error.set("無法處理這張照片，請換一張重試。".to_string()),
        }
        let _ = web_sys::Url::revoke_object_url(&src);
        file_url.set(None);
        let _ = hidden_img_ref
            .get_untracked()
            .map(|img| img.set_attribute("src", ""));
    };

    // 檔案載入失敗（壞檔）：revoke＋清追蹤，不留懸空 URL（Codex 審 Important）
    let on_hidden_img_error = move |_| {
        if let Some(u) = file_url.get_untracked() {
            let _ = web_sys::Url::revoke_object_url(&u);
            file_url.set(None);
        }
        if let Some(img) = hidden_img_ref.get_untracked() {
            let _ = img.set_attribute("src", "");
        }
        error.set("無法處理這張照片，請換一張重試。".to_string());
    };

    let on_submit = move |_| {
        if submitting.get_untracked() {
            return;
        }
        let Some(c) = preview.get_untracked() else {
            return;
        };
        let Some(request_id) = web_sys::window()
            .and_then(|w| w.crypto().ok())
            .map(|c| c.random_uuid())
        else {
            error.set("無法產生請求識別碼，請重新整理頁面。".to_string());
            return;
        };
        let body = PalmAnalyzeRequest {
            request_id,
            hand: hand.get_untracked().to_string(),
            image_base64: c.base64.clone(),
            mime_type: c.mime.clone(),
        };
        submitting.set(true);
        error.set(String::new());
        spawn_local(async move {
            let outcome = api::palm_analyze(&body).await;
            submitting.set(false);
            match outcome {
                Ok(view) => {
                    result.set(Some(view));
                    preview.set(None);
                    refresh_history();
                }
                Err(e) => error.set(palm_error_message(&e)),
            }
        });
    };

    view! {
        <div class="page">
            <A href="/palm" attr:class="back-link">"← 手相教學"</A>
            <h1 style="margin-bottom:0.5rem">"拍攝手掌"</h1>
            <p class="muted" style="margin-bottom:1.5rem">
                "掌心朝鏡頭、手指朝上，光線充足掌紋越清楚。分析後只保留特徵與解讀，照片即棄。"
            </p>

            // 首次知情（P6：二段式——首次點擊先明示，再點才開相機）
            <Show when=move || notice_pending.get()>
                <div class="card">
                    <h2 style="margin:0 0 0.5rem 0">"開始之前"</h2>
                    <p style="margin:0">{NOTICE_TEXT}</p>
                    <button
                        class="btn-primary"
                        style="margin-top:0.75rem"
                        on:click=move |_| {
                            notice_pending.set(false);
                            store_notice();
                        }
                    >"了解，繼續"</button>
                </div>
            </Show>

            <Show when=move || !notice_pending.get()>
                <div class="card">
                    // 手別（P13：hand ∈ left/right）
                    <div class="palm-hand-toggle" role="group" aria-label="拍攝哪一隻手">
                        <button
                            class="palm-hand-btn"
                            class:active=move || hand.get() == "left"
                            aria-pressed=move || hand.get() == "left"
                            on:click=move |_| hand.set("left")
                        >"左手"</button>
                        <button
                            class="palm-hand-btn"
                            class:active=move || hand.get() == "right"
                            aria-pressed=move || hand.get() == "right"
                            on:click=move |_| hand.set("right")
                        >"右手"</button>
                    </div>

                    // 預覽優先；無預覽時顯示相機或開啟按鈕
                    {move || match preview.get() {
                        Some(c) => view! {
                            <div class="palm-preview">
                                <img
                                    class="palm-preview-img"
                                    src={c.data_url}
                                    alt="本次拍攝的手掌照片（分析後即棄）"
                                />
                                <div class="palm-preview-actions">
                                    <button class="btn-link" on:click=move |_| preview.set(None)>
                                        "重拍"
                                    </button>
                                    <button
                                        class="btn-primary"
                                        disabled=move || submitting.get()
                                        on:click=on_submit
                                    >
                                        {move || if submitting.get() { "分析中…" } else { "開始分析" }}
                                    </button>
                                </div>
                                <p class="muted" style="font-size:0.8rem;margin:0.5rem 0 0 0">
                                    "照片會送至第三方 AI 服務解析，解析後即棄，我們不保存照片。"
                                </p>
                            </div>
                        }.into_any(),
                        None => view! {
                            <Show when=move || cam_state.get() == CamState::On>
                                <div class="palm-video-wrap">
                                    <video
                                        node_ref=video_ref
                                        class="palm-video"
                                        autoplay=true
                                        playsinline=true
                                        muted=true
                                        aria-label="相機預覽——掌心朝鏡頭、手指朝上"
                                    ></video>
                                    // 掌形導引 overlay（RW1：與 L0 同一款示意）
                                    <div class="palm-overlay" aria-hidden="true">
                                        <PalmDiagram labels=false />
                                    </div>
                                </div>
                                <div class="palm-preview-actions">
                                    <button class="btn-link" on:click=move |_| stop_camera()>
                                        "關閉相機"
                                    </button>
                                    <button class="btn-primary" on:click=on_shoot>"拍照"</button>
                                </div>
                            </Show>
                            <Show when=move || cam_state.get() != CamState::On>
                                <button class="btn-primary" on:click=start_camera>
                                    {move || if cam_state.get() == CamState::Failed {
                                        "再試一次開啟相機"
                                    } else {
                                        "開啟相機"
                                    }}
                                </button>
                                <Show when=move || cam_state.get() == CamState::Failed>
                                    <p class="muted" style="font-size:0.85rem;margin:0.5rem 0 0 0">
                                        "無法開啟相機（可能被拒絕或裝置不支援）——可以用下面的檔案方式選一張照片。"
                                    </p>
                                </Show>
                            </Show>
                            <label class="btn-link palm-file-label">
                                "改用相簿或檔案"
                                <input
                                    class="palm-file-input"
                                    type="file"
                                    accept="image/*"
                                    capture="environment"
                                    on:change=on_file
                                />
                            </label>
                        }.into_any(),
                    }}

                    <Show when=move || !error.get().is_empty()>
                        <p class="error" style="margin-top:0.75rem">{move || error.get()}</p>
                    </Show>
                    // fallback 路徑的隱藏 <img>（EXIF 由瀏覽器顯示時自動轉正——RW4）
                    <img node_ref=hidden_img_ref class="palm-file-img" alt="" on:load=on_hidden_img_load on:error=on_hidden_img_error />
                </div>

                // 結果（P22：規則式解讀快照為主體＋特徵區中文標籤）
                {move || result.get().map(|r| {
                    let interpretation = r.interpretation.clone();
                    let hand_note = if r.hand == "left" { "左手" } else { "右手" };
                    let date = r.created_at.split_once('T').map(|(d, _)| d.to_string())
                        .unwrap_or_else(|| r.created_at.clone());
                    let tags = feature_tags(&r.features);
                    view! {
                        <div class="card" style="margin-top:1rem">
                            <p class="muted" style="margin:0 0 0.5rem 0;font-size:0.85rem">
                                {format!("{}・{}", date, hand_note)}
                            </p>
                            <p style="white-space:pre-line;margin:0">{interpretation}</p>
                            <div class="palm-tags">
                                {tags.into_iter().map(|t| view! {
                                    <span class="palm-chip">{t}</span>
                                }).collect_view()}
                            </div>
                        </div>
                    }.into_any()
                })}
            </Show>

            // 歷史（P1：GET/DELETE——tarot 同款兩段刪除＋整批）
            <h2 style="margin:2rem 0 0.75rem 0">"歷史紀錄"</h2>
            <Show when=move || !history_note.get().is_empty()>
                <p class="muted" style="margin:0 0 0.5rem 0">{move || history_note.get()}</p>
            </Show>
            <Show when=move || !history_error.get().is_empty()>
                <p class="error">{move || history_error.get()}</p>
            </Show>
            <Show
                when=move || !items.get().is_empty()
                fallback=|| view! { <p class="muted">"還沒有紀錄。拍一張手掌照吧。"</p> }
            >
                <div style="display:flex;justify-content:flex-end">
                    {move || {
                        let is_armed = move || armed_delete.get_untracked().as_deref() == Some("__all__");
                        view! {
                            <button
                                class="btn-link"
                                on:click=move |_| {
                                    if is_armed() {
                                        armed_delete.set(None);
                                        spawn_local(async move {
                                            match api::palm_delete_all().await {
                                                Ok(n) => {
                                                    history_note.set(format!("已刪除 {n} 筆"));
                                                    result.set(None);
                                                    // P1：DELETE 回應後重取列表（跨裝置競態 last-write-wins；Codex 審）
                                                    refresh_history();
                                                }
                                                Err(_) => {
                                                    history_error.set("刪除失敗，請稍後再試。".to_string())
                                                }
                                            }
                                        });
                                    } else {
                                        armed_delete.set(Some("__all__".to_string()));
                                    }
                                }
                            >
                                {move || if armed_delete.get().as_deref() == Some("__all__") {
                                    "確定刪除全部紀錄？再點一次執行".to_string()
                                } else {
                                    "刪除全部紀錄".to_string()
                                }}
                            </button>
                        }
                    }}
                </div>
                {move || items.get().into_iter().map(|item| {
                    let id = item.id.clone();
                    let id_label = id.clone();
                    let hand_note = if item.hand == "left" { "左手" } else { "右手" };
                    let date = item.created_at.split_once('T').map(|(d, _)| d.to_string())
                        .unwrap_or_else(|| item.created_at.clone());
                    let interpretation = item.interpretation.clone();
                    let tags = feature_tags(&item.features);
                    view! {
                        <div class="card" style="margin-top:1rem">
                            <p class="muted" style="margin:0 0 0.25rem 0;font-size:0.85rem">
                                {format!("{}・{}", date, hand_note)}
                            </p>
                            <p class="muted" style="white-space:pre-line;margin:0 0 0.5rem 0">
                                {interpretation}
                            </p>
                            <div class="palm-tags">
                                {tags.into_iter().map(|t| view! {
                                    <span class="palm-chip">{t}</span>
                                }).collect_view()}
                            </div>
                            <div style="margin-top:0.5rem">
                                <button class="btn-link" on:click=move |_| {
                                    let this_id = id.clone();
                                    if armed_delete.get_untracked().as_deref() == Some(this_id.as_str()) {
                                        armed_delete.set(None);
                                        let id_for_del = id.clone();
                                        spawn_local(async move {
                                            match api::palm_delete(&id_for_del).await {
                                                Ok(()) | Err(api::ApiErr::Api { status: 404, .. }) => {
                                                    if result
                                                        .get_untracked()
                                                        .as_ref()
                                                        .map(|r| r.id.as_str())
                                                        == Some(id_for_del.as_str())
                                                    {
                                                        result.set(None);
                                                    }
                                                    // P1：DELETE 回應後重取列表（Codex 審）
                                                    refresh_history();
                                                }
                                                Err(_) => {
                                                    history_error.set("刪除失敗，請稍後再試。".to_string())
                                                }
                                            }
                                        });
                                    } else {
                                        armed_delete.set(Some(id.clone()));
                                    }
                                }>
                                    {move || if armed_delete.get().as_deref() == Some(id_label.as_str()) {
                                        "確定刪除？再點一次".to_string()
                                    } else {
                                        "刪除".to_string()
                                    }}
                                </button>
                            </div>
                        </div>
                    }.into_any()
                }).collect_view()}
                {move || next_cursor.get().map(|cursor| {
                    view! {
                        <div style="margin-top:1rem">
                            <button class="btn-link" on:click=move |_| {
                                if loading_history.get_untracked() { return; }
                                loading_history.set(true);
                                let cursor = cursor.clone();
                                spawn_local(async move {
                                    match api::palm_history(Some(&cursor)).await {
                                        Ok(page) => {
                                            items.update(|v| v.extend(page.items));
                                            next_cursor.set(page.next_cursor);
                                        }
                                        Err(_) => history_error.set("載入更多失敗。".to_string()),
                                    }
                                    loading_history.set(false);
                                });
                            }>"載入更多"</button>
                        </div>
                    }.into_any()
                })}
            </Show>
        </div>
    }
}
