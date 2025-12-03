/**
 * Groq AI Integration
 * Uses Groq's fast inference API for chart interpretations
 */

export interface GroqConfig {
  apiKey: string;
  model?: string;
  maxTokens?: number;
}

export interface InterpretationRequest {
  chartType: 'ziwei' | 'western';
  chartData: Record<string, unknown>;
  language?: 'zh' | 'en';
  focus?: string; // e.g., 'career', 'love', 'health'
}

export interface InterpretationResponse {
  interpretation: string;
  model: string;
  tokensUsed: number;
}

const GROQ_API_URL = 'https://api.groq.com/openai/v1/chat/completions';
const DEFAULT_MODEL = 'llama-3.1-8b-instant';
const DEFAULT_MAX_TOKENS = 1024;

export class GroqClient {
  private apiKey: string;
  private model: string;
  private maxTokens: number;

  constructor(config: GroqConfig) {
    this.apiKey = config.apiKey;
    this.model = config.model || DEFAULT_MODEL;
    this.maxTokens = config.maxTokens || DEFAULT_MAX_TOKENS;
  }

  async interpret(req: InterpretationRequest): Promise<InterpretationResponse> {
    const prompt = this.buildPrompt(req);
    
    const response = await fetch(GROQ_API_URL, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: this.model,
        messages: [
          { role: 'system', content: this.getSystemPrompt(req.chartType, req.language) },
          { role: 'user', content: prompt }
        ],
        max_tokens: this.maxTokens,
        temperature: 0.7,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Groq API error: ${response.status} - ${error}`);
    }

    const data = await response.json() as {
      choices: { message: { content: string } }[];
      usage: { total_tokens: number };
    };

    return {
      interpretation: data.choices[0]?.message?.content || '',
      model: this.model,
      tokensUsed: data.usage?.total_tokens || 0,
    };
  }

  private getSystemPrompt(chartType: string, language?: string): string {
    const lang = language === 'en' ? 'English' : 'Traditional Chinese';
    
    if (chartType === 'ziwei') {
      return `You are an expert in 紫微斗數 (Zi Wei Dou Shu / Purple Star Astrology). 
Provide insightful, balanced interpretations based on the chart data provided.
Focus on practical guidance while respecting the tradition.
Respond in ${lang}. Keep responses concise but meaningful.`;
    }
    
    return `You are an expert Western astrologer.
Provide insightful interpretations based on the natal chart data provided.
Focus on psychological insights and practical guidance.
Respond in ${lang}. Keep responses concise but meaningful.`;
  }

  private buildPrompt(req: InterpretationRequest): string {
    const { chartType, chartData, focus } = req;
    
    let prompt = '';
    
    if (chartType === 'ziwei') {
      const data = chartData as {
        birthInfo?: { lunar?: { year: number; month: number; day: number }; gender?: string };
        fiveElement?: string;
        palaces?: { name: string; stars: { name: string }[] }[];
      };
      
      prompt = `請解讀以下紫微斗數命盤：

五行局：${data.fiveElement || '未知'}
性別：${data.birthInfo?.gender || '未知'}

十二宮星曜分布：
${(data.palaces || []).map(p => `${p.name}：${p.stars?.map(s => s.name).join('、') || '無主星'}`).join('\n')}`;
      
      if (focus) {
        prompt += `\n\n請特別分析：${focus}`;
      }
    } else {
      const data = chartData as {
        sunSign?: { name: string };
        moonSign?: { name: string };
        planets?: { name: string; sign: string }[];
      };
      
      prompt = `Please interpret this natal chart:

Sun Sign: ${data.sunSign?.name || 'Unknown'}
Moon Sign: ${data.moonSign?.name || 'Unknown'}
Planets: ${(data.planets || []).map(p => `${p.name} in ${p.sign}`).join(', ')}`;
      
      if (focus) {
        prompt += `\n\nPlease focus on: ${focus}`;
      }
    }
    
    return prompt;
  }
}
