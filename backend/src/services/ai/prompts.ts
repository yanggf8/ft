/**
 * Shared prompt builders for all AI providers
 */

export interface PromptInput {
  chartType: 'ziwei' | 'western' | 'story';
  chartData: Record<string, unknown>;
  language?: 'zh' | 'en';
  focus?: string;
}

export function getSystemPrompt(chartType: string, language?: string): string {
  if (chartType === 'ziwei') {
    return language === 'en'
      ? 'You are an expert in 紫微斗數 (Zi Wei Dou Shu). Provide insightful interpretations in English.'
      : '你是紫微斗數專家。請用繁體中文提供深入且實用的命盤解讀，語氣溫和專業。';
  }
  if (chartType === 'story') {
    return language === 'en'
      ? 'You are a gifted storyteller who weaves together Zi Wei Dou Shu and Western astrology into one cohesive life story. Write in warm, narrative prose, structured as exactly four chapters titled Essence / Path / Relationships / Treasure.'
      : '你是一位融合紫微斗數與西洋占星的生命說書人。請用溫暖、敘事性強的繁體中文，將兩套體系交織成一個完整的人生故事。故事分為四個章節，依序為：本質、道路、關係、寶藏。不要條列術語，而是用故事的語言把兩套系統的洞見融為一體。';
  }
  return language === 'zh'
    ? '你是西洋占星專家。請用繁體中文提供深入的星盤解讀。'
    : 'You are an expert Western astrologer. Provide insightful natal chart interpretations.';
}

export function buildPrompt(req: PromptInput): string {
  const { chartType, chartData, focus } = req;

  if (chartType === 'ziwei') {
    const d = chartData as { birthInfo?: { gender?: string }; fiveElement?: string; palaces?: { name: string; stars?: { name: string }[] }[] };
    let p = `請解讀以下紫微斗數命盤：\n\n五行局：${d.fiveElement || '未知'}\n性別：${d.birthInfo?.gender === 'male' ? '男' : d.birthInfo?.gender === 'female' ? '女' : '未知'}\n\n十二宮星曜分布：\n${(d.palaces || []).map(p => `${p.name}：${p.stars?.length ? p.stars.map(s => s.name).join('、') : '無主星'}`).join('\n')}`;
    if (focus) p += `\n\n請特別分析：${focus}`;
    return p;
  }

  if (chartType === 'story') {
    const d = chartData as { ziwei?: Record<string, unknown>; western?: Record<string, unknown> };
    const z = (d.ziwei || {}) as { birthInfo?: { gender?: string }; fiveElement?: string; palaces?: { name: string; stars?: { name: string }[] }[] };
    const w = (d.western || {}) as { sunSign?: { name: string }; moonSign?: { name: string }; planets?: { name: string; sign: string }[] };

    const ziweiGender = z.birthInfo?.gender === 'male' ? '男' : z.birthInfo?.gender === 'female' ? '女' : '未知';
    const ziweiPalaces = (z.palaces || [])
      .map(p => `${p.name}：${p.stars?.length ? p.stars.map(s => s.name).join('、') : '無主星'}`)
      .join('\n');

    const westernPlanets = (w.planets || [])
      .map(p => `${p.name} in ${p.sign}`)
      .join('、');

    let p = `請根據以下同一個人的兩張命盤，創作一篇融合紫微斗數與西洋占星的生命故事。\n\n`;
    p += `【紫微斗數】\n五行局：${z.fiveElement || '未知'}\n性別：${ziweiGender}\n十二宮星曜：\n${ziweiPalaces}\n\n`;
    p += `【西洋占星】\n太陽：${w.sunSign?.name || '未知'}\n月亮：${w.moonSign?.name || '未知'}\n行星：${westernPlanets}\n\n`;
    p += `請輸出恰好四個章節，每個章節以 markdown 標題行開頭，順序固定如下：\n`;
    p += `## 第一章：本質\n## 第二章：道路\n## 第三章：關係\n## 第四章：寶藏\n\n`;
    p += `每個章節 2-3 段，請把紫微斗數與西洋占星的洞見交織融會在敘事之中，而不是分開條列兩套術語。`;
    if (focus) p += `\n\n請特別著墨：${focus}`;
    return p;
  }

  const d = chartData as { sunSign?: { name: string }; moonSign?: { name: string }; planets?: { name: string; sign: string }[] };
  let p = `Interpret this natal chart:\nSun: ${d.sunSign?.name || 'Unknown'}\nMoon: ${d.moonSign?.name || 'Unknown'}\nPlanets: ${(d.planets || []).map(p => `${p.name} in ${p.sign}`).join(', ')}`;
  if (focus) p += `\n\nFocus on: ${focus}`;
  return p;
}
