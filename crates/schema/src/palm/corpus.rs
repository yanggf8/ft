//! 手相語料（spec §1／§3.3）——組裝式而非全枚舉：每線「基調句＋屬性短語＋提問」
//! 拼接，避免 6 線 × 3×3×3 屬性的笛卡兒積爆炸。措辭原則：提問式非宣判式、
//! 禁確定性斷言（醫療／法律／財務一律不觸）、每段 ≤120 字。
//!
//! 紅線：娛樂與自我反思定位（比照塔羅 system prompt 紅線句）。

use super::{HandShape, LineCurve, LineDepth, LineKey, LineLength};

/// 手型總論（4 型；`Unknown` 不輸出段落）。
pub fn hand_shape_intro(shape: HandShape) -> Option<&'static str> {
    match shape {
        HandShape::Fire => Some("你的手掌偏火型——掌長指短、掌心溫暖。火型手常見熱情先行、行動力強的特質，想到就做是你的節奏，偶爾也需要替衝動留一點緩衝。"),
        HandShape::Earth => Some("你的手掌偏土型——方掌方指、厚實穩重。土型手常見務實可靠的特質，你偏好看得見的累積，節奏雖慢但走得遠。"),
        HandShape::Air => Some("你的手掌偏風型——掌指皆長、靈活俐落。風型手常見思路敏捷、好奇心旺盛的特質，你喜歡交流新想法，也留意別讓思緒跑得太快。"),
        HandShape::Water => Some("你的手掌偏水型——掌長指細、柔軟敏感。水型手常見感受細膩、同理心強的特質，你對氛圍的察覺力是天賦，記得也照顧自己的情緒界線。"),
        HandShape::Unknown => None,
    }
}

/// 線中文名（顯示與語料開頭用）。
pub fn line_name(key: LineKey) -> &'static str {
    match key {
        LineKey::Life => "生命線",
        LineKey::Head => "智慧線",
        LineKey::Heart => "感情線",
        LineKey::Fate => "命運線",
        LineKey::Marriage => "婚姻線",
        LineKey::Sun => "太陽線",
    }
}

/// 線基調句（提問式收尾的核心語意；每線一句）。
pub fn line_theme(key: LineKey) -> &'static str {
    match key {
        LineKey::Life => {
            "生命線環繞拇指、談的是生命力的節奏與身心的韌性——它的形態常被拿來對照你的能量狀態。"
        }
        LineKey::Head => "智慧線橫越掌心、談的是思考的風格——你傾向怎麼想事情、怎麼做決定。",
        LineKey::Heart => "感情線談的是情感的表達方式——你怎麼愛、怎麼讓人靠近。",
        LineKey::Fate => {
            "命運線縱走掌心、談的是方向感與階段性的耕耘——不少人此線天生不明顯，與命好不好無關。"
        }
        LineKey::Marriage => {
            "婚姻線談的是親密關係的樣貌——它描繪的比較是你對親密的態度，而非婚姻事件本身。"
        }
        LineKey::Sun => "太陽線談的是自我實現的光采——你被什麼點亮、又在哪裡發光。",
    }
}

/// 深淺短語（`Unknown` 回空——組裝時跳過）。
pub fn depth_phrase(d: LineDepth) -> Option<&'static str> {
    match d {
        LineDepth::Deep => Some("本次拍攝中它紋路清晰深刻，顯示這個層面的能量相當鮮明"),
        LineDepth::Normal => Some("紋路深淺適中，狀態平穩"),
        LineDepth::Shallow => Some("紋路偏淺，這個層面可能還在醞釀、尚未定型"),
        LineDepth::Unknown => None,
    }
}

/// 長短短語。
pub fn length_phrase(l: LineLength) -> Option<&'static str> {
    match l {
        LineLength::Long => Some("線條偏長，跨度完整"),
        LineLength::Medium => Some("長度適中"),
        LineLength::Short => Some("線條偏短，聚焦在當下"),
        LineLength::Unknown => None,
    }
}

/// 弧度短語。
pub fn curve_phrase(c: LineCurve) -> Option<&'static str> {
    match c {
        LineCurve::Deep => Some("弧度明顯，情感或能量的流動感較強"),
        LineCurve::Slight => Some("弧度柔和"),
        LineCurve::Straight => Some("走勢平直，風格務實直接"),
        LineCurve::Unknown => None,
    }
}

/// 解讀組裝的固定語（開場與收尾）。
pub const INTRO_LINE: &str =
    "以下解讀基於本次拍攝呈現的掌紋特徵，手相是娛樂與自我反思的工具——它描繪傾向，不預言結局。";
pub const CLOSING_LINE: &str =
    "這些描述有貼近你的部分嗎？掌紋會隨歲月緩慢變化，把它當作一面鏡子，而不是一張考卷。";
