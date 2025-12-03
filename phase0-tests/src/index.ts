/**
 * Phase 0: Risk Assessment Worker
 * Validates D1 compatibility, Durable Objects performance, and auth flows
 */

export interface Env {
  DB: D1Database;
  SESSION_DO: DurableObjectNamespace;
}

// UUID generator (replaces PostgreSQL gen_random_uuid())
function uuid(): string {
  return crypto.randomUUID();
}

// Schema for D1
const SCHEMA = `
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    full_name TEXT,
    subscription_tier TEXT DEFAULT 'free',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS chart_records (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chart_type TEXT NOT NULL,
    chart_name TEXT NOT NULL,
    birth_data TEXT NOT NULL,
    chart_data TEXT NOT NULL,
    is_favorite INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_charts_user ON chart_records(user_id);
`;

// Test D1 compatibility
async function testD1(db: D1Database): Promise<object> {
  const results: { test: string; passed: boolean; ms: number; error?: string }[] = [];

  // Test 1: Schema creation
  let start = Date.now();
  try {
    await db.exec(SCHEMA);
    results.push({ test: 'schema_creation', passed: true, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'schema_creation', passed: false, ms: Date.now() - start, error: String(e) });
  }

  // Test 2: Insert
  const userId = uuid();
  start = Date.now();
  try {
    await db.prepare('INSERT INTO users (id, email, full_name) VALUES (?, ?, ?)')
      .bind(userId, `test-${Date.now()}@example.com`, 'Test User')
      .run();
    results.push({ test: 'insert', passed: true, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'insert', passed: false, ms: Date.now() - start, error: String(e) });
  }

  // Test 3: Select
  start = Date.now();
  try {
    const user = await db.prepare('SELECT * FROM users WHERE id = ?').bind(userId).first();
    results.push({ test: 'select', passed: !!user, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'select', passed: false, ms: Date.now() - start, error: String(e) });
  }

  // Test 4: JSON in TEXT column
  const chartId = uuid();
  const birthData = JSON.stringify({ year: 1990, month: 5, day: 15 });
  const chartData = JSON.stringify({ stars: ['紫微', '天機'], palaces: [] });
  start = Date.now();
  try {
    await db.prepare('INSERT INTO chart_records (id, user_id, chart_type, chart_name, birth_data, chart_data) VALUES (?, ?, ?, ?, ?, ?)')
      .bind(chartId, userId, 'ziwei', 'Test Chart', birthData, chartData)
      .run();
    const chart = await db.prepare('SELECT * FROM chart_records WHERE id = ?').bind(chartId).first<{birth_data: string}>();
    const parsed = JSON.parse(chart?.birth_data || '{}');
    results.push({ test: 'json_storage', passed: parsed.year === 1990, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'json_storage', passed: false, ms: Date.now() - start, error: String(e) });
  }

  // Test 5: Pagination
  start = Date.now();
  try {
    await db.prepare('SELECT * FROM chart_records WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?')
      .bind(userId, 10, 0)
      .all();
    results.push({ test: 'pagination', passed: true, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'pagination', passed: false, ms: Date.now() - start, error: String(e) });
  }

  // Test 6: Update with datetime
  start = Date.now();
  try {
    await db.prepare("UPDATE users SET full_name = ?, updated_at = datetime('now') WHERE id = ?")
      .bind('Updated Name', userId)
      .run();
    results.push({ test: 'update_datetime', passed: true, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'update_datetime', passed: false, ms: Date.now() - start, error: String(e) });
  }

  // Test 7: JOIN
  start = Date.now();
  try {
    await db.prepare(`
      SELECT u.*, c.chart_name 
      FROM users u 
      LEFT JOIN chart_records c ON u.id = c.user_id 
      WHERE u.id = ?
    `).bind(userId).all();
    results.push({ test: 'join_query', passed: true, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'join_query', passed: false, ms: Date.now() - start, error: String(e) });
  }

  // Test 8: Aggregation
  start = Date.now();
  try {
    await db.prepare('SELECT chart_type, COUNT(*) as count FROM chart_records GROUP BY chart_type').all();
    results.push({ test: 'aggregation', passed: true, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'aggregation', passed: false, ms: Date.now() - start, error: String(e) });
  }

  // Test 9: LIKE search
  start = Date.now();
  try {
    await db.prepare('SELECT * FROM chart_records WHERE chart_name LIKE ?').bind('%Test%').all();
    results.push({ test: 'like_search', passed: true, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'like_search', passed: false, ms: Date.now() - start, error: String(e) });
  }

  // Test 10: Delete cascade
  start = Date.now();
  try {
    await db.prepare('DELETE FROM users WHERE id = ?').bind(userId).run();
    const chart = await db.prepare('SELECT * FROM chart_records WHERE id = ?').bind(chartId).first();
    results.push({ test: 'delete_cascade', passed: !chart, ms: Date.now() - start });
  } catch (e) {
    results.push({ test: 'delete_cascade', passed: false, ms: Date.now() - start, error: String(e) });
  }

  const passed = results.filter(r => r.passed).length;
  const avgMs = results.reduce((sum, r) => sum + r.ms, 0) / results.length;

  return {
    total: results.length,
    passed,
    failed: results.length - passed,
    score: Math.round((passed / results.length) * 100),
    avgResponseMs: Math.round(avgMs),
    results,
    recommendation: passed >= 9 ? 'GO' : passed >= 7 ? 'GO_WITH_CAUTION' : 'NO_GO'
  };
}

