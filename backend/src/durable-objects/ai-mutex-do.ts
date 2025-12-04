/**
 * AI Mutex Durable Object
 * - 3-provider failover: iFlow → Groq → Cerebras
 * - Serializes requests (1 concurrent)
 * - Tracks exresource (external resource usage + errors)
 */

import { DurableObject } from 'cloudflare:workers';

const PROVIDERS = [
  { name: 'iflow', model: 'GLM-4.6', rpm: 1, rpd: Infinity },
  { name: 'groq', model: 'moonshotai/kimi-k2-instruct-0905', rpm: 30, rpd: 14400 },
  { name: 'cerebras', model: 'llama-3.3-70b', rpm: 30, rpd: 14400 },
] as const;

type ProviderName = typeof PROVIDERS[number]['name'];

interface QueuedRequest {
  resolve: (value: Response) => void;
  reject: (error: Error) => void;
  request: Request;
}

interface ExResource {
  requests: number;
  tokens: number;
  errors: number;
  lastError?: { time: string; code: string; message: string };
  latencySum: number;
  failovers: number;
}

interface MinuteRecord { count: number; reset: number }

export class AIMutexDO extends DurableObject {
  private processing = false;
  private queue: QueuedRequest[] = [];

  async fetch(request: Request): Promise<Response> {
    return new Promise<Response>((resolve, reject) => {
      this.queue.push({ resolve, reject, request });
      this.processQueue();
    });
  }

  private async processQueue(): Promise<void> {
    if (this.processing || this.queue.length === 0) return;
    this.processing = true;
    const item = this.queue.shift()!;
    try {
      item.resolve(await this.handleRequest(item.request));
    } catch (e) {
      item.reject(e instanceof Error ? e : new Error(String(e)));
    } finally {
      this.processing = false;
      if (this.queue.length > 0) this.processQueue();
    }
  }

  private async checkRpm(provider: ProviderName, limit: number): Promise<boolean> {
    const now = Date.now();
    const key = `rpm:${provider}`;
    const rec = await this.ctx.storage.get<MinuteRecord>(key);
    if (!rec || now > rec.reset) {
      await this.ctx.storage.put(key, { count: 1, reset: now + 60000 });
      return true;
    }
    if (rec.count >= limit) return false;
    await this.ctx.storage.put(key, { count: rec.count + 1, reset: rec.reset });
    return true;
  }

  private async getExResource(provider: ProviderName, date: string): Promise<ExResource> {
    return await this.ctx.storage.get<ExResource>(`exresource:${provider}:${date}`) || { requests: 0, tokens: 0, errors: 0, latencySum: 0, failovers: 0 };
  }

  private async updateExResource(provider: ProviderName, date: string, update: { requests?: number; addTokens?: number; errors?: number; addLatency?: number; failovers?: number; error?: { code: string; message: string } }): Promise<void> {
    const key = `exresource:${provider}:${date}`;
    const cur = await this.getExResource(provider, date);
    await this.ctx.storage.put(key, {
      requests: cur.requests + (update.requests || 0),
      tokens: cur.tokens + (update.addTokens || 0),
      errors: cur.errors + (update.errors || 0),
      latencySum: cur.latencySum + (update.addLatency || 0),
      failovers: cur.failovers + (update.failovers || 0),
      lastError: update.error ? { time: new Date().toISOString(), ...update.error } : cur.lastError,
    });
  }

  private async handleRequest(request: Request): Promise<Response> {
    const body = await request.json() as {
      keys: { iflow?: string; groq?: string; cerebras?: string };
      interpretRequest: { chartType: 'ziwei' | 'western'; chartData: Record<string, unknown>; language?: 'zh' | 'en'; focus?: string };
    };

    const { keys, interpretRequest } = body;
    const today = new Date().toISOString().split('T')[0];
    let lastError: { provider: string; code: string; message: string } | null = null;
    let failoverCount = 0;

    for (const p of PROVIDERS) {
      const apiKey = keys[p.name as keyof typeof keys];
      if (!apiKey) continue;

      const ex = await this.getExResource(p.name, today);
      if (ex.requests >= p.rpd || !(await this.checkRpm(p.name, p.rpm))) continue;

      const start = Date.now();
      try {
        const result = await this.callProvider(p.name, p.model, apiKey, interpretRequest);
        await this.updateExResource(p.name, today, { requests: 1, addTokens: result.tokensUsed || 0, addLatency: Date.now() - start, failovers: failoverCount });
        return Response.json({ ...result, exresource: { provider: p.name, model: p.model, latency: Date.now() - start, failovers: failoverCount, date: today } });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        const code = msg.includes('429') ? 'RATE_LIMIT' : msg.includes('401') ? 'AUTH' : 'API_ERROR';
        await this.updateExResource(p.name, today, { errors: 1, error: { code, message: msg.slice(0, 200) } });
        lastError = { provider: p.name, code, message: msg };
        failoverCount++;
      }
    }

    return Response.json({ error: 'All providers failed', code: 'ALL_PROVIDERS_FAILED', lastError, failovers: failoverCount }, { status: 503 });
  }

  private async callProvider(name: ProviderName, model: string, apiKey: string, req: { chartType: 'ziwei' | 'western'; chartData: Record<string, unknown>; language?: 'zh' | 'en'; focus?: string }): Promise<{ interpretation: string; provider: string; model: string; tokensUsed?: number }> {
    const { getSystemPrompt, buildPrompt } = await import('../services/ai/prompts');

    if (name === 'iflow') {
      const { IFlowProvider } = await import('../services/ai/iflow');
      return new IFlowProvider({ apiKey, model }).interpret(req);
    }

    // Groq & Cerebras: OpenAI-compatible
    const baseUrl = name === 'groq' ? 'https://api.groq.com/openai/v1' : 'https://api.cerebras.ai/v1';
    const res = await fetch(`${baseUrl}/chat/completions`, {
      method: 'POST',
      headers: { 'Authorization': `Bearer ${apiKey}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model,
        messages: [{ role: 'system', content: getSystemPrompt(req.chartType, req.language) }, { role: 'user', content: buildPrompt(req) }],
        max_tokens: 1024,
        temperature: 0.7,
      }),
    });

    if (!res.ok) throw new Error(`${name} ${res.status}: ${await res.text()}`);
    const data = await res.json() as { choices: { message: { content: string } }[]; usage?: { total_tokens: number } };
    return { interpretation: data.choices[0]?.message?.content || '', provider: name, model, tokensUsed: data.usage?.total_tokens };
  }
}
