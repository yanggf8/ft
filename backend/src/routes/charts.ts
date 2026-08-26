import { Hono } from 'hono';
import type { Env } from '../index';
import { authMiddleware } from '../middleware/auth';
import { setCacheHeaders, createETag } from '../middleware/cache';
import { iztroAdapter } from '../services/ziwei/iztro-adapter';
import { westernCalculator } from '../services/western';
import { ENGINE_VERSION_ZIWEI, ENGINE_VERSION_WESTERN, CHART_SCHEMA_VERSION } from '../services/engine-version';
import { ZiWeiV3ResponseSchema } from '../shared/schemas/ziwei-v3';

const charts = new Hono<{ Bindings: Env }>();

// Rate limit for AI (10 req/min/IP)
const aiRateLimit = new Map<string, { count: number; reset: number }>();
const AI_LIMIT = 10;
const WINDOW_MS = 60000;

function checkAiRateLimit(ip: string): boolean {
  const now = Date.now();
  const entry = aiRateLimit.get(ip);
  if (!entry || now > entry.reset) {
    aiRateLimit.set(ip, { count: 1, reset: now + WINDOW_MS });
    return true;
  }
  if (entry.count >= AI_LIMIT) return false;
  entry.count++;
  return true;
}

interface UserBirthData {
  birth_year: number | null;
  birth_month: number | null;
  birth_day: number | null;
  birth_hour: number | null;
  birth_minute: number | null;
  gender: string | null;
  timezone: string | null;
  latitude: number | null;
  longitude: number | null;
  birth_data_hash: string | null;
}

// Get user's birth data
async function getUserBirthData(db: D1Database, userId: string): Promise<UserBirthData | null> {
  return db.prepare(
    `SELECT birth_year, birth_month, birth_day, birth_hour, birth_minute,
            gender, timezone, latitude, longitude, birth_data_hash
     FROM users WHERE id = ?`
  ).bind(userId).first<UserBirthData>();
}

// List user's interpretations (cached charts)
charts.get('/', authMiddleware, async (c) => {
  const { userId } = c.get('user');

  const results = await c.env.DB.prepare(
    'SELECT * FROM interpretations WHERE user_id = ? ORDER BY created_at DESC'
  ).bind(userId).all();

  // Parse chart_data from JSON string to object
  const interpretations = (results.results || []).map((row: Record<string, unknown>) => ({
    ...row,
    chart_data: typeof row.chart_data === 'string' ? JSON.parse(row.chart_data as string) : row.chart_data
  }));

  return c.json({ interpretations });
});

