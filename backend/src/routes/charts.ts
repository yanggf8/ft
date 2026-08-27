import { Hono } from 'hono';
import type { Env } from '../index';
import { authMiddleware } from '../middleware/auth';
import { setCacheHeaders, createETag } from '../middleware/cache';
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

// A story merges the ziwei AND western charts, so it is only current when both
// engine versions and the chart schema still match. Mirrors the per-type guard on
// GET /:type and POST /:type/interpret.
function isStoryChartCurrent(rawChartData: string | null | undefined): boolean {
  if (!rawChartData) return false;
  try {
    const parsed = JSON.parse(rawChartData) as { meta?: Record<string, unknown> };
    return parsed?.meta?.engineVersionZiwei === ENGINE_VERSION_ZIWEI
      && parsed?.meta?.engineVersionWestern === ENGINE_VERSION_WESTERN
      && parsed?.meta?.chartSchemaVersion === CHART_SCHEMA_VERSION;
  } catch {
    return false;
  }
}


// ── Rust engine service binding ─────────────────────────────────────────────
// The ziwei/western calculations moved to the Rust `fortunet-engine` Worker
// (service binding FT_ENGINE). Production adapter logic (hour→timeIndex,
// zh-TW mapping, sihua codes) is replicated there; this helper only marshals
// birth data and unwraps the engine's { chart } envelope.

interface EngineBirth {
  year: number | null;
  month: number | null;
  day: number | null;
  hour: number;
  gender: string | null;
  latitude: number | null;
  longitude: number | null;
  timezone: string | null;
}

async function fetchEngineChart(
  fetcher: Fetcher,
  type: 'ziwei' | 'western',
  birth: EngineBirth,
): Promise<Record<string, unknown>> {
  const date = `${birth.year}-${String(birth.month).padStart(2, '0')}-${String(birth.day).padStart(2, '0')}`;
  const url = type === 'ziwei'
    ? `/engine/ziwei?date=${encodeURIComponent(date)}&hour=${birth.hour}&gender=${encodeURIComponent(birth.gender ?? 'male')}&fixLeap=true`
    : `/engine/western?jdUtc=${encodeURIComponent(String(jdFromBirth(birth)))}&lat=${encodeURIComponent(String(birth.latitude ?? 25))}&lon=${encodeURIComponent(String(birth.longitude ?? 121.5))}`;
  // Service binding: host is ignored; path+query forwarded. Ensure single slash.
  const fullUrl = `https://ft-engine${url.startsWith('/') ? '' : '/'}${url}`;

  const res = await fetcher.fetch(fullUrl);
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`ft-engine ${type} failed: ${res.status} ${body.slice(0, 200)}`);
  }
  const data = await res.json() as { chart?: Record<string, unknown>; error?: string };
  if (!data.chart) throw new Error(`ft-engine ${type}: no chart in response ${JSON.stringify(data).slice(0, 200)}`);
  return data.chart;
}

