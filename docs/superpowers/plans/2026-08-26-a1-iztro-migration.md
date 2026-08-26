# A1：紫微引擎換裝 iztro 實作計畫

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把紫微斗數計算核心從自製 calculator 切換到 iztro 2.6.0，API 回應升級為完整盤 V3（四化/亮度/大限/三方四正），並建立 per-type 引擎版本失效與 interpret 409 守衛。

**Architecture:** 新增 adapter 層隔離 iztro 型別；Zod schema 定於 `backend/src/shared/schemas/` 供前後端共用；charts.ts 的 ziwei 分支改呼叫 adapter；前端 DivinationPage 紫微區整段重寫（現況讀取後端不存在的欄位，零回歸風險）。自製 calculator 保留匯出做對照基準。

**Tech Stack:** Cloudflare Workers + Hono、iztro 2.6.0（MIT）、zod、React 18 + Vite。

**Spec:** `docs/superpowers/specs/2026-08-26-engine-modernization-big5-design.md`（§3.1、§3.4、§5）

## Global Constraints

- 一律 `unset CLOUDFLARE_API_TOKEN &&` 前綴執行 wrangler（OAuth 政策）
- 啟動任何 dev server / 部署前必須取得使用者確認；部署前必 commit
- Integration tests only：不寫 unit test、不用 mock；已提交測試預設不打網路，需 `RUN_INTEGRATION=true`
- gzip 上限 3MB（`wrangler deploy --dry-run` 驗證）
- TS strict、單引號、分號、kebab-case 檔名
- `fixLeap=true`、`language='zh-TW'`、`dayDivide='forward'` 顯式設定
- 版本值：`ENGINE_VERSION_ZIWEI='3.0.0'`、`ENGINE_VERSION_WESTERN='2.0.0'`（A2 才動）
- 本 repo `.gitignore` 全域忽略 package-lock.json——只 commit `package.json`
- 不使用 feature flags

---

### Task 1: 安裝 iztro 並以探針釘死行為

**Files:**
- Modify: `backend/package.json`（dependencies）
- Create（拋棄式，不入 repo）: `/tmp/iztro-probe.mjs`

**Interfaces:**
- Produces: 已安裝的 `iztro@2.6.0`；一份「實際欄位名紀錄」（palace/star 欄位、toJSON 可用性、timeIndex 映射驗證），供 Task 4 的映射程式碼對照。若探針輸出與下方假設不符，**以探針輸出為準**並把差異記在 commit message。

- [ ] **Step 1: 安裝依賴**

```bash
cd /home/yanggf/a/ft/backend && npm install iztro@2.6.0
```

- [ ] **Step 2: 執行行為探針**

```javascript
// /tmp/iztro-probe.mjs
import { astro } from 'iztro';
// ESM 若不可用則改: const { astro } = require('/home/yanggf/a/ft/backend/node_modules/iztro/lib/index.js');
astro.config({ dayDivide: 'forward' });
const chart = astro.bySolar('2000-8-16', 6, '女', true, 'zh-TW'); // 午時
console.log('top-level keys:', Object.keys(chart));
console.log('chineseDate:', chart.chineseDate);
console.log('fiveElementsClass:', chart.fiveElementsClass);
console.log('soul/body:', chart.soul, chart.body);
const p0 = chart.palaces[0];
console.log('palace keys:', Object.keys(p0));
console.log('palace[0]:', JSON.stringify({
  index: p0.index, name: p0.name, earthlyBranch: p0.earthlyBranch,
  heavenlyStem: p0.heavenlyStem, isBodyPalace: p0.isBodyPalace,
  isOriginalPalace: p0.isOriginalPalace, decadal: p0.decadal,
}, null, 1));
if (p0.majorStars[0]) console.log('star keys:', Object.keys(p0.majorStars[0]), p0.majorStars[0]);
console.log('toJSON usable:', typeof chart.toJSON === 'function');
// 23:00 案例（晚子）：比對 bySolar(...,12,...) 與次日 bySolar(...,0,...) 的命宮主星是否相同（forward 語意）
const late = astro.bySolar('2000-8-16', 12, '女', true, 'zh-TW');
const nextEarly = astro.bySolar('2000-8-17', 0, '女', true, 'zh-TW');
console.log('late-zi soul:', late.soul, '| next-day early-zi soul:', nextEarly.soul);
```