// Get or calculate chart for a divination type
charts.get('/:type', authMiddleware, setCacheHeaders({ maxAge: 3600, shared: false }), async (c) => {
  const { userId } = c.get('user');
  const divType = c.req.param('type');

  if (!['ziwei', 'western'].includes(divType)) {
    return c.json({ error: 'Invalid type. Use: ziwei, western' }, 400);
  }

  // Get user birth data
  const birth = await getUserBirthData(c.env.DB, userId);
  if (!birth?.birth_year || !birth?.birth_month || !birth?.birth_day) {
    return c.json({ error: 'Birth data required', code: 'NO_BIRTH_DATA' }, 400);
  }

  const expectedVersion = divType === 'ziwei' ? ENGINE_VERSION_ZIWEI : ENGINE_VERSION_WESTERN;

  // Check cache
  const cached = await c.env.DB.prepare(
    'SELECT * FROM interpretations WHERE user_id = ? AND divination_type = ? AND birth_data_hash = ?'
  ).bind(userId, divType, birth.birth_data_hash).first<{
    id: string;
    chart_data: string;
    ai_interpretation: string | null;
    created_at: string;
    updated_at: string;
  }>();

  // Generate ETag from birth_data_hash, version and updated_at (per-type version included)
  const etag = createETag(`${birth.birth_data_hash || ''}-${expectedVersion}-${CHART_SCHEMA_VERSION}`, cached?.updated_at || Date.now());

  // Check If-None-Match header for conditional request
  const ifNoneMatch = c.req.header('if-none-match');
  if (ifNoneMatch && ifNoneMatch === etag) {
    return c.newResponse(null, { status: 304 });
  }

  if (cached) {
    const parsedChart = typeof cached.chart_data === 'string'
      ? JSON.parse(cached.chart_data)
      : cached.chart_data;
    const storedVersion = parsedChart?.meta?.engineVersionZiwei ?? parsedChart?.engineVersion;
    // Per-type version check: mismatch is treated as cache miss
    if (storedVersion === expectedVersion) {
      const response = {
        ...cached,
        chart_data: parsedChart,
        fromCache: true
      };
      c.res.headers.set('ETag', etag);
      return c.json(response);
    }
  }

  // Calculate chart
  const hour = birth.birth_hour ?? 12; // default noon if unknown
  let chartData: unknown;

  if (divType === 'ziwei') {
    if (!birth.gender) {
      return c.json({ error: 'Gender required for ZiWei', code: 'NO_GENDER' }, 400);
    }
    chartData = iztroAdapter.calculate({
      year: birth.birth_year,
      month: birth.birth_month,
      day: birth.birth_day,
      hour,
      minute: birth.birth_minute ?? undefined,
      gender: birth.gender as 'male' | 'female'
    });
  } else {
    chartData = westernCalculator.calculate({
      year: birth.birth_year,
      month: birth.birth_month,
      day: birth.birth_day,
      hour,
      minute: birth.birth_minute ?? undefined,
      latitude: birth.latitude ?? undefined,
      longitude: birth.longitude ?? undefined
    });
  }

  // Embed per-type engine version and schema version into stored chart data
  const chartDataWithVersion = {
    ...(chartData as Record<string, unknown>),
    meta: {
      ...((chartData as Record<string, unknown>).meta as Record<string, unknown> ?? {}),
      engineVersionZiwei: divType === 'ziwei' ? ENGINE_VERSION_ZIWEI : undefined,
      engineVersionWestern: divType === 'western' ? ENGINE_VERSION_WESTERN : undefined,
      chartSchemaVersion: CHART_SCHEMA_VERSION,
    },
    // Back-compat top-level (keep one release):
    engineVersion: expectedVersion,
    chartSchemaVersion: CHART_SCHEMA_VERSION,
  };

  // Upsert to cache (INSERT OR REPLACE handles concurrent first-load race).
  // DO UPDATE only fires when an existing row was stale (hash mismatch or old engine
  // version), so clearing ai_interpretation here never discards a valid reading —
  // an old-chart interpretation must not survive onto the recalculated chart.
  const id = crypto.randomUUID();
  await c.env.DB.prepare(
    `INSERT INTO interpretations (id, user_id, divination_type, chart_data, birth_data_hash)
     VALUES (?, ?, ?, ?, ?)
     ON CONFLICT(user_id, divination_type) DO UPDATE SET
       chart_data = excluded.chart_data,
       birth_data_hash = excluded.birth_data_hash,
       ai_interpretation = NULL,
       updated_at = datetime('now')`
  ).bind(id, userId, divType, JSON.stringify(chartDataWithVersion), birth.birth_data_hash).run();

  const response = {
    id,
    user_id: userId,
    divination_type: divType,
    chart_data: chartDataWithVersion,
    ai_interpretation: null,
    birth_data_hash: birth.birth_data_hash,
    fromCache: false,
    engineVersion: expectedVersion,
    chartSchemaVersion: CHART_SCHEMA_VERSION
  };

  // Zod validation for ziwei V3 response (shape guard before shipping)
  if (divType === 'ziwei') {
    const parsed = ZiWeiV3ResponseSchema.safeParse(response);
    if (!parsed.success) {
      console.error('ZiWei V3 schema violation', parsed.error.flatten());
      return c.json({ error: 'Chart schema violation' }, 500);
    }
  }

  c.res.headers.set('ETag', etag);
  return c.json(response);
});

