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
            "【紫微斗數】\n五行局：{}\n性別：{}\n十二宮星曜：\n{}\n",
            z_five,
            z_gender_cn,
            z_palaces.join("\n")
        ));

        // ── P1 增厚：四柱 / 大限 / 閏月 / 命宮亮度四化 ──
        // 全部 Option 處理：缺欄自動跳過，不炸 prompt。
        {
            let mut extra = String::new();

            // 四柱 fourPillars { year: {stem,branch}, month, day, hour }
            if let Some(fp) = z.get("fourPillars").and_then(|v| v.as_object()) {
                let mut parts: Vec<String> = Vec::new();
                for key in ["year", "month", "day", "hour"] {
                    if let Some(sb) = fp.get(key).and_then(|v| v.as_object()) {
                        let stem = sb.get("stem").and_then(|v| v.as_str()).unwrap_or("");
                        let branch = sb.get("branch").and_then(|v| v.as_str()).unwrap_or("");
                        if !stem.is_empty() || !branch.is_empty() {
                            let label = match key {
                                "year" => "年柱",
                                "month" => "月柱",
                                "day" => "日柱",
                                "hour" => "時柱",
                                _ => key,
                            };
                            parts.push(format!("{} {}{}", label, stem, branch));
                        }
                    }
                }
                if !parts.is_empty() {
                    extra.push_str(&format!("四柱：{}\n", parts.join("、")));
                }
            }

            // 大限 majorLimits 前3 段
            if let Some(arr) = z.get("majorLimits").and_then(|v| v.as_array()) {
                if !arr.is_empty() {
                    let mut segs: Vec<String> = Vec::new();
                    for m in arr.iter().take(3) {
                        let sa = m.get("startAge").and_then(|v| v.as_u64());
                        let ea = m.get("endAge").and_then(|v| v.as_u64());
                        let stem = m.get("stem").and_then(|v| v.as_str()).unwrap_or("");
                        let branch = m.get("branch").and_then(|v| v.as_str()).unwrap_or("");
                        let pi = m
                            .get("palaceIndex")
                            .and_then(|v| v.as_u64())
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "-".to_string());
                        if let (Some(s), Some(e)) = (sa, ea) {
                            if !stem.is_empty() || !branch.is_empty() {
                                segs.push(format!(
                                    "{}-{}歲 {}{}（宮位{}）",
                                    s, e, stem, branch, pi
                                ));
                            } else {
                                segs.push(format!("{}-{}歲（宮位{}）", s, e, pi));
                            }
                        }
                    }
                    if !segs.is_empty() {
                        extra.push_str(&format!("大限前三段：{}\n", segs.join("、")));
                    }
                }
            }

            // 是否閏月：優先 meta.isLeap，其次 birthInfo.lunar.isLeap / lunar.isLeap
            let is_leap_opt = z
                .pointer("/meta/isLeap")
                .and_then(|v| v.as_bool())
                .or_else(|| z.pointer("/meta/is_leap").and_then(|v| v.as_bool()))
                .or_else(|| {
                    z.pointer("/birthInfo/lunar/isLeap")
                        .and_then(|v| v.as_bool())
                })
                .or_else(|| z.pointer("/lunar/isLeap").and_then(|v| v.as_bool()));
            if let Some(is_leap) = is_leap_opt {
                extra.push_str(&format!(
                    "閏月：{}\n",
                    if is_leap {
                        "是（閏月出生）"
                    } else {
                        "否"
                    }
                ));
            }

            // 命宮星曜亮度/四化（brightness/sihua 若有則列）
            if let Some(life_idx) = z
                .get("lifePalaceIndex")
                .and_then(|v| v.as_u64())
                .or_else(|| z.pointer("/lifePalaceIndex").and_then(|v| v.as_u64()))
            {
                if let Some(palaces) = z.get("palaces").and_then(|v| v.as_array()) {
                    // palaces 可能以 index 排序，也可能亂序，故同時支援 index 匹配與 isLifePalace
                    let mut life_palace: Option<&serde_json::Value> = None;
                    for pal in palaces {
                        if pal.get("index").and_then(|v| v.as_u64()) == Some(life_idx) {
                            life_palace = Some(pal);
                            break;
                        }
                    }
                    if life_palace.is_none() {
                        for pal in palaces {
                            if pal
                                .get("isLifePalace")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                            {
                                life_palace = Some(pal);
                                break;
                            }
                        }
                    }
                    if let Some(lp) = life_palace {
                        let pname = lp.get("name").and_then(|v| v.as_str()).unwrap_or("命宮");
                        if let Some(stars) = lp.get("stars").and_then(|v| v.as_array()) {
                            if !stars.is_empty() {
                                let mut star_strs: Vec<String> = Vec::new();
                                for st in stars {
                                    let n = st.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                    if n.is_empty() {
                                        continue;
                                    }
                                    let bright = st
                                        .get("brightness")
                                        .and_then(|v| v.as_str())
                                        .filter(|s| !s.is_empty());
                                    let sihua = st
                                        .get("sihua")
                                        .and_then(|v| v.as_str())
                                        .filter(|s| !s.is_empty());
                                    let mut label = n.to_string();
                                    let mut suffixes: Vec<String> = Vec::new();
                                    if let Some(b) = bright {
                                        suffixes.push(b.to_string());
                                    }
                                    if let Some(s) = sihua {
                                        // 人類可讀：lu/quan/ke/ji 對應祿權科忌，但保持原值避免術語外露，僅作標記
                                        suffixes.push(s.to_string());
                                    }
                                    if !suffixes.is_empty() {
                                        label.push_str(&format!("（{}）", suffixes.join("/")));
                                    }
                                    star_strs.push(label);
                                }
                                if !star_strs.is_empty() {
                                    extra.push_str(&format!(
                                        "命宮「{}」星曜（含亮度/四化）：{}\n",
                                        pname,
                                        star_strs.join("、")
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            if !extra.is_empty() {
                // 去掉末尾換行後，作為紫微細節段落插入
                p.push_str(&extra);
                if !p.ends_with('\n') {
                    p.push('\n');
                }
                p.push('\n');
            }
        }

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
        // ── 西洋占星：基礎 + ascendant / houses / 行星 house 統計 ──
        {
            let mut western_section = format!(
                "【西洋占星】\n太陽：{}\n月亮：{}\n行星：{}\n",
                w_sun,
                w_moon,
                w_planets.join("、")
            );

            let mut extra_w = String::new();

            // 上升 ascendant
            if let Some(asc) = w.get("ascendant").and_then(|v| v.as_object()) {
                let sign = asc.get("sign").and_then(|v| v.as_str()).unwrap_or("");
                let deg = asc.get("degree").and_then(|v| v.as_f64());
                let lon = asc.get("longitude").and_then(|v| v.as_f64());
                if !sign.is_empty() {
                    if let Some(d) = deg {
                        extra_w.push_str(&format!("上升：{} {:.1}°", sign, d));
                        if let Some(l) = lon {
                            extra_w.push_str(&format!("（黃經 {:.1}°）", l));
                        }
                        extra_w.push('\n');
                    } else if let Some(l) = lon {
                        extra_w.push_str(&format!("上升：{}（黃經 {:.1}°）\n", sign, l));
                    } else {
                        extra_w.push_str(&format!("上升：{}\n", sign));
                    }
                }
            } else if let Some(sign) = w.pointer("/ascendant/sign").and_then(|v| v.as_str()) {
                extra_w.push_str(&format!("上升：{}\n", sign));
            }

            // 宮位 houses
            if let Some(houses) = w.get("houses").and_then(|v| v.as_array()) {
                if !houses.is_empty() {
                    let mut h_parts: Vec<String> = Vec::new();
                    for h in houses {
                        let idx = h.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                        let sign = h.get("sign").and_then(|v| v.as_str()).unwrap_or("");
                        let cusp = h.get("cusp").and_then(|v| v.as_f64());
                        if idx > 0 && !sign.is_empty() {
                            if let Some(c) = cusp {
                                h_parts.push(format!("第{}宮：{}（{:.1}°）", idx, sign, c));
                            } else {
                                h_parts.push(format!("第{}宮：{}", idx, sign));
                            }
                        }
                    }
                    if !h_parts.is_empty() {
                        extra_w.push_str(&format!("宮位：{}\n", h_parts.join("、")));
                    }
                }
            }

            // 行星的 house 統計（若有 ascendant longitude 與 planets longitude 則計算，否則按 sign 計數退化）
            if let Some(planets) = w.get("planets").and_then(|v| v.as_array()) {
                if !planets.is_empty() {
                    // 嘗試用黃經計算 house
                    let asc_lon_opt = w.pointer("/ascendant/longitude").and_then(|v| v.as_f64());
                    if let Some(asc_lon) = asc_lon_opt {
                        let mut house_counts: std::collections::BTreeMap<u8, usize> =
                            std::collections::BTreeMap::new();
                        let mut planet_houses: Vec<String> = Vec::new();
                        for pl in planets {
                            let name = pl.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let lon = pl.get("longitude").and_then(|v| v.as_f64());
                            if name.is_empty() {
                                continue;
                            }
                            if let Some(l) = lon {
                                let diff = (l - asc_lon).rem_euclid(360.0);
                                let house = (diff / 30.0).floor() as u8 + 1;
                                let house = house.clamp(1, 12);
                                *house_counts.entry(house).or_insert(0) += 1;
                                planet_houses.push(format!("{}在第{}宮", name, house));
                            }
                        }
                        if !planet_houses.is_empty() {
                            extra_w.push_str(&format!("行星宮位：{}\n", planet_houses.join("、")));
                        }
                        if !house_counts.is_empty() {
                            let summary: Vec<String> = house_counts
                                .iter()
                                .map(|(h, c)| format!("第{}宮{}顆", h, c))
                                .collect();
                            extra_w.push_str(&format!("宮位行星統計：{}\n", summary.join("、")));
                        }
                    } else {
                        // 退化：按星座計數
                        let mut sign_counts: std::collections::BTreeMap<String, usize> =
                            std::collections::BTreeMap::new();
                        for pl in planets {
                            if let Some(sign) = pl.get("sign").and_then(|v| v.as_str()) {
                                *sign_counts.entry(sign.to_string()).or_insert(0) += 1;
                            }
                        }
                        if !sign_counts.is_empty() {
                            let summary: Vec<String> = sign_counts
                                .iter()
                                .map(|(s, c)| format!("{} {}顆", s, c))
                                .collect();
                            extra_w.push_str(&format!("行星星座統計：{}\n", summary.join("、")));
                        }
                    }
                }
            }

            if !extra_w.is_empty() {
                western_section.push_str(&extra_w);
            }
            // 確保段落間空行
            if !western_section.ends_with('\n') {
                western_section.push('\n');
            }
            western_section.push('\n');
            p.push_str(&western_section);
        }
        p.push_str("請輸出恰好四個章節，每個章節以 markdown 標題行開頭，順序固定如下：\n");
        p.push_str("## 第一章：本質\n## 第二章：道路\n## 第三章：關係\n## 第四章：寶藏\n\n");
        p.push_str("每個章節 2-3 段，請把紫微斗數與西洋占星的洞見交織融會在敘事之中，而不是分開條列兩套術語。");
        if let Some(f) = focus {
            p.push_str(&format!("\n\n請特別著墨：{}", f));
        }
        // P2 telemetry: generation tags + prompt length (story generation path)
        {
            let prompt_tags_len = chart
                .get("generation_tags")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let prompt_has_generation_story = if prompt_tags_len == 0 {
                false
            } else if let Some(arr) = chart.get("generation_stories").and_then(|v| v.as_array()) {
                !arr.is_empty()
            } else {
                chart
                    .get("generation_tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().filter_map(|v| v.as_str()).any(|t| {
                            crate::services::generation::generation_story_for_tag(t).is_some()
                        })
                    })
                    .unwrap_or(false)
            };
            worker::console_log!(
                "metric: prompt_chart_type=story prompt_generation_tags_len={} prompt_has_generation_story={} prompt_chars={}",
                prompt_tags_len,
                prompt_has_generation_story,
                p.chars().count()
            );
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
