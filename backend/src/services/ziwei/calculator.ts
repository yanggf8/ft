import { HEAVENLY_STEMS, EARTHLY_BRANCHES, PALACE_NAMES, HOUR_TO_BRANCH, WUHU_DUNYUAN } from './constants';
import { BirthData, FourPillars, Palace, ZiWeiChart } from './types';
import { solarToLunar } from './lunar';

// 六十甲子納音五行，每兩柱一組共30組（金火木土金｜火水土金木｜水土火木水 ×2）
const NAYIN_WUXING = [
  '金', '火', '木', '土', '金',
  '火', '水', '土', '金', '木',
  '水', '土', '火', '木', '水',
  '金', '火', '木', '土', '金',
  '火', '水', '土', '金', '木',
  '水', '土', '火', '木', '水'
];

const JU_BY_WUXING: Record<string, string> = {
  金: '金四局', 木: '木三局', 水: '水二局', 火: '火六局', 土: '土五局'
};

// 天魁／天鉞安星表（口訣：甲戊庚牛羊，乙己鼠猴鄉，丙丁豬雞位，壬癸兔蛇藏，六辛逢馬虎），依年干 甲..癸
const TIAN_KUI = ['丑', '子', '亥', '亥', '丑', '子', '亥', '午', '卯', '卯'];
const TIAN_YUE = ['未', '申', '酉', '酉', '未', '申', '酉', '寅', '巳', '巳'];