```bash
node /tmp/iztro-probe.mjs
```

Expected：印出 palace 結構（應含 `index/name/majorStars/minorStars/adjectiveStars/earthlyBranch/heavenlyStem/isBodyPalace/isOriginalPalace/decadal`）、star 含 `name/type/scale(brightness)/mutagen`、`toJSON` 為 function、晚子 soul 與次日早子一致（forward）。**任何欄位名不同 → 記錄實名，Task 4 以實名映射。**

- [ ] **Step 3: 體積預估**

```bash
cd /home/yanggf/a/ft/backend && npx esbuild --bundle --minify --format=esm \
  --define:process.env.NODE_ENV='"production"' \
  src/services/ziwei/calculator.ts > /dev/null 2>/tmp/esb.err || true
echo "iztro alone:"; npx esbuild --bundle --minify --format=esm \
  node_modules/iztro/lib/index.js > /tmp/iztro-bundle.js 2>/dev/null && gzip -c /tmp/iztro-bundle.js | wc -c
```

Expected：gzip 後遠低於 3MB（量級應為數百 KB 內）。若接近上限即停手回報。

- [ ] **Step 4: Commit**

```bash
cd /home/yanggf/a/ft && git add backend/package.json && \
git commit -m "feat(backend): add iztro@2.6.0 dependency (probe facts: see message)"
```

---

### Task 2: ZiWeiChartV3 共享 Zod schema

**Files:**
- Create: `backend/src/shared/schemas/ziwei-chart.ts`

**Interfaces:**
- Produces: `ZiWeiChartV3Schema`（ZodObject）與推導型別 `ZiWeiChartV3`、`PalaceV3`、`StarV3`、`MajorLimitV3`、`FourPillarsV3`——Task 4/5/6 直接 import。
- 注意：frontend 將以 `import type` 跨目錄引用此檔（Task 6 處理 tsconfig），故此檔**不得** import 任何 backend 執行環境專屬模組，僅准 import `zod`。

- [ ] **Step 1: 撰寫 schema**

```typescript
import { z } from 'zod';

export const StarV3Schema = z.object({
  name: z.string(),
  type: z.enum(['main', 'minor']),
  brightness: z.string().optional(),          // 廟旺得利平不陷（iztro scale）
  mutagen: z.enum(['祿', '權', '科', '忌']).optional(), // 生年四化
});
export type StarV3 = z.infer<typeof StarV3Schema>;

export const MajorLimitV3Schema = z.object({
  branch: z.string(),                          // 地支（地支序語意）
  stem: z.string(),
  range: z.tuple([z.number(), z.number()]),    // 虛歲起訖
});
export type MajorLimitV3 = z.infer<typeof MajorLimitV3Schema>;

export const PalaceV3Schema = z.object({
  branchIndex: z.number().int().min(0).max(11), // 本專案座標系：子=0
  name: z.string(),                             // 十二宮名
  branch: z.string(),
  stem: z.string(),
  stars: z.array(StarV3Schema),                 // 主星＋副星合併，以 type 區分
  isLifePalace: z.boolean(),
  isBodyPalace: z.boolean(),
  surrounds: z.tuple([z.number(), z.number(), z.number()]), // 三方四正：對宮+兩三合（branchIndex 空間）
});
export type PalaceV3 = z.infer<typeof PalaceV3Schema>;

export const PillarPairSchema = z.object({ stem: z.string(), branch: z.string() });
export const FourPillarsV3Schema = z.object({
  year: PillarPairSchema, month: PillarPairSchema, day: PillarPairSchema, hour: PillarPairSchema,
});
export type FourPillarsV3 = z.infer<typeof FourPillarsV3Schema>;

export const ZiWeiChartV3Schema = z.object({
  chartSchemaVersion: z.literal(3),
  birthInfo: z.object({
    solar: z.object({ year: z.number(), month: z.number(), day: z.number() }),
    lunar: z.object({ year: z.number(), month: z.number(), day: z.number(), isLeap: z.boolean().optional() }),
    lunarDateStr: z.string(),                  // iztro 原字串，如 '二〇〇〇年七月十七'
    hour: z.number(),
    hourBranch: z.string(),
    timeIndex: z.number().int().min(0).max(12),
    gender: z.string(),
  }),
  fourPillars: FourPillarsV3Schema,
  fiveElement: z.string(),                     // 如 '火六局'
  soulStar: z.string(),                        // 命宮主星（iztro soul）
  bodyStar: z.string(),                        // 身宮主星（iztro body）
  lifePalaceBranchIndex: z.number().int().min(0).max(11),
  bodyPalaceBranchIndex: z.number().int().min(0).max(11),
  palaces: z.array(PalaceV3Schema).length(12),
  majorLimits: z.array(MajorLimitV3Schema),
  meta: z.object({
    engineVersion: z.string(),
    dayDivide: z.enum(['forward', 'current']),
    fixLeap: z.boolean(),
    assumedHour: z.boolean(),                  // birth_hour 為 null 時預設午時
  }),
});
export type ZiWeiChartV3 = z.infer<typeof ZiWeiChartV3Schema>;
```

