/**
 * Western Astrology Calculator
 * Calculates sun sign, moon sign (approximate), and basic natal chart info
 */

export interface WesternBirthData {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute?: number;
  latitude?: number;
  longitude?: number;
}

export interface WesternChart {
  sunSign: { name: string; symbol: string; element: string; quality: string };
  moonSign: { name: string; symbol: string }; // Approximate
  ascendant: { name: string; symbol: string } | null; // Requires location
  planets: Planet[];
}

interface Planet {
  name: string;
  sign: string;
  degree: number;
}

const ZODIAC_SIGNS = [
  { name: 'Aries', symbol: '♈', element: 'Fire', quality: 'Cardinal', start: [3, 21], end: [4, 19] },
  { name: 'Taurus', symbol: '♉', element: 'Earth', quality: 'Fixed', start: [4, 20], end: [5, 20] },
  { name: 'Gemini', symbol: '♊', element: 'Air', quality: 'Mutable', start: [5, 21], end: [6, 20] },
  { name: 'Cancer', symbol: '♋', element: 'Water', quality: 'Cardinal', start: [6, 21], end: [7, 22] },
  { name: 'Leo', symbol: '♌', element: 'Fire', quality: 'Fixed', start: [7, 23], end: [8, 22] },
  { name: 'Virgo', symbol: '♍', element: 'Earth', quality: 'Mutable', start: [8, 23], end: [9, 22] },
  { name: 'Libra', symbol: '♎', element: 'Air', quality: 'Cardinal', start: [9, 23], end: [10, 22] },
  { name: 'Scorpio', symbol: '♏', element: 'Water', quality: 'Fixed', start: [10, 23], end: [11, 21] },
  { name: 'Sagittarius', symbol: '♐', element: 'Fire', quality: 'Mutable', start: [11, 22], end: [12, 21] },
  { name: 'Capricorn', symbol: '♑', element: 'Earth', quality: 'Cardinal', start: [12, 22], end: [1, 19] },
  { name: 'Aquarius', symbol: '♒', element: 'Air', quality: 'Fixed', start: [1, 20], end: [2, 18] },
  { name: 'Pisces', symbol: '♓', element: 'Water', quality: 'Mutable', start: [2, 19], end: [3, 20] },
];

export class WesternCalculator {
  calculate(data: WesternBirthData): WesternChart {
    const sunSign = this.getSunSign(data.month, data.day);
    const moonSign = this.getApproximateMoonSign(data.year, data.month, data.day);
    const ascendant = data.latitude && data.longitude 
      ? this.getApproximateAscendant(data.hour, data.latitude)
      : null;

    return {
      sunSign,
      moonSign,
      ascendant,
      planets: this.getBasicPlanets(data)
    };
  }

  private getSunSign(month: number, day: number) {
    // 以「各星座起始日」判斷，含跨年環繞（摩羯 12/22 起）。
    // ord = 月*100+日 便於比較；舊版的區間寫法會讓 1/20–1/31 提前落入摩羯。
    const ord = month * 100 + day;

    for (let i = 0; i < ZODIAC_SIGNS.length; i++) {
      const cur = ZODIAC_SIGNS[i];
      const next = ZODIAC_SIGNS[(i + 1) % ZODIAC_SIGNS.length];
      const start = cur.start[0] * 100 + cur.start[1];
      const end = next.start[0] * 100 + next.start[1]; // 摩羯跨年時 end < start
      const inRange = start < end ? ord >= start && ord < end : ord >= start || ord < end;
      if (inRange) {
        return { name: cur.name, symbol: cur.symbol, element: cur.element, quality: cur.quality };
      }
    }

    // 理論上不可達（範圍涵蓋全年）；防禦性預設
    const sign = ZODIAC_SIGNS[0];
    return { name: sign.name, symbol: sign.symbol, element: sign.element, quality: sign.quality };
  }

  private getApproximateMoonSign(year: number, month: number, day: number) {
    // Simplified moon sign calculation (moon moves ~13° per day, full cycle ~27.3 days)
    const baseDate = new Date(2000, 0, 6); // Known new moon in Capricorn
    const targetDate = new Date(year, month - 1, day);
    const daysDiff = Math.floor((targetDate.getTime() - baseDate.getTime()) / 86400000);
    const moonCycle = 27.3;
    const daysInCycle = ((daysDiff % moonCycle) + moonCycle) % moonCycle; // Handle negative
    const signIndex = Math.floor((daysInCycle / moonCycle) * 12);
    const sign = ZODIAC_SIGNS[signIndex] || ZODIAC_SIGNS[0];
    return { name: sign.name, symbol: sign.symbol };
  }

  private getApproximateAscendant(hour: number, latitude: number) {
    // Very simplified - ascendant changes every ~2 hours
    // This is a rough approximation, real calculation needs ephemeris
    const signIndex = Math.floor(hour / 2) % 12;
    const sign = ZODIAC_SIGNS[signIndex];
    return { name: sign.name, symbol: sign.symbol };
  }

  private getBasicPlanets(data: WesternBirthData): Planet[] {
    // Simplified - just return sun position
    // Real planetary positions require ephemeris data
    const sunSign = this.getSunSign(data.month, data.day);
    const dayInSign = this.getDayInSign(data.month, data.day);
    
    return [
      { name: 'Sun', sign: sunSign.name, degree: dayInSign },
      { name: 'Moon', sign: this.getApproximateMoonSign(data.year, data.month, data.day).name, degree: 0 }
    ];
  }

  private getDayInSign(month: number, day: number): number {
    // Simplified - return approximate degree (0-29)
    const sunSign = this.getSunSign(month, day);
    const signIndex = ZODIAC_SIGNS.findIndex(s => s.name === sunSign.name);
    if (signIndex === -1) return 0;
    
    const sign = ZODIAC_SIGNS[signIndex];
    const [startMonth, startDay] = sign.start;
    
    if (month === startMonth) {
      return Math.min(day - startDay, 29);
    }
    // Second month of sign
    const daysInPrevMonth = new Date(2000, startMonth, 0).getDate();
    return Math.min((daysInPrevMonth - startDay) + day, 29);
  }
}

export const westernCalculator = new WesternCalculator();
