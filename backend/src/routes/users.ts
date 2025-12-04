import { Hono } from 'hono';
import type { Env } from '../index';
import { authMiddleware } from '../middleware/auth';
import { checkUserAccess } from '../services/billing';

const users = new Hono<{ Bindings: Env }>();

// Get current user profile
users.get('/me', authMiddleware, async (c) => {
  const { userId } = c.get('user');
  
  const user = await c.env.DB.prepare(
    'SELECT id, email, full_name, avatar_url, subscription_tier, trial_ends_at, created_at FROM users WHERE id = ?'
  ).bind(userId).first<{ id: string; email: string; full_name: string | null; avatar_url: string | null; subscription_tier: string; trial_ends_at: string | null; created_at: string }>();
  
  if (!user) {
    return c.json({ error: 'User not found' }, 404);
  }

  const billing = checkUserAccess(user);
  
  return c.json({ ...user, billing });
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
