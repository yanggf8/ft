import { Context, Next } from 'hono';
import { z, ZodSchema } from 'zod';

export function validate<T>(schema: ZodSchema<T>) {
  return async (c: Context, next: Next) => {
    try {
      const body = await c.req.json();
      const parsed = schema.parse(body);
      c.set('validated', parsed);
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

export const createChartSchema = z.object({
  chart_type: z.enum(['ziwei', 'western_natal', 'western_transit']),
  chart_name: z.string().min(1).max(100),
  birth_data: z.object({
    year: z.int().min(1900).max(2100),
    month: z.int().min(1).max(12),
    day: z.int().min(1).max(31),
    hour: z.int().min(0).max(23).optional(),
    gender: z.enum(['male', 'female']).optional(),
  }),
  chart_data: z.record(z.string(), z.unknown()),
});

export const updateChartSchema = z.object({
  chart_name: z.string().min(1).max(100).optional(),
  is_favorite: z.union([z.literal(0), z.literal(1)]).optional(),
});
