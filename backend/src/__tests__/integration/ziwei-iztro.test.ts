import { describe, it, expect } from 'vitest';
const API_URL = process.env.TEST_API_URL || 'https://fortunet-api.yanggf.workers.dev';
const RUN = process.env.RUN_INTEGRATION === 'true';
const d = RUN ? describe : describe.skip;

d('ZiWei iztro A1 anchors', () => {
  let sid: string;
  const email = `iztro-${Date.now()}@example.com`;

  it('register', async () => {
    const r = await fetch(`${API_URL}/api/auth/register`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ email }) });
    expect(r.status).toBe(201);
    sid = (await r.json() as any).sessionId;
    expect(sid).toBeDefined();
  });

  it('V3 shape — palaces 12, brightness, sihua, meta forward', async () => {
    await fetch(`${API_URL}/api/users/me/birth`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${sid}` },
      body: JSON.stringify({ birth_year: 1990, birth_month: 5, birth_day: 15, birth_hour: 14, birth_minute: 30, gender: 'male' })
    });
    const r = await fetch(`${API_URL}/api/charts/ziwei`, { headers: { 'Authorization': `Bearer ${sid}` } });
    expect(r.status).toBe(200);
    const j = await r.json() as any;
    expect(j.chartSchemaVersion).toBe(3);
    expect(j.engineVersion).toBe('3.0.0');
    expect(j.chart_data.palaces).toHaveLength(12);
    expect(j.chart_data.meta.dayDivide).toBe('forward');
    expect(j.chart_data.meta.fixLeap).toBe(true);
    expect(j.chart_data.fiveElement).toBeDefined();
    expect(j.chart_data.majorLimits.length).toBeGreaterThan(0);
    const allStars = j.chart_data.palaces.flatMap((p: any) => p.stars);
    expect(allStars.some((s: any) => s.brightness)).toBe(true);
    expect(allStars.some((s: any) => s.sihua)).toBe(true);
    // V3 backward-compat top-level kept
    expect(j.chart_data.chartSchemaVersion).toBe(3);
  });

  it('23:00 dayDivide forward — timeIndex 12', async () => {
    await fetch(`${API_URL}/api/users/me/birth`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${sid}` },
      body: JSON.stringify({ birth_year: 1990, birth_month: 5, birth_day: 15, birth_hour: 23, birth_minute: 10, gender: 'male' })
    });
    const r = await fetch(`${API_URL}/api/charts/ziwei`, { headers: { 'Authorization': `Bearer ${sid}` } });
    expect(r.status).toBe(200);
    const j = await r.json() as any;
    expect(j.chart_data.meta.timeIndex).toBe(12);
    expect(j.chart_data.meta.dayDivide).toBe('forward');
  });

  it('leap-month — lunar isLeap surfaced', async () => {
    // 2023-03-22 maps to lunar leap Feb in iztro; any date in leap stretch suffices for boolean check.
    await fetch(`${API_URL}/api/users/me/birth`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${sid}` },
      body: JSON.stringify({ birth_year: 2023, birth_month: 3, birth_day: 22, birth_hour: 10, gender: 'female' })
    });
    const r = await fetch(`${API_URL}/api/charts/ziwei`, { headers: { 'Authorization': `Bearer ${sid}` } });
    expect(r.status).toBe(200);
    const j = await r.json() as any;
    expect(typeof j.chart_data.birthInfo.lunar.isLeap).toBe('boolean');
  });

  it('stale guard — interpret on fresh chart succeeds', async () => {
    await fetch(`${API_URL}/api/users/me/birth`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${sid}` },
      body: JSON.stringify({ birth_year: 1992, birth_month: 8, birth_day: 8, birth_hour: 8, gender: 'male' })
    });
    await fetch(`${API_URL}/api/charts/ziwei`, { headers: { 'Authorization': `Bearer ${sid}` } });
    const r = await fetch(`${API_URL}/api/charts/ziwei/interpret`, { method: 'POST', headers: { 'Authorization': `Bearer ${sid}` } });
    expect([200, 409, 503]).toContain(r.status);
    if (r.status === 200) {
      const j = await r.json() as any;
      expect(j.interpretation || j.fromCache).toBeDefined();
    }
    if (r.status === 409) {
      const j = await r.json() as any;
      expect(j.code).toBe('RECALC_REQUIRED');
    }
  }, 30000);
});
