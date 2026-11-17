//! 證據帳本（spec §2.4）：每篇文獻的唯一來源。
//! 敘述與原文核對筆記寫在 `docs/superpowers/reports/fengshui-research/evidence-ledger.md`。
//!
//! 規則：
//! - `full_text_read` 只有讀過原文全文才可設 `true`；目前一律 `false`（依研究報告的摘要／
//!   二手轉述填入，原文核對另案進行）。
//! - 卡片徽章升為 `direct` 時，所引文獻必須全部 `full_text_read == true`（T6 守門）。
//! - `n`／`url` 未經核對者寫 `None`／照研究報告；改文獻語意需 bump 語料版本。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evidence {
    pub key: &'static str,
    /// 作者、年份、期刊。
    pub citation: &'static str,
    pub url: &'static str,
    /// 研究設計（統合分析、實驗、相關研究……）。
    pub design: &'static str,
    /// 樣本數；未核對或不適用為 `None`。
    pub n: Option<u32>,
    /// 測的是什麼結果。
    pub outcome: &'static str,
    pub limitation: &'static str,
    pub full_text_read: bool,
}

pub static EVIDENCE: &[Evidence] = &[
  Evidence {
    key: "zijlstra2024",
    citation: "Zijlstra et al. 2024, PLOS ONE 19(6): e0303032",
    url: "https://journals.plos.org/plosone/article?id=10.1371%2Fjournal.pone.0303032",
    design: "線上隨機實驗：比較依風水佈置與一般佈置的虛擬病房",
    n: Some(558),
    outcome: "對房間的愉悅感評分、焦慮",
    limitation: "線上進行、看的是虛擬房間；風水佈置讓房間顯得較愉悅，對焦慮沒有直接效果",
    full_text_read: false,
  },
  Evidence {
    key: "kryzanowski2019",
    citation: "Kryžanowski 2019, Urbani izziv 30(2): 124–134",
    url: "https://www.jstor.org/stable/26843928",
    design: "文獻比對：118 條形勢派建議對照環境心理學與 Pattern Language",
    n: None,
    outcome: "建議與既有理論是否一致（45% 與環境心理學全部或部分一致）",
    limitation: "比對的是理論一致性，不是實驗結果；118 條明細待讀全文",
    full_text_read: false,
  },
  Evidence {
    key: "dosen_ostwald2016",
    citation: "Dosen & Ostwald 2016, City, Territory and Architecture 3: 4",
    url: "https://link.springer.com/article/10.1186/s40410-016-0033-1",
    design: "瞭望－庇護理論實證研究的系統性回顧與統合分析",
    n: None,
    outcome: "對景觀與空間位置的偏好評分",
    limitation: "各研究結果不一致；「庇護」一半偏中性；沒有床位或睡眠研究",
    full_text_read: false,
  },
  Evidence {
    key: "fich2014",
    citation: "Fich et al. 2014, Physiology & Behavior",
    url: "https://vbn.aau.dk/en/publications/can-architectural-design-alter-the-physiological-reaction-to-psyc/",
    design: "虛擬實境實驗：在開放或封閉的虛擬房間中接受社會壓力測驗",
    n: None,
    outcome: "壓力下的皮質醇反應",
    limitation: "單一虛擬實境研究；操弄的是房間開口，不是橫樑或頭頂重物",
    full_text_read: false,
  },
  Evidence {
    key: "bar_neta2006",
    citation: "Bar & Neta 2006, Psychological Science 17(8): 645–648",
    url: "https://pubmed.ncbi.nlm.nih.gov/16913943/",
    design: "實驗：比較曲線與尖角版本的物品圖片",
    n: None,
    outcome: "對圖片的喜好評分",
    limitation: "用的是圖片，不是真實空間；沒有測睡眠或專注",
    full_text_read: false,
  },
  Evidence {
    key: "chuquichambi2022",
    citation: "Chuquichambi et al. 2022, 曲線偏好統合分析",
    url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC10091794",
    design: "統合分析",
    n: None,
    outcome: "對曲線與尖角刺激的偏好（平均效果約 g≈0.39）",
    limitation: "刺激多為抽象圖形；換成真實家具的研究結果不一致",
    full_text_read: false,
  },
  Evidence {
    key: "furniture2025",
    citation: "i-Perception 2025（真實桌椅的曲線偏好研究，作者待核對）",
    url: "https://journals.sagepub.com/doi/10.1177/20416695251341682",
    design: "實驗：以真實桌椅檢驗曲線偏好",
    n: None,
    outcome: "對家具的偏好評分",
    limitation: "沒有重現抽象圖形研究裡的曲線偏好",
    full_text_read: false,
  },
  Evidence {
    key: "vartanian2013",
    citation: "Vartanian et al. 2013, PNAS",
    url: "https://pubmed.ncbi.nlm.nih.gov/23754408/",
    design: "腦造影實驗：觀看曲線與直線的室內空間照片",
    n: None,
    outcome: "對室內空間的美感判斷與腦部活動",
    limitation: "看的是照片；測的是美感判斷，不是睡眠或專注",
    full_text_read: false,
  },
  Evidence {
    key: "saxbe_repetti2010",
    citation: "Saxbe & Repetti 2010, Personality and Social Psychology Bulletin",
    url: "https://pubmed.ncbi.nlm.nih.gov/19934011/",
    design: "相關研究：分析雙薪夫妻導覽自家時的描述用語",
    n: None,
    outcome: "一日間的皮質醇變化型態與情緒",
    limitation: "只有相關、方向不明；測的是對整個家的描述，不是床底或桌面",
    full_text_read: false,
  },
  Evidence {
    key: "roster2016",
    citation: "Roster, Ferrari & Jurkat 2016, Journal of Environmental Psychology",
    url: "https://doi.org/10.1016/j.jenvp.2016.03.003",
    design: "問卷調查",
    n: None,
    outcome: "自評雜物程度與主觀幸福感",
    limitation: "自陳問卷、相關研究，不能推論因果",
    full_text_read: false,
  },
  Evidence {
    key: "stevenson2018",
    citation: "Stevenson et al. 2018, 注意力恢復理論系統性回顧與統合分析",
    url: "https://pubmed.ncbi.nlm.nih.gov/30130463/",
    design: "系統性回顧與統合分析",
    n: None,
    outcome: "接觸自然環境後的注意力測驗表現",
    limitation: "效果小、研究之間差異大；沒有測座位面牆",
    full_text_read: false,
  },
  Evidence {
    key: "ulrich1984",
    citation: "Ulrich 1984, Science 224: 420–421",
    url: "https://doi.org/10.1126/science.6143402",
    design: "回溯性病歷比較：窗外看得到樹或磚牆的術後病人",
    n: None,
    outcome: "術後住院天數與止痛藥用量",
    limitation: "單一醫院、回溯資料；對象是術後病人，不是一般居家",
    full_text_read: false,
  },
  Evidence {
    key: "boubekri2014",
    citation: "Boubekri et al. 2014, Journal of Clinical Sleep Medicine",
    url: "https://doi.org/10.5664/jcsm.3780",
    design: "比較研究：有窗與無窗辦公室的員工",
    n: None,
    outcome: "睡眠品質與生活品質自評",
    limitation: "樣本小、非隨機分組；測的是白天光照與睡眠，沒有測專心",
    full_text_read: false,
  },
];

/// 依 key 查文獻。
pub fn evidence_by_key(key: &str) -> Option<&'static Evidence> {
    EVIDENCE.iter().find(|e| e.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn keys_unique_and_fields_present() {
        let mut seen = HashSet::new();
        for e in EVIDENCE {
            assert!(seen.insert(e.key), "duplicate evidence key {}", e.key);
            assert!(!e.citation.is_empty(), "{}", e.key);
            assert!(e.url.starts_with("https://"), "{}", e.key);
            assert!(!e.design.is_empty(), "{}", e.key);
            assert!(!e.outcome.is_empty(), "{}", e.key);
            assert!(!e.limitation.is_empty(), "{}", e.key);
        }
    }
}
