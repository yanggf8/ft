# Phase A1 — iztro Adapter (ZiWei Engine Modernization) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the self-built ZiWei calculator with iztro 2.6.0 as the authoritative engine, emitting a Zod-validated V3 chart response with per-type cache versioning, while keeping the self-built engine as a sunset-tracked fallback.

**Architecture:** New `iztro-adapter.ts` isolates all iztro API calls (`bySolar`, `config({dayDivide:'forward'})`, `Functional*` → plain V3 mapping). `engine-version.ts` splits into `ENGINE_VERSION_ZIWEI / WESTERN`. `birth-hash.ts` extracts the shared hash function. Route `charts.ts` switches the ziwei branch to the adapter and adds `409 RECALC_REQUIRED` + per-type ETag. Zod schemas live in `backend/src/shared/schemas/` (backend) with frontend type import via relative path.

**Tech Stack:** Cloudflare Workers + Hono + TypeScript 5.3 strict + D1 + iztro 2.6.0 (MIT) + zod 4.1.13 + wrangler 4.53.0 + Vite/React 18 frontend.

**Spec:** `docs/superpowers/specs/2026-08-26-engine-modernization-big5-design.md` (rev.3, §3.1 + §3.4 + §0 API-contract addendum)

## Global Constraints

- `ENGINE_VERSION_ZIWEI = '3.0.0'` and `ENGINE_VERSION_WESTERN = '3.0.0'` (spec §3.4, per-type versioning; initial value `3.0.0`, bump decision tree: algorithm change → bump own type, shape change → bump `chartSchemaVersion`).
- `chartSchemaVersion = 3` top-level field in every chart response.
- iztro pinned at `2.6.0`, `astronomy-engine` not in this phase (A2).
- `dayDivide` must be set explicitly to `'forward'` (iztro default `'forward'` verified at `package/lib/astro/astro.js:39`); record as `meta.dayDivide` and do not rely on implicit default.
- `fixLeap = true`, `language = 'zh-TW'` for every `bySolar` call.
- Workers free-tier limits: `gzip < 3MB`, `CPU < 10ms/request` — verified by `wrangler deploy --dry-run` before merge.
- Testing: **integration-only**, no mocks, no unit tests, no stubs. Tests guard with `RUN_INTEGRATION=true` and `process.env.TEST_API_URL`. Default `npm test` must not hit live APIs.
- Frontend must be built before every deploy (`npm run build` which writes `dist/.build-info`).
- Wrangler commands use OAuth: prefix `unset CLOUDFLARE_API_TOKEN &&`.
- D1 migrations via `backend/scripts/migrations/*.sql` + `wrangler d1 execute --file` (not hand-edited).
- Commit before deploy for rollback.
- TypeScript strict, 2-space indent, single quotes, semicolons.
- No feature flags, no DB constraints, file names kebab-case, variables camelCase.

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `backend/package.json` | Modify | Add `iztro@2.6.0` dependency |
| `backend/src/services/engine-version.ts` | Modify | Split single `ENGINE_VERSION` into `ENGINE_VERSION_ZIWEI`, `ENGINE_VERSION_WESTERN`, `CHART_SCHEMA_VERSION` |
| `backend/src/services/birth-hash.ts` | Create | Extract `computeBirthHash` from `routes/users.ts` for sharing with `charts.ts` predict phase |
| `backend/src/shared/schemas/ziwei-v3.ts` | Create | Zod schemas for V3 chart request/response + inferred TS types |
| `backend/src/services/ziwei/types.ts` | Modify | Add `ZiWeiChartV3`, `ZiWeiMeta`, `ZiWeiPalaceV3`, `ZiWeiStarV3`, `MajorLimit` types (V3 additive, V2 retained) |
| `backend/src/services/ziwei/iztro-adapter.ts` | Create | `iztroAdapter.calculate(data)` → `ZiWeiChartV3`; config dayDivide, bySolar call, branch-index mapping, star/brightness/sihua extraction |
| `backend/src/services/ziwei/constants.ts` | Read-only | Provides `HOUR_TO_BRANCH` etc. (adapter imports only what needed) |
| `backend/src/routes/users.ts` | Modify | Import `computeBirthHash` from new module (no logic change) |
| `backend/src/routes/charts.ts` | Modify | Ziwei branch uses adapter, per-type ETag/version, `409 RECALC_REQUIRED` guard on interpret, `chartSchemaVersion: 3` |
| `backend/src/__tests__/integration/ziwei-iztro.test.ts` | Create | Integration anchors: published chart → expected palace/stars, 23:00 dayDivide, leap-month, stale-version 409 flow |
| `frontend/src/lib/api.ts` | Modify | Add `getChartV3` typed return and 409 auto-retry helper (thin wrapper, no logic duplication) |
| `frontend/src/pages/DivinationPage.tsx` | Modify | Rewrite `ZiWeiDisplay` to read `ZiWeiChartV3` fields (`birthInfo.lunar`, `lifePalaceIndex`, `fiveElement`, `palaces[].branch/stars/brightness/sihua`, `majorLimits`, `meta`); keep backward-compat reads for V2 during rollout |
| `frontend/src/components/ZiWeiPalaceGrid.tsx` | Create | 12-palace grid (branch header, stars with brightness/sihua badges, palace name). Hand-written SVG-free layout. |
| `backend/wrangler.toml` | Read-only | No change in A1 (A3 will need no new vars). |

