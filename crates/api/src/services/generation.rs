//! Generation stories — cohort narratives tied to birth year / decade tag.
//! Ported from `crates/web/src/generation.rs` to keep 1940s-2010s 文案一致.
//! Used in `routes/charts.rs` to embed generation context into the merged
//! chart JSON and in `services/ai/prompts.rs` to render 【世代語境】.

/// Decade story for a concrete birth year. Matches `ft-web::generation::generation_story`.
pub fn generation_story(birth_year: i64) -> Option<(&'static str, &'static str)> {
    let decade = (birth_year / 10) * 10;
    match decade {
    y if (1930..1940).contains(&y) => Some((
      "1930s · 動盪與奠基世代",
      "生於戰前動盪與全球大蕭條的餘波中，家族以節儉與韌性維繫日常，動盪讓你早熟於責任與取捨，學會在不安中分辨輕重。那時廣播與報紙是少數窗口，電氣化初普及、手工業仍占主流，穩定被視為最稀缺的資源，人們以勤儉換取明日。那份在匱乏中守住所愛、於無序中重建秩序的定力，是你的世代印記——適合在逆境中扛起修復與奠基的角色，把破碎縫回完整。",
    )),
    y if (1940..1950).contains(&y) => Some((
      "1940s · 戰後重建世代",
      "童年在戰火與重建的交界度過，物資匱乏卻人情緊密，互助與承諾是生存的底色，一諾千金的重量遠勝帳面數字。戰後美援與基礎建設重塑街景，農業轉向輕工業，識字與技職成為翻身階梯，勤奮被視為最穩的投資。那份扛起大局、把零散資源縫成家的能力，正是你的命格隱喻——適合穩中求進、以長期累積換取厚實回報，在漫長賽局中勝出。",
    )),
    y if (1950..1960).contains(&y) => Some((
      "1950s · 開創世代",
      "成長於戰後嬰兒潮與工業化的加速期，村落湧向城市，工廠與校園同時擴張，機會對敢嘗試的人敞開，膽識本身就是通行證。收音機轉向電視，公路與電力網連起產銷，教育普及把技術帶進家庭，日漸豐裕讓人敢於規劃未來。你身上常見開創與領路的氣質——務實與好奇並重，樂於點火、敢於承擔，適合主導專案、帶隊伍從零到一，把藍圖變成街景。",
    )),
    y if (1960..1970).contains(&y) => Some((
      "1960s · 電腦與當代科技興起",
      "與電視、半導體、登月同時長大，世界從類比走向數位，知識以前所未有的速度流動，學習本身成為新的生產力。電腦從實驗室走進辦公室，跨國企業與大學研究室成為新舞台，跨領域成為常態，整合者比專才更搶手。你擅長把零散資訊譯成人人可用的方法與產品，讓智慧真正變成影響力，適合跨界整合與規模化，在流動中建立標準並帶動他人。",
    )),
    y if (1970..1980).contains(&y) => Some((
      "1970s · 網路原生前夜",
      "在卡帶、電視遊樂器到個人電腦的轉折中長大，類比與數位在你家客廳交會，變化本身就是日常，不變反而令人不安。石油危機與經濟轉型迫使產業重組，彈性與自學成為必備技能，兼職與創業不再是例外。那份對轉折特別敏銳、能在新舊之間搭橋、在混沌中抓機會的直覺，適合擔任變局中的翻譯者與整合者，把斷層接成通路並讓前後世代都能通行。",
    )),
    y if (1980..1990).contains(&y) => Some((
      "1980s · 網路與全球化世代",
      "伴隨網際網路、行動通訊與全球化長大，國界在螢幕前淡化，資訊與人脈以前所未有的密度流動，距離不再是成本。新創與品牌在錄影帶與入口網站間冒出，協作變成核心能力，社群的共鳴決定成敗。那份對深層動機的直覺、對集體氛圍的共感與重塑的勇氣，讓你擅長整合資源與人心，適合品牌、社群與跨文化合作，在連結中創造信任並引領轉化。",
    )),
    y if (1990..2000).contains(&y) => Some((
      "1990s · 數位原住民",
      "在智慧型手機與社群媒體中形塑自我，按讚與分享重寫了表達與認同的方式，真實成為最稀缺的濾鏡，效率與真誠同等重要。網路泡沫後的重建讓小團隊也能撬動大市場，內容本身就是產品，創意可以直接兌現。你兼具務實與理想，既會算成本也敢做夢，擅長以小搏大、用創意把限制翻成特色，適合內容、產品與新媒體，在喧囂中說出清晰的故事。",
    )),
    y if (2000..2010).contains(&y) => Some((
      "2000s · AI 與永續世代",
      "與 AI、社群平台、永續議題同行，氣候與演算法同時成為日常語彙，價值與影響力被放在天平中央，選擇本身就是表態。遠距協作與開源讓地理不再是門檻，行動裝置把世界放進口袋，個人也能發動協作網絡。那份把願景落為可執行方案的動能，理想不懸空、實作有溫度，適合將理念轉化為產品與社群，讓善意具備可擴展性並持續生長。",
    )),
    y if (2010..2020).contains(&y) => Some((
      "2010s · 韌性世代",
      "在快速變動與挑戰中成長，疫情、極端氣候與資訊爆炸輪番考驗專注與信任，真實與韌性成為新貨幣，安定需要主動營造。串流、短影音與零工經濟重塑工作與學習，適應力本身就是競爭力，學習曲線決定生存半徑。你對修復與重建有天然的耐心，能在新領域建立秩序、讓人心安定，適合在變局中守護與創新並行，為他人搭起可依靠的結構。",
    )),
    y if (2020..2030).contains(&y) => Some((
      "2020s · 智慧共創世代",
      "出生於疫情後與生成式 AI 爆發的交界，遠距成為常態、虛擬與現實的邊界日益模糊，永續與身心健康被擺上檯面，效率不再是唯一指標。自動化與創作者經濟並進，個人也能組裝自己的生產線，工具普及讓想像更快落地。你對群體與未來的連動特別敏感，渴望以科技放大善意、在分散中重建連結，適合以共創與系統思維開闢新局，讓分散的個體長成有機的網絡。",
    )),
    _ => None,
  }
}

