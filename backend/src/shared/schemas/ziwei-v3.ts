import { z } from 'zod';

export const ZiWeiStarV3Schema = z.object({
  name: z.string(),
  type: z.enum(['main', 'auxiliary', 'transformation']),
  brightness: z.string().optional(),
  sihua: z.enum(['lu', 'quan', 'ke', 'ji']).optional(),
});

export const ZiWeiPalaceV3Schema = z.object({
  index: z.number().int().min(0).max(11),
  name: z.string(),
  branch: z.string(),
  stem: z.string(),
  stars: z.array(ZiWeiStarV3Schema),
  isLifePalace: z.boolean().optional(),
  isBodyPalace: z.boolean().optional(),
});

export const ZiWeiFourPillarsSchema = z.object({
  year: z.object({ stem: z.string(), branch: z.string() }),
  month: z.object({ stem: z.string(), branch: z.string() }),
  day: z.object({ stem: z.string(), branch: z.string() }),
  hour: z.object({ stem: z.string(), branch: z.string() }),
});

export const ZiWeiMetaSchema = z.object({
  dayDivide: z.enum(['forward', 'current']),
  isLeap: z.boolean(),
  fixLeap: z.boolean(),
  timeIndex: z.number().int().min(0).max(12),
  hourShifted: z.boolean().optional(),
  assumed: z.boolean().optional(),
  engineVersionZiwei: z.string(),
  chartSchemaVersion: z.number(),
});

export const ZiWeiChartV3Schema = z.object({
  birthInfo: z.object({
    solar: z.object({ year: z.number(), month: z.number(), day: z.number() }),
    lunar: z.object({ year: z.number(), month: z.number(), day: z.number(), isLeap: z.boolean().optional() }),
    hour: z.number(),
    hourBranch: z.string(),
    gender: z.string(),
  }),
  fourPillars: ZiWeiFourPillarsSchema,
  fiveElement: z.string(),
  lifePalaceIndex: z.number().int().min(0).max(11),
  bodyPalaceIndex: z.number().int().min(0).max(11),
  palaces: z.array(ZiWeiPalaceV3Schema).length(12),
  majorLimits: z.array(z.object({ startAge: z.number(), endAge: z.number(), stem: z.string(), branch: z.string(), palaceIndex: z.number() })),
  meta: ZiWeiMetaSchema,
});

export const ZiWeiV3ResponseSchema = z.object({
  id: z.string(),
  user_id: z.string(),
  divination_type: z.literal('ziwei'),
  chart_data: ZiWeiChartV3Schema,
  chartSchemaVersion: z.literal(3),
  engineVersion: z.string(),
  ai_interpretation: z.string().nullable(),
  birth_data_hash: z.string().nullable(),
  fromCache: z.boolean(),
});

export type ZiWeiChartV3 = z.infer<typeof ZiWeiChartV3Schema>;
export type ZiWeiPalaceV3 = z.infer<typeof ZiWeiPalaceV3Schema>;
export type ZiWeiStarV3 = z.infer<typeof ZiWeiStarV3Schema>;
export type ZiWeiMeta = z.infer<typeof ZiWeiMetaSchema>;
export type MajorLimit = ZiWeiChartV3['majorLimits'][number];
export type ZiWeiV3Response = z.infer<typeof ZiWeiV3ResponseSchema>;
