import { Hono } from 'hono';
import type { Env } from '../index';
import { authMiddleware } from '../middleware/auth';
import { ziWeiCalculator } from '../services/ziwei';
import { westernCalculator } from '../services/western';
import { GroqClient } from '../services/ai';

const charts = new Hono<{ Bindings: Env }>();

const ENGINE_VERSION = '1.0.0';

// Rate limit for calculation endpoints (30 req/min/IP)
const calcRateLimit = new Map<string, { count: number; reset: number }>();
const CALC_LIMIT = 30;
const CALC_WINDOW = 60000;

// Stricter rate limit for AI (10 req/min/IP)
const aiRateLimit = new Map<string, { count: number; reset: number }>();
const AI_LIMIT = 10;

function checkCalcRateLimit(ip: string): boolean {
  const now = Date.now();
  const entry = calcRateLimit.get(ip);
  if (!entry || now > entry.reset) {
    calcRateLimit.set(ip, { count: 1, reset: now + CALC_WINDOW });
    return true;
  }
  if (entry.count >= CALC_LIMIT) return false;
  entry.count++;
  return true;
}

function checkAiRateLimit(ip: string): boolean {
  const now = Date.now();
  const entry = aiRateLimit.get(ip);
  if (!entry || now > entry.reset) {
    aiRateLimit.set(ip, { count: 1, reset: now + CALC_WINDOW });
    return true;
  }
  if (entry.count >= AI_LIMIT) return false;
  entry.count++;
  return true;
}

// Calculate ZiWei chart (no auth required for calculation only)
charts.post('/calculate/ziwei', async (c) => {
  const ip = c.req.header('cf-connecting-ip') || 'unknown';
  if (!checkCalcRateLimit(ip)) {
    return c.json({ error: 'Too many requests', code: 'RATE_LIMIT' }, 429);
  }
  
  const { year, month, day, hour, gender } = await c.req.json();
  
  // Validation
  if (!year || !month || !day || hour === undefined || !gender) {
    return c.json({ error: 'Missing required fields', code: 'MISSING_FIELDS', details: 'Required: year, month, day, hour, gender' }, 400);
  }
  
  const y = Number(year), m = Number(month), d = Number(day), h = Number(hour);
  
  if (y < 1900 || y > 2100) {
    return c.json({ error: 'Year out of range', code: 'INVALID_YEAR', details: 'Supported range: 1900-2100' }, 400);
  }
  if (m < 1 || m > 12) {
    return c.json({ error: 'Invalid month', code: 'INVALID_MONTH', details: 'Month must be 1-12' }, 400);
  }
  if (d < 1 || d > 31) {
    return c.json({ error: 'Invalid day', code: 'INVALID_DAY', details: 'Day must be 1-31' }, 400);
  }
  if (h < 0 || h > 23) {
    return c.json({ error: 'Invalid hour', code: 'INVALID_HOUR', details: 'Hour must be 0-23' }, 400);
  }
  
  const genderNorm = String(gender).toLowerCase();
  if (!['male', 'female', 'm', 'f', '男', '女'].includes(genderNorm)) {
    return c.json({ error: 'Invalid gender', code: 'INVALID_GENDER', details: 'Gender must be male/female/m/f/男/女' }, 400);
  }
  
  try {
    const chart = ziWeiCalculator.calculate({
      year: y, month: m, day: d, hour: h,
      gender: ['female', 'f', '女'].includes(genderNorm) ? 'female' : 'male'
    });
    return c.json({ ...chart, engineVersion: ENGINE_VERSION });
  } catch (e) {
    return c.json({ error: 'Calculation failed', code: 'CALC_ERROR', details: String(e) }, 500);
  }
});

// Calculate Western zodiac chart
charts.post('/calculate/western', async (c) => {
  const ip = c.req.header('cf-connecting-ip') || 'unknown';
  if (!checkCalcRateLimit(ip)) {
    return c.json({ error: 'Too many requests', code: 'RATE_LIMIT' }, 429);
  }
  
  const { year, month, day, hour, minute, latitude, longitude } = await c.req.json();
  
  if (!year || !month || !day) {
    return c.json({ error: 'Missing required fields', code: 'MISSING_FIELDS', details: 'Required: year, month, day' }, 400);
  }
  
  const y = Number(year), m = Number(month), d = Number(day);
  
  if (y < 1900 || y > 2100) {
    return c.json({ error: 'Year out of range', code: 'INVALID_YEAR', details: 'Supported range: 1900-2100' }, 400);
  }
  if (m < 1 || m > 12) {
    return c.json({ error: 'Invalid month', code: 'INVALID_MONTH' }, 400);
  }
  if (d < 1 || d > 31) {
    return c.json({ error: 'Invalid day', code: 'INVALID_DAY' }, 400);
  }
  
  try {
    const chart = westernCalculator.calculate({
      year: y, month: m, day: d,
      hour: Number(hour || 12),
      minute: minute ? Number(minute) : undefined,
      latitude: latitude ? Number(latitude) : undefined,
      longitude: longitude ? Number(longitude) : undefined
    });
    return c.json({ ...chart, engineVersion: ENGINE_VERSION, note: 'Moon position is approximate (±2 signs)' });
  } catch (e) {
    return c.json({ error: 'Calculation failed', code: 'CALC_ERROR', details: String(e) }, 500);
  }
});

