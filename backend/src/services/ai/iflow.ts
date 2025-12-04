/**
 * iFlow AI Provider - GLM-4.6
 */

import type { AIProvider, InterpretationRequest, InterpretationResponse } from './types';
import { getSystemPrompt, buildPrompt } from './prompts';

const IFLOW_API_URL = 'https://apis.iflow.cn/v1/chat/completions';

export interface IFlowConfig {
  apiKey: string;
  model?: string;
  maxTokens?: number;
}

export class IFlowProvider implements AIProvider {
  name = 'iflow';
  private apiKey: string;
  private model: string;
  private maxTokens: number;

  constructor(config: IFlowConfig) {
    this.apiKey = config.apiKey;
    this.model = config.model || 'GLM-4.6';
    this.maxTokens = config.maxTokens || 1024;
  }

  async interpret(req: InterpretationRequest): Promise<InterpretationResponse> {
    const res = await fetch(IFLOW_API_URL, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${this.apiKey}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: this.model,
        messages: [
          { role: 'system', content: getSystemPrompt(req.chartType, req.language) },
          { role: 'user', content: buildPrompt(req) }
        ],
        max_tokens: this.maxTokens,
        temperature: 0.7,
      }),
    });

    if (!res.ok) throw new Error(`iFlow ${res.status}: ${await res.text()}`);

    const data = await res.json() as { choices: { message: { content: string } }[]; usage?: { total_tokens: number } };
    return {
      interpretation: data.choices[0]?.message?.content || '',
      provider: this.name,
      model: this.model,
      tokensUsed: data.usage?.total_tokens,
    };
  }
}
