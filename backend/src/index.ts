/**
 * FortuneT V2 API
 * Cloudflare Workers + Hono
 */

import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { SessionDO } from './durable-objects/session-do';
import { AIMutexDO } from './durable-objects/ai-mutex-do';
import auth from './routes/auth';
import users from './routes/users';
import charts from './routes/charts';

// Environment bindings
export interface Env {
  DB: D1Database;
  SESSION_DO: DurableObjectNamespace;
  AI_MUTEX: DurableObjectNamespace;
  STORAGE: R2Bucket;
  ENVIRONMENT: string;
  IFLOW_API_KEY?: string;
  GROQ_API_KEY?: string;
  CEREBRAS_API_KEY?: string;
}

const app = new Hono<{ Bindings: Env }>();

// Request ID for debugging
app.use('*', async (c, next) => {
  const reqId = crypto.randomUUID().slice(0, 8);
  c.res.headers.set('x-request-id', reqId);
  await next();
});

// CORS - tighten for production
app.use('*', cors({
  origin: (origin) => {
    // Allow localhost for dev, specific domains for prod
    if (!origin) return '*';
    if (origin.includes('localhost') || origin.includes('127.0.0.1')) return origin;
    if (origin.endsWith('.pages.dev') || origin.endsWith('.workers.dev')) return origin;
    return null;
  },
  credentials: true,
}));

// Health endpoints
app.get('/health', (c) => c.json({ status: 'ok', timestamp: new Date().toISOString(), environment: c.env.ENVIRONMENT }));
app.get('/health/db', async (c) => {
  try {
    const result = await c.env.DB.prepare('SELECT 1 as ok').first();
    return c.json({ status: 'ok', db: result });
  } catch (e) {
    return c.json({ status: 'error', error: String(e) }, 500);
  }
});

// API routes
app.route('/api/auth', auth);
app.route('/api/users', users);
app.route('/api/charts', charts);

// Root
app.get('/', (c) => c.json({ name: 'FortuneT V2 API', version: '1.0.0' }));

// 404
app.notFound((c) => c.json({ error: 'Not found' }, 404));

// Error handler
app.onError((err, c) => {
  console.error('Error:', err);
  return c.json({ error: 'Internal server error' }, 500);
});

export default app;
export { SessionDO, AIMutexDO };