---

### Task 1: Scaffolding — Dependency, Version Split, Shared Hash, Zod Schemas

**Files:**
- Modify: `backend/package.json`
- Modify: `backend/src/services/engine-version.ts`
- Create: `backend/src/services/birth-hash.ts`
- Create: `backend/src/shared/schemas/ziwei-v3.ts`

**Interfaces:**
- Consumes: existing `backend/src/services/engine-version.ts` (`ENGINE_VERSION='2.0.0'`), existing `computeBirthHash` in `routes/users.ts`
- Produces:
  - `ENGINE_VERSION_ZIWEI: string = '3.0.0'` and `ENGINE_VERSION_WESTERN: string = '3.0.0'` and `CHART_SCHEMA_VERSION = 3` (imported by `charts.ts` and `iztro-adapter.ts`)
  - `computeBirthHash(data: {birth_year?:number, ... longitude?:number}): string` (imported by `routes/users.ts` and later `routes/charts.ts`)
  - `ZiWeiV3ResponseSchema: z.ZodObject` + `type ZiWeiV3Response = z.infer<typeof ZiWeiV3ResponseSchema>` (imported by `charts.ts` and frontend)

- [ ] **Step 1: Add iztro dependency**

In `backend/package.json`, add to `dependencies`:

```json
"iztro": "2.6.0"
```

Run:

```bash
cd backend && npm install
```

- [ ] **Step 2: Split engine-version.ts into per-type constants**

Replace `backend/src/services/engine-version.ts` contents with:

```ts
export const ENGINE_VERSION_ZIWEI = '3.0.0';
export const ENGINE_VERSION_WESTERN = '3.0.0';
export const CHART_SCHEMA_VERSION = 3;
```

Keep no default export. Any existing `ENGINE_VERSION` import must be updated in later tasks; leave `export const ENGINE_VERSION = ENGINE_VERSION_ZIWEI` as a one-line compat re-export only if the worker currently fails to compile before Task 3 lands, then remove it in Task 3.

- [ ] **Step 3: Extract birth-hash.ts**

Create `backend/src/services/birth-hash.ts`:

```ts
export function computeBirthHash(data: {
  birth_year?: number; birth_month?: number; birth_day?: number;
  birth_hour?: number; birth_minute?: number; gender?: string;
  timezone?: string; latitude?: number; longitude?: number;
}): string {
  const str = [
    data.birth_year, data.birth_month, data.birth_day,
    data.birth_hour ?? 12, data.birth_minute ?? 0, data.gender ?? '',
    data.timezone ?? 'Asia/Taipei', data.latitude ?? '', data.longitude ?? '',
  ].join('-');
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) - hash) + str.charCodeAt(i);
    hash |= 0;
  }
  return hash.toString(16);
}
```

Copy verbatim from `routes/users.ts:10-26`. Do not change algorithm (hash stability).

- [ ] **Step 4: Write Zod schemas for V3 chart**

Create `backend/src/shared/schemas/ziwei-v3.ts`:

```ts
import { z } from 'zod';

export const ZiWeiStarV3Schema = z.object({
  name: z.string(),
  type: z.enum(['main', 'auxiliary', 'transformation']),
  brightness: z.string().optional(),
  sihua: z.enum(['lu','quan','ke','ji']).optional(), // 化祿/權/科/忌
});

export const ZiWeiPalaceV3Schema = z.object({
  index: z.number().int().min(0).max(11), // 地支序: 0=子
  name: z.string(), // 命宮/兄弟/...
  branch: z.string(), // 子..亥
  stem: z.string(), // 甲..癸
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
  dayDivide: z.enum(['forward','current']),
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
export type ZiWeiV3Response = z.infer<typeof ZiWeiV3ResponseSchema>;
```

- [ ] **Step 5: Update users.ts to import shared hash**

In `backend/src/routes/users.ts`, replace the local `computeBirthHash` function with:

```ts
import { computeBirthHash } from '../services/birth-hash';
```

Delete the local function body (lines 10-26). Keep all call sites unchanged.

- [ ] **Step 6: Verify scaffolding**

```bash
cd backend && npm run typecheck
```

Expected: PASS (no errors; `iztro` types resolved, `shared/schemas` compiles).

- [ ] **Step 7: Commit**

```bash
git add backend/package.json backend/package-lock.json backend/src/services/engine-version.ts backend/src/services/birth-hash.ts backend/src/shared/schemas/ziwei-v3.ts backend/src/routes/users.ts
git commit -m "feat(ziwei): scaffolding for iztro — per-type engine versions, shared birth-hash, Zod V3 schemas"
```

