import { describe, it, expect } from 'vitest';
import { westernCalculator } from '../../services/western';

describe('Western Calculator', () => {
  it('should calculate sun sign correctly', () => {
    const result = westernCalculator.calculate({
      year: 1990,
      month: 3,
      day: 25,
      hour: 12
    });

    expect(result).toBeDefined();
    expect(result.sunSign.name).toBe('Aries');
    expect(result.sunSign.element).toBe('Fire');
  });

  it('should calculate different zodiac signs', () => {
    const testCases = [
      { month: 1, day: 15, expected: 'Capricorn' },
      { month: 4, day: 20, expected: 'Taurus' },
      { month: 7, day: 23, expected: 'Leo' },
      { month: 10, day: 30, expected: 'Scorpio' }
    ];

    testCases.forEach(({ month, day, expected }) => {
      const result = westernCalculator.calculate({
        year: 2000,
        month,
        day,
        hour: 12
      });
      expect(result.sunSign.name).toBe(expected);
    });
  });

  it('should include moon sign approximation', () => {
    const result = westernCalculator.calculate({
      year: 1995,
      month: 6,
      day: 15,
      hour: 14
    });

    expect(result.moonSign).toBeDefined();
    expect(result.moonSign.name).toBeDefined();
    expect(typeof result.moonSign.name).toBe('string');
  });

  it('should include planets data', () => {
    const result = westernCalculator.calculate({
      year: 2000,
      month: 1,
      day: 1,
      hour: 12
    });

    expect(result.planets).toBeDefined();
    expect(Array.isArray(result.planets)).toBe(true);
  });
});
