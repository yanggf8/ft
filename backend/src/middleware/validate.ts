import { Context, Next } from 'hono';
import { z, ZodSchema } from 'zod';

export function validate<T>(schema: ZodSchema<T>) {
  return async (c: Context, next: Next) => {
    try {
      const body = await c.req.json();
      const parsed = schema.parse(body);
      c.set('validated', parsed as { email?: string; full_name?: string; avatar_url?: string });
      await next();
    } catch (e) {
      if (e instanceof z.ZodError) {
        return c.json({ error: 'Validation failed', details: e.issues }, 400);
      }
      return c.json({ error: 'Invalid JSON' }, 400);
    }
  };
}

// Schemas
export const registerSchema = z.object({
  email: z.email().max(255),
  full_name: z.string().max(100).optional(),
});

export const loginSchema = z.object({
  email: z.email().max(255),
});

export const updateProfileSchema = z.object({
  full_name: z.string().max(100).optional(),
  avatar_url: z.url().max(500).optional(),
});
