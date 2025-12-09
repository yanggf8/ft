import { describe, it, expect } from 'vitest';

const API_URL = process.env.TEST_API_URL || 'https://fortunet-api.yanggf.workers.dev';
const RUN_INTEGRATION = process.env.RUN_INTEGRATION === 'true';

const testOrSkip = RUN_INTEGRATION ? describe : describe.skip;

testOrSkip('Charts API Integration', () => {
  let sessionId: string;
  const testEmail = `test-${Date.now()}@example.com`;

  describe('Authentication', () => {
    it('should register and create session', async () => {
      const response = await fetch(`${API_URL}/api/auth/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: testEmail })
      });

      expect(response.status).toBe(201);
      const data = await response.json() as any;
      expect(data.sessionId).toBeDefined();
      sessionId = data.sessionId;
    });

    it('should reject requests without session', async () => {
      const response = await fetch(`${API_URL}/api/users/me`, {
        method: 'GET'
      });

      expect(response.status).toBe(401);
    });

    it('should retrieve user profile with session', async () => {
      const response = await fetch(`${API_URL}/api/users/me`, {
        method: 'GET',
        headers: { 'Authorization': `Bearer ${sessionId}` }
      });

      expect(response.status).toBe(200);
      const data = await response.json() as any;
      expect(data.email).toBe(testEmail);
      expect(data.trial_ends_at).toBeDefined();
    });
  });

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

  describe('POST /api/charts/interpret', () => {
    let ziweiChart: any;

    it('should interpret ziwei chart with AI', async () => {
      // First get a chart
      const chartRes = await fetch(`${API_URL}/api/charts/calculate/ziwei`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          year: 1990,
          month: 5,
          day: 15,
          hour: 14,
          gender: 'male'
        })
      });
      ziweiChart = await chartRes.json();

      // Then interpret it
      const response = await fetch(`${API_URL}/api/charts/interpret`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${sessionId}`
        },
        body: JSON.stringify({
          chartType: 'ziwei',
          chartData: ziweiChart
        })
      });

      expect(response.status).toBe(200);
      const data = await response.json() as any;
      expect(data.interpretation).toBeDefined();
      expect(data.interpretation.length).toBeGreaterThan(50);
    }, 30000); // 30s timeout for AI

    it('should interpret western chart with AI', async () => {
      // First get a chart
      const chartRes = await fetch(`${API_URL}/api/charts/calculate/western`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          year: 1990,
          month: 3,
          day: 25,
          hour: 12
        })
      });
      const westernChart = await chartRes.json();

      // Then interpret it
      const response = await fetch(`${API_URL}/api/charts/interpret`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${sessionId}`
        },
        body: JSON.stringify({
          chartType: 'western',
          chartData: westernChart
        })
      });

      expect(response.status).toBe(200);
      const data = await response.json() as any;
      expect(data.interpretation).toBeDefined();
      expect(data.interpretation.length).toBeGreaterThan(50);
    }, 30000); // 30s timeout for AI
  });
});