// Julian Day (UT) from local civil date + hour + IANA timezone.
// Converts local birth time to UTC via Intl tz-offset reversal, then Fliegel–Van Flandern.
function jdFromBirth(birth: EngineBirth): number {
  const y = birth.year ?? 2000;
  const m = birth.month ?? 1;
  const d = birth.day ?? 1;
  const h = birth.hour ?? 12;
  const tz = birth.timezone ?? 'Asia/Taipei';

  // Build a UTC instant guessed as if the local wall-clock were UTC (padded hour 24→0).
  const localAsUtc = Date.UTC(y, m - 1, d, h === 24 ? 0 : h, 0, 0);
  // Ask Intl what time this instant has in `tz`, and measure the offset it reports.
  let offsetMs: number;
  try {
    const dtf = new Intl.DateTimeFormat('en-US', {
      timeZone: tz, year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
    });
    const parts = Object.fromEntries(dtf.formatToParts(new Date(localAsUtc)).map(p => [p.type, p.value]));
    const localHour = parts.hour === '24' ? 0 : Number(parts.hour);
    const localAsUtc2 = Date.UTC(Number(parts.year), Number(parts.month) - 1, Number(parts.day), localHour, Number(parts.minute), Number(parts.second));
    offsetMs = localAsUtc2 - localAsUtc;
  } catch {
    offsetMs = 0; // unknown tz — fall back to treating local wall-clock as UTC
  }

  const utcMs = localAsUtc - offsetMs;
  const u = new Date(utcMs);
  const yy = u.getUTCFullYear();
  const mm = u.getUTCMonth() + 1;
  const dd = u.getUTCDate();
  const hh = u.getUTCHours();
  // Fliegel–Van Flandern
  const a = Math.floor((14 - mm) / 12);
  const yyq = yy + 4800 - a;
  const mmq = mm + 12 * a - 3;
  const jdn = dd + Math.floor((153 * mmq + 2) / 5) + 365 * yyq + Math.floor(yyq / 4)
    - Math.floor(yyq / 100) + Math.floor(yyq / 400) - 32045;
  // Meeus: JD = JDN + (hh-12)/24. JDN is the integer day number at 00:00 UT.
  return jdn + (hh - 12) / 24;
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

// Read cached synthesis story (404 if none). Registered BEFORE /:type so Hono doesn't capture /story.
charts.get('/story', authMiddleware, setCacheHeaders({ maxAge: 86400, shared: false, mustRevalidate: true }), async (c) => {
  const { userId } = c.get('user');

  // Read-only: serve the cached story for the current birth_data_hash if one exists.
  // A story is only ever cached after a successful POST /story/generate (which requires
  // complete birth data), so a cache miss here always means "not generated yet" -> 404.
  const birth = await getUserBirthData(c.env.DB, userId);
  const row = await c.env.DB.prepare(
    `SELECT id, chart_data, ai_interpretation, updated_at FROM interpretations
     WHERE user_id = ? AND divination_type = 'story' AND birth_data_hash = ?`
  ).bind(userId, birth?.birth_data_hash ?? null).first<{ id: string; chart_data: string | null; ai_interpretation: string | null; updated_at: string }>();

  if (row && row.ai_interpretation && !isStoryChartCurrent(row.chart_data)) {
    return c.json({ error: 'Chart version stale, regeneration required', code: 'RECALC_REQUIRED' }, 409);
  }

  if (row && row.ai_interpretation) {
    const etag = createETag((birth?.birth_data_hash || '') + '-story', row.updated_at);
    const ifNoneMatch = c.req.header('if-none-match');
    if (ifNoneMatch && ifNoneMatch === etag) {
      return c.newResponse(null, { status: 304 });
    }
    c.res.headers.set('ETag', etag);
    return c.json({ story: row.ai_interpretation, fromCache: true });
  }

  return c.json({ error: 'No story yet. POST /story/generate first', code: 'NO_STORY' }, 404);
});

// Generate synthesis story. Registered BEFORE /:type/interpret.
charts.post('/story/generate', authMiddleware, async (c) => {
  const { userId } = c.get('user');
  const ip = c.req.header('cf-connecting-ip') || 'unknown';

  if (!checkAiRateLimit(ip)) {
    return c.json({ error: 'Too many requests', code: 'RATE_LIMIT' }, 429);
  }

  const birth = await getUserBirthData(c.env.DB, userId);
  if (!birth?.birth_year || !birth?.birth_month || !birth?.birth_day) {
    return c.json({ error: 'Birth data required', code: 'NO_BIRTH_DATA' }, 400);
  }
  if (!birth.gender) {
    return c.json({ error: 'Gender required', code: 'NO_GENDER' }, 400);
  }

  // Per-user cache short-circuit: never regenerate an existing story for this birth hash.
  // A story built by a superseded engine is not a cache hit — fall through and regenerate.
  const existing = await c.env.DB.prepare(
    `SELECT chart_data, ai_interpretation FROM interpretations
     WHERE user_id = ? AND divination_type = 'story' AND birth_data_hash = ?`
  ).bind(userId, birth.birth_data_hash).first<{ chart_data: string | null; ai_interpretation: string | null }>();
  if (existing?.ai_interpretation && isStoryChartCurrent(existing.chart_data)) {
    return c.json({ story: existing.ai_interpretation, fromCache: true });
  }

  // Compute both charts and merge (via Rust engine service binding)
  const hour = birth.birth_hour ?? 12;
  const ziwei = await fetchEngineChart(c.env.FT_ENGINE, 'ziwei', {
    year: birth.birth_year, month: birth.birth_month, day: birth.birth_day,
    hour, gender: birth.gender, latitude: null, longitude: null, timezone: birth.timezone,
  });
  const western = await fetchEngineChart(c.env.FT_ENGINE, 'western', {
    year: birth.birth_year, month: birth.birth_month, day: birth.birth_day,
    hour, gender: null, latitude: birth.latitude, longitude: birth.longitude, timezone: birth.timezone,
  });
  const merged = {
    ziwei,
    western,
    meta: {
      engineVersionZiwei: ENGINE_VERSION_ZIWEI,
      engineVersionWestern: ENGINE_VERSION_WESTERN,
      chartSchemaVersion: CHART_SCHEMA_VERSION,
    },
  };

  if (!c.env.IFLOW_API_KEY && !c.env.GROQ_API_KEY && !c.env.CEREBRAS_API_KEY) {
    return c.json({ error: 'AI service not configured' }, 503);
  }

  // Call AI via mutex
  const mutexId = c.env.AI_MUTEX.idFromName('global');
  const mutex = c.env.AI_MUTEX.get(mutexId);

  const response = await mutex.fetch('https://ai-mutex/interpret', {
    method: 'POST',
    body: JSON.stringify({
      keys: { iflow: c.env.IFLOW_API_KEY, groq: c.env.GROQ_API_KEY, cerebras: c.env.CEREBRAS_API_KEY },
      interpretRequest: { chartType: 'story', chartData: merged, language: 'zh' }
    })
  });

  if (!response.ok) {
    if (response.status === 503) {
      return c.json({ error: 'AI service temporarily unavailable, please try again', code: 'AI_UNAVAILABLE' }, 503);
    }
    const err = await response.json() as { error?: string };
    return c.json(err, response.status as 400 | 500);
  }

  const result = await response.json() as { interpretation: string; provider: string; model: string };
  if (!result.interpretation) {
    return c.json({ error: 'AI returned an empty story, please try again', code: 'EMPTY_STORY' }, 502);
  }

  // Upsert story
  const id = crypto.randomUUID();
  await c.env.DB.prepare(
    `INSERT INTO interpretations (id, user_id, divination_type, chart_data, ai_interpretation, birth_data_hash)
     VALUES (?, ?, 'story', ?, ?, ?)
     ON CONFLICT(user_id, divination_type) DO UPDATE SET
       chart_data = excluded.chart_data,
       ai_interpretation = excluded.ai_interpretation,
       birth_data_hash = excluded.birth_data_hash,
       updated_at = datetime('now')`
  ).bind(id, userId, JSON.stringify(merged), result.interpretation, birth.birth_data_hash).run();

  return c.json({
    story: result.interpretation,
    provider: result.provider,
    model: result.model,
    fromCache: false
  });
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
    chartData = await fetchEngineChart(c.env.FT_ENGINE, 'ziwei', {
      year: birth.birth_year, month: birth.birth_month, day: birth.birth_day,
      hour, gender: birth.gender, latitude: null, longitude: null, timezone: birth.timezone,
    });
  } else {
    chartData = await fetchEngineChart(c.env.FT_ENGINE, 'western', {
      year: birth.birth_year, month: birth.birth_month, day: birth.birth_day,
      hour, gender: null, latitude: birth.latitude, longitude: birth.longitude, timezone: birth.timezone,
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
