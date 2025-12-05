import { describe, it, expect } from 'vitest';

// Integration tests require deployed API - skip by default
// Run with: npm test -- --run integration
describe.skip('Charts API Integration', () => {
  const API_URL = 'https://fortunet-api.yanggf.workers.dev';

  describe('POST /api/charts/calculate/ziwei', () => {
    it('should calculate ziwei chart', async () => {
      const response = await fetch(`${API_URL}/api/charts/calculate/ziwei`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          year: 1990,
          month: 5,
          day: 15,
          hour: 14,
          gender: 'male'
        })
      });

      expect(response.status).toBe(200);
      const data = await response.json() as any;
      expect(data.birthInfo).toBeDefined();
      expect(data.palaces).toHaveLength(12);
    });

    it('should reject invalid year', async () => {
      const response = await fetch(`${API_URL}/api/charts/calculate/ziwei`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          year: 1800,
          month: 5,
          day: 15,
          hour: 14,
          gender: 'male'
        })
      });

      expect(response.status).toBe(400);
    });
  });

  describe('POST /api/charts/calculate/western', () => {
    it('should calculate western chart', async () => {
      const response = await fetch(`${API_URL}/api/charts/calculate/western`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          year: 1990,
          month: 3,
          day: 25,
          hour: 12
        })
      });

      expect(response.status).toBe(200);
      const data = await response.json() as any;
      expect(data.sunSign).toBeDefined();
      expect(data.sunSign.name).toBe('Aries');
    });
  });
});
