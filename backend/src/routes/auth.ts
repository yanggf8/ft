import { Hono } from 'hono';
import type { Env } from '../index';

const auth = new Hono<{ Bindings: Env }>();

// Simple in-memory rate limit (resets on cold start - good enough for auth abuse)
const rateLimitMap = new Map<string, { count: number; reset: number }>();
const RATE_LIMIT = 10; // requests per window
const WINDOW_MS = 60000; // 1 minute

function checkRateLimit(ip: string): boolean {
  const now = Date.now();
  const entry = rateLimitMap.get(ip);
  if (!entry || now > entry.reset) {
    rateLimitMap.set(ip, { count: 1, reset: now + WINDOW_MS });
    return true;
  }
  if (entry.count >= RATE_LIMIT) return false;
  entry.count++;
  return true;
}

// Rate limit middleware for auth
auth.use('*', async (c, next) => {
  const ip = c.req.header('cf-connecting-ip') || c.req.header('x-forwarded-for') || 'unknown';
  if (!checkRateLimit(ip)) {
    return c.json({ error: 'Too many requests' }, 429);
  }
  await next();
});

// Register new user
auth.post('/register', async (c) => {
  const { email, full_name } = await c.req.json();
  
  if (!email) {
    return c.json({ error: 'Email required' }, 400);
  }
  
  // Check if user exists
  const existing = await c.env.DB.prepare(
    'SELECT id FROM users WHERE email = ?'
  ).bind(email).first();
  
  if (existing) {
    return c.json({ error: 'User already exists' }, 409);
  }
  
  const userId = crypto.randomUUID();
  const trialEndsAt = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString(); // 30 days trial
  
  await c.env.DB.prepare(
    'INSERT INTO users (id, email, full_name, trial_ends_at) VALUES (?, ?, ?, ?)'
  ).bind(userId, email, full_name || null, trialEndsAt).run();
  
  // Create session
  const sessionId = crypto.randomUUID();
  const doId = c.env.SESSION_DO.idFromName(sessionId);
  const stub = c.env.SESSION_DO.get(doId);
  
  await stub.fetch(new Request('http://do/create', {
    method: 'POST',
    body: JSON.stringify({ userId, email }),
  }));
  
  return c.json({ sessionId, userId, email }, 201);
});

// Login existing user
auth.post('/login', async (c) => {
  const { email } = await c.req.json();
  
  if (!email) {
    return c.json({ error: 'Email required' }, 400);
  }
  
  const user = await c.env.DB.prepare(
    'SELECT id, email FROM users WHERE email = ?'
  ).bind(email).first<{ id: string; email: string }>();
  
  if (!user) {
    return c.json({ error: 'User not found' }, 404);
  }
  
  // Create session
  const sessionId = crypto.randomUUID();
  const doId = c.env.SESSION_DO.idFromName(sessionId);
  const stub = c.env.SESSION_DO.get(doId);
  
  await stub.fetch(new Request('http://do/create', {
    method: 'POST',
    body: JSON.stringify({ userId: user.id, email: user.email }),
  }));
  
  return c.json({ sessionId, userId: user.id, email: user.email });
});

// Logout
auth.post('/logout', async (c) => {
  const authHeader = c.req.header('Authorization');
  
  if (!authHeader?.startsWith('Bearer ')) {
    return c.json({ success: true });
  }
  
  const sessionId = authHeader.replace('Bearer ', '');
  const doId = c.env.SESSION_DO.idFromName(sessionId);
  const stub = c.env.SESSION_DO.get(doId);
  
  await stub.fetch(new Request('http://do/destroy', { method: 'POST' }));
  
  return c.json({ success: true });
});

export default auth;