// AI Interpretation endpoint (requires auth for usage tracking)
charts.post('/interpret', authMiddleware, async (c) => {
  const ip = c.req.header('cf-connecting-ip') || 'unknown';
  if (!checkAiRateLimit(ip)) {
    return c.json({ error: 'Too many requests', code: 'RATE_LIMIT', details: 'AI limit: 10 req/min' }, 429);
  }
  
  if (!c.env.GROQ_API_KEY) {
    return c.json({ error: 'AI service not configured', code: 'SERVICE_UNAVAILABLE' }, 503);
  }
  
  const { chartType, chartData, language, focus } = await c.req.json();
  
  if (!chartType || !chartData) {
    return c.json({ error: 'Missing required fields', code: 'MISSING_FIELDS', details: 'Required: chartType, chartData' }, 400);
  }
  
  if (!['ziwei', 'western'].includes(chartType)) {
    return c.json({ error: 'Invalid chart type', code: 'INVALID_TYPE', details: 'Must be ziwei or western' }, 400);
  }
  
  try {
    const groq = new GroqClient({ apiKey: c.env.GROQ_API_KEY });
    const result = await groq.interpret({
      chartType,
      chartData,
      language: language || 'zh',
      focus
    });
    
    return c.json({
      interpretation: result.interpretation,
      model: result.model,
      tokensUsed: result.tokensUsed,
      engineVersion: ENGINE_VERSION
    });
  } catch (e) {
    console.error('AI interpretation error:', e);
    return c.json({ error: 'Interpretation failed', code: 'AI_ERROR', details: String(e) }, 500);
  }
});

// List user's charts
charts.get('/', authMiddleware, async (c) => {
  const { userId } = c.get('user');
  const limit = Math.min(Number(c.req.query('limit')) || 20, 100);
  const offset = Number(c.req.query('offset')) || 0;
  
  const results = await c.env.DB.prepare(
    'SELECT * FROM chart_records WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?'
  ).bind(userId, limit, offset).all();
  
  return c.json({ charts: results.results, limit, offset });
});

// Get single chart
charts.get('/:id', authMiddleware, async (c) => {
  const { userId } = c.get('user');
  const chartId = c.req.param('id');
  
  const chart = await c.env.DB.prepare(
    'SELECT * FROM chart_records WHERE id = ? AND user_id = ?'
  ).bind(chartId, userId).first();
  
  if (!chart) {
    return c.json({ error: 'Chart not found' }, 404);
  }
  
  return c.json(chart);
});

// Create chart
charts.post('/', authMiddleware, async (c) => {
  const { userId } = c.get('user');
  const { chart_type, chart_name, birth_data, chart_data } = await c.req.json();
  
  if (!chart_type || !chart_name || !birth_data || !chart_data) {
    return c.json({ error: 'Missing required fields' }, 400);
  }
  
  const id = crypto.randomUUID();
  
  await c.env.DB.prepare(
    'INSERT INTO chart_records (id, user_id, chart_type, chart_name, birth_data, chart_data) VALUES (?, ?, ?, ?, ?, ?)'
  ).bind(
    id, userId, chart_type, chart_name,
    JSON.stringify(birth_data),
    JSON.stringify(chart_data)
  ).run();
  
  const chart = await c.env.DB.prepare(
    'SELECT * FROM chart_records WHERE id = ?'
  ).bind(id).first();
  
  return c.json(chart, 201);
});

// Update chart
charts.put('/:id', authMiddleware, async (c) => {
  const { userId } = c.get('user');
  const chartId = c.req.param('id');
  const { chart_name, is_favorite } = await c.req.json();
  
  const existing = await c.env.DB.prepare(
    'SELECT id FROM chart_records WHERE id = ? AND user_id = ?'
  ).bind(chartId, userId).first();
  
  if (!existing) {
    return c.json({ error: 'Chart not found' }, 404);
  }
  
  await c.env.DB.prepare(
    "UPDATE chart_records SET chart_name = COALESCE(?, chart_name), is_favorite = COALESCE(?, is_favorite), updated_at = datetime('now') WHERE id = ?"
  ).bind(chart_name, is_favorite, chartId).run();
  
  const chart = await c.env.DB.prepare(
    'SELECT * FROM chart_records WHERE id = ?'
  ).bind(chartId).first();
  
  return c.json(chart);
});

// Delete chart
charts.delete('/:id', authMiddleware, async (c) => {
  const { userId } = c.get('user');
  const chartId = c.req.param('id');
  
  const result = await c.env.DB.prepare(
    'DELETE FROM chart_records WHERE id = ? AND user_id = ?'
  ).bind(chartId, userId).run();
  
  if (!result.meta.changes) {
    return c.json({ error: 'Chart not found' }, 404);
  }
  
  return c.json({ success: true });
});

export default charts;
