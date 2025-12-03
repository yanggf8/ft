/**
 * Auth Middleware
 * Validates session and attaches user to context
 */

import { Context, Next } from 'hono';
import type { Env } from '../index';

export interface AuthUser {
  userId: string;
  email: string;
}

// Extend Hono context with user
declare module 'hono' {
  interface ContextVariableMap {
    user: AuthUser;
  }
}

export async function authMiddleware(c: Context<{ Bindings: Env }>, next: Next) {
  const authHeader = c.req.header('Authorization');
  
  if (!authHeader?.startsWith('Bearer ')) {
    return c.json({ error: 'Missing authorization header' }, 401);
  }

  const sessionId = authHeader.replace('Bearer ', '');
  
  try {
    const doId = c.env.SESSION_DO.idFromName(sessionId);
    const stub = c.env.SESSION_DO.get(doId);
    const res = await stub.fetch(new Request('http://do/get'));
    
    if (!res.ok) {
      return c.json({ error: 'Invalid or expired session' }, 401);
    }
    
    const session = await res.json() as { userId: string; email: string };
    c.set('user', { userId: session.userId, email: session.email });
    
    await next();
  } catch {
    return c.json({ error: 'Authentication failed' }, 401);
  }
}

// Optional auth - doesn't fail if no token
export async function optionalAuth(c: Context<{ Bindings: Env }>, next: Next) {
  const authHeader = c.req.header('Authorization');
  
  if (authHeader?.startsWith('Bearer ')) {
    const sessionId = authHeader.replace('Bearer ', '');
    try {
      const doId = c.env.SESSION_DO.idFromName(sessionId);
      const stub = c.env.SESSION_DO.get(doId);
      const res = await stub.fetch(new Request('http://do/get'));
      
      if (res.ok) {
        const session = await res.json() as { userId: string; email: string };
        c.set('user', { userId: session.userId, email: session.email });
      }
    } catch {
      // Ignore auth errors for optional auth
    }
  }
  
  await next();
}