（若 backend 尚無 zod 依賴：`cd backend && npm install zod` 一併處理。）

- [ ] **Step 2: 驗證**

```bash
cd /home/yanggf/a/ft/backend && npx tsc --noEmit
node -e "
const {z} = require('zod');
const m = require('./src/shared/schemas/ziwei-chart.ts');
" 2>/dev/null || npx tsx -e "import {ZiWeiChartV3Schema} from './src/shared/schemas/ziwei-chart'; const r = ZiWeiChartV3Schema.safeParse({}); console.log(r.success === false ? 'schema loads, rejects empty ✓' : 'UNEXPECTED');" 2>&1 | tail -1
```

Expected：typecheck 通過；schema 可載入且拒絕空物件。（tsx 不可用時改用 `npx esbuild --bundle` 打包後 node 執行，擇一即可。）

- [ ] **Step 3: Commit**

```bash
git add backend/src/shared/schemas/ziwei-chart.ts backend/package.json && \
git commit -m "feat(backend): ZiWeiChartV3 zod schema (shared front/back contract)"
```

---

### Task 3: iztro-adapter

**Files:**
- Create: `backend/src/services/ziwei/iztro-adapter.ts`

**Interfaces:**
- Consumes: `ZiWeiChartV3`（Task 2）、iztro `astro.bySolar/config`（Task 1 探針欄位名）
- Produces: `calculateZiWeiV3(data: BirthData): ZiWeiChartV3`（BirthData 沿用 `services/ziwei/types.ts` 既有型別）——Task 5 路由呼叫

- [ ] **Step 1: 撰寫 adapter**

