import { HEAVENLY_STEMS, EARTHLY_BRANCHES, PALACE_NAMES, HOUR_TO_BRANCH, WUHU_DUNYUAN } from './constants';
import { BirthData, FourPillars, Palace, Star, ZiWeiChart } from './types';
import { solarToLunar } from './lunar';

export class ZiWeiCalculator {
  calculate(data: BirthData): ZiWeiChart {
    const hourBranch = HOUR_TO_BRANCH[data.hour] || '子';
    const lunarResult = solarToLunar(data.year, data.month, data.day);
    const lunar = { year: lunarResult.year, month: lunarResult.month, day: lunarResult.day };
    const fourPillars = this.calculateFourPillars(lunar.year, lunar.month, lunar.day, hourBranch);
    const lifePalaceIndex = this.calculateLifePalace(lunar.month, hourBranch);
    const bodyPalaceIndex = this.calculateBodyPalace(lunar.month, hourBranch);
    const fiveElement = this.calculateFiveElement(fourPillars.year.stem, lifePalaceIndex);
    const palaces = this.buildPalaces(fourPillars.year.stem, lifePalaceIndex);
    
    // Place main stars
    this.placeMainStars(palaces, fiveElement, lunar.day);
    
    // Place auxiliary stars
    this.placeAuxiliaryStars(palaces, fourPillars, data.gender);

    return {
      birthInfo: {
        solar: { year: data.year, month: data.month, day: data.day },
        lunar,
        hour: data.hour,
        hourBranch,
        gender: data.gender === 'male' ? '男' : '女'
      },
      fourPillars,
      fiveElement,
      lifePalaceIndex,
      bodyPalaceIndex,
      palaces
    };
  }

  private calculateFourPillars(lunarYear: number, lunarMonth: number, lunarDay: number, hourBranch: string): FourPillars {
    // Year pillar
    const yearStemIdx = (lunarYear - 4) % 10;
    const yearBranchIdx = (lunarYear - 4) % 12;
    
    // Month pillar (simplified - uses year stem to determine month stem)
    const monthBranchIdx = (lunarMonth + 1) % 12; // 寅月=正月
    const monthStemIdx = (yearStemIdx * 2 + lunarMonth) % 10;
    
    // Day pillar (simplified calculation)
    const baseDate = new Date(1900, 0, 31);
    const targetDate = new Date(lunarYear, lunarMonth - 1, lunarDay);
    const daysDiff = Math.floor((targetDate.getTime() - baseDate.getTime()) / 86400000);
    const dayStemIdx = (daysDiff + 10) % 10;
    const dayBranchIdx = (daysDiff + 12) % 12;
    
    // Hour pillar (五鼠遁)
    const hourBranchIdx = EARTHLY_BRANCHES.indexOf(hourBranch);
    const hourStemIdx = ((dayStemIdx % 5) * 2 + hourBranchIdx) % 10;

    return {
      year: { stem: HEAVENLY_STEMS[yearStemIdx], branch: EARTHLY_BRANCHES[yearBranchIdx] },
      month: { stem: HEAVENLY_STEMS[monthStemIdx], branch: EARTHLY_BRANCHES[monthBranchIdx] },
      day: { stem: HEAVENLY_STEMS[dayStemIdx], branch: EARTHLY_BRANCHES[dayBranchIdx] },
      hour: { stem: HEAVENLY_STEMS[hourStemIdx], branch: hourBranch }
    };
  }

  private calculateLifePalace(lunarMonth: number, hourBranch: string): number {
    // 命宮 = 寅宮起正月，逆數至生月，再順數至生時
    const hourIdx = EARTHLY_BRANCHES.indexOf(hourBranch);
    const yinIdx = 2; // 寅 index
    // 從寅宮逆數月份，再順數時辰
    return (yinIdx - (lunarMonth - 1) + hourIdx + 12) % 12;
  }

  private calculateBodyPalace(lunarMonth: number, hourBranch: string): number {
    // 身宮 = 寅宮起正月，順數至生月，再順數至生時
    const hourIdx = EARTHLY_BRANCHES.indexOf(hourBranch);
    const yinIdx = 2;
    return (yinIdx + (lunarMonth - 1) + hourIdx) % 12;
  }

  private calculateFiveElement(yearStem: string, lifePalaceIdx: number): string {
    // 五行局由命宮納音決定
    const stemIdx = HEAVENLY_STEMS.indexOf(yearStem);
    const palaceBranch = EARTHLY_BRANCHES[lifePalaceIdx];
    const branchIdx = EARTHLY_BRANCHES.indexOf(palaceBranch);
    
    // 納音五行局對照表 (簡化版)
    const nayin = [
      [4, 4, 5, 5, 1, 1, 2, 2, 3, 3], // 子午
      [5, 5, 1, 1, 2, 2, 3, 3, 4, 4], // 丑未
      [1, 1, 2, 2, 3, 3, 4, 4, 5, 5], // 寅申
      [2, 2, 3, 3, 4, 4, 5, 5, 1, 1], // 卯酉
      [3, 3, 4, 4, 5, 5, 1, 1, 2, 2], // 辰戌
      [4, 4, 5, 5, 1, 1, 2, 2, 3, 3], // 巳亥
    ];
    
    const row = branchIdx % 6;
    const col = stemIdx;
    const element = nayin[row][col];
    
    const elements = ['', '水二局', '木三局', '金四局', '土五局', '火六局'];
    return elements[element] || '水二局';
  }

