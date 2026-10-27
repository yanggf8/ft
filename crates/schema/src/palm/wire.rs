//! palm 路由用 wire 契約（spec P1/P3/P8b/P13/P17）：請求 DTO＋驗證器、回應
//! view、抽取回應 fence 剝除、品質閘 per-model 閾值映射。
//! 無 IO，native 與 wasm32 皆可編（mod.rs 同款約束）。

use super::{PalmFeatures, GATE_THRESHOLD_SCOUT};

/// 主環模型（§6.2 選型終局）——api 的 DO PALM_CHAIN 與 route 品質閘共用此
/// 單一來源（防兩處字串漂移）。
pub const SCOUT_MODEL: &str = "@cf/meta/llama-4-scout-17b-16e-instruct";
/// 備環模型（confidence 校準不可靠——§6.2，品質閘只靠 quality 欄）。
pub const BACKUP_MODEL: &str = "@cf/meta/llama-3.2-11b-vision-instruct";

/// P3 單一實質約束：imageBase64 字串 ≤512KiB×4/3（同時封頂 body 與解碼後大小）。
pub const MAX_IMAGE_BASE64_CHARS: usize = 682_667;
/// P3 mime 宣告白名單。
pub const PALM_MIMES: [&str; 3] = ["image/png", "image/jpeg", "image/webp"];

/// P13 請求 DTO（wire 鍵 camelCase；deny 擋多餘欄位）。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PalmAnalyzeRequest {
    pub request_id: String,
    pub hand: String,
    pub image_base64: String,
    pub mime_type: String,
}

/// requestId UUID 形狀（比照 tarot `valid_tarot_request_id`）。
pub fn valid_palm_request_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

/// hand ∈ {left, right}（P13）。
pub fn valid_hand(hand: &str) -> bool {
    hand == "left" || hand == "right"
}

/// mime 宣告值 ∈ PALM_MIMES（P3）。
pub fn valid_mime(mime: &str) -> bool {
    PALM_MIMES.contains(&mime)
}

/// imageBase64 非空且 ≤ [`MAX_IMAGE_BASE64_CHARS`]（P3 單一約束）。
pub fn valid_image_b64(b64: &str) -> bool {
    !b64.is_empty() && b64.len() <= MAX_IMAGE_BASE64_CHARS
}

/// P8b：剝 ```json 圍籬（模型不守「只輸出 JSON」規矩時的防呆）→ trim。
pub fn strip_json_fence(s: &str) -> &str {
    let t = s.trim();
    let body = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    body.trim().strip_suffix("```").unwrap_or(body).trim()
}

/// §6.2/P8：per-model 品質閘閾值——scout `Some(0.3)`；備環（及其他）`None`
/// （llama-3.2 confidence 不可校準，`passes_gate` 只靠 quality 欄短路）。
pub fn gate_min_confidence(model: &str) -> Option<f32> {
    if model == SCOUT_MODEL {
        Some(GATE_THRESHOLD_SCOUT)
    } else {
        None
    }
}

/// P17 回應 view（camelCase；ai 欄位 skip-if-none；interpretation／features
/// 恆回傳）。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PalmReadingView {
    pub id: String,
    pub hand: String,
    pub features: PalmFeatures,
    pub interpretation: String,
    pub corpus_version: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_chart_context: Option<bool>,
}

/// 歷史分頁（比照 `TarotHistoryPage`）。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PalmHistoryPage {
    pub items: Vec<PalmReadingView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palm::{FingerLength, FingerLengths, HandShape};

    fn view() -> PalmReadingView {
        PalmReadingView {
            id: "r".into(),
            hand: "left".into(),
            features: PalmFeatures {
                hand_shape: HandShape::Unknown,
                finger_lengths: FingerLengths {
                    thumb: FingerLength::Unknown,
                    index: FingerLength::Unknown,
                    middle: FingerLength::Unknown,
                    ring: FingerLength::Unknown,
                    pinky: FingerLength::Unknown,
                },
                lines: Default::default(),
                mounts: Default::default(),
            },
            interpretation: "解讀".into(),
            corpus_version: "1".into(),
            created_at: "2026-10-27T00:00:00.000Z".into(),
            ai_chart_context: None,
        }
    }

    #[test]
    fn request_id_shape() {
        assert!(valid_palm_request_id(
            "01234567-89ab-cdef-0123-456789abcdef"
        ));
        assert!(!valid_palm_request_id(
            "01234567-89ab-cdef-0123-456789abcde"
        ));
        assert!(!valid_palm_request_id(
            "01234567_89ab_cdef_0123_456789abcdef"
        ));
        assert!(!valid_palm_request_id(""));
    }

    #[test]
    fn hand_and_mime_whitelists() {
        assert!(valid_hand("left"));
        assert!(valid_hand("right"));
        assert!(!valid_hand("Left"));
        assert!(!valid_hand("upside"));
        assert!(!valid_hand(""));
        for m in PALM_MIMES {
            assert!(valid_mime(m), "{m}");
        }
        assert!(!valid_mime("image/gif"));
        assert!(!valid_mime("image/jpg"), "宣告值須全名 jpeg");
    }

    #[test]
    fn image_base64_cap_boundary() {
        assert!(valid_image_b64(&"A".repeat(682_667)), "P3 上限本身應過");
        assert!(!valid_image_b64(&"A".repeat(682_668)), "超過一字即拒");
        assert!(!valid_image_b64(""));
    }

    #[test]
    fn fence_stripping() {
        assert_eq!(
            strip_json_fence("```json\n{\"quality\":\"ok\"}\n```"),
            "{\"quality\":\"ok\"}"
        );
        assert_eq!(
            strip_json_fence("```\n{\"quality\":\"ok\"}\n```"),
            "{\"quality\":\"ok\"}"
        );
        assert_eq!(
            strip_json_fence("{\"quality\":\"ok\"}"),
            "{\"quality\":\"ok\"}"
        );
        assert_eq!(strip_json_fence("  {\"q\":1}  "), "{\"q\":1}");
    }

    #[test]
    fn gate_threshold_per_model() {
        assert_eq!(gate_min_confidence(SCOUT_MODEL), Some(GATE_THRESHOLD_SCOUT));
        assert_eq!(gate_min_confidence(SCOUT_MODEL), Some(0.3));
        assert_eq!(gate_min_confidence(BACKUP_MODEL), None);
        assert_eq!(gate_min_confidence("anything-else"), None);
    }

    #[test]
    fn reading_view_wire_shape() {
        let mut v = view();
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains("\"corpusVersion\""), "camelCase");
        assert!(!s.contains("aiChartContext"), "None → skip-if-none");
        assert!(
            s.contains("\"interpretation\"") && s.contains("\"features\""),
            "恆回傳欄位"
        );
        v.ai_chart_context = Some(false);
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains("\"aiChartContext\":false"));
    }

    #[test]
    fn analyze_request_camel_case_keys() {
        let v: PalmAnalyzeRequest = serde_json::from_str(
            r#"{"requestId":"01234567-89ab-cdef-0123-456789abcdef","hand":"left","imageBase64":"QUJD","mimeType":"image/webp"}"#,
        )
        .expect("camelCase wire 必須可解");
        assert_eq!(v.hand, "left");
        assert_eq!(v.image_base64, "QUJD");
        assert_eq!(v.mime_type, "image/webp");
        assert!(
            serde_json::from_str::<PalmAnalyzeRequest>(
                r#"{"requestId":"x","hand":"left","imageBase64":"QUJD","mimeType":"image/webp","extra":1}"#
            )
            .is_err(),
            "deny_unknown_fields 擋多餘欄位"
        );
    }
}