```typescript
import { astro } from 'iztro';
import type { BirthData } from './types';
import type {
  ZiWeiChartV3, PalaceV3, StarV3, MajorLimitV3,
} from '../shared/schemas/ziwei-chart';
import { ENGINE_VERSION_ZIWEI } from '../../services/engine-version';

// 晚子時歸屬採 iztro forward（次日安星）；顯式設定，不依賴隱含預設（設計 §3.1）
astro.config({ dayDivide: 'forward' });

/** 0–23 時 → iztro timeIndex 0–12（早子0…亥11…晚子12） */
export function timeIndexFromHour(hour: number): number {
  if (hour === 23) return 12;
  return ((hour + 1) >> 1) % 12;
}

/** iztro 宮位陣列（寅起0）→ 本專案地支序（子0） */
function branchIndexOf(palaceIndex: number): number {
  return (palaceIndex + 2) % 12;
}

const BRANCH_ORDER = ['子', '丑', '寅', '卯', '辰', '巳', '午', '未', '申', '酉', '戌', '亥'];

export function calculateZiWeiV3(data: BirthData): ZiWeiChartV3 {
  const timeIndex = timeIndexFromHour(data.hour);
  const chart = astro.bySolar(
    `${data.year}-${data.month}-${data.day}`,
    timeIndex,
    data.gender === 'male' ? '男' : '女',
    true,      // fixLeap：閏月前十五日歸本月
    'zh-TW',
  );

  const palaces: PalaceV3[] = chart.palaces.map((p) => {
    const bi = branchIndexOf(p.index);
    const stars: StarV3[] = [
      ...p.majorStars.map((s) => ({
        name: s.name,
        type: 'main' as const,
        ...(s.scale ? { brightness: s.scale } : {}),
        ...(s.mutagen ? { mutagen: s.mutagen as StarV3['mutagen'] } : {}),
      })),
      ...p.minorStars.map((s) => ({ name: s.name, type: 'minor' as const })),
    ];
    return {
      branchIndex: bi,
      name: p.name,
      branch: BRANCH_ORDER[bi],
      stem: p.heavenlyStem,
      stars,
      isLifePalace: p.isOriginalPalace === true,
      isBodyPalace: p.isBodyPalace === true,
      surrounds: [(bi + 6) % 12, (bi + 4) % 12, (bi + 8) % 12],
    };
  });

  // 大限：各宮 decadal range 即該宮大限起訖虛歲
  const majorLimits: MajorLimitV3[] = chart.palaces
    .filter((p) => Array.isArray(p.decadal?.range))
    .map((p) => ({
      branch: p.earthlyBranch,
      stem: p.heavenlyStem,
      range: [p.decadal.range[0], p.decadal.range[p.decadal.range.length - 1]],
    }));

  // 四柱取自 iztro 中文干支字串拆解（格式如 '庚辰 甲申 丙午 甲午'）
  const gz = String(chart.chineseDate).split(' ').map((s) => s.trim()).filter(Boolean);
  const pair = (s?: string) => ({ stem: s?.[0] ?? '', branch: s?.[1] ?? '' });

  return {
    chartSchemaVersion: 3,
    birthInfo: {
      solar: { year: data.year, month: data.month, day: data.day },
      lunar: { year: 0, month: 0, day: 0 },           // 由 lunarDateStr 承載，見下
      lunarDateStr: String(chart.lunarDate),
      hour: data.hour,
      hourBranch: BRANCH_ORDER[timeIndexFromHour(data.hour) % 12] ?? '子',
      timeIndex,
      gender: data.gender === 'male' ? '男' : '女',
    },
    fourPillars: {
      year: pair(gz[0]), month: pair(gz[1]), day: pair(gz[2]), hour: pair(gz[3]),
    },
    fiveElement: String(chart.fiveElementsClass),
    soulStar: String(chart.soul),
    bodyStar: String(chart.body),
    lifePalaceBranchIndex: branchIndexOf(
      chart.palaces.findIndex((p) => p.isOriginalPalace === true),
    ),
    bodyPalaceBranchIndex: branchIndexOf(
      chart.palaces.findIndex((p) => p.isBodyPalace === true),
    ),
    palaces,
    majorLimits,
    meta: {
      engineVersion: ENGINE_VERSION_ZIWEI,
      dayDivide: 'forward',
      fixLeap: true,
      assumedHour: false,
    },
  };
}
```

**執行注意**：(a) `lunar.year/month/day` 若探針顯示 iztro 無結構化農曆數字，保留 0 值並靠 `lunarDateStr`（schema 中兩者皆存在即是為此）；不得為湊數自行用自製 solarToLunar 回填（雙曆法疊加禁令）。(b) `hourBranch` 若上式有誤直接以 `p0` 所在資訊或查表重寫，確保正確。(c) 欄位名一律以 Task 1 探針實名為準修正。

- [ ] **Step 2: 拋棄式結構驗證（非 unit test，不入 repo）**

