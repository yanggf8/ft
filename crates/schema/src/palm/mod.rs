//! 手相 v1 — 純函式模組（設計定稿：docs/superpowers/specs/2026-10-27-palm-v1-design.md）。
//! 無 IO、native 與 wasm32 皆可編；web（規則式解讀顯示）與 api（VLM 抽取驗證＋落庫）
//! 共用同一型別，三端單一事實源。
//!
//! 紅線：手相資料永不寫入 predictions／prediction_feedback／f8_assignments；
//! 照片永不落庫（本模組只見特徵，不見影像）。
//!
//! serde 形狀＝camelCase（抽取 prompt 的 JSON 範本同形，VLM 照抄範本 key；
//! features_json 落庫與 PalmReadingView wire 同一形狀）。`deny_unknown_fields`
//! 擋 VLM 幻覺欄位（spec P7）。

pub mod corpus;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 抽取 prompt＋本 schema 版本（改任一必 bump，spec P18）。
pub const PALM_FEATURES_VERSION: &str = "1";

/// 規則式解讀語料版本（`corpus.rs` 改版必 bump）。
pub const PALM_CORPUS_VERSION: &str = "1";

/// v2 AI 深度解讀 prompt 版本（spec P16；v2 實作時啟用）。
pub const PALM_PROMPT_VERSION: &str = "1";

/// 主環（llama-4-scout）品質閘 confidence 門檻（spike §6.2：合格 0.6 vs 注入 0.0，
/// 0.3 居中完美分隔）。
pub const GATE_THRESHOLD_SCOUT: f32 = 0.3;

/// 整體照片品質自評（VLM 先自評再填特徵；`Unknown`＝模型不確定——一律擋）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PalmQuality {
    Ok,
    PoorLight,
    Blurry,
    HandNotFound,
    Unknown,
}

/// 手型（火土風水；`Unknown`＝無法判讀）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandShape {
    Fire,
    Earth,
    Air,
    Water,
    Unknown,
}

/// 手指相對掌長。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FingerLength {
    Short,
    Medium,
    Long,
    Unknown,
}

/// 五指相對長度（VLM 估計值，非量測）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FingerLengths {
    pub thumb: FingerLength,
    pub index: FingerLength,
    pub middle: FingerLength,
    pub ring: FingerLength,
    pub pinky: FingerLength,
}

/// 掌線鍵（中文掌相術語對照寫死在抽取 prompt：生命線=life、智慧線=head、
/// 感情線=heart、命運線=fate、婚姻線=marriage、太陽線=sun）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineKey {
    Life,
    Head,
    Heart,
    Fate,
    Marriage,
    Sun,
}

/// 線存在性三態（`Absent`＝照片中確實不存在——命運線先天缺席是常態，
/// 與「看不清楚」的 `Unknown` 分離；spec P7/Muse B5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LinePresence {
    Present,
    Absent,
    Unknown,
}

/// 線深淺。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineDepth {
    Shallow,
    Normal,
    Deep,
    Unknown,
}

/// 線長短（相對手掌比例的拓撲判斷，非量測）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineLength {
    Short,
    Medium,
    Long,
    Unknown,
}

/// 線弧度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineCurve {
    Straight,
    Slight,
    Deep,
    Unknown,
}

/// 線特徵（`visible_breaks`：`None`＝無法判讀；spike 教訓——計數對 VLM 是偽精確，
/// 只取 boolean，spec §6 開放問題 3）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineTraits {
    pub depth: LineDepth,
    pub length: LineLength,
    pub curve: LineCurve,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_breaks: Option<bool>,
}

/// 單線狀態（`presence != Present` 時 `traits` 應為 `None`——僅 present 才解讀）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineStatus {
    pub presence: LinePresence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traits: Option<LineTraits>,
}

/// 掌丘鍵（抽取 prompt 對照：金星丘=venus、木星丘=jupiter、土星丘=saturn、
/// 太陽丘=sun、水星丘=mercury）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MountKey {
    Venus,
    Jupiter,
    Saturn,
    Sun,
    Mercury,
}

/// 掌丘隆起度（wire 值 `none` 對應 Rust `None` variant——注意勿與 `Option::None` 混淆）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MountProminence {
    None,
    Slight,
    Prominent,
    Unknown,
}

/// 手掌特徵（VLM 抽取目標；落庫 `features_json` 與前端 wire 同形）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PalmFeatures {
    pub hand_shape: HandShape,
    pub finger_lengths: FingerLengths,
    pub lines: HashMap<LineKey, LineStatus>,
    pub mounts: HashMap<MountKey, MountProminence>,
}