/// Story for a decade tag like "1980s".
pub fn generation_story_for_tag(tag: &str) -> Option<(&'static str, &'static str)> {
    let year: i64 = tag.trim_end_matches('s').parse().ok()?;
    generation_story(year)
}

/// Combined narrative for multiple tags (preserves tag order).
/// 主世代權重：首個 tag 為敘事主幹（全文），後續 tag 為副世代對比補充（各取首句以「而／同時」連接）。
/// 標題仍以 " × " 串接；描述以主世代全文為首段，副世代各取第一句（以 。 分句）並加「而」或「同時」連接詞作對比，避免單純 "；" 串接。
/// Matches `ft-web::generation::combined_generation_story`.
pub fn combined_generation_story(tags: &[String]) -> Option<(String, String)> {
    if tags.is_empty() {
        return None;
    }
    if tags.len() == 1 {
        let (t, d) = generation_story_for_tag(&tags[0])?;
        return Some((t.to_string(), d.to_string()));
    }
    let mut titles = Vec::new();
    let mut descs = Vec::new();
    for tag in tags {
        if let Some((t, d)) = generation_story_for_tag(tag) {
            titles.push(t);
            descs.push(d);
        }
    }
    if titles.is_empty() {
        return None;
    }
    let title = titles.join(" × ");
    // 主世代為首個有效 tag 的全文，副世代各取第一句作對比補充
    let primary = descs[0];
    let mut combined = String::from(primary);
    for (idx, sec) in descs.iter().skip(1).enumerate() {
        let first = sec.split('。').next().unwrap_or(sec);
        let trimmed = first.trim();
        if trimmed.is_empty() {
            continue;
        }
        let first_sentence = if sec.contains('。') {
            format!("{trimmed}。")
        } else {
            trimmed.to_string()
        };
        let connector = if idx % 2 == 0 { "而" } else { "同時" };
        combined.push_str(connector);
        combined.push_str(&first_sentence);
    }
    Some((title, format!("跨世代合寫：{combined}")))
}

/// Per-tag stories, filtered to known tags and preserving input order.
/// 主世代權重：回傳順序即權重順序，首個元素為主世代（敘事主幹），後續為副世代（對比／補充）；
/// 僅作語意註解，不改 API 行為與排序（保持輸入 tags 順序）。
/// Useful for building `generation_stories` array aligned with `generation_tags`.
pub fn stories_for_tags(tags: &[String]) -> Vec<(&'static str, &'static str)> {
    tags.iter()
        .filter_map(|t| generation_story_for_tag(t))
        .collect()
}