```bash
cd /home/yanggf/a/ft/backend && npx esbuild src/services/ziwei/iztro-adapter.ts src/shared/schemas/ziwei-chart.ts \
  --bundle --outdir=/tmp/v3chk --format=cjs --external:iztro 2>/dev/null
cat > /tmp/v3-run.mjs <<'EOF'
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { calculateZiWeiV3 } = require('/tmp/v3chk/iztro-adapter.js');
const { ZiWeiChartV3Schema } = require('/tmp/v3chk/ziwei-chart.js');
const c = calculateZiWeiV3({ year: 2000, month: 8, day: 16, hour: 12, gender: 'female' });
const parsed = ZiWeiChartV3Schema.safeParse(c);
console.log('schema ok:', parsed.success);
if (!parsed.success) console.log(JSON.stringify(parsed.error.issues, null, 1).slice(0, 800));
const mains = c.palaces.flatMap(p => p.stars).length;
console.log('palaces:', c.palaces.length, '| lifePalace:', c.lifePalaceBranchIndex, '| five:', c.fiveElement);
console.log('fourPillars:', JSON.stringify(c.fourPillars));
console.log('majorLimits sample:', JSON.stringify(c.majorLimits[0]));
EOF
node /tmp/v3-run.mjs
```

Expected：`schema ok: true`、palaces 12、lifePalace 落在 0–11、fourPillars 四柱齊。失敗時依 issue 修映射（多半是探針欄位名差異）。

- [ ] **Step 3: typecheck + Commit**

```bash
cd /home/yanggf/a/ft/backend && npm run typecheck && cd .. && \
git add backend/src/services/ziwei/iztro-adapter.ts && \
git commit -m "feat(backend): iztro adapter producing ZiWeiChartV3"
```

---

### Task 4: per-type 版本常數

**Files:**
- Modify: `backend/src/services/engine-version.ts`

**Interfaces:**
- Produces: `ENGINE_VERSION_ZIWEI = '3.0.0'`、`ENGINE_VERSION_WESTERN = '2.0.0'`、helper `engineVersionFor(type: 'ziwei'|'western'): string`——Task 5 使用。舊的單一 `ENGINE_VERSION` export 移除（grep 確認僅 charts.ts 引用後同步改）。

- [ ] **Step 1: 改寫檔案**

```typescript
/**
 * Per-type engine algorithm versions, embedded into cached chart_data.
 * Bump the relevant constant whenever that type's calculation changes;
 * bumping one must NOT invalidate the other type's caches.
 */
export const ENGINE_VERSION_ZIWEI = '3.0.0';
export const ENGINE_VERSION_WESTERN = '2.0.0';

export type DivinationType = 'ziwei' | 'western';

export function engineVersionFor(type: DivinationType): string {
  return type === 'ziwei' ? ENGINE_VERSION_ZIWEI : ENGINE_VERSION_WESTERN;
}
```

- [ ] **Step 2: 驗證引用面乾淨**

```bash
cd /home/yanggf/a/ft/backend && grep -rn "ENGINE_VERSION\b" src/ | grep -v "ENGINE_VERSION_"
npm run typecheck   # 此步會因 charts.ts 尚引用舊名而失敗 → 屬預期，Task 5 修復；先確認唯一引用點就是 charts.ts
```

Expected：舊名只剩 `routes/charts.ts` 引用。

- [ ] **Step 3: Commit（連同 Task 3 若尚未提交）**

```bash
git add backend/src/services/engine-version.ts && \
git commit -m "refactor(backend): split ENGINE_VERSION into per-type constants"
```

---

### Task 5: 路由切換＋409 守衛＋ETag 摻版本

**Files:**
- Modify: `backend/src/routes/charts.ts`

**Interfaces:**
- Consumes: `calculateZiWeiV3`（Task 3）、`engineVersionFor`（Task 4）、`ZiWeiChartV3Schema`（Task 2）
- Produces: `GET /api/charts/ziwei` 回傳 V3 盤（chart_data 內嵌 `meta.engineVersion`）；`POST /api/charts/:type/interpret` 對版本不符回 `409 {"code":"RECALC_REQUIRED"}`

- [ ] **Step 1: 修改路由**

