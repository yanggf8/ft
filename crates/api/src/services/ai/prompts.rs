//! Prompt builders — mirrors backend/src/services/ai/prompts.ts verbatim.

/// `getSystemPrompt(chartType, language)`.
pub fn get_system_prompt(chart_type: &str, language: Option<&str>) -> String {
    if chart_type == "ziwei" {
        return if language == Some("en") {
            "You are an expert in 紫微斗數 (Zi Wei Dou Shu). Provide insightful interpretations in English.".to_string()
        } else {
            "你是紫微斗數專家。請用繁體中文提供深入且實用的命盤解讀，語氣溫和專業。".to_string()
        };
    }
    if chart_type == "story" {
        return if language == Some("en") {
            "You are a gifted storyteller who weaves together Zi Wei Dou Shu and Western astrology into one cohesive life story. Write in warm, narrative prose, structured as exactly four chapters titled Essence / Path / Relationships / Treasure.".to_string()
        } else {
            "你是一位融合紫微斗數與西洋占星的生命說書人。請用溫暖、敘事性強的繁體中文，將兩套體系交織成一個完整的人生故事。故事分為四個章節，依序為：本質、道路、關係、寶藏。不要條列術語，而是用故事的語言把兩套系統的洞見融為一體。".to_string()
        };
    }
    if language == Some("zh") {
        "你是西洋占星專家。請用繁體中文提供深入的星盤解讀。".to_string()
    } else {
        "You are an expert Western astrologer. Provide insightful natal chart interpretations."
            .to_string()
    }
}