export class ZiWeiCalculator {
  calculate(data: BirthData): ZiWeiChart {
    const hourBranch = HOUR_TO_BRANCH[data.hour] || '子';
    const lunarResult = solarToLunar(data.year, data.month, data.day);
    const lunar = {
      year: lunarResult.year,
      month: lunarResult.month,
      day: lunarResult.day,
      isLeap: lunarResult.isLeap
    };
    // 閏月歸屬採十五日界慣例：閏月上半月隨本月、下半月隨次月。
    // 僅影響「月數」相關的計算（四柱月柱、命宮身宮、左輔右弼）；安星仍用實際生日數。
    const effectiveMonth = Math.min(
      lunarResult.isLeap && lunarResult.day > 15 ? lunarResult.month + 1 : lunarResult.month,
      12
    );
    const fourPillars = this.calculateFourPillars(
      data.year, data.month, data.day, lunar.year, effectiveMonth, hourBranch
    );
    const lifePalaceIndex = this.calculateLifePalace(effectiveMonth, hourBranch);
    const bodyPalaceIndex = this.calculateBodyPalace(effectiveMonth, hourBranch);
    const fiveElement = this.calculateFiveElement(lunar.year, lifePalaceIndex);
    const palaces = this.buildPalaces(lunar.year, lifePalaceIndex);

    // Place main stars
    this.placeMainStars(palaces, fiveElement, lunar.day);

    // Place auxiliary stars
    this.placeAuxiliaryStars(palaces, fourPillars, effectiveMonth, hourBranch);

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

  private calculateFourPillars(
    solarYear: number, solarMonth: number, solarDay: number,
    lunarYear: number, lunarMonth: number, hourBranch: string
  ): FourPillars {
    // Year pillar（斗數慣例以農曆正月初一換年）
    const yearStemIdx = ((lunarYear - 4) % 10 + 10) % 10;
    const yearBranchIdx = ((lunarYear - 4) % 12 + 12) % 12;

    // Month pillar：月支（正月建寅）＋ 五虎遁月干
    // 五虎遁：甲己之年丙作首、乙庚之歲戊為頭、丙辛必定尋庚起、丁壬壬位順行流、戊癸甲寅好追求
    // 月干 = ((年干 % 5) * 2 + 2 + (月 - 1)) % 10
    const monthBranchIdx = (lunarMonth + 1) % 12;
    const monthStemIdx = (((yearStemIdx % 5) * 2 + 2 + (lunarMonth - 1)) % 10 + 10) % 10;

    // Day pillar：必須以「國曆」日期計算（過去誤用農曆日期導致最多約 ±1 個月的位移）。
    // 錨點：1949-10-01 為甲子日。以 UTC 計算避免時區影響。
    const baseMs = Date.UTC(1949, 9, 1);
    const targetMs = Date.UTC(solarYear, solarMonth - 1, solarDay);
    const daysDiff = Math.round((targetMs - baseMs) / 86400000);
    const dayStemIdx = ((daysDiff % 10) + 10) % 10;
    const dayBranchIdx = ((daysDiff % 12) + 12) % 12;

    // Hour pillar (五鼠遁)：甲己還加甲，乙庚丙作初，丙辛從戊起，丁壬庚子居，戊癸何方發，壬子是真途
    const hourBranchIdx = EARTHLY_BRANCHES.indexOf(hourBranch);
    const hourStemIdx = (((dayStemIdx % 5) * 2 + hourBranchIdx) % 10 + 10) % 10;

    return {
      year: { stem: HEAVENLY_STEMS[yearStemIdx], branch: EARTHLY_BRANCHES[yearBranchIdx] },
      month: { stem: HEAVENLY_STEMS[monthStemIdx], branch: EARTHLY_BRANCHES[monthBranchIdx] },
      day: { stem: HEAVENLY_STEMS[dayStemIdx], branch: EARTHLY_BRANCHES[dayBranchIdx] },
      hour: { stem: HEAVENLY_STEMS[hourStemIdx], branch: hourBranch }
    };
  }

  private calculateLifePalace(lunarMonth: number, hourBranch: string): number {
    // 命宮：從寅宮起正月「順數」至生月，再從生月宮起子時「逆數」至生時
    const hourIdx = EARTHLY_BRANCHES.indexOf(hourBranch);
    return (2 + (lunarMonth - 1) - hourIdx + 24) % 12;
  }

  private calculateBodyPalace(lunarMonth: number, hourBranch: string): number {
    // 身宮：從寅宮起正月「順數」至生月，再從生月宮起子時「順數」至生時
    const hourIdx = EARTHLY_BRANCHES.indexOf(hourBranch);
    return (2 + (lunarMonth - 1) + hourIdx) % 12;
  }

  private nayinWuxing(stemIdx: number, branchIdx: number): string {
    // 由干支求六十甲子序號，再查納音（每兩柱一組）
    let g = 0;
    for (let i = 0; i < 60; i++) {
      if (i % 10 === stemIdx && i % 12 === branchIdx) {
        g = i;
        break;
      }
    }
    return NAYIN_WUXING[Math.floor(g / 2)];
  }

  private calculateFiveElement(lunarYear: number, lifePalaceIdx: number): string {
    // 五行局 = 命宮干支的納音五行。
    // 命宮天干由五虎遁年起寅宮順排：寅宮干 = WUHU_DUNYUAN[年干]，其餘宮位依地支順序遞推。
    const yearStemIdx = ((lunarYear - 4) % 10 + 10) % 10;
    const yinStemIdx = HEAVENLY_STEMS.indexOf(WUHU_DUNYUAN[HEAVENLY_STEMS[yearStemIdx]] || '丙');
    const stemOffsetFromYin = (lifePalaceIdx - 2 + 24) % 12;
    const palaceStemIdx = (yinStemIdx + stemOffsetFromYin) % 10;
    const wuxing = this.nayinWuxing(palaceStemIdx, lifePalaceIdx);
    return JU_BY_WUXING[wuxing] || '水二局';
  }

  private buildPalaces(lunarYear: number, lifePalaceIdx: number): Palace[] {
    // 十二宮以「地支索引」建立陣列（palaces[b] 即地支索引 b 的宮位），
    // 宮名自命宮起逆布：命宮、兄弟、夫妻……父母。
    // 宮干自五虎遁年起寅宮順排。
    const yearStemIdx = ((lunarYear - 4) % 10 + 10) % 10;
    const yinStemIdx = HEAVENLY_STEMS.indexOf(WUHU_DUNYUAN[HEAVENLY_STEMS[yearStemIdx]] || '丙');
    const palaces: Palace[] = [];

    for (let b = 0; b < 12; b++) {
      const nameIdx = (lifePalaceIdx - b + 24) % 12;
      const stemOffsetFromYin = (b - 2 + 24) % 12;
      const stemIdx = (yinStemIdx + stemOffsetFromYin) % 10;

      palaces.push({
        index: b,
        name: PALACE_NAMES[nameIdx],
        branch: EARTHLY_BRANCHES[b],
        stem: HEAVENLY_STEMS[stemIdx],
        stars: [],
        isLifePalace: b === lifePalaceIdx
      });
    }
    return palaces;
  }

  private placeMainStars(palaces: Palace[], fiveElement: string, lunarDay: number): void {
    // 紫微星位置由五行局和農曆日決定（查表值為地支索引）
    const elementNum = parseInt(fiveElement.match(/\d/)?.[0] || '2');
    const ziweiPos = this.getZiweiPosition(elementNum, lunarDay);

    // 紫微星系：自紫微起，天機逆一、太陽逆三、武曲逆四、天同逆五、廉貞逆八
    const ziweiStars = ['紫微', '天機', '', '太陽', '武曲', '天同', '', '', '廉貞'];
    const ziweiOffsets = [0, -1, 0, -3, -4, -5, 0, 0, -8];

    for (let i = 0; i < ziweiStars.length; i++) {
      if (ziweiStars[i]) {
        const pos = (ziweiPos + ziweiOffsets[i] + 24) % 12;
        palaces[pos].stars.push({ name: ziweiStars[i], type: 'main' });
      }
    }

    // 天府星系：天府與紫微對稱於寅申軸（天府 = 4 - 紫微），自天府起順行，
    // 七殺後隔三宮安破軍
    const tianfuPos = (4 - ziweiPos + 24) % 12;
    const tianfuStars = ['天府', '太陰', '貪狼', '巨門', '天相', '天梁', '七殺', '', '', '', '破軍'];

    for (let i = 0; i < tianfuStars.length; i++) {
      if (tianfuStars[i]) {
        const pos = (tianfuPos + i) % 12;
        palaces[pos].stars.push({ name: tianfuStars[i], type: 'main' });
      }
    }
  }

  private getZiweiPosition(elementNum: number, lunarDay: number): number {
    // 安紫微星訣：「六五四三二，酉午亥辰丑；局數除日數，商數宮前走；
    //   若見數無餘，便要起虎口，日數小於局，還直宮中守。」
    // 局數除生日，不足者借數 a 補足至可整除；商 Q 自寅宮起順數得基準宮；
    // 借數 a 為偶數（含 0）則自基準宮「順行」a 宮，為奇數則「逆行」a 宮。
    // （舊版硬編碼查表經逐格核對僅水二局前五格正確，已改為古典演算法）
    const day = Math.min(Math.max(lunarDay, 1), 30);
    const borrow = (elementNum - (day % elementNum)) % elementNum;
    const quotient = (day + borrow) / elementNum;
    const basePos = 2 + quotient - 1; // 寅起順數第 Q 宮
    return ((basePos + (borrow % 2 === 0 ? borrow : -borrow)) % 12 + 12) % 12;
  }

  private placeAuxiliaryStars(
    palaces: Palace[], fourPillars: FourPillars, lunarMonth: number, hourBranch: string
  ): void {
    const yearStemIdx = HEAVENLY_STEMS.indexOf(fourPillars.year.stem);
    const hourIdx = EARTHLY_BRANCHES.indexOf(hourBranch);
    const monthStep = lunarMonth - 1;

    // 文昌：從戌宮起子時逆數至生時；文曲：從辰宮起子時順數至生時
    const wenchangPos = (10 - hourIdx + 24) % 12;
    palaces[wenchangPos].stars.push({ name: '文昌', type: 'auxiliary' });

    const wenquPos = (4 + hourIdx) % 12;
    palaces[wenquPos].stars.push({ name: '文曲', type: 'auxiliary' });

    // 左輔：從辰宮起正月順數至生月；右弼：從戌宮起正月逆數至生月
    const zuofuPos = (4 + monthStep) % 12;
    palaces[zuofuPos].stars.push({ name: '左輔', type: 'auxiliary' });

    const youbiPos = (10 - monthStep + 24) % 12;
    palaces[youbiPos].stars.push({ name: '右弼', type: 'auxiliary' });

    // 天魁、天鉞（依年干）
    palaces[EARTHLY_BRANCHES.indexOf(TIAN_KUI[yearStemIdx])].stars.push({ name: '天魁', type: 'auxiliary' });
    palaces[EARTHLY_BRANCHES.indexOf(TIAN_YUE[yearStemIdx])].stars.push({ name: '天鉞', type: 'auxiliary' });

    // 祿存（依年干：甲祿到寅…），擎羊在祿存前一宮、陀羅在後一宮
    const lucunPos = [2, 3, 5, 6, 5, 6, 8, 9, 11, 0][yearStemIdx];
    palaces[lucunPos].stars.push({ name: '祿存', type: 'auxiliary' });
    palaces[(lucunPos + 1) % 12].stars.push({ name: '擎羊', type: 'auxiliary' });
    palaces[(lucunPos - 1 + 12) % 12].stars.push({ name: '陀羅', type: 'auxiliary' });

    // 地劫：從亥宮起子時順數至生時；地空：從亥宮起子時逆數至生時
    palaces[(11 + hourIdx) % 12].stars.push({ name: '地劫', type: 'auxiliary' });
    palaces[(11 - hourIdx + 24) % 12].stars.push({ name: '地空', type: 'auxiliary' });
  }
}

export const ziWeiCalculator = new ZiWeiCalculator();
