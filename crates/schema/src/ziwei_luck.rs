//! 紫微子單元：流年大運／今年運勢（spec 2026-11-21-sub-units §A.4–A.5）。
//!
//! - [`ZiWeiHoroscopeV1`]：引擎 `ft_ziwei::horoscope` 的輸出（worker → api → web）。
//!   **索引一律 子=0**（與 `ZiWeiChartV3` 相同），並同時帶 `branch` 字串；宮格以地支定位。
//!   引擎版本不放進 DTO（R16）：api 回應外層 `engineVersionZiweiHoroscope` 蓋章。
//! - 規則說明文字（流年大運頁每種標記一行）＋禁字測試：只描述「傳統上怎麼看」，
//!   不得出現吉凶、注定、預告類詞。
//! - `ZIWEI_YEAR_PROMPT_VERSION`：今年運勢 AI prompt 版本，改 prompt 必 bump。

use serde::{Deserialize, Serialize};

/// 今年運勢 AI prompt 版本（快取鍵之一）。改 `build_ziwei_year_prompt`／system prompt 必 bump。
pub const ZIWEI_YEAR_PROMPT_VERSION: &str = "1";

/// 今年運勢 AI 解讀結尾提醒句：由 route **附加**，不靠 AI 生成。
pub const ZIWEI_YEAR_CLOSING: &str = "流年是傳統的觀察框架，不是預測。";

/// 大限（起運前為童限：`is_childhood=true`、無年齡區間）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HoroscopeDecadal {
    /// 子=0
    pub palace_index: u8,
    pub branch: String,
    pub stem: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_age: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_age: Option<u8>,
    pub is_childhood: bool,
}

/// 小限。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HoroscopeAge {
    /// 子=0
    pub palace_index: u8,
    pub branch: String,
    pub stem: String,
}

/// 流年十二宮名（以流年命宮起排），以地支為鍵。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BranchName {
    pub branch: String,
    pub name: String,
}

/// 流年四化（星名 zh-TW）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoroscopeMutagen {
    pub lu: String,
    pub quan: String,
    pub ke: String,
    pub ji: String,
}

impl HoroscopeMutagen {
    /// `[(流祿, 星), (流權, 星), (流科, 星), (流忌, 星)]`
    pub fn labeled(&self) -> [(&'static str, &str); 4] {
        [
            ("流祿", self.lu.as_str()),
            ("流權", self.quan.as_str()),
            ("流科", self.ke.as_str()),
            ("流忌", self.ji.as_str()),
        ]
    }
}

/// 流年。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HoroscopeYearly {
    /// 子=0
    pub palace_index: u8,
    pub branch: String,
    pub stem: String,
    /// 12 筆，依地支 子→亥 排列
    pub palace_names: Vec<BranchName>,
    pub mutagen: HoroscopeMutagen,
}

/// 引擎流年輸出（不含流月以下層級）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZiWeiHoroscopeV1 {
    /// 引擎實際計算的陽曆日期（`YYYY-M-D`）
    pub target_date: String,
    /// 這個流年對應的農曆年（春節前的日期屬前一年）
    pub luck_year: i32,
    /// 虛歲
    pub nominal_age: u32,
    pub decadal: HoroscopeDecadal,
    pub age: HoroscopeAge,
    pub yearly: HoroscopeYearly,
}

/// `GET /api/ziwei/horoscope` 回應。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZiweiHoroscopeResponse {
    pub horoscope: ZiWeiHoroscopeV1,
    pub engine_version_ziwei_horoscope: String,
}

/// 今年運勢上方的純規則摘要。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZiweiYearSummary {
    pub horoscope: ZiWeiHoroscopeV1,
    pub engine_version_ziwei_horoscope: String,
}

/// 今年運勢 AI 解讀。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZiweiYearReading {
    pub luck_year: i32,
    pub interpretation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub cached: bool,
}

/// `GET /api/ziwei/year` 回應：摘要＋已有的解讀（沒有則 null）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ZiweiYearResponse {
    pub summary: ZiweiYearSummary,
    pub reading: Option<ZiweiYearReading>,
}

// ── 規則說明文字（流年大運頁「怎麼看這張圖」）──

pub const RULE_DECADAL: &str = "這十年的關注主軸落在的宮位，十年換一宮。";
pub const RULE_CHILDHOOD: &str =
    "還沒起大限的幼年期，傳統上依年齡輪流看命宮、財帛、疾厄、夫妻、福德、官祿。";
pub const RULE_YEARLY: &str =
    "這一年關注焦點落在的宮位；各宮上方的「流年○○」是以它為起點重新排的十二宮。";
pub const RULE_AGE: &str = "依年齡每年移一宮的輔助參考。";
pub const RULE_MUTAGEN: &str = "這一年的天干帶動的四化，和出生時的「化○」分開標示。";
pub const RULE_FRAME: &str = "流年是傳統的觀察框架，用來整理「可以留意什麼」，不是預測。";

/// 所有固定規則文字（禁字測試與前端共用）。
pub const RULE_TEXTS: &[&str] = &[
    RULE_DECADAL,
    RULE_CHILDHOOD,
    RULE_YEARLY,
    RULE_AGE,
    RULE_MUTAGEN,
    RULE_FRAME,
    ZIWEI_YEAR_CLOSING,
];

/// 流年大運／今年運勢固定文字禁用詞：吉凶裁決、宿命、事件預告。
pub const LUCK_BANNED_WORDS: &[&str] = &[
    "吉",
    "凶",
    "注定",
    "一定",
    "必然",
    "保證",
    "命中",
    "災",
    "厄運",
    "劫數",
    "破財",
    "血光",
    "犯太歲",
    "沖煞",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_texts_have_no_banned_words() {
        for t in RULE_TEXTS {
            for w in LUCK_BANNED_WORDS {
                assert!(!t.contains(w), "「{w}」出現在：{t}");
            }
        }
    }

    #[test]
    fn horoscope_wire_shape_is_camel_case_and_childhood_omits_ages() {
        let h = ZiWeiHoroscopeV1 {
            target_date: "2026-7-1".into(),
            luck_year: 2026,
            nominal_age: 3,
            decadal: HoroscopeDecadal {
                palace_index: 11,
                branch: "亥".into(),
                stem: "丁".into(),
                start_age: None,
                end_age: None,
                is_childhood: true,
            },
            age: HoroscopeAge {
                palace_index: 4,
                branch: "辰".into(),
                stem: "庚".into(),
            },
            yearly: HoroscopeYearly {
                palace_index: 6,
                branch: "午".into(),
                stem: "丙".into(),
                palace_names: vec![],
                mutagen: HoroscopeMutagen {
                    lu: "天同".into(),
                    quan: "天機".into(),
                    ke: "文昌".into(),
                    ji: "廉貞".into(),
                },
            },
        };
        let v = serde_json::to_value(&h).unwrap();
        assert_eq!(v["luckYear"], 2026);
        assert_eq!(v["nominalAge"], 3);
        assert_eq!(v["decadal"]["isChildhood"], true);
        assert!(v["decadal"].get("startAge").is_none());
        assert_eq!(v["yearly"]["palaceIndex"], 6);
        let back: ZiWeiHoroscopeV1 = serde_json::from_value(v).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn prompt_version_pinned() {
        assert_eq!(ZIWEI_YEAR_PROMPT_VERSION, "1");
    }
}
