export interface BirthData {
  year: number;
  month: number;
  day: number;
  hour: number;
  gender: 'male' | 'female';
}

export interface FourPillars {
  year: { stem: string; branch: string };
  month: { stem: string; branch: string };
  day: { stem: string; branch: string };
  hour: { stem: string; branch: string };
}

export interface Star {
  name: string;
  type: 'main' | 'auxiliary' | 'transformation';
  brightness?: string;
}

export interface Palace {
  index: number;
  name: string;
  branch: string;
  stem: string;
  stars: Star[];
  isLifePalace?: boolean;
}

export interface ZiWeiChart {
  birthInfo: {
    solar: { year: number; month: number; day: number };
    lunar: { year: number; month: number; day: number };
    hour: number;
    hourBranch: string;
    gender: string;
  };
  fourPillars: FourPillars;
  fiveElement: string;
  lifePalaceIndex: number;
  bodyPalaceIndex: number;
  palaces: Palace[];
}