// Request AI interpretation for a chart
charts.post('/:type/interpret', authMiddleware, setCacheHeaders({ maxAge: 86400, shared: false, mustRevalidate: true }), async (c) => {
  const { userId } = c.get('user');
  const divType = c.req.param('type');
  const ip = c.req.header('cf-connecting-ip') || 'unknown';

  if (!checkAiRateLimit(ip)) {
    return c.json({ error: 'Too many requests', code: 'RATE_LIMIT' }, 429);
  }

  if (!['ziwei', 'western'].includes(divType)) {
    return c.json({ error: 'Invalid type' }, 400);
  }

  // Get cached chart
  const interp = await c.env.DB.prepare(
    'SELECT * FROM interpretations WHERE user_id = ? AND divination_type = ?'
  ).bind(userId, divType).first<{
    id: string;
    chart_data: string;
    ai_interpretation: string | null;
    updated_at: string;
    birth_data_hash: string;
  }>();

  if (!interp) {
    return c.json({ error: 'Chart not found. Call GET /:type first' }, 404);
  }

  // Stale version guard: if cached chart was built with old engine, require recalc first
  {
    const chartDataParsed = typeof interp.chart_data === 'string' ? JSON.parse(interp.chart_data) : JSON.parse(interp.chart_data as unknown as string);
    const storedVersion = (chartDataParsed as Record<string, unknown>)?.meta
      ? ((chartDataParsed as Record<string, unknown>).meta as Record<string, unknown>).engineVersionZiwei as string ?? (chartDataParsed as Record<string, unknown>).engineVersion as string
      : (chartDataParsed as Record<string, unknown>).engineVersion as string;
    const expectedVersion = divType === 'ziwei' ? ENGINE_VERSION_ZIWEI : ENGINE_VERSION_WESTERN;
    if (storedVersion !== expectedVersion) {
      return c.json({ error: 'Chart version stale, recalculation required', code: 'RECALC_REQUIRED' }, 409);
    }
  }

  // Return cached interpretation if exists
  if (interp.ai_interpretation) {
    // Create ETag from interpretation content hash
    const etag = createETag(interp.birth_data_hash + '-ai', interp.updated_at);

    // Check If-None-Match header for conditional request
    const ifNoneMatch = c.req.header('if-none-match');
    if (ifNoneMatch && ifNoneMatch === etag) {
      return c.newResponse(null, { status: 304 });
    }

    c.res.headers.set('ETag', etag);
    return c.json({ interpretation: interp.ai_interpretation, fromCache: true });
  }

  // Check AI providers
  if (!c.env.IFLOW_API_KEY && !c.env.GROQ_API_KEY && !c.env.CEREBRAS_API_KEY) {
    return c.json({ error: 'AI service not configured' }, 503);
  }

  // Call AI via mutex
  const mutexId = c.env.AI_MUTEX.idFromName('global');
  const mutex = c.env.AI_MUTEX.get(mutexId);

  const chartData = JSON.parse(interp.chart_data);
  const response = await mutex.fetch('https://ai-mutex/interpret', {
    method: 'POST',
    body: JSON.stringify({
      keys: { iflow: c.env.IFLOW_API_KEY, groq: c.env.GROQ_API_KEY, cerebras: c.env.CEREBRAS_API_KEY },
      interpretRequest: { chartType: divType, chartData, language: 'zh' }
    })
  });

  if (!response.ok) {
    const err = await response.json() as { error?: string };
    return c.json(err, response.status as 400 | 500);
  }

  const result = await response.json() as { interpretation: string; provider: string; model: string };

  // Save interpretation
  await c.env.DB.prepare(
    "UPDATE interpretations SET ai_interpretation = ?, updated_at = datetime('now') WHERE id = ?"
  ).bind(result.interpretation, interp.id).run();

  return c.json({
    interpretation: result.interpretation,
    provider: result.provider,
    model: result.model,
    fromCache: false
  });
});

export default charts;
