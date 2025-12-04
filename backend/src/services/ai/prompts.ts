/**
 * Shared prompt builders for all AI providers
 */

export interface PromptInput {
  chartType: 'ziwei' | 'western';
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

  const d = chartData as { sunSign?: { name: string }; moonSign?: { name: string }; planets?: { name: string; sign: string }[] };
  let p = `Interpret this natal chart:\nSun: ${d.sunSign?.name || 'Unknown'}\nMoon: ${d.moonSign?.name || 'Unknown'}\nPlanets: ${(d.planets || []).map(p => `${p.name} in ${p.sign}`).join(', ')}`;
  if (focus) p += `\n\nFocus on: ${focus}`;
  return p;
}
