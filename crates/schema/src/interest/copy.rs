//! 興趣探索卡片與結果頁文案（spec §2.1、§3、§3.2）。放在 ft-schema 而非 ft-web，
//! 讓禁用詞掃描（spec §5.5）能在 CI 的 native 測試裡跑（ft-web 無測試執行環境）。

use super::RiasecType;

/// 一型的結果頁文字：名稱、偏好描述（spec §2.1 表，逐字）、反思提問。
pub struct TypeCopy {
    pub name: &'static str,
    pub description: &'static str,
    pub reflection: &'static str,
}

/// 反思提問只問經驗與感受，不附職業或能力判斷（spec §2.1）。
pub fn type_copy(t: RiasecType) -> TypeCopy {
    let (name, description, reflection) = match t {
        RiasecType::R => (
            "實作型",
            "喜歡動手操作、使用工具或在戶外進行具體的工作。",
            "最近一次動手修理、組裝或製作東西是什麼時候？過程中哪個部分讓你投入？",
        ),
        RiasecType::I => (
            "研究型",
            "喜歡觀察、分析問題，弄清楚事情運作的原理。",
            "最近有沒有一個問題讓你很想弄清楚原因？你是怎麼找答案的？",
        ),
        RiasecType::A => (
            "藝術型",
            "喜歡用文字、圖像、聲音或表演表達想法。",
            "最近一次用文字、圖像、聲音或表演表達自己是什麼時候？感覺如何？",
        ),
        RiasecType::S => (
            "社會型",
            "喜歡教導、協助、照顧或與人合作。",
            "最近一次教別人、幫忙或照顧別人是什麼時候？那次你有什麼感受？",
        ),
        RiasecType::E => (
            "企業型",
            "喜歡說服、帶領團隊，推動計畫往前走。",
            "最近一次說服別人或帶著大家推動一件事是什麼時候？感覺如何？",
        ),
        RiasecType::C => (
            "事務型",
            "喜歡整理資料、遵循清楚的流程，把細節處理好。",
            "最近一次把資料、物品或流程整理清楚是什麼時候？完成後感覺如何？",
        ),
    };
    TypeCopy {
        name,
        description,
        reflection,
    }
}

/// 卡片說明（spec §3）。
pub const CARD_INTRO: &str = "看看你比較喜歡哪些類型的活動。這是興趣，不是能力測驗；作答與結果存在你的帳號，只供你自己參考，可以隨時全部刪除。";

/// 結果頁固定註腳（spec §3）。
pub const RESULT_FOOTNOTE: &str = "題目來自公開的研究量表（Armstrong 等人，2008），中文為本站翻譯，沒有台灣常模，所以只顯示你自己六型之間的相對高低。興趣會隨經驗改變，歡迎之後再做一次。";

/// 最高分 < 3.0 時取代「較明顯的方向」的文字（spec §3）。
pub const NO_PREFERENCE: &str =
    "目前沒有特別突出的偏好，這很常見。可以從任一類型挑一件小事試試，再回來看看感覺有沒有不同。";

/// 原始出處（spec §2.2：授權條件為註明出處）。
pub const SOURCE: &str = "出處：Armstrong, P. I., Allison, W., & Rounds, J. (2008). Development and initial validation of brief public domain RIASEC marker scales. Journal of Vocational Behavior, 73(2), 287–299. DOI 10.1016/j.jvb.2008.06.003";

/// 人格測驗結果頁的構念區分導覽句（spec §3.2）。
pub const PERSONALITY_BRIDGE: &str = "人格描述你平常怎麼反應；興趣探索看你比較喜歡做哪些活動。";

#[cfg(test)]
mod tests {
    use super::super::BANNED_WORDS;
    use super::*;

    #[test]
    fn type_names_and_descriptions_follow_spec_table() {
        let expected = [
            ("實作型", "喜歡動手操作、使用工具或在戶外進行具體的工作。"),
            ("研究型", "喜歡觀察、分析問題，弄清楚事情運作的原理。"),
            ("藝術型", "喜歡用文字、圖像、聲音或表演表達想法。"),
            ("社會型", "喜歡教導、協助、照顧或與人合作。"),
            ("企業型", "喜歡說服、帶領團隊，推動計畫往前走。"),
            ("事務型", "喜歡整理資料、遵循清楚的流程，把細節處理好。"),
        ];
        for (t, (name, description)) in RiasecType::ALL.iter().zip(expected) {
            let c = type_copy(*t);
            assert_eq!(c.name, name, "{t:?}");
            assert_eq!(c.description, description, "{t:?}");
        }
    }

    #[test]
    fn every_type_has_a_reflection_question() {
        for t in RiasecType::ALL {
            let r = type_copy(t).reflection;
            assert!(r.ends_with('？'), "{t:?}: {r}");
        }
    }

    #[test]
    fn fixed_texts_follow_spec() {
        assert_eq!(
            CARD_INTRO,
            "看看你比較喜歡哪些類型的活動。這是興趣，不是能力測驗；作答與結果存在你的帳號，只供你自己參考，可以隨時全部刪除。"
        );
        assert_eq!(
            RESULT_FOOTNOTE,
            "題目來自公開的研究量表（Armstrong 等人，2008），中文為本站翻譯，沒有台灣常模，所以只顯示你自己六型之間的相對高低。興趣會隨經驗改變，歡迎之後再做一次。"
        );
        assert_eq!(
            NO_PREFERENCE,
            "目前沒有特別突出的偏好，這很常見。可以從任一類型挑一件小事試試，再回來看看感覺有沒有不同。"
        );
        assert!(SOURCE.contains("10.1016/j.jvb.2008.06.003"));
        assert!(!PERSONALITY_BRIDGE.is_empty());
    }

    #[test]
    fn interest_card_source_avoids_banned_words() {
        // ft-web 無測試執行環境；直接掃元件原始碼，涵蓋未集中在本模組的按鈕、訊息與提示字串
        const CARD_SRC: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../web/src/components/interest_card.rs"
        ));
        for w in BANNED_WORDS {
            assert!(!CARD_SRC.contains(w), "interest_card.rs contains {w}");
        }
    }

    #[test]
    fn all_copy_avoids_banned_words() {
        let mut texts = vec![
            CARD_INTRO,
            RESULT_FOOTNOTE,
            NO_PREFERENCE,
            SOURCE,
            PERSONALITY_BRIDGE,
        ];
        for t in RiasecType::ALL {
            let c = type_copy(t);
            texts.extend([c.name, c.description, c.reflection]);
        }
        for text in texts {
            for w in BANNED_WORDS {
                assert!(!text.contains(w), "{text} contains {w}");
            }
        }
    }
}
