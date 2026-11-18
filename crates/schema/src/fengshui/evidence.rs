//! 證據帳本（spec §2.4）：每篇文獻的唯一來源。
//! 敘述與原文核對筆記寫在 `docs/superpowers/reports/fengshui-research/evidence-ledger.md`
//! （key 與該檔一致；2026-11-18 依其 §1–§3 核對結果更新）。
//!
//! 規則：
//! - `full_text_read` 只在親自讀到全文主體時為 `true`；只讀摘要、工具代讀、二手報導一律 `false`。
//! - 卡片徽章為 `direct` 時，`refs` 所引文獻必須全部 `full_text_read == true`（T6 守門）；
//!   未讀全文者只能放在卡片的 `further`（延伸閱讀）。
//! - `n` 照原文抄；原文未報或未核實寫 `None`。改文獻語意需 bump 語料版本。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evidence {
    pub key: &'static str,
    /// 作者、年份、期刊。
    pub citation: &'static str,
    pub url: &'static str,
    /// 研究設計（統合分析、實驗、相關研究……）。
    pub design: &'static str,
    /// 參與者人數；原文未報、未核實或不適用為 `None`。
    pub n: Option<u32>,
    /// 測的是什麼結果。
    pub outcome: &'static str,
    pub limitation: &'static str,
    pub full_text_read: bool,
}