/// VLM 抽取回應 wrapper（品質自評＋信心＋特徵；strict serde 的目標形狀）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PalmExtraction {
    pub quality: PalmQuality,
    pub confidence: f32,
    pub features: PalmFeatures,
}

/// confidence 伺服端驗證：必須有限且 ∈[0,1]（NaN／±inf 會使 `< threshold` 恆 false
/// 而穿閘——spec P8／Codex B5；serde `f32` 會收 NaN，這道防線不可省）。
pub fn valid_confidence(c: f32) -> bool {
    c.is_finite() && (0.0..=1.0).contains(&c)
}

/// 品質閘（spec P8）：`quality != Ok` 一律擋；`min_confidence` 為 `Some(t)` 時
/// 另驗 `confidence` 有效且 `>= t`。備環 llama-3.2-11b-vision 的 confidence 校準
/// 不可靠（spike §6.2），呼叫端傳 `None` 只靠 quality 欄把關。
pub fn passes_gate(ext: &PalmExtraction, min_confidence: Option<f32>) -> bool {
    if ext.quality != PalmQuality::Ok {
        return false;
    }
    match min_confidence {
        Some(t) => valid_confidence(ext.confidence) && ext.confidence >= t,
        None => true,
    }
}

/// 組裝輸出的固定線序（與抽取 prompt 的 schema 範本序一致）。
pub const LINE_ORDER: [LineKey; 6] = [
    LineKey::Life,
    LineKey::Head,
    LineKey::Heart,
    LineKey::Fate,
    LineKey::Marriage,
    LineKey::Sun,
];