/// `buildPrompt(req)` — builds the user message from chart data.
pub fn build_prompt(
    chart_type: &str,
    chart: &serde_json::Value,
    language: Option<&str>,
    focus: Option<&str>,
) -> String {
    if chart_type == "ziwei" {
        let five_element = chart
            .pointer("/fiveElement")
            .and_then(|v| v.as_str())
            .unwrap_or("未知");
        let gender = chart.pointer("/birthInfo/gender").and_then(|v| v.as_str());
        let gender_cn = match gender {
            Some("male") => "男",
            Some("female") => "女",
            _ => "未知",
        };
        let palaces: Vec<String> = chart
            .get("palaces")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|p| {
                        let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let stars = p
                            .get("stars")
                            .and_then(|v| v.as_array())
                            .map(|s| {
                                s.iter()
                                    .filter_map(|st| {
                                        st.get("name")
                                            .and_then(|n| n.as_str())
                                            .map(|n| n.to_string())
                                    })
                                    .collect::<Vec<_>>()
                                    .join("、")
                            })
                            .unwrap_or_else(|| "無主星".to_string());
                        format!("{}：{}", name, stars)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut p = format!(
            "請解讀以下紫微斗數命盤：\n\n五行局：{}\n性別：{}\n\n十二宮星曜分布：\n{}\n",
            five_element,
            gender_cn,
            palaces.join("\n")
        );
        if let Some(f) = focus {
            p.push_str(&format!("\n\n請特別分析：{}", f));
        }
        return p;
    }

    if chart_type == "story" {
        let z = chart
            .get("ziwei")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let w = chart
            .get("western")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let z_gender = z.pointer("/birthInfo/gender").and_then(|v| v.as_str());
        let z_gender_cn = match z_gender {
            Some("male") => "男",
            Some("female") => "女",
            _ => "未知",
        };
        let z_five = z
            .pointer("/fiveElement")
            .and_then(|v| v.as_str())
            .unwrap_or("未知");
        let z_palaces: Vec<String> = z
            .get("palaces")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|p| {
                        let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let stars = p
                            .get("stars")
                            .and_then(|v| v.as_array())
                            .map(|s| {
                                s.iter()
                                    .filter_map(|st| {
                                        st.get("name")
                                            .and_then(|n| n.as_str())
                                            .map(|n| n.to_string())
                                    })
                                    .collect::<Vec<_>>()
                                    .join("、")
                            })
                            .unwrap_or_else(|| "無主星".to_string());
                        format!("{}：{}", name, stars)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let w_sun = w
            .pointer("/sunSign/name")
            .and_then(|v| v.as_str())
            .unwrap_or("未知");
        let w_moon = w
            .pointer("/moonSign/name")
            .and_then(|v| v.as_str())
            .unwrap_or("未知");
        let w_planets: Vec<String> = w
            .get("planets")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|p| {
                        let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let sign = p.get("sign").and_then(|v| v.as_str()).unwrap_or("");
                        format!("{} in {}", name, sign)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut p = String::from(
            "請根據以下同一個人的兩張命盤，創作一篇融合紫微斗數與西洋占星的生命故事。\n\n",
        );
        p.push_str(&format!(
            "【紫微斗數】\n五行局：{}\n性別：{}\n十二宮星曜：\n{}\n\n",
            z_five,
            z_gender_cn,
            z_palaces.join("\n")
        ));
        // P0: 【世代語境】 — if the merged chart carries generation_tags, render them
        // as contextual narrative for the LLM, keeping existing palace/planet templates untouched.
        {
            let tags: Vec<String> = chart
                .get("generation_tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if !tags.is_empty() {
                let stories = chart
                    .get("generation_stories")
                    .and_then(|v| v.as_array())
                    .cloned();
                let mut section = String::from("【世代語境】\n");
                // Prefer the embedded generation_stories (tag/title/story) when present,
                // otherwise fall back to the canonical generation_story_for_tag lookup so
                // the prompt stays correct even for older cached charts.
                if let Some(arr) = stories {
                    for entry in arr {
                        let tag = entry.get("tag").and_then(|v| v.as_str()).unwrap_or("");
                        let title = entry.get("title").and_then(|v| v.as_str()).unwrap_or(tag);
                        let story = entry.get("story").and_then(|v| v.as_str()).unwrap_or("");
                        if !story.is_empty() {
                            section.push_str(&format!("{}：{}\n", title, story));
                        }
                    }
                } else {
                    for tag in &tags {
                        if let Some((title, story)) =
                            crate::services::generation::generation_story_for_tag(tag)
                        {
                            section.push_str(&format!("{}：{}\n", title, story));
                        }
                    }
                }
                // Only emit the section if we actually resolved at least one story line.
                if section.lines().count() > 1 {
                    section.push_str(
                        "請將上述世代背景自然融入四個章節的敘事，讓故事既回應命盤也回應時代。\n\n",
                    );
                    p.push_str(&section);
                } else if tags
                    .iter()
                    .any(|t| crate::services::generation::generation_story_for_tag(t).is_some())
                {
                    // Fallback: rebuild via generation module if embedded stories were empty
                    let mut fb = String::from("【世代語境】\n");
                    for tag in &tags {
                        if let Some((title, story)) =
                            crate::services::generation::generation_story_for_tag(tag)
                        {
                            fb.push_str(&format!("{}：{}\n", title, story));
                        }
                    }
                    fb.push_str(
                        "請將上述世代背景自然融入四個章節的敘事，讓故事既回應命盤也回應時代。\n\n",
                    );
                    p.push_str(&fb);
                }
            }
        }
        p.push_str(&format!(
            "【西洋占星】\n太陽：{}\n月亮：{}\n行星：{}\n\n",
            w_sun,
            w_moon,
            w_planets.join("、")
        ));
        p.push_str("請輸出恰好四個章節，每個章節以 markdown 標題行開頭，順序固定如下：\n");
        p.push_str("## 第一章：本質\n## 第二章：道路\n## 第三章：關係\n## 第四章：寶藏\n\n");
        p.push_str("每個章節 2-3 段，請把紫微斗數與西洋占星的洞見交織融會在敘事之中，而不是分開條列兩套術語。");
        if let Some(f) = focus {
            p.push_str(&format!("\n\n請特別著墨：{}", f));
        }
        return p;
    }

    // western
    let w_sun = chart
        .pointer("/sunSign/name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let w_moon = chart
        .pointer("/moonSign/name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let w_planets: Vec<String> = chart
        .get("planets")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|p| {
                    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let sign = p.get("sign").and_then(|v| v.as_str()).unwrap_or("");
                    format!("{} in {}", name, sign)
                })
                .collect()
        })
        .unwrap_or_default();
    let _ = language;
    let mut p = format!(
        "Interpret this natal chart:\nSun: {}\nMoon: {}\nPlanets: {}\n",
        w_sun,
        w_moon,
        w_planets.join(", ")
    );
    if let Some(f) = focus {
        p.push_str(&format!("\n\nFocus on: {}", f));
    }
    p
}

#[allow(dead_code)]
fn _lang_hint(l: Option<&str>) -> Option<&str> {
    l
}