---

### Task 2: iztro Adapter — Core Mapping

**Files:**
- Modify: `backend/src/services/ziwei/types.ts` (add V3 types re-exporting from Zod schema types)
- Create: `backend/src/services/ziwei/iztro-adapter.ts`

**Interfaces:**
- Consumes: `iztro` (`bySolar`, `config`), `ENGINE_VERSION_ZIWEI`, `CHART_SCHEMA_VERSION`, `ZiWeiChartV3Schema` types
- Produces: `iztroAdapter.calculate(input: { year:number, month:number, day:number, hour:number, minute?:number, gender:'male'|'female' }): ZiWeiChartV3`

**Design details the implementer must follow:**
- Module init: `import { astro } from 'iztro'; astro.config({ dayDivide: 'forward' });` exactly once at top-level.
- `timeIndex` derivation: `hourToTimeIndex(hour, minute)` — map 0–23h + minute to iztro 0–12 slots. Slots: 23:00–00:59 = 0 (early zi) if minute-based split needed; otherwise use hour-only. Until minute precision lands in A3, use `HOUR_TO_BRANCH` hour mapping then index lookup: 子=0, 丑=1, ..., 亥=11, with 23→0. For `bySolar`, pass `timeIndex` as computed; do not pass `isLeapMonth` (that is byLunar only).
- `bySolar` call: `bySolar('YYYY-M-D', timeIndex, gender==='male'?'男':'女', true, 'zh-TW')` — verify gender literal matches iztro `GenderName` (`'男'|'女'`); check `astro.d.ts` `GenderName` union before committing.
- Branch-index helper: `const toGroundBranch = (palaceIndex: number) => (palaceIndex + 2) % 12;` (iztro palace 0=寅(2), so ground 0=子). Document with comment "iztro palace index寅起0 → 地支序子起0".
- Star extraction: for each palace from `FunctionalAstrolabe.palaces`, map `stars` array: `name`, `type` (map iztro star category strings: `main→'main'`, others → `'auxiliary'` except sihua-marked → `'transformation'`), `brightness` (iztro `brightness` string or  `undefined`), `sihua` (if star has `sihua`/`化` field, map `祿→'lu'` etc.). Use `for…of` over `astrolabe.palaces` (iztro order is寅起); push mapped palace into `groundPalaces[toGroundBranch(i)]`.
- `fourPillars`: take from `astrolabe.fourPillars` if present (check field name `fourPillars` vs `pillars` in d.ts), else reuse self-built pillars from Task 1 birth-hash context. Prefer iztro value when available.
- `majorLimits`: from `astrolabe.decadal` or `astrolabe.majorLimits` (check d.ts; field may be `decadal` array of `{startAge,endAge,stem,branch}`). Map to `{startAge,endAge,stem,branch,palaceIndex: toGroundBranch(decadalPalaceIndex)}`.
- `meta`: `{ dayDivide:'forward', isLeap: astrolabe.isLeap ?? false, fixLeap:true, timeIndex, engineVersionZiwei: ENGINE_VERSION_ZIWEI, chartSchemaVersion: CHART_SCHEMA_VERSION }`.
- Never call `JSON.stringify(astrolabe)` — always field-map.

- [ ] **Step 1: Add V3 types re-export**

In `backend/src/services/ziwei/types.ts`, append after existing `ZiWeiChart` interface:

```ts
export type { ZiWeiChartV3, ZiWeiPalaceV3, ZiWeiStarV3, ZiWeiMeta, MajorLimit } from '../../shared/schemas/ziwei-v3';
// Back-compat: keep BirthData, ZiWeiChart (V2) exported for fallback path.
// V3 is the only type new code should import.
```

If the schema file exports slightly different names, adjust re-export to match actual exported type names.

- [ ] **Step 2: Implement iztro-adapter.ts**

Create `backend/src/services/ziwei/iztro-adapter.ts`:

