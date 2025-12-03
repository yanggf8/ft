import { Hono } from 'hono';
import type { Env } from '../index';
import { authMiddleware } from '../middleware/auth';

const users = new Hono<{ Bindings: Env }>();

// Get current user profile
users.get('/me', authMiddleware, async (c) => {
  const { userId } = c.get('user');
  
  const user = await c.env.DB.prepare(
    'SELECT id, email, full_name, avatar_url, subscription_tier, created_at FROM users WHERE id = ?'
  ).bind(userId).first();
  
  if (!user) {
    return c.json({ error: 'User not found' }, 404);
  }
  
  return c.json(user);
});

// Update current user profile
users.put('/me', authMiddleware, async (c) => {
  const { userId } = c.get('user');
  const { full_name, avatar_url } = await c.req.json();
  
  await c.env.DB.prepare(
    "UPDATE users SET full_name = ?, avatar_url = ?, updated_at = datetime('now') WHERE id = ?"
  ).bind(full_name || null, avatar_url || null, userId).run();
  
  const user = await c.env.DB.prepare(
    'SELECT id, email, full_name, avatar_url, subscription_tier, created_at FROM users WHERE id = ?'
  ).bind(userId).first();
  
  return c.json(user);
});

export default users;
