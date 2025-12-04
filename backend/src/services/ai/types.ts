/**
 * AI Provider Adapter Types
 */

export interface InterpretationRequest {
  chartType: 'ziwei' | 'western';
  chartData: Record<string, unknown>;
  language?: 'zh' | 'en';
  focus?: string;
}

export interface InterpretationResponse {
  interpretation: string;
  provider: string;
  model: string;
  tokensUsed?: number;
}

export interface AIProvider {
  name: string;
  interpret(req: InterpretationRequest): Promise<InterpretationResponse>;
}