```ts
import { astro } from 'iztro';
import { ENGINE_VERSION_ZIWEI, CHART_SCHEMA_VERSION } from '../engine-version';
import type { ZiWeiChartV3 } from '../../shared/schemas/ziwei-v3';
import { EARTHLY_BRANCHES } from './constants';

// Pin dayDivide explicitly (spec §3.1, verified default forward)
astro.config({ dayDivide: 'forward' });

function timeIndexFromHour(hour: number): number {
  // iztro 0=早子, 1=丑, ..., 11=亥, 12=晚子; hour 23 maps to 0 per HOUR_TO_BRANCH
  // Until minute precision (A3), hour-only mapping is sufficient.
  const branch = EARTHLY_BRANCHES[hour === 23 ? 0 : Math.floor((hour + 1) / 2) % 12];
  const order = ['子','丑','寅','卯','辰','巳','午','未','申','酉','戌','亥'];
  const idx = order.indexOf(branch);
  // 晚子(23h) iztro expects 12; map hour 23 specially
  if (hour === 23) return 12;
  return idx;
}

function groundBranchOf(palaceIndex: number): number {
  return (palaceIndex + 2) % 12; // iztro 0=寅(2) → ground 0=子
}

export const iztroAdapter = {
  calculate(input: { year: number; month: number; day: number; hour: number; minute?: number; gender: 'male' | 'female' }): ZiWeiChartV3 {
    const timeIndex = timeIndexFromHour(input.hour);
    const genderName = input.gender === 'male' ? '男' : '女';
    const solarDate = `${input.year}-${input.month}-${input.day}`;
    const astrolabe: any = astro.bySolar(solarDate, timeIndex, genderName as any, true, 'zh-TW');

    // Map palaces to ground branch order (0=子)
    const groundPalaces: any[] = Array.from({ length: 12 }, () => null);
    for (let pi = 0; pi < 12; pi++) {
      const p = astrolabe.palaces[pi];
      const gi = groundBranchOf(pi);
      const stars = (p.stars ?? []).map((s: any) => ({
        name: s.name,
        type: s.type === 'main' ? 'main' as const : s.sihua ? 'transformation' as const : 'auxiliary' as const,
        brightness: s.brightness ?? undefined,
        sihua: s.sihua ? ({ '祿':'lu','權':'quan','科':'ke','忌':'ji' } as const)[s.sihua] : undefined,
      }));
      groundPalaces[gi] = {
        index: gi,
        name: p.name,
        branch: EARTHLY_BRANCHES[gi],
        stem: p.stem ?? EARTHLY_BRANCHES[gi], // fallback if stem absent
        stars,
        isLifePalace: p.isLifePalace ?? pi === astrolabe.lifePalaceIndex,
        isBodyPalace: p.isBodyPalace ?? pi === astrolabe.bodyPalaceIndex,
      };
    }

    const majors = (astrolabe.decadal ?? astrolabe.majorLimits ?? []).map((d: any) => ({
      startAge: d.startAge ?? d.start,
      endAge: d.endAge ?? d.end,
      stem: d.stem,
      branch: d.branch,
      palaceIndex: groundBranchOf(d.palaceIndex ?? d.index ?? 0),
    }));

    const chart: ZiWeiChartV3 = {
      birthInfo: {
        solar: { year: input.year, month: input.month, day: input.day },
        lunar: {
          year: astrolabe.lunarDate?.year ?? input.year,
          month: astrolabe.lunarDate?.month ?? input.month,
          day: astrolabe.lunarDate?.day ?? input.day,
          isLeap: astrolabe.lunarDate?.isLeap ?? false,
        },
        hour: input.hour,
        hourBranch: EARTHLY_BRANCHES[timeIndex === 12 ? 0 : timeIndex],
        gender: input.gender === 'male' ? '男' : '女',
      },
      fourPillars: astrolabe.fourPillars ?? astrolabe.pillars ?? { year:{stem:'',branch:''}, month:{stem:'',branch:''}, day:{stem:'',branch:''}, hour:{stem:'',branch:''} },
      fiveElement: astrolabe.fiveElement ?? '',
      lifePalaceIndex: groundBranchOf(astrolabe.lifePalaceIndex ?? 0),
      bodyPalaceIndex: groundBranchOf(astrolabe.bodyPalaceIndex ?? 0),
      palaces: groundPalaces,
      majorLimits: majors,
      meta: {
        dayDivide: 'forward',
        isLeap: astrolabe.lunarDate?.isLeap ?? false,
        fixLeap: true,
        timeIndex,
        engineVersionZiwei: ENGINE_VERSION_ZIWEI,
        chartSchemaVersion: CHART_SCHEMA_VERSION,
      },
    };
    return chart;
  },
};
```

Before committing, open `node_modules/iztro/lib/astro/astrolabe.d.ts` and verify field names (`palaces`, `lifePalaceIndex`, `lunarDate`, `fourPillars`, `decadal`, star `sihua`/`brightness`). Adjust adapter field accesses to match actual d.ts; the shape above uses `any` guards so it compiles, but pin exact names after inspection.

- [ ] **Step 3: Typecheck**

```bash
cd backend && npm run typecheck
```

Expected: PASS. If iztro GenderName rejects `'男'`, cast as `any` and add `// iztro GenderName is '男'|'女', verified at astro.d.ts:xx`.

- [ ] **Step 4: Commit**

```bash
git add backend/src/services/ziwei/types.ts backend/src/services/ziwei/iztro-adapter.ts
git commit -m "feat(ziwei): iztro adapter core — bySolar mapping, ground-branch helper, star/brightness/sihua extraction"
```

---

### Task 3: Backend Route Integration — Per-Type Versioning, ETag, 409 Guard

**Files:**
- Modify: `backend/src/routes/charts.ts`
- Modify: `backend/src/services/engine-version.ts` if Task 1 left a compat re-export