/// 規則式解讀組裝（spec §3.3）：開場紅線句 → 手型總論（非 Unknown）→ 主線逐條
/// （僅 `presence == Present`；屬性 unknown 的短語自動跳過）→ 提問式收尾。
/// 全文 ≤600 字元——超過時自末線起捨棄段落直到達標（收尾句恆保留）。
pub fn assemble_palm_interpretation(features: &PalmFeatures) -> String {
    use corpus::{
        curve_phrase, depth_phrase, hand_shape_intro, length_phrase, line_name, line_theme,
        CLOSING_LINE, INTRO_LINE,
    };

    let mut sections: Vec<String> = Vec::new();
    if let Some(intro) = hand_shape_intro(features.hand_shape) {
        sections.push(intro.to_string());
    }
    for key in LINE_ORDER {
        let Some(status) = features.lines.get(&key) else {
            continue;
        };
        if status.presence != LinePresence::Present {
            continue;
        }
        let mut parts: Vec<String> = vec![line_theme(key).to_string()];
        if let Some(t) = &status.traits {
            for p in [
                depth_phrase(t.depth),
                length_phrase(t.length),
                curve_phrase(t.curve),
            ]
            .into_iter()
            .flatten()
            {
                parts.push(p.to_string());
            }
            if t.visible_breaks == Some(true) {
                parts.push("紋路中可見斷點，常被解讀為階段的轉換而非阻斷".to_string());
            }
        }
        sections.push(format!("{}：{}", line_name(key), parts.join("，")));
    }

    // 開場恆在；段落超額自末段捨棄；收尾恆保留。
    let head = INTRO_LINE;
    let tail = CLOSING_LINE;
    let mut out = String::new();
    for s in &sections {
        let candidate = if out.is_empty() {
            format!("{}{}{}", head, s, tail)
        } else {
            format!("{}{}{}{}", head, out, s, tail)
        };
        if candidate.chars().count() > 600 {
            break;
        }
        out = if out.is_empty() {
            format!("{}{}", head, s)
        } else {
            format!("{}{}", out, s)
        };
    }
    if out.is_empty() {
        out = format!("{}{}", head, tail);
    } else {
        out = format!("{}{}", out, tail);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_traits(d: LineDepth, l: LineLength, c: LineCurve) -> LineTraits {
        LineTraits {
            depth: d,
            length: l,
            curve: c,
            visible_breaks: Some(false),
        }
    }

    fn present(t: LineTraits) -> LineStatus {
        LineStatus {
            presence: LinePresence::Present,
            traits: Some(t),
        }
    }

    fn full_features() -> PalmFeatures {
        let mut lines = HashMap::new();
        lines.insert(
            LineKey::Life,
            present(full_traits(
                LineDepth::Deep,
                LineLength::Long,
                LineCurve::Deep,
            )),
        );
        lines.insert(
            LineKey::Head,
            present(full_traits(
                LineDepth::Normal,
                LineLength::Medium,
                LineCurve::Straight,
            )),
        );
        lines.insert(
            LineKey::Heart,
            present(full_traits(
                LineDepth::Deep,
                LineLength::Medium,
                LineCurve::Slight,
            )),
        );
        lines.insert(
            LineKey::Fate,
            present(full_traits(
                LineDepth::Unknown,
                LineLength::Unknown,
                LineCurve::Unknown,
            )),
        );
        lines.insert(
            LineKey::Marriage,
            present(full_traits(
                LineDepth::Shallow,
                LineLength::Short,
                LineCurve::Slight,
            )),
        );
        lines.insert(
            LineKey::Sun,
            present(full_traits(
                LineDepth::Unknown,
                LineLength::Unknown,
                LineCurve::Unknown,
            )),
        );
        PalmFeatures {
            hand_shape: HandShape::Earth,
            finger_lengths: FingerLengths {
                thumb: FingerLength::Medium,
                index: FingerLength::Medium,
                middle: FingerLength::Long,
                ring: FingerLength::Medium,
                pinky: FingerLength::Short,
            },
            lines,
            mounts: HashMap::new(),
        }
    }

    fn full_json() -> String {
        serde_json::json!({
            "quality": "ok",
            "confidence": 0.88,
            "features": {
                "handShape": "earth",
                "fingerLengths": {
                    "thumb": "medium", "index": "medium", "middle": "long",
                    "ring": "medium", "pinky": "short"
                },
                "lines": {
                    "life": {"presence": "present", "traits": {"depth": "deep", "length": "long", "curve": "deep", "visibleBreaks": false}},
                    "head": {"presence": "present", "traits": {"depth": "normal", "length": "medium", "curve": "straight"}},
                    "fate": {"presence": "absent"}
                },
                "mounts": {"venus": "slight", "jupiter": "none"}
            }
        })
        .to_string()
    }

    // ── serde 契約 ──

    #[test]
    fn extraction_round_trip_camel_case() {
        let json = serde_json::json!({
            "quality": "ok",
            "confidence": 0.88,
            "features": {
                "handShape": "earth",
                "fingerLengths": {"thumb": "medium", "index": "medium", "middle": "long", "ring": "medium", "pinky": "short"},
                "lines": {
                    "life": {"presence": "present", "traits": {"depth": "deep", "length": "long", "curve": "deep", "visibleBreaks": false}},
                    "fate": {"presence": "absent"}
                },
                "mounts": {"venus": "slight", "jupiter": "none"}
            }
        });
        let ext: PalmExtraction = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(ext.quality, PalmQuality::Ok);
        assert!((ext.confidence - 0.88).abs() < 1e-6);
        assert_eq!(ext.features.hand_shape, HandShape::Earth);
        assert_eq!(
            ext.features.lines[&LineKey::Life]
                .traits
                .as_ref()
                .unwrap()
                .depth,
            LineDepth::Deep
        );
        assert_eq!(
            ext.features.lines[&LineKey::Fate].presence,
            LinePresence::Absent
        );
        assert_eq!(
            ext.features.mounts[&crate::palm::MountKey::Jupiter],
            MountProminence::None
        );
        // 序列化回 camelCase 且 absent 線不帶 traits（skip_serializing_if）
        let back = serde_json::to_value(&ext).unwrap();
        assert_eq!(back["features"]["fingerLengths"]["middle"], "long");
        assert!(back["features"]["lines"]["fate"].get("traits").is_none());
        assert_eq!(back["features"]["mounts"]["jupiter"], "none");
    }

    #[test]
    fn deny_unknown_fields_blocks_hallucinations() {
        let mut v: serde_json::Value = serde_json::from_str(&full_json()).unwrap();
        v["moneyLine"] = "gold".into(); // 幻覺欄位
        assert!(serde_json::from_value::<PalmExtraction>(v).is_err());
        let mut v2: serde_json::Value = serde_json::from_str(&full_json()).unwrap();
        v2["features"]["skinTone"] = "tan".into(); // 特徵層幻覺欄位
        assert!(serde_json::from_value::<PalmExtraction>(v2).is_err());
        let mut v3: serde_json::Value = serde_json::from_str(&full_json()).unwrap();
        v3["features"]["lines"]["money"] = serde_json::json!({"presence": "present"}); // 幻覺線 key
        assert!(serde_json::from_value::<PalmExtraction>(v3).is_err());
    }

    #[test]
    fn unknown_variants_accepted() {
        let v = serde_json::json!({
            "quality": "unknown",
            "confidence": 0.0,
            "features": {
                "handShape": "unknown",
                "fingerLengths": {"thumb": "unknown", "index": "unknown", "middle": "unknown", "ring": "unknown", "pinky": "unknown"},
                "lines": {"life": {"presence": "unknown"}},
                "mounts": {}
            }
        });
        let ext: PalmExtraction = serde_json::from_value(v).unwrap();
        assert_eq!(ext.quality, PalmQuality::Unknown);
        assert_eq!(ext.features.hand_shape, HandShape::Unknown);
        assert!(ext.features.lines[&LineKey::Life].traits.is_none());
    }

    // ── 品質閘 ──

    #[test]
    fn valid_confidence_rejects_nan_inf_and_range() {
        assert!(!valid_confidence(f32::NAN));
        assert!(!valid_confidence(f32::INFINITY));
        assert!(!valid_confidence(f32::NEG_INFINITY));
        assert!(!valid_confidence(-0.01));
        assert!(!valid_confidence(1.5));
        assert!(valid_confidence(0.0));
        assert!(valid_confidence(1.0));
        assert!(valid_confidence(0.55));
    }

    #[test]
    fn gate_blocks_non_ok_quality_first() {
        let mut ext: PalmExtraction = serde_json::from_str(&full_json()).unwrap();
        ext.quality = PalmQuality::PoorLight;
        assert!(!passes_gate(&ext, None)); // 備環（llama-3.2）只靠 quality 欄
        assert!(!passes_gate(&ext, Some(GATE_THRESHOLD_SCOUT)));
        ext.quality = PalmQuality::HandNotFound;
        assert!(!passes_gate(&ext, None));
    }

    #[test]
    fn gate_confidence_threshold_paths() {
        let mut ext: PalmExtraction = serde_json::from_str(&full_json()).unwrap();
        ext.confidence = 0.6;
        assert!(passes_gate(&ext, Some(GATE_THRESHOLD_SCOUT)));
        ext.confidence = 0.0; // scout 主環：0.0（注入樣本實測值）必須被擋
        assert!(!passes_gate(&ext, Some(GATE_THRESHOLD_SCOUT)));
        assert!(passes_gate(&ext, None)); // None 模式不驗 confidence
        ext.confidence = f32::NAN; // NaN 穿透防護（Codex B5）
        assert!(!passes_gate(&ext, Some(GATE_THRESHOLD_SCOUT)));
    }

    // ── 規則式解讀組裝 ──

    #[test]
    fn assembly_full_features_within_600_chars() {
        let out = assemble_palm_interpretation(&full_features());
        assert!(out.chars().count() <= 600, "len = {}", out.chars().count());
        assert!(out.contains("土型"));
        assert!(out.contains("生命線"));
        assert!(out.contains("命運線"));
        assert!(out.ends_with("不是一張考卷。"));
    }

    #[test]
    fn assembly_skips_absent_unknown_lines_and_phrases() {
        let mut f = full_features();
        f.hand_shape = HandShape::Unknown;
        f.lines.clear();
        f.lines.insert(
            LineKey::Life,
            present(full_traits(
                LineDepth::Deep,
                LineLength::Unknown,
                LineCurve::Unknown,
            )),
        );
        f.lines.insert(
            LineKey::Fate,
            LineStatus {
                presence: LinePresence::Absent,
                traits: None,
            },
        );
        f.lines.insert(
            LineKey::Sun,
            LineStatus {
                presence: LinePresence::Unknown,
                traits: None,
            },
        );
        let out = assemble_palm_interpretation(&f);
        assert!(out.contains("生命線"));
        assert!(!out.contains("命運線")); // absent 不解讀（語料原則：present 才出場）
        assert!(!out.contains("太陽線"));
        assert!(!out.contains("偏長")); // length Unknown 短語跳過
        assert!(
            !out.contains("火型")
                && !out.contains("土型")
                && !out.contains("風型")
                && !out.contains("水型")
        );
    }

    #[test]
    fn assembly_mentions_visible_breaks() {
        let mut f = full_features();
        let t = LineTraits {
            depth: LineDepth::Deep,
            length: LineLength::Long,
            curve: LineCurve::Deep,
            visible_breaks: Some(true),
        };
        f.lines.insert(LineKey::Heart, present(t));
        let out = assemble_palm_interpretation(&f);
        assert!(out.contains("斷點"));
    }

    #[test]
    fn assembly_empty_features_still_has_intro_and_closing() {
        let f = PalmFeatures {
            hand_shape: HandShape::Unknown,
            finger_lengths: FingerLengths {
                thumb: FingerLength::Unknown,
                index: FingerLength::Unknown,
                middle: FingerLength::Unknown,
                ring: FingerLength::Unknown,
                pinky: FingerLength::Unknown,
            },
            lines: HashMap::new(),
            mounts: HashMap::new(),
        };
        let out = assemble_palm_interpretation(&f);
        assert!(out.contains("娛樂與自我反思"));
        assert!(out.contains("一面鏡子"));
    }
}