pub static EVIDENCE: &[Evidence] = &[
    Evidence {
        key: "zijlstra2024",
        citation: "Zijlstra, van der Zwaag, Kullak, Rogers, Walker, van Dellen & Mobach 2024, PLOS ONE 19(6): e0303032",
        url: "https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0303032",
        design: "線上四組隨機對照試驗：看單人病房的影片、圖片與平面圖（不是 VR），風水組同時改動 15 處",
        n: Some(558),
        outcome: "焦慮（主要結果）、房間愉悅度、預期社會支持",
        limitation: "對焦慮沒有整體效果；15 處同時改動，看不出是哪一項起作用；模擬而非真實住院；共同作者之一隸屬國際風水協會",
        full_text_read: true,
    },
    Evidence {
        key: "kryzanowski2019",
        citation: "Kryžanowski 2019, Urbani izziv 30(2): 124–134",
        url: "https://www.jstor.org/stable/26843928",
        design: "文獻比對：118 條形勢派建議對照環境心理學與 Pattern Language",
        n: None,
        outcome: "建議與既有理論是否一致（45% 與環境心理學全部或部分一致）",
        limitation: "比對的是理論一致性，不是實驗結果；只核對過摘要數字",
        full_text_read: false,
    },
    Evidence {
        key: "dosen2016",
        citation: "Dosen & Ostwald 2016, City, Territory and Architecture 3: 4",
        url: "https://link.springer.com/article/10.1186/s40410-016-0033-1",
        design: "文獻彙整（計票式分類，沒有合併效果量）：34 篇量化研究，室內 21 篇",
        n: None,
        outcome: "對照片與場景的偏好、舒適、寬敞等評分",
        limitation: "結果不一致；「背後有遮蔽」在室內研究多為中性；不評估研究品質；沒有睡眠、壓力或工作表現",
        full_text_read: true,
    },
    Evidence {
        key: "sporrle2010",
        citation: "Spörrle & Stich 2010, Evolutionary Psychology 8(3): 405–419",
        url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC10426856/",
        design: "受試者間實驗：在紙本平面圖上擺放比例家具；床只能沿長軸擺，不能斜擺",
        n: Some(138),
        outcome: "想像中的擺床位置：床頭方向、與對門牆的距離、擺在哪一側",
        limitation: "只在平面圖上想像佈置、沒有真的睡；沒有測睡眠或舒適感；平面圖限制了擺法",
        full_text_read: true,
    },
    Evidence {
        key: "bonin2023",
        citation: "Bonin, Thiebaut, Didierjean, Fančovičová, Kubjatková, Prokop & Méot 2023, Evolutionary Psychological Science 9(4): 463–476",
        url: "https://hal.science/hal-04321992v1/document",
        design: "Spörrle & Stich 2010 的重現與延伸：法國 2D 平面圖、斯洛伐克 3D 房間圖兩研究",
        n: None,
        outcome: "想像中的擺床位置",
        limitation: "尚未讀到全文，只見摘要片段；數字未核實",
        full_text_read: false,
    },
    Evidence {
        key: "psathiti2017",
        citation: "Psathiti & Sailer 2017, Proceedings of the 11th International Space Syntax Symposium, paper 137",
        url: "https://discovery.ucl.ac.uk/1568213/",
        design: "觀察研究：3 個休息室共 158 組座位、9 天快照，以空間分析與迴歸比較座位使用率",
        n: Some(526),
        outcome: "座位被坐的比例",
        limitation: "只有三個場所；沒有「背靠牆」變項；背向走道的座位最常被坐；沒有測壓力、專注或舒適感",
        full_text_read: true,
    },
    Evidence {
        key: "sailer2021",
        citation: "Sailer, Koutsolampros & Pachilova 2021, PLOS ONE 16(4): e0250058",
        url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC8081188/",
        design: "單一開放式辦公室的橫斷面研究：員工問卷＋座位可見範圍分析",
        n: Some(167),
        outcome: "自評工作空間對專注、有效率工作與團隊合作的支持程度",
        limitation: "單一公司、橫斷面、自評；測的是「面向整個房間」的可見範圍，不是門的可見性或背後靠牆",
        full_text_read: true,
    },
    Evidence {
        key: "barneta2006",
        citation: "Bar & Neta 2006, Psychological Science 17(8): 645–648",
        url: "https://pubmed.ncbi.nlm.nih.gov/16913943/",
        design: "受試者內實驗：同一物件的尖角版與曲線版圖片，每張呈現 84 毫秒",
        n: Some(14),
        outcome: "短暫一瞥後選喜歡或不喜歡",
        limitation: "樣本 14 人；去脈絡的圖片；沒有直接測威脅感，也沒有測睡眠或專注",
        full_text_read: true,
    },
    Evidence {
        key: "chuquichambi2022",
        citation: "Chuquichambi et al. 2022, Annals of the New York Academy of Sciences 1518(1): 151–165",
        url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC10091794",
        design: "系統性回顧與統合分析：61 篇研究、106 個樣本",
        n: Some(11023),
        outcome: "對曲線與尖角刺激的偏好評分（整體 g=0.39）",
        limitation: "效果主要在抽象圖形與物件圖片；室內空間與建築類圖片 g=−0.04、不顯著；異質性大",
        full_text_read: true,
    },
    Evidence {
        key: "chuquichambi2025",
        citation: "Chuquichambi, Trawinski, Munar & Palumbo 2025, i-Perception 16(3): 1–20",
        url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC12099157/",
        design: "兩個實驗：80 張曲線／直線成對的桌椅照片，實驗室眼動與線上評分",
        n: Some(90),
        outcome: "喜歡評分與凝視",
        limitation: "兩個實驗都沒有曲線偏好；只有桌椅兩類物件；看的是照片不是實物",
        full_text_read: true,
    },
    Evidence {
        key: "vartanian2013",
        citation: "Vartanian et al. 2013, PNAS 110(Suppl 2): 10446–10453",
        url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC3690611/",
        design: "腦造影實驗：200 張曲線與直線的室內空間照片",
        n: Some(18),
        outcome: "美或不美、要進去或離開的判斷，以及腦部活動",
        limitation: "曲線空間較常被判為美，但不影響要不要進去；看的是整體空間輪廓的照片，不是尖角朝向人",
        full_text_read: true,
    },
    Evidence {
        key: "fich2014",
        citation: "Fich, Jönsson, Kirkegaard, Wallergård, Garde & Hansen 2014, Physiology & Behavior 135: 91–97",
        url: "https://pubmed.ncbi.nlm.nih.gov/24907691/",
        design: "虛擬實境社會壓力測驗：封閉房間與有開口房間的組間比較",
        n: None,
        outcome: "唾液皮質醇、心率變異",
        limitation: "只讀到摘要；單一研究；操弄的是牆面開口，不是天花板、橫樑或頭頂物",
        full_text_read: false,
    },
    Evidence {
        key: "meyerslevy2007",
        citation: "Meyers-Levy & Zhu 2007, Journal of Consumer Research 34(2): 174–186",
        url: "https://doi.org/10.1086/519146",
        design: "三個實驗（各 32、100、34 人）：10 呎與 8 呎天花板，以吊掛燈籠提高天花板的顯著性",
        n: None,
        outcome: "思考方式（關聯或具體處理）、自評自由或受限感；補充研究測情緒與舒適感",
        limitation: "學生樣本、短時停留；補充研究中情緒與舒適評分在各條件間沒有差異；全文來自非官方複本",
        full_text_read: true,
    },
    Evidence {
        key: "saxbe2010",
        citation: "Saxbe & Repetti 2010, Personality and Social Psychology Bulletin 36(1): 71–81",
        url: "https://pubmed.ncbi.nlm.nih.gov/19934011/",
        design: "相關研究：30 對雙薪夫妻錄影導覽自家，分析描述用語（「雜亂」與「房子沒完成」合併計分）",
        n: Some(60),
        outcome: "日間皮質醇變化型態、憂鬱情緒",
        limitation: "只在妻子身上看到；測的是用字，不是實際雜亂程度；因果方向不明；沒有測床底或桌面",
        full_text_read: true,
    },
    Evidence {
        key: "roster2016",
        citation: "Roster, Ferrari & Jurkat 2016, Journal of Environmental Psychology 46: 32–41",
        url: "https://doi.org/10.1016/j.jenvp.2016.03.003",
        design: "問卷調查（僅見二手報導）",
        n: None,
        outcome: "自評雜物程度與主觀幸福感（未核實）",
        limitation: "連摘要都未從原始來源讀到；卡片不引用",
        full_text_read: false,
    },
    Evidence {
        key: "stevenson2018",
        citation: "Stevenson, Schilhab & Bentsen 2018, Journal of Toxicology and Environmental Health, Part B 21(4): 227–268",
        url: "https://pubmed.ncbi.nlm.nih.gov/30130463/",
        design: "系統性回顧與統合分析：42 篇文章",
        n: None,
        outcome: "接觸自然環境後的客觀認知測驗表現",
        limitation: "只讀到摘要；效果小到中等；沒有測座位面牆",
        full_text_read: false,
    },
    Evidence {
        key: "ulrich1984",
        citation: "Ulrich 1984, Science 224(4647): 420–421",
        url: "https://pubmed.ncbi.nlm.nih.gov/6143402/",
        design: "回溯病歷配對研究：窗外是落葉樹或磚牆的膽囊切除病人",
        n: Some(46),
        outcome: "術後住院天數、護理紀錄評語、止痛藥用量",
        limitation: "回溯、非隨機；單一醫院、單一手術；磚牆是窗外景物，不是貼近眼前的牆；不是工作或注意力",
        full_text_read: true,
    },
    Evidence {
        key: "boubekri2014",
        citation: "Boubekri, Cheung, Reid, Wang & Zee 2014, Journal of Clinical Sleep Medicine 10(6): 603–611",
        url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC4031400/",
        design: "病例對照前導研究：有窗與無窗辦公室的員工，其中 21 人戴腕動計",
        n: Some(49),
        outcome: "工作時段光照量、睡眠時間、睡眠品質與生活品質自評",
        limitation: "樣本小、便利抽樣、非隨機；沒有測專心",
        full_text_read: true,
    },
    Evidence {
        key: "mason2022",
        citation: "Mason, Grimaldi, Reid, Warlick, Malkani, Abbott & Zee 2022, PNAS 119(12): e2113290119",
        url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC8944904/",
        design: "實驗室隨機分組：開著 100 lux 室內燈睡一晚，或兩晚都昏暗",
        n: Some(20),
        outcome: "睡眠階段比例、夜間心率",
        limitation: "20 人、單晚、年輕健康者；100 lux 是頭頂室內燈，遠亮於窗外透進來的光",
        full_text_read: true,
    },
    Evidence {
        key: "sweeney2023",
        citation: "Sweeney et al. 2024（2023 線上刊出）, Sleep 47(2): zsad100",
        url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC10851850/",
        design: "大型世代研究基線資料的橫斷面分析",
        n: Some(47765),
        outcome: "自陳睡覺時的臥室光源與多項自陳睡眠指標",
        limitation: "橫斷面、皆為自陳；「房外的光」與多數睡眠指標無關；數字經工具代讀、未親自逐字核對",
        full_text_read: false,
    },
    Evidence {
        key: "xu2023",
        citation: "Xu, Zhang, Tao & Sun 2023, Science of the Total Environment 857: 159303",
        url: "https://pubmed.ncbi.nlm.nih.gov/36228789/",
        design: "觀察研究的統合分析：7 篇橫斷面研究",
        n: Some(577_932),
        outcome: "夜間光照與自陳睡眠問題",
        limitation: "只讀到摘要；全為橫斷面相關；作者自評證據少且不一致",
        full_text_read: false,
    },
    Evidence {
        key: "tamaki2016",
        citation: "Tamaki, Bang, Watanabe & Sasaki 2016, Current Biology 26(9): 1190–1194",
        url: "https://pmc.ncbi.nlm.nih.gov/articles/PMC4864126/",
        design: "實驗室睡眠研究：比較換地方睡的第 1 晚與第 2 晚",
        n: Some(35),
        outcome: "左右腦慢波活動不對稱、對聲音的反應",
        limitation: "與房間佈置無關，沒有提到鏡子或格局；卡片不引用",
        full_text_read: true,
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