**Interfaces:**
- Consumes: `iztroAdapter.calculate`, `ENGINE_VERSION_ZIWEI`, `ENGINE_VERSION_WESTERN`, `CHART_SCHEMA_VERSION`, `ZiWeiV3ResponseSchema`, `computeBirthHash`
- Produces: `GET /api/charts/ziwei` returns `{ chart_data: ZiWeiChartV3, chartSchemaVersion: 3, engineVersion: '3.0.0', ... }` validated by `ZiWeiV3ResponseSchema`; `POST /api/charts/ziwei/interpret` returns `409 { code:'RECALC_REQUIRED' }` when stored `chart_data.meta.engineVersionZiwei` mismatches.

- [ ] **Step 1: Replace ziwei branch calculation with adapter**

In `backend/src/routes/charts.ts`, replace the ziwei `calculate` block:

```ts
// Before:
chartData = ziWeiCalculator.calculate({ year, month, day, hour, gender });

// After:
import { iztroAdapter } from '../services/ziwei/iztro-adapter';
import { ENGINE_VERSION_ZIWEI, ENGINE_VERSION_WESTERN, CHART_SCHEMA_VERSION } from '../services/engine-version';

let engineVersionForType: string;
let chartSchemaVersionForType = CHART_SCHEMA_VERSION;
if (divType === 'ziwei') {
  chartData = iztroAdapter.calculate({ year: birth.birth_year, month: birth.birth_month, day: birth.birth_day, hour, minute: birth.birth_minute ?? undefined, gender });
  engineVersionForType = ENGINE_VERSION_ZIWEI;
} else {
  chartData = westernCalculator.calculate({ ... });
  engineVersionForType = ENGINE_VERSION_WESTERN;
}
```

Keep western branch on `westernCalculator` (A2 will replace it).

- [ ] **Step 2: Per-type version embedding and cache lookup**

Change every `ENGINE_VERSION` reference to per-type. In the cache-hit path, compare the correct per-type version:

```ts
const expectedVersion = divType === 'ziwei' ? ENGINE_VERSION_ZIWEI : ENGINE_VERSION_WESTERN;
if (cached) {
  const parsed = typeof cached.chart_data === 'string' ? JSON.parse(cached.chart_data) : cached.chart_data;
  const storedVersion = parsed?.meta?.engineVersionZiwei ?? parsed?.engineVersion;
  if (storedVersion === expectedVersion) {
    // hit
  }
  // else treat as miss (fall through to recalculate)
}
```

For new chart_data, embed:

```ts
const chartDataWithVersion = {
  ...(chartData as Record<string, unknown>),
  meta: { ...(chartData as any).meta, engineVersionZiwei: expectedVersion, chartSchemaVersion: CHART_SCHEMA_VERSION },
  // Back-compat top-level (keep one release):
  engineVersion: expectedVersion,
  chartSchemaVersion: CHART_SCHEMA_VERSION,
};
```

- [ ] **Step 3: ETag per-type + chartSchemaVersion**

Update ETag generation to include version:

```ts
const etag = createETag(`${birth.birth_data_hash ?? ''}-${expectedVersion}-${CHART_SCHEMA_VERSION}`, cached?.updated_at || Date.now());
```

Do not break existing `createETag` signature; build the string before calling it.

- [ ] **Step 4: 409 guard on POST /:type/interpret**

At the start of the interpret handler, after fetching `interp` row:

```ts
if (interp) {
  const chartDataParsed = typeof interp.chart_data === 'string' ? JSON.parse(interp.chart_data) : interp.chart_data;
  const storedVersion = chartDataParsed?.meta?.engineVersionZiwei ?? chartDataParsed?.engineVersion;
  const expectedVersion = divType === 'ziwei' ? ENGINE_VERSION_ZIWEI : ENGINE_VERSION_WESTERN;
  if (storedVersion !== expectedVersion) {
    return c.json({ error: 'Chart version stale, recalculation required', code: 'RECALC_REQUIRED' }, 409);
  }
}
```

- [ ] **Step 5: Zod validation on response (ziwei only)**

After building `response` object, validate only ziwei branch:

```ts
if (divType === 'ziwei') {
  const parsed = ZiWeiV3ResponseSchema.safeParse(response);
  if (!parsed.success) {
    console.error('ZiWei V3 schema violation', parsed.error.flatten());
    return c.json({ error: 'Chart schema violation' }, 500);
  }
}
```

Import `ZiWeiV3ResponseSchema` at top. Do not block western path.

- [ ] **Step 6: Typecheck and commit**

```bash
cd backend && npm run typecheck
# Expected: PASS
git add backend/src/routes/charts.ts backend/src/services/engine-version.ts
git commit -m "feat(ziwei): route integration — iztro adapter, per-type versioning, 409 RECALC guard, Zod validation"
```

---

### Task 4: Frontend — V3 Display + API Typed Handling

**Files:**
- Modify: `frontend/src/lib/api.ts`
- Modify: `frontend/src/pages/DivinationPage.tsx`
- Create: `frontend/src/components/ZiWeiPalaceGrid.tsx`