關鍵 diff（套用到現有檔案）：

```typescript
// import 區
import { ziWeiCalculator } from '../services/ziwei';            // 保留（對照基準）
import { calculateZiWeiV3 } from '../services/ziwei/iztro-adapter';
import { engineVersionFor, DivinationType } from '../services/engine-version';
import { ZiWeiChartV3Schema } from '../shared/schemas/ziwei-chart';

// 移除頂層 ENGINE_VERSION 常數，改用 engineVersionFor()

// GET /:type 內：
const engVer = engineVersionFor(divType as DivinationType);
const etag = createETag(`${birth.birth_data_hash}-${engVer}`, cached?.updated_at || Date.now());

// cache-hit 條件改為 per-type 版本比對：
if (parsedChart?.meta?.engineVersion === engVer || parsedChart?.engineVersion === engVer) { ...回快取... }
// （第二個條件相容 western 2.0.0 舊內嵌位置；A2 時移除）

// ziwei 分支：
if (divType === 'ziwei') {
  if (!birth.gender) return c.json({ error: 'Gender required for ZiWei', code: 'NO_GENDER' }, 400);
  const hour = birth.birth_hour ?? 12;
  const chart = calculateZiWeiV3({ year: birth.birth_year, month: birth.birth_month, day: birth.birth_day, hour, gender: birth.gender as 'male' | 'female' });
  chart.meta.assumedHour = birth.birth_hour == null;
  chartData = chart;
}

// upsert 前：
const chartDataWithVersion = { ...(chartData as Record<string, unknown>) }; // V3 已內嵌 meta.engineVersion
// western 分支維持原狀但包一層 { ...orig, engineVersion: engVer }

// POST /:type/interpret 於取得 interp 後加：
let staleVersion: string | null = null;
try {
  const pc = typeof interp.chart_data === 'string' ? JSON.parse(interp.chart_data) : interp.chart_data;
  staleVersion = pc?.meta?.engineVersion ?? pc?.engineVersion ?? null;
} catch { /* fallthrough */ }
if (staleVersion !== engineVersionFor(divType as DivinationType)) {
  return c.json({ error: 'Chart engine version outdated; refetch the chart', code: 'RECALC_REQUIRED' }, 409);
}
```

- [ ] **Step 2: 驗證**

```bash
cd /home/yanggf/a/ft/backend && npm run typecheck
# 回應形狀抽驗（拋棄式，不起 server）：直接呼叫 adapter 已於 Task 3 做；
# 此處以 grep 確認三件事：
grep -n "RECALC_REQUIRED" src/routes/charts.ts
grep -n "engineVersionFor" src/routes/charts.ts
grep -n "calculateZiWeiV3" src/routes/charts.ts
```

Expected：typecheck 綠、三個 grep 各有命中。

- [ ] **Step 3: Commit**

```bash
git add backend/src/routes/charts.ts && \
git commit -m "feat(backend): route ziwei to iztro V3; per-type cache versions; interpret 409 guard"
```

---

### Task 6: 前端紫微顯示重寫（讀 V3）

**Files:**
- Modify: `frontend/tsconfig.json`（include 加 `"../backend/src/shared/**/*.ts"`）
- Modify: `frontend/src/pages/DivinationPage.tsx`（紫微渲染段整段重寫）

**Interfaces:**
- Consumes: `import type { ZiWeiChartV3 } from '../../backend/src/shared/schemas/ziwei-chart'`（type-only，建構時擦除，前端不需安裝 zod）

- [ ] **Step 1: tsconfig include**

`frontend/tsconfig.json` 的 `include` 陣列加入 `"../backend/src/shared/**/*.ts"`。

- [ ] **Step 2: 重寫紫微區塊**

刪除讀取 `lunarDate/lifePalace/mainStars` 的舊 stub（`DivinationPage.tsx:105` 附近整段），替換：

