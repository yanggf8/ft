import { astro } from 'iztro';
import { ENGINE_VERSION_ZIWEI, CHART_SCHEMA_VERSION } from '../engine-version';
import type { ZiWeiChartV3 } from '../../shared/schemas/ziwei-v3';
import { EARTHLY_BRANCHES } from './constants';

// Pin dayDivide explicitly (spec §3.1, verified default forward at astro.js:39)
astro.config({ dayDivide: 'forward' });

function timeIndexFromHour(hour: number): number {
  // iztro 0=早子, 1=丑, ..., 11=亥, 12=晚子. Hour 23 → 12.
  if (hour === 23) return 12;
  const branch = EARTHLY_BRANCHES[Math.floor((hour + 1) / 2) % 12] as string;
  const order = ['子', '丑', '寅', '卯', '辰', '巳', '午', '未', '申', '酉', '戌', '亥'];
  return order.indexOf(branch);
}

function branchIndex(name: string): number {
  return EARTHLY_BRANCHES.indexOf(name);
}

const MUTAGEN_MAP: Record<string, 'lu' | 'quan' | 'ke' | 'ji'> = {
  '祿': 'lu',
  '禄': 'lu',
  '權': 'quan',
  '权': 'quan',
  '科': 'ke',
  '忌': 'ji',
};

export const iztroAdapter = {
  calculate(input: { year: number; month: number; day: number; hour: number; minute?: number; gender: 'male' | 'female' }): ZiWeiChartV3 {
    const timeIndex = timeIndexFromHour(input.hour);
    const genderName = input.gender === 'male' ? '男' : '女';
    const solarDate = `${input.year}-${input.month}-${input.day}`;
    const astrolabe: any = astro.bySolar(solarDate, timeIndex, genderName as any, true, 'zh-TW');

    // Build ground-branch-ordered palaces (0=子). Use earthlyBranch field for robust mapping.
    const groundPalaces: any[] = new Array(12);
    for (const p of astrolabe.palaces as any[]) {
      const gi = branchIndex(p.earthlyBranch);
      const allStars: any[] = [...(p.majorStars ?? []), ...(p.minorStars ?? []), ...(p.adjectiveStars ?? [])];
      const stars = allStars.map((s: any) => {
        const mutagen = s.mutagen && MUTAGEN_MAP[s.mutagen] ? MUTAGEN_MAP[s.mutagen] : undefined;
        // iztro Star.type values: '主星' etc. Map to our enum:
        let type: 'main' | 'auxiliary' | 'transformation' = 'auxiliary';
        if (s.type === '主星') type = 'main';
        else if (mutagen) type = 'transformation';
        return {
          name: s.name as string,
          type,
          brightness: s.brightness || undefined,
          sihua: mutagen,
        };
      });
      groundPalaces[gi] = {
        index: gi,
        name: p.name as string,
        branch: p.earthlyBranch as string,
        stem: p.heavenlyStem as string,
        stars,
        isLifePalace: p.earthlyBranch === astrolabe.earthlyBranchOfSoulPalace,
        isBodyPalace: p.earthlyBranch === astrolabe.earthlyBranchOfBodyPalace,
      };
    }

    // Fill any gaps (should not happen) with empty palaces
    for (let i = 0; i < 12; i++) {
      if (!groundPalaces[i]) {
        groundPalaces[i] = {
          index: i,
          name: '',
          branch: EARTHLY_BRANCHES[i],
          stem: '',
          stars: [],
        };
      }
    }

    const raw = astrolabe.rawDates as any;
    const fourPillars = {
      year: { stem: raw.chineseDate.yearly[0], branch: raw.chineseDate.yearly[1] },
      month: { stem: raw.chineseDate.monthly[0], branch: raw.chineseDate.monthly[1] },
      day: { stem: raw.chineseDate.daily[0], branch: raw.chineseDate.daily[1] },
      hour: { stem: raw.chineseDate.hourly[0], branch: raw.chineseDate.hourly[1] },
    };

    const decadalList: any[] = typeof astrolabe.decadalList === 'function' ? astrolabe.decadalList() : [];
    const majorLimits = decadalList.map((d: any) => ({
      startAge: d.ageRange ? d.ageRange[0] : d.range ? d.range[0] : 0,
      endAge: d.ageRange ? d.ageRange[1] : d.range ? d.range[1] : 0,
      stem: d.heavenlyStem ?? d.stem ?? '',
      branch: d.earthlyBranch ?? d.branch ?? '',
      palaceIndex: d.palaceName ? branchIndex((astrolabe.palace(d.palaceName) as any)?.earthlyBranch ?? '') : 0,
    }));

    // Fallback if decadalList empty: use palace decadal fields
    const majorLimitsFinal = majorLimits.length > 0
      ? majorLimits
      : (astrolabe.palaces as any[]).map((p: any) => ({
          startAge: p.decadal?.range?.[0] ?? 0,
          endAge: p.decadal?.range?.[1] ?? 0,
          stem: p.decadal?.heavenlyStem ?? '',
          branch: p.decadal?.earthlyBranch ?? '',
          palaceIndex: branchIndex(p.earthlyBranch),
        })).filter((m: any) => m.startAge > 0);

    const lifePalaceIndex = branchIndex(astrolabe.earthlyBranchOfSoulPalace);
    const bodyPalaceIndex = branchIndex(astrolabe.earthlyBranchOfBodyPalace);

    const chart: ZiWeiChartV3 = {
      birthInfo: {
        solar: { year: input.year, month: input.month, day: input.day },
        lunar: {
          year: raw.lunarDate.lunarYear,
          month: raw.lunarDate.lunarMonth,
          day: raw.lunarDate.lunarDay,
          isLeap: raw.lunarDate.isLeap,
        },
        hour: input.hour,
        hourBranch: EARTHLY_BRANCHES[timeIndex === 12 ? 0 : timeIndex] ?? '子',
        gender: input.gender === 'male' ? '男' : '女',
      },
      fourPillars,
      fiveElement: astrolabe.fiveElementsClass ?? '',
      lifePalaceIndex,
      bodyPalaceIndex,
      palaces: groundPalaces,
      majorLimits: majorLimitsFinal,
      meta: {
        dayDivide: 'forward',
        isLeap: raw.lunarDate.isLeap,
        fixLeap: true,
        timeIndex,
        engineVersionZiwei: ENGINE_VERSION_ZIWEI,
        chartSchemaVersion: CHART_SCHEMA_VERSION,
      },
    };
    return chart;
  },
};
