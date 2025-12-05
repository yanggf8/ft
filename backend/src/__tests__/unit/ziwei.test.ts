import { describe, it, expect } from 'vitest';
import { ziWeiCalculator } from '../../services/ziwei';

describe('ZiWei Calculator', () => {
  it('should calculate chart for valid birth data', () => {
    const result = ziWeiCalculator.calculate({
      year: 1990,
      month: 5,
      day: 15,
      hour: 14,
      gender: 'male'
    });

    expect(result).toBeDefined();
    expect(result.birthInfo).toBeDefined();
    expect(result.birthInfo.solar).toEqual({ year: 1990, month: 5, day: 15 });
    expect(result.birthInfo.lunar).toBeDefined();
    expect(result.fourPillars).toBeDefined();
    expect(result.lifePalaceIndex).toBeGreaterThanOrEqual(0);
    expect(result.bodyPalaceIndex).toBeGreaterThanOrEqual(0);
    expect(result.palaces).toHaveLength(12);
  });

  it('should handle female gender', () => {
    const result = ziWeiCalculator.calculate({
      year: 1995,
      month: 8,
      day: 20,
      hour: 10,
      gender: 'female'
    });

    expect(result).toBeDefined();
    expect(result.birthInfo.solar.year).toBe(1995);
    expect(result.birthInfo.gender).toBe('女');
  });

  it('should handle edge case dates', () => {
    const result = ziWeiCalculator.calculate({
      year: 2000,
      month: 2,
      day: 29, // leap year
      hour: 23,
      gender: 'male'
    });

    expect(result).toBeDefined();
    expect(result.birthInfo.solar.day).toBe(29);
  });

  it('should place stars in palaces', () => {
    const result = ziWeiCalculator.calculate({
      year: 1990,
      month: 5,
      day: 15,
      hour: 14,
      gender: 'male'
    });

    const hasStars = result.palaces.some(p => p.stars && p.stars.length > 0);
    expect(hasStars).toBe(true);
  });
});
