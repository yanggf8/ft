/**
 * Cerebras AI Provider - llama-3.3-70b
 */

import type { AIProvider, InterpretationRequest, InterpretationResponse } from './types';
import { getSystemPrompt, buildPrompt } from './prompts';

const CEREBRAS_API_URL = 'https://api.cerebras.ai/v1/chat/completions';

export interface CerebrasConfig {
  apiKey: string;
  model?: string;
  maxTokens?: number;
}

export class CerebrasProvider implements AIProvider {
  name = 'cerebras';
  private apiKey: string;
  private model: string;
  private maxTokens: number;

  constructor(config: CerebrasConfig) {
    this.apiKey = config.apiKey;
    this.model = config.model || 'llama-3.3-70b';
    this.maxTokens = config.maxTokens || 1024;
  }

  async interpret(req: InterpretationRequest): Promise<InterpretationResponse> {
    const res = await fetch(CEREBRAS_API_URL, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${this.apiKey}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: this.model,
        messages: [
          { role: 'system', content: getSystemPrompt(req.chartType, req.language) },
          { role: 'user', content: buildPrompt(req) }
        ],
        max_completion_tokens: this.maxTokens,
        temperature: 0.7,
      }),
    });

    if (!res.ok) throw new Error(`Cerebras ${res.status}: ${await res.text()}`);

    const data = await res.json() as { choices: { message: { content: string } }[]; usage?: { total_tokens: number } };
    return {
      interpretation: data.choices[0]?.message?.content || '',
      provider: this.name,
      model: this.model,
      tokensUsed: data.usage?.total_tokens,
    };
  }
}