```tsx
import type { ZiWeiChartV3 } from '../../backend/src/shared/schemas/ziwei-chart';

function ZiWeiPanelV3({ data }: { data: ZiWeiChartV3 }) {
  return (
    <section className="ziwei-panel">
      <header>
        <h3>紫微斗數</h3>
        <p>{data.fiveElement}・命主{data.soulStar}・身主{data.bodyStar}</p>
        <p>{['year','month','day','hour'].map(k => `${data.fourPillars[k as 'year'].stem}${data.fourPillars[k as 'year'].branch}`).join('　')}</p>
        {data.meta.assumedHour && <small>⚠ 未提供出生時間，以午時計算</small>}
      </header>
      <div className="limits">
        {data.majorLimits.slice(0, 12).map(l => (
          <span key={l.branch}>{l.stem}{l.branch} {l.range[0]}–{l.range[1]}</span>
        ))}
      </div>
      <div className="palace-grid">
        {data.palaces.map(p => (
          <article key={p.branchIndex} className={p.isLifePalace ? 'palace life' : 'palace'}>
            <h4>{p.name}・{p.stem}{p.branch}</h4>
            <ul>
              {p.stars.map(s => (
                <li key={s.name}>
                  {s.name}
                  {s.brightness && <em>{s.brightness}</em>}
                  {s.mutagen && <b className={`mu-${s.mutagen}`}>{s.mutagen}</b>}
                </li>
              ))}
            </ul>
            {p.isBodyPalace && <footer>身宮</footer>}
          </article>
        ))}
      </div>
    </section>
  );
}
```

接線處：原本 `renderZiWei(data)` 的呼叫點改為判斷 `parsed?.chartSchemaVersion === 3 ? <ZiWeiPanelV3 data={parsed}/> : 舊渲染`，`parsed` 由既有 `JSON.parse(chart_data)` 取得。

- [ ] **Step 3: 驗證 build**

```bash
cd /home/yanggf/a/ft/frontend && npm run build
```

Expected：tsc＋vite 全綠。**視覺驗證需啟 dev server——向使用者確認後才執行**（`npm run dev` ×2 + 登入看盤）。

- [ ] **Step 4: Commit**

```bash
git add frontend/tsconfig.json frontend/src/pages/DivinationPage.tsx && \
git commit -m "feat(frontend): render ZiWei V3 full chart (stars/brightness/mutagens/limits)"
```

---

### Task 7: 整合測試＋部署閘門

**Files:**
- Create: `backend/tests/integration/ziwei-v3.test.ts`
- Modify: `backend/package.json`（若尚無 `test` script 接 vitest，補最小設定）

**Interfaces:**
- Consumes: `TEST_API_URL`（預設 production）、`RUN_INTEGRATION` 旗標——遵循 `.testing-rules`

- [ ] **Step 1: 確認測試 runner**

```bash
cd /home/yanggf/a/ft/backend && ls node_modules/.bin | grep -E "^vitest|^tsx" ; cat package.json | grep -A3 '"scripts"'
```

若有 vitest 直接用；否則以 `node --test`（Node 22 內建）撰寫，零新依賴。以下範例採 `node --test` 風格：

- [ ] **Step 2: 撰寫整合測試**

