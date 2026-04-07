import { Hono } from 'hono';
import type { Env } from '../index';
import { authMiddleware } from '../middleware/auth';
import { setCacheHeaders, createETag } from '../middleware/cache';
import { ziWeiCalculator } from '../services/ziwei';
import { westernCalculator } from '../services/western';

const charts = new Hono<{ Bindings: Env }>();

const ENGINE_VERSION = '1.0.0';

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
  
  return c.json({ interpretations: results.results });
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

  // Generate ETag from birth_data_hash and updated_at
  const etag = createETag(birth.birth_data_hash || '', cached?.updated_at || Date.now());

  // Check If-None-Match header for conditional request
  const ifNoneMatch = c.req.header('if-none-match');
  if (ifNoneMatch && ifNoneMatch === etag) {
    return c.newResponse(null, { status: 304 });
  }

  if (cached) {
    // Parse chart_data from JSON string to object for consistent response shape
    const response = {
      ...cached,
      chart_data: typeof cached.chart_data === 'string'
        ? JSON.parse(cached.chart_data)
        : cached.chart_data,
      fromCache: true
    };
    c.res.headers.set('ETag', etag);
    return c.json(response);
  }

  // Calculate chart
  const hour = birth.birth_hour ?? 12; // default noon if unknown
  let chartData: unknown;

  if (divType === 'ziwei') {
    if (!birth.gender) {
      return c.json({ error: 'Gender required for ZiWei', code: 'NO_GENDER' }, 400);
    }
    chartData = ziWeiCalculator.calculate({
      year: birth.birth_year,
      month: birth.birth_month,
      day: birth.birth_day,
      hour,
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

  // Save to cache
  const id = crypto.randomUUID();
  await c.env.DB.prepare(
    `INSERT INTO interpretations (id, user_id, divination_type, chart_data, birth_data_hash)
     VALUES (?, ?, ?, ?, ?)`
  ).bind(id, userId, divType, JSON.stringify(chartData), birth.birth_data_hash).run();

  const response = {
    id,
    user_id: userId,
    divination_type: divType,
    chart_data: chartData,
    ai_interpretation: null,
    birth_data_hash: birth.birth_data_hash,
    fromCache: false,
    engineVersion: ENGINE_VERSION
  };
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
