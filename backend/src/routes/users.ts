import { Hono } from 'hono';
import type { Env } from '../index';
import { authMiddleware } from '../middleware/auth';
import { setCacheHeaders } from '../middleware/cache';
import { checkUserAccess } from '../services/billing';

const users = new Hono<{ Bindings: Env }>();

// Compute hash of birth data for cache invalidation (all fields that affect any chart)
function computeBirthHash(data: {
  birth_year?: number; birth_month?: number; birth_day?: number;
  birth_hour?: number; birth_minute?: number; gender?: string;
  timezone?: string; latitude?: number; longitude?: number;
}): string {
  const str = [
    data.birth_year, data.birth_month, data.birth_day,
    data.birth_hour ?? 12, data.birth_minute ?? 0, data.gender ?? '',
    data.timezone ?? 'Asia/Taipei', data.latitude ?? '', data.longitude ?? ''
  ].join('-');
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) - hash) + str.charCodeAt(i);
    hash |= 0;
  }
  return hash.toString(16);
}

interface UserRow {
  id: string;
  email: string;
  full_name: string | null;
  avatar_url: string | null;
  birth_year: number | null;
  birth_month: number | null;
  birth_day: number | null;
  birth_hour: number | null;
  birth_minute: number | null;
  gender: string | null;
  timezone: string | null;
  subscription_tier: string;
  trial_ends_at: string | null;
  created_at: string;
}

// Get current user profile
users.get('/me', authMiddleware, setCacheHeaders({ maxAge: 300, shared: false }), async (c) => {
  const { userId } = c.get('user');
  
  const user = await c.env.DB.prepare(
    `SELECT id, email, full_name, avatar_url, 
            birth_year, birth_month, birth_day, birth_hour, birth_minute, gender, timezone,
            subscription_tier, trial_ends_at, created_at 
     FROM users WHERE id = ?`
  ).bind(userId).first<UserRow>();
  
  if (!user) {
    return c.json({ error: 'User not found' }, 404);
  }

  const billing = checkUserAccess(user);
  const hasBirthData = !!(user.birth_year && user.birth_month && user.birth_day);
  
  return c.json({ 
    ...user, 
    billing,
    hasBirthData
  });
});

// Update birth data
users.put('/me/birth', authMiddleware, setCacheHeaders({ maxAge: 0 }), async (c) => {
  const { userId } = c.get('user');
  const { birth_year, birth_month, birth_day, birth_hour, birth_minute, gender, timezone, latitude, longitude } = await c.req.json();
  
  // Validation
  if (!birth_year || !birth_month || !birth_day) {
    return c.json({ error: 'birth_year, birth_month, birth_day required' }, 400);
  }
  if (birth_year < 1900 || birth_year > 2100) {
    return c.json({ error: 'birth_year must be 1900-2100' }, 400);
  }
  if (birth_month < 1 || birth_month > 12) {
    return c.json({ error: 'birth_month must be 1-12' }, 400);
  }
  if (birth_day < 1 || birth_day > 31) {
    return c.json({ error: 'birth_day must be 1-31' }, 400);
  }
  if (birth_hour !== undefined && birth_hour !== null && (birth_hour < 0 || birth_hour > 23)) {
    return c.json({ error: 'birth_hour must be 0-23' }, 400);
  }
  
  const hash = computeBirthHash({ birth_year, birth_month, birth_day, birth_hour, birth_minute, gender, timezone, latitude, longitude });

  // Update user birth data
  await c.env.DB.prepare(
    `UPDATE users SET
       birth_year = ?, birth_month = ?, birth_day = ?,
       birth_hour = ?, birth_minute = ?, gender = ?,
       timezone = ?, latitude = ?, longitude = ?,
       birth_data_hash = ?, updated_at = datetime('now')
     WHERE id = ?`
  ).bind(
    birth_year, birth_month, birth_day,
    birth_hour ?? null, birth_minute ?? null, gender ?? null,
    timezone ?? 'Asia/Taipei', latitude ?? null, longitude ?? null,
    hash, userId
  ).run();
  
  // Invalidate ALL cached interpretations when birth data changes
  await c.env.DB.prepare(
    'DELETE FROM interpretations WHERE user_id = ?'
  ).bind(userId).run();
  
  return c.json({ success: true, birth_data_hash: hash });
});

// Update profile (name, avatar)
users.put('/me', authMiddleware, setCacheHeaders({ maxAge: 0 }), async (c) => {
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
