//! Generation stories — cohort narratives tied to birth year / decade tag.
//! Ported from `crates/web/src/generation.rs` to keep 1940s-2010s 文案一致.
//! Used in `routes/charts.rs` to embed generation context into the merged
//! chart JSON and in `services/ai/prompts.rs` to render 【世代語境】.

/// Decade story for a concrete birth year. Matches `ft-web::generation::generation_story`.
pub fn generation_story(birth_year: i64) -> Option<(&'static str, &'static str)> {
    let decade = (birth_year / 10) * 10;
    match decade {
        y if (1940..1950).contains(&y) => Some((
            "1940s · 戰後重建世代",
            "在資源稀缺中學會堅韌與互助，重視家庭與承諾。你的命格常帶「扛大局」的格局，適合穩中求進、長期累積。",
        )),
        y if (1950..1960).contains(&y) => Some((
            "1950s · 開創世代",
            "見證工業化與教育普及，樂於嘗試、敢於創業。命盤中常見開創與領導格局，適合主導專案、帶團隊。",
        )),
        y if (1960..1970).contains(&y) => Some((
            "1960s · 電腦與當代科技興起",
            "與電腦、半導體、網際網路同代成長，高成就人物輩出。你的世代擅長將技術轉為產品，命格中常見「化知識為影響力」的格局，適合跨界整合與規模化。",
        )),
        y if (1970..1980).contains(&y) => Some((
            "1970s · 網路原生前夜",
            "在卡帶、錄音帶到個人電腦的轉折中長大，適應力強、學習曲線陡峭。命格常見「轉折中抓機會」的特質，適合在變局中擔任橋接角色。",
        )),
        y if (1980..1990).contains(&y) => Some((
            "1980s · 網路與全球化世代",
            "伴隨網際網路、行動通訊與全球化長大，視野廣、協作快。命盤常見人際與資源整合格局，適合品牌、社群、跨文化合作。",
        )),
        y if (1990..2000).contains(&y) => Some((
            "1990s · 數位原住民",
            "在智慧型手機與社群媒體中形塑自我，重視真實與效率。命格中常見「以小搏大」的創意格局，適合內容、產品與新媒體。",
        )),
        y if (2000..2010).contains(&y) => Some((
            "2000s · AI 與永續世代",
            "與 AI、永續、遠距協作同行，關注價值與影響力。命格常見「理想驅動實作」的格局，適合將願景落為可執行的產品與社群。",
        )),
        y if (2010..2020).contains(&y) => Some((
            "2010s · 韌性世代",
            "在快速變動與挑戰中成長，韌性高、共感強。命格常見「修復與重建」的格局，適合在新領域中建立秩序與信任。",
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
    let desc = descs.join("；");
    Some((title, format!("跨世代合寫：{desc}")))
}

/// Per-tag stories, filtered to known tags and preserving input order.
/// Useful for building `generation_stories` array aligned with `generation_tags`.
pub fn stories_for_tags(tags: &[String]) -> Vec<(&'static str, &'static str)> {
    tags.iter()
        .filter_map(|t| generation_story_for_tag(t))
        .collect()
}
