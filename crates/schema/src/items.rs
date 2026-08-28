//! IPIP-15 題目資料 — 逐字引用李仁豪、陳怡君 (2016) 附錄 2
//! （底本 Zheng et al. 2008 簡體版改繁；非 IPIP 官網 Sih-Ci Jhu 譯本）。
//! 來源 PDF：docs/superpowers/specs/assets/ipip15-lee-chen-2016.pdf
//! 授權：IPIP items are in the public domain。
//! 反向題依據：論文附錄 1 之 29R/34R/49R 標記（新編號 10–12）。
//! 題目與常模放 schema：web 禁依賴 domain/big5，但結果頁需要題目文字與常模。

/// 維度索引：0=外向性 1=友善性 2=嚴謹性 3=情緒穩定性 4=智性／想像
pub const DIMENSION_NAMES: [&str; 5] = ["外向性", "友善性", "嚴謹性", "情緒穩定性", "智性／想像"];

/// 作答錨點（論文頁 97）：1=非常不精確 … 5=非常精確。
pub const SCALE_ANCHORS: [&str; 5] = ["非常不精確", "有些不精確", "普通", "有些精確", "非常精確"];

pub struct Item {
    /// 1-based 新編號（answers 陣列索引 = no − 1）
    pub no: usize,
    /// IPIP-50 原題號（選題淵源）
    pub source_no: usize,
    pub text: &'static str,
    pub dimension: usize,
    pub reverse: bool,
}

pub const ITEMS: [Item; 15] = [
    Item {
        no: 1,
        source_no: 11,
        text: "和別人相處時感覺很自然",
        dimension: 0,
        reverse: false,
    },
    Item {
        no: 2,
        source_no: 21,
        text: "主動與別人交談",
        dimension: 0,
        reverse: false,
    },
    Item {
        no: 3,
        source_no: 31,
        text: "在聚會上和很多不同的人聊天",
        dimension: 0,
        reverse: false,
    },
    Item {
        no: 4,
        source_no: 37,
        text: "抽空幫助別人",
        dimension: 1,
        reverse: false,
    },
    Item {
        no: 5,
        source_no: 42,
        text: "能感受別人的情緒",
        dimension: 1,
        reverse: false,
    },
    Item {
        no: 6,
        source_no: 47,
        text: "讓別人在和我相處時感覺很放鬆",
        dimension: 1,
        reverse: false,
    },
    Item {
        no: 7,
        source_no: 33,
        text: "喜歡有條理",
        dimension: 2,
        reverse: false,
    },
    Item {
        no: 8,
        source_no: 43,
        text: "按計畫做事",
        dimension: 2,
        reverse: false,
    },
    Item {
        no: 9,
        source_no: 48,
        text: "對工作要求準確無誤",
        dimension: 2,
        reverse: false,
    },
    Item {
        no: 10,
        source_no: 29,
        text: "很容易不高興",
        dimension: 3,
        reverse: true,
    },
    Item {
        no: 11,
        source_no: 34,
        text: "情緒變化很大",
        dimension: 3,
        reverse: true,
    },
    Item {
        no: 12,
        source_no: 49,
        text: "經常感到憂鬱",
        dimension: 3,
        reverse: true,
    },
    Item {
        no: 13,
        source_no: 5,
        text: "詞彙豐富",
        dimension: 4,
        reverse: false,
    },
    Item {
        no: 14,
        source_no: 15,
        text: "有生動的想像力",
        dimension: 4,
        reverse: false,
    },
    Item {
        no: 15,
        source_no: 25,
        text: "總有好點子",
        dimension: 4,
        reverse: false,
    },
];

// ── 常模（Grok 審 #10：權威在 schema，ft-big5 只 re-export；web 合法讀取）──

pub struct DimensionNorm {
    pub mean: f64,
    pub sd: f64,
}

pub const SOURCE: &str =
    "李仁豪、陳怡君 (2016)《教育研究與發展期刊》12(4), 87–119；合併樣本 N=738，臺灣中老年立意取樣";

/// 維度順序同 DIMENSION_NAMES：E, A, C, ES, I/Im。
/// 換算：mean100 = (M − 3) × 25/3，sd100 = SD × 25/3（原始分 3–15 → 0–100）。
pub const NORMS: [DimensionNorm; 5] = [
    DimensionNorm {
        mean: 56.166667,
        sd: 16.25,
    }, // E  (9.74, 1.95)
    DimensionNorm {
        mean: 61.416667,
        sd: 14.166667,
    }, // A  (10.37, 1.70)
    DimensionNorm {
        mean: 62.5,
        sd: 16.583333,
    }, // C  (10.50, 1.99)
    DimensionNorm {
        mean: 58.5,
        sd: 19.25,
    }, // ES (10.02, 2.31)
    DimensionNorm {
        mean: 50.666667,
        sd: 17.25,
    }, // I  (9.08, 2.07)
];

#[cfg(test)]
mod norm_tests {
    use super::*;

    /// 換算鎖定：以論文原始 M/SD（表 2，N=738）反推 0–100 值。
    #[test]
    fn norms_match_paper_values() {
        fn to100(raw: f64) -> f64 {
            (raw - 3.0) * 25.0 / 3.0
        }
        let expected_m = [9.74, 10.37, 10.50, 10.02, 9.08];
        let expected_sd = [1.95, 1.70, 1.99, 2.31, 2.07];
        for (i, n) in NORMS.iter().enumerate() {
            assert!((n.mean - to100(expected_m[i])).abs() < 1e-6, "dim {i} mean");
            assert!(
                (n.sd - expected_sd[i] * 25.0 / 3.0).abs() < 1e-6,
                "dim {i} sd"
            );
        }
    }
}