```typescript
// backend/tests/integration/ziwei-v3.test.ts
import { test } from 'node:test';
import assert from 'node:assert/strict';

const API = process.env.TEST_API_URL ?? 'https://fortunet-api.yanggf.workers.dev';
const RUN = process.env.RUN_INTEGRATION === 'true';

// 錨點案例：固定輸入 → 結構與不變量斷言（期望值來自 Task 1 探針實測，執行時填入實際值）
const ANCHOR = { year: 2000, month: 8, day: 16, hour: 12, gender: 'female' };

test.skipIf(!RUN)('ziwei V3 chart returns full-chart invariants', async () => {
  // 1) 建立測試 session（密碼免登入流程）
  const email = `ziwei-v3-${Date.now()}@test.example`;
  const reg = await fetch(`${API}/api/auth/register`, {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email }),
  });
  const regBody = await reg.json() as { sessionId?: string };
  assert.ok(reg.ok, `register failed: ${reg.status}`);
  const H = { Authorization: `Bearer ${regBody.sessionId}` };

  // 2) 存生日
  const put = await fetch(`${API}/api/users/me/birth`, {
    method: 'PUT', headers: { ...H, 'Content-Type': 'application/json' },
    body: JSON.stringify(ANCHOR),
  });
  assert.equal(put.status, 200);

  // 3) 取盤
  const res = await fetch(`${API}/api/charts/ziwei`, { headers: H });
  assert.equal(res.status, 200);
  const body = await res.json() as { chart_data: any };
  const c = body.chart_data;

  // 不變量斷言（不鎖具體星位——那由 adapter 拋棄式驗證與探針負責；這裡鎖契約）
  assert.equal(c.chartSchemaVersion, 3);
  assert.equal(c.meta.engineVersion, '3.0.0');
  assert.equal(c.meta.dayDivide, 'forward');
  assert.equal(c.palaces.length, 12);
  const allStars = c.palaces.flatMap((p: any) => p.stars);
  const mainNames = ['紫微','天機','太陽','武曲','天同','廉貞','天府','太陰','貪狼','巨門','天相','天梁','七殺','破軍'];
  for (const n of mainNames) assert.ok(allStars.some((s: any) => s.name === n), `missing main star ${n}`);
  const mutagens = allStars.filter((s: any) => s.mutagen).length;
  assert.ok(mutagens >= 4, `expected >=4 mutagen stars, got ${mutagens}`);
  assert.ok(c.lifePalaceBranchIndex >= 0 && c.lifePalaceBranchIndex <= 11);

  // 4) interpret 對新盤不應 409（版本相符）——不打 AI 的前提是有快取解讀；此處僅驗「非 409」
  const interp = await fetch(`${API}/api/charts/ziwei/interpret`, { method: 'POST', headers: H });
  assert.notEqual(interp.status, 409, 'fresh V3 chart must not trigger RECALC_REQUIRED');

  // 5) stale 版本守衛：直接再 PUT 同生日不會產生 stale；改以「偽造舊版」路徑無法從外部注入，
  //    故 409 路徑由本地 dev 驗證（見 Step 4），線上僅驗正向。
});
```

- [ ] **Step 3: 預設安全驗證**

```bash
cd /home/yanggf/a/ft/backend && npm test 2>&1 | tail -3
```

Expected：skip（未設 RUN_INTEGRATION），不發任何網路請求。

- [ ] **Step 4: 本地端到端（啟 server 前先徵得使用者同意）**

```bash
unset CLOUDFLARE_API_TOKEN && cd backend && npx wrangler dev &
sleep 6
RUN_INTEGRATION=true TEST_API_URL=http://localhost:8787 node --test tests/integration/ziwei-v3.test.ts
```

Expected：全過。另以 curl 手工驗 409 路徑：暫時將 DB 中某筆 interpretations.chart_data 的 meta.engineVersion 改為 '0.0.0'（local D1），POST interpret → 409 RECALC_REQUIRED。

- [ ] **Step 5: 部署閘門（dry-run）**

```bash
unset CLOUDFLARE_API_TOKEN && cd backend && npx wrangler deploy --dry-run --outdir /tmp/dryrun-a1 2>&1 | grep -E "Total Upload|gzip"
```

Expected：gzip < 3000000 bytes。超標 → 停止並回報（fallback 決策交使用者）。

- [ ] **Step 6: Commit**

```bash
git add backend/tests/integration/ziwei-v3.test.ts backend/package.json && \
git commit -m "test(backend): ziwei V3 integration anchors (opt-in) + deploy gates passed"
```

---

## 收尾清單（全任務完成後）

1. `git log --oneline` 檢視 7 個 commit 敘事完整
2. 向使用者報告 dry-run gzip 數字與本地 e2e 結果
3. 部署（需使用者明示同意）：`./scripts/deploy-backend.sh` ＋ 前端 `npm run deploy`
4. 部署後跑一次 `RUN_INTEGRATION=true npm test`（打 production）確認線上行為
5. 更新 CLAUDE.md：ziwei 引擎段落改為「iztro adapter」描述