  private buildPalaces(yearStem: string, lifePalaceIdx: number): Palace[] {
    const palaces: Palace[] = [];
    const yinStem = WUHU_DUNYUAN[yearStem] || '丙';
    const yinStemIdx = HEAVENLY_STEMS.indexOf(yinStem);
    
    for (let i = 0; i < 12; i++) {
      const palaceIdx = (lifePalaceIdx + i) % 12;
      const branchIdx = (2 + palaceIdx) % 12; // 從寅宮開始
      const stemIdx = (yinStemIdx + palaceIdx) % 10;
      
      palaces.push({
        index: i,
        name: PALACE_NAMES[i],
        branch: EARTHLY_BRANCHES[branchIdx],
        stem: HEAVENLY_STEMS[stemIdx],
        stars: [],
        isLifePalace: i === 0
      });
    }
    return palaces;
  }

  private placeMainStars(palaces: Palace[], fiveElement: string, lunarDay: number): void {
    // 紫微星位置由五行局和農曆日決定
    const elementNum = parseInt(fiveElement.match(/\d/)?.[0] || '2');
    const ziweiPos = this.getZiweiPosition(elementNum, lunarDay);
    
    // 紫微星系 (紫微、天機、太陽、武曲、天同、廉貞)
    const ziweiStars = ['紫微', '天機', '', '太陽', '武曲', '天同', '', '', '廉貞'];
    const ziweiOffsets = [0, -1, 0, -3, -4, -5, 0, 0, -8];
    
    for (let i = 0; i < ziweiStars.length; i++) {
      if (ziweiStars[i]) {
        const pos = (ziweiPos + ziweiOffsets[i] + 12) % 12;
        palaces[pos].stars.push({ name: ziweiStars[i], type: 'main' });
      }
    }
    
    // 天府星系 (天府、太陰、貪狼、巨門、天相、天梁、七殺、破軍)
    const tianfuPos = (12 - ziweiPos + 4) % 12; // 天府與紫微對稱
    const tianfuStars = ['天府', '太陰', '貪狼', '巨門', '天相', '天梁', '七殺', '', '', '', '破軍'];
    
    for (let i = 0; i < tianfuStars.length; i++) {
      if (tianfuStars[i]) {
        const pos = (tianfuPos + i) % 12;
        palaces[pos].stars.push({ name: tianfuStars[i], type: 'main' });
      }
    }
  }

  private getZiweiPosition(elementNum: number, lunarDay: number): number {
    // 紫微星位置查表 (簡化版)
    const table: Record<number, number[]> = {
      2: [1, 2, 2, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 0],
      3: [2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 0],
      4: [2, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 0, 0, 0],
      5: [3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 0, 0, 0, 1],
      6: [3, 4, 4, 5, 5, 5, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 0, 0, 0, 1, 1, 1]
    };
    const row = table[elementNum] || table[2];
    return row[Math.min(lunarDay - 1, 29)] || 0;
  }

  private placeAuxiliaryStars(palaces: Palace[], fourPillars: FourPillars, gender: string): void {
    const yearStemIdx = HEAVENLY_STEMS.indexOf(fourPillars.year.stem);
    const yearBranchIdx = EARTHLY_BRANCHES.indexOf(fourPillars.year.branch);
    
    // 文昌 (年干決定)
    const wenchangPos = [5, 6, 8, 9, 8, 9, 11, 0, 2, 3][yearStemIdx];
    palaces[wenchangPos].stars.push({ name: '文昌', type: 'auxiliary' });
    
    // 文曲 (年干決定)
    const wenquPos = [9, 8, 6, 5, 6, 5, 3, 2, 0, 11][yearStemIdx];
    palaces[wenquPos].stars.push({ name: '文曲', type: 'auxiliary' });
    
    // 左輔 (月支決定)
    const monthBranchIdx = EARTHLY_BRANCHES.indexOf(fourPillars.month.branch);
    const zuofuPos = (4 + monthBranchIdx) % 12;
    palaces[zuofuPos].stars.push({ name: '左輔', type: 'auxiliary' });
    
    // 右弼 (月支決定)
    const youbiPos = (10 - monthBranchIdx + 12) % 12;
    palaces[youbiPos].stars.push({ name: '右弼', type: 'auxiliary' });
    
    // 祿存 (年干決定)
    const lucunPos = [2, 3, 5, 6, 5, 6, 8, 9, 11, 0][yearStemIdx];
    palaces[lucunPos].stars.push({ name: '祿存', type: 'auxiliary' });
    
    // 擎羊、陀羅 (祿存前後)
    palaces[(lucunPos + 1) % 12].stars.push({ name: '擎羊', type: 'auxiliary' });
    palaces[(lucunPos - 1 + 12) % 12].stars.push({ name: '陀羅', type: 'auxiliary' });
  }
}

export const ziWeiCalculator = new ZiWeiCalculator();