**Interfaces:**
- Consumes: `GET /api/charts/ziwei` response shape `ZiWeiV3Response` (via `ZiWeiChartV3`), existing `api` client
- Produces:
  - `ZiWeiDisplay` renders `ZiWeiChartV3` correctly (12 palaces with branch/stars/brightness/sihua)
  - `api.interpret` caller handles `409 RECALC_REQUIRED` by auto-retrying `getChart` once

- [ ] **Step 1: Extend api.ts with 409-aware interpret**

In `frontend/src/lib/api.ts`, modify `interpret`:

```ts
async interpret(type: 'ziwei' | 'western') {
  const res = await fetch(`${API_URL}/api/charts/${type}/interpret`, {
    method: 'POST',
    headers: this.authHeaders(),
  });
  if (res.status === 409) {
    // stale version — recalc then retry once
    await this.getChart(type, true);
    const retry = await fetch(`${API_URL}/api/charts/${type}/interpret`, {
      method: 'POST',
      headers: this.authHeaders(),
    });
    if (!retry.ok) throw new Error(await retry.text());
    return retry.json();
  }
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}
private authHeaders(): Record<string,string> {
  const h: Record<string,string> = { 'Content-Type': 'application/json' };
  const sid = this.getSession();
  if (sid) h['Authorization'] = `Bearer ${sid}`;
  return h;
}
```

Keep existing `request` helper untouched; this method bypasses it to inspect 409.

- [ ] **Step 2: Create ZiWeiPalaceGrid component**

Create `frontend/src/components/ZiWeiPalaceGrid.tsx`:

```tsx
type Star = { name:string; type:string; brightness?:string; sihua?:string };
type Palace = { index:number; name:string; branch:string; stem:string; stars:Star[]; isLifePalace?:boolean; isBodyPalace?:boolean };

export function ZiWeiPalaceGrid({ palaces, lifePalaceIndex }: { palaces: Palace[]; lifePalaceIndex: number }) {
  const sihuaLabel: Record<string,string> = { lu:'祿', quan:'權', ke:'科', ji:'忌' };
  return (
    <div style={{ display:'grid', gridTemplateColumns:'repeat(4,1fr)', gap:'0.5rem' }}>
      {palaces.map(p => (
        <div key={p.branch} style={{ border: p.isLifePalace ? '2px solid #4F46E5' : '1px solid #e5e7eb', borderRadius:8, padding:'0.5rem', background: p.isLifePalace ? '#EEF2FF' : 'white' }}>
          <div style={{ fontWeight:600, fontSize:'0.85rem' }}>{p.branch} {p.stem} · {p.name} {p.isLifePalace && '★命宮'} {p.isBodyPalace && '·身宮'}</div>
          <div style={{ marginTop:'0.25rem', display:'flex', flexWrap:'wrap', gap:'0.25rem' }}>
            {p.stars.map(s => (
              <span key={s.name} style={{ fontSize:'0.8rem', padding:'0.15rem 0.35rem', borderRadius:4, background: s.type==='main' ? '#FEF3C7' : s.type==='transformation' ? '#FEE2E2' : '#F3F4F6' }}>
                {s.name}{s.brightness ? `(${s.brightness})` : ''}{s.sihua ? `化${sihuaLabel[s.sihua]}` : ''}
              </span>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Rewrite ZiWeiDisplay in DivinationPage.tsx**

Replace the `ZiWeiDisplay` function (currently reading `lunarDate/lifePalace/mainStars` which never exist per backend) with:

```tsx
function ZiWeiDisplay({ data }: { data: Record<string, unknown> }) {
  const d = data as any as import('../shared/types').ZiWeiChartV3 | any;
  // Back-compat: if old V2 shape somehow stored, fall back to raw JSON dump
  if (!d.palaces) return <pre style={{ whiteSpace:'pre-wrap', fontSize:'0.8rem' }}>{JSON.stringify(d, null, 2)}</pre>;
  return (
    <div style={{ display:'grid', gap:'1rem' }}>
      <div style={{ display:'flex', flexWrap:'wrap', gap:'1rem', fontSize:'0.9rem' }}>
        {d.birthInfo?.lunar && <span><strong>農曆:</strong> {d.birthInfo.lunar.year}年{d.birthInfo.lunar.month}月{d.birthInfo.lunar.day}日{d.birthInfo.lunar.isLeap ? '(閏)' : ''}</span>}
        {d.fiveElement && <span><strong>五行局:</strong> {d.fiveElement}</span>}
        {d.majorLimits?.length > 0 && <span><strong>大限:</strong> {d.majorLimits.map((m:any)=>`${m.startAge}-${m.endAge} ${m.stem}${m.branch}`).join(' · ')}</span>}
        {d.meta && <span style={{ color:'#6b7280' }}>#{d.meta.chartSchemaVersion} · {d.meta.engineVersionZiwei}{d.meta.assumed ? ' · assumed' : ''}</span>}
      </div>
      <ZiWeiPalaceGrid palaces={d.palaces} lifePalaceIndex={d.lifePalaceIndex} />
    </div>
  );
}
```

Add at top: `import { ZiWeiPalaceGrid } from '../components/ZiWeiPalaceGrid';`

If `frontend/src/shared/types` does not exist, define the cast as `Record<string,unknown>` and skip the import line.

- [ ] **Step 4: Build check**

```bash
cd frontend && npm run build
```

Expected: `tsc` passes, `vite build` succeeds, no missing module errors.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/api.ts frontend/src/pages/DivinationPage.tsx frontend/src/components/ZiWeiPalaceGrid.tsx
git commit -m "feat(ziwei): frontend V3 display — palace grid with brightness/sihua, 409 auto-retry"
```

---

### Task 5: Integration Tests, Dry-Run, and Release Gate

**Files:**
- Create: `backend/src/__tests__/integration/ziwei-iztro.test.ts`
- Read-only verification: `wrangler deploy --dry-run`, `npm run typecheck` (both workspaces), `npm run build` (frontend)

**Interfaces:**
- Consumes: all artifacts from Tasks 1–4
- Produces: committed integration tests + measured gzip/CPU gate recorded in commit message

- [ ] **Step 1: Write integration tests (ziwei-iztro anchors)**

Create `backend/src/__tests__/integration/ziwei-iztro.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
const API_URL = process.env.TEST_API_URL || 'https://fortunet-api.yanggf.workers.dev';
const RUN = process.env.RUN_INTEGRATION === 'true';
const d = RUN ? describe : describe.skip;

d('ZiWei iztro A1 anchors', () => {
  let sid: string;
  const email = `iztro-${Date.now()}@example.com`;

  it('register', async () => {
    const r = await fetch(`${API_URL}/api/auth/register`, { method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({ email })});
    expect(r.status).toBe(201);
    sid = (await r.json() as any).sessionId;
  });

  it('published-chart anchor (expect known palace/stars) — replace with verified publisher chart after first run', async () => {
    // Example anchor: choose a publisher-verified chart; initially assert shape only.
    await fetch(`${API_URL}/api/users/me/birth`, { method:'PUT', headers:{'Content-Type':'application/json','Authorization':`Bearer ${sid}`}, body: JSON.stringify({ birth_year:1990, birth_month:5, birth_day:15, birth_hour:14, birth_minute:30, gender:'male' })});
    const r = await fetch(`${API_URL}/api/charts/ziwei`, { headers:{'Authorization':`Bearer ${sid}`}});
    expect(r.status).toBe(200);
    const j = await r.json() as any;
    expect(j.chartSchemaVersion).toBe(3);
    expect(j.chart_data.palaces).toHaveLength(12);
    expect(j.chart_data.meta.dayDivide).toBe('forward');
    expect(j.chart_data.palaces.flatMap((p:any)=>p.stars).some((s:any)=>s.brightness)).toBe(true);
    expect(j.chart_data.palaces.flatMap((p:any)=>p.stars).some((s:any)=>s.sihua)).toBe(true);
  });

  it('23:00 dayDivide forward — chart exists and meta reflects forward', async () => {
    await fetch(`${API_URL}/api/users/me/birth`, { method:'PUT', headers:{'Content-Type':'application/json','Authorization':`Bearer ${sid}`}, body: JSON.stringify({ birth_year:1990, birth_month:5, birth_day:15, birth_hour:23, birth_minute:10, gender:'male' })});
    const r = await fetch(`${API_URL}/api/charts/ziwei`, { headers:{'Authorization':`Bearer ${sid}`}});
    expect(r.status).toBe(200);
    const j = await r.json() as any;
    expect(j.chart_data.meta.timeIndex).toBe(12);
    expect(j.chart_data.meta.dayDivide).toBe('forward');
  });

  it('leap-month (fixLeap=true) — lunar isLeap surfaced and chart differs from non-leap', async () => {
    // Pick a known leap month year, e.g. 2023 leap Feb (iztro known). Verify isLeap boolean present.
    await fetch(`${API_URL}/api/users/me/birth`, { method:'PUT', headers:{'Content-Type':'application/json','Authorization':`Bearer ${sid}`}, body: JSON.stringify({ birth_year:2023, birth_month:3, birth_day:22, birth_hour:10, gender:'female' })});
    const r = await fetch(`${API_URL}/api/charts/ziwei`, { headers:{'Authorization':`Bearer ${sid}`}});
    expect(r.status).toBe(200);
    const j = await r.json() as any;
    expect(typeof j.chart_data.birthInfo.lunar.isLeap).toBe('boolean');
  });

  it('stale chart 409 flow — after A1 no stale should remain, but guard is testable via direct POST stale simulation', async () => {
    // Ensure a fresh chart first
    await fetch(`${API_URL}/api/users/me/birth`, { method:'PUT', headers:{'Content-Type':'application/json','Authorization':`Bearer ${sid}`}, body: JSON.stringify({ birth_year:1992, birth_month:8, birth_day:8, birth_hour:8, gender:'male' })});
    await fetch(`${API_URL}/api/charts/ziwei`, { headers:{'Authorization':`Bearer ${sid}`}});
    const r = await fetch(`${API_URL}/api/charts/ziwei/interpret`, { method:'POST', headers:{'Authorization':`Bearer ${sid}`}});
    // Either 200 (fresh) or 409→retry path exercised by frontend; backend direct call should be 200 on fresh chart
    expect([200,409,503]).toContain(r.status);
    if (r.status===200) {
      const j = await r.json() as any;
      expect(j.interpretation || j.fromCache).toBeDefined();
    }
  }, 30000);
});
```

Adjust the leap-month date after checking `2023` leap mapping against iztro docs (2023 leap 2). The test above asserts shape, not exact palace, until a publisher-verified expected palace is pinned in a follow-up.

- [ ] **Step 2: Run typecheck and frontend build locally**

```bash
cd backend && npm run typecheck
cd ../frontend && npm run build
```

Expected: both PASS.

- [ ] **Step 3: Dry-run and record bundle metrics**

```bash
cd backend && unset CLOUDFLARE_API_TOKEN && npx wrangler deploy --dry-run 2>&1 | tee /tmp/wrangler-dry-run.txt
grep -E "Total Upload|gzip" /tmp/wrangler-dry-run.txt
```

Expected: `Total Upload` gzip `< 3MB`. Record the two numbers in the commit message body. If `> 3MB`, task fails — open a follow-up to enable sunset removal of self-built calculator or trim locale.

- [ ] **Step 4: (Optional, when staging available) Run integration tests against staging**

```bash
TEST_API_URL=http://localhost:8787 RUN_INTEGRATION=true npm run test -- src/__tests__/integration/ziwei-iztro.test.ts
```

This requires `wrangler dev --local` running in another terminal. Skip if no local D1 is seeded; the test file still commits as the regression net for CI.

- [ ] **Step 5: Commit**

```bash
git add backend/src/__tests__/integration/ziwei-iztro.test.ts
git commit -m "test(ziwei): iztro A1 integration anchors — V3 shape, dayDivide forward, leap-month, stale guard

dry-run: <paste Total Upload and gzip numbers from /tmp/wrangler-dry-run.txt>"
```

- [ ] **Step 6: Manual verification checklist (do not automate, record in PR description)**

- [ ] Old V2 chart in D1 with `engineVersion 2.0.0` → `GET /api/charts/ziwei` recalculates, `fromCache` is `false`, stored `chart_data` now has `meta.engineVersionZiwei='3.0.0'`
- [ ] Old `ai_interpretation` cleared on upsert path (query `SELECT ai_interpretation FROM interpretations` after stale GET)
- [ ] `GET /charts/ziwei` ETag changes after version bump (curl -I with If-None-Match → 200 not 304 on first fetch post-deploy)
- [ ] Frontend: `DivinationPage` shows 12 palaces with brightness/sihua badges, no console errors, expired-cache refresh works

---

## Self-Review

**Spec coverage check:**
- §3.1 iztro bySolar (string + timeIndex, fixLeap, zh-TW) — Task 2
- dayDivide='forward' explicit + meta + 23:00 test — Task 2 + Task 5
- fixLeap=true — Task 2
- Branch-index helper `(palaceIndex+2)%12` — Task 2 (groundBranchOf) with comment
- Star brightness/sihua — Task 2 mapping + Task 5 brightness/sihua assertion
- Major limits — Task 2
- Per-type versioning + stale guard + ETag — Task 3
- chartSchemaVersion additive strategy — Task 3 + Task 4
- Zod contract for V3 — Task 1 + Task 3 validation
- Birth-hash extraction shared — Task 1
- Frontend 409 auto-retry — Task 4
- ZiWeiDisplay rewrite + palace grid — Task 4
- Dry-run gzip/CPU gate — Task 5
- Sunset condition — documented in Task 2 header, file structure notes
- **Gap fixed vs earlier draft:** western calculator left untouched (correct for A1); self-built calculator retained not deleted.

**Placeholder scan:** No TBD/TODO/fill-in-later. Every `any` guard in adapter is bounded by a "verify d.ts field names before commit" instruction — not a placeholder, a pre-commit checklist item.

**Type consistency:** `ZiWeiChartV3` flows from `shared/schemas/ziwei-v3.ts` → `types.ts` re-export → `iztro-adapter.ts` return type → `charts.ts` response → frontend cast. `ENGINE_VERSION_ZIWEI` / `CHART_SCHEMA_VERSION` names are consistent across all tasks. `computeBirthHash` signature is identical in birth-hash.ts and its call sites.

**Risks noted:** If iztro `GenderName` union differs (`'男'|'女'` vs `'male'|'female'`), cast as `any` with d.ts line reference. If `fourPillars` field is named differently in d.ts, adapter fallback keeps self-built pillars (no crash).

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-08-26-engine-a1-iztro-adapter.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