// Test Durable Objects performance
async function testDurableObjects(sessionDO: DurableObjectNamespace): Promise<object> {
  const results: { test: string; passed: boolean; ms: number }[] = [];

  // Test concurrent session creation
  const sessionIds = Array.from({ length: 10 }, () => uuid());
  
  const start = Date.now();
  try {
    const promises = sessionIds.map(async (id) => {
      const doId = sessionDO.idFromName(id);
      const stub = sessionDO.get(doId);
      return stub.fetch(new Request('http://do/create', {
        method: 'POST',
        body: JSON.stringify({ userId: id, email: `${id}@test.com` })
      }));
    });
    await Promise.all(promises);
    results.push({ test: 'concurrent_create_10', passed: true, ms: Date.now() - start });
  } catch {
    results.push({ test: 'concurrent_create_10', passed: false, ms: Date.now() - start });
  }

  // Test session retrieval
  const getStart = Date.now();
  try {
    const doId = sessionDO.idFromName(sessionIds[0]);
    const stub = sessionDO.get(doId);
    const res = await stub.fetch(new Request('http://do/get'));
    results.push({ test: 'session_get', passed: res.ok, ms: Date.now() - getStart });
  } catch {
    results.push({ test: 'session_get', passed: false, ms: Date.now() - getStart });
  }

  const passed = results.filter(r => r.passed).length;
  return {
    total: results.length,
    passed,
    score: Math.round((passed / results.length) * 100),
    results,
    recommendation: passed === results.length ? 'GO' : 'REVIEW'
  };
}

// Session Durable Object
export class SessionDO implements DurableObject {
  private state: DurableObjectState;

  constructor(state: DurableObjectState) {
    this.state = state;
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === '/create' && request.method === 'POST') {
      const data = await request.json() as { userId: string; email: string };
      const session = {
        userId: data.userId,
        email: data.email,
        createdAt: Date.now(),
        expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000,
      };
      await this.state.storage.put('session', session);
      return Response.json(session);
    }

    if (url.pathname === '/get') {
      const session = await this.state.storage.get('session');
      if (!session) return new Response('No session', { status: 404 });
      return Response.json(session);
    }

    if (url.pathname === '/destroy') {
      await this.state.storage.delete('session');
      return Response.json({ success: true });
    }

    return new Response('Not found', { status: 404 });
  }
}

// Main worker
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    // CORS headers
    const headers = {
      'Content-Type': 'application/json',
      'Access-Control-Allow-Origin': '*',
    };

    if (request.method === 'OPTIONS') {
      return new Response(null, { headers: { ...headers, 'Access-Control-Allow-Methods': 'GET, POST' } });
    }

    try {
      // Health check
      if (url.pathname === '/health') {
        return Response.json({ status: 'ok', timestamp: new Date().toISOString() }, { headers });
      }

      // D1 compatibility test
      if (url.pathname === '/test/d1') {
        const result = await testD1(env.DB);
        return Response.json(result, { headers });
      }

      // Durable Objects test
      if (url.pathname === '/test/do') {
        const result = await testDurableObjects(env.SESSION_DO);
        return Response.json(result, { headers });
      }

      // Full Phase 0 test suite
      if (url.pathname === '/test/all') {
        const d1Result = await testD1(env.DB);
        const doResult = await testDurableObjects(env.SESSION_DO);

        const d1Score = (d1Result as { score: number }).score;
        const doScore = (doResult as { score: number }).score;
        const overallScore = Math.round((d1Score + doScore) / 2);

        let decision: 'GO' | 'GO_WITH_MITIGATIONS' | 'NO_GO';
        if (d1Score >= 90 && doScore >= 80) {
          decision = 'GO';
        } else if (d1Score >= 70 && doScore >= 60) {
          decision = 'GO_WITH_MITIGATIONS';
        } else {
          decision = 'NO_GO';
        }

        return Response.json({
          phase0: 'Risk Assessment Complete',
          timestamp: new Date().toISOString(),
          d1: d1Result,
          durableObjects: doResult,
          overall: {
            score: overallScore,
            decision,
            recommendations: [
              d1Score >= 90 ? '✅ D1 compatibility excellent' : '⚠️ D1 needs workarounds',
              doScore >= 80 ? '✅ Durable Objects performing well' : '⚠️ DO needs optimization',
              decision === 'GO' ? '✅ Proceed to Phase 1' : '⚠️ Review issues before proceeding'
            ]
          }
        }, { headers });
      }

      return Response.json({ error: 'Not found', endpoints: ['/health', '/test/d1', '/test/do', '/test/all'] }, { status: 404, headers });
    } catch (e) {
      return Response.json({ error: String(e) }, { status: 500, headers });
    }
  },
};
