/**
 * Phase 0: D1 Compatibility Test
 * Tests critical queries against D1 (SQLite) to validate migration feasibility
 */

interface TestResult {
  name: string;
  passed: boolean;
  duration: number;
  error?: string;
}

interface CompatibilityReport {
  totalTests: number;
  passed: number;
  failed: number;
  score: number;
  results: TestResult[];
  recommendations: string[];
}

// Simulated D1 interface for local testing
interface D1Database {
  prepare(query: string): D1PreparedStatement;
  exec(query: string): Promise<D1ExecResult>;
  batch<T = unknown>(statements: D1PreparedStatement[]): Promise<D1Result<T>[]>;
}

interface D1PreparedStatement {
  bind(...values: unknown[]): D1PreparedStatement;
  first<T = unknown>(colName?: string): Promise<T | null>;
  run(): Promise<D1Result>;
  all<T = unknown>(): Promise<D1Result<T>>;
}

interface D1Result<T = unknown> {
  results?: T[];
  success: boolean;
  meta: { duration: number };
}

interface D1ExecResult {
  count: number;
  duration: number;
}

// Test queries converted from PostgreSQL
const TEST_QUERIES = {
  // Basic CRUD operations
  insertUser: `INSERT INTO users (id, email, full_name, subscription_tier) VALUES (?, ?, ?, ?)`,
  selectUserByEmail: `SELECT * FROM users WHERE email = ?`,
  selectUserById: `SELECT * FROM users WHERE id = ?`,
  updateUser: `UPDATE users SET full_name = ?, updated_at = datetime('now') WHERE id = ?`,
  deleteUser: `DELETE FROM users WHERE id = ?`,

  // Chart operations
  insertChart: `INSERT INTO chart_records (id, user_id, chart_type, chart_name, birth_data, chart_data) VALUES (?, ?, ?, ?, ?, ?)`,
  selectChartsByUser: `SELECT * FROM chart_records WHERE user_id = ? ORDER BY created_at DESC LIMIT ?`,
  selectChartById: `SELECT * FROM chart_records WHERE id = ?`,
  selectFavoriteCharts: `SELECT * FROM chart_records WHERE user_id = ? AND is_favorite = 1`,
  updateChartFavorite: `UPDATE chart_records SET is_favorite = ?, updated_at = datetime('now') WHERE id = ?`,

  // Subscription operations
  insertSubscription: `INSERT INTO subscriptions (id, user_id, stripe_subscription_id, tier, status) VALUES (?, ?, ?, ?, ?)`,
  selectActiveSubscription: `SELECT * FROM subscriptions WHERE user_id = ? AND status = 'active' LIMIT 1`,

  // Usage tracking
  insertUsage: `INSERT INTO usage_tracking (id, user_id, app_type, feature_type, usage_date) VALUES (?, ?, ?, ?, date('now'))`,
  selectUsageByDate: `SELECT * FROM usage_tracking WHERE user_id = ? AND usage_date = date('now')`,
  countUsageByFeature: `SELECT feature_type, COUNT(*) as count FROM usage_tracking WHERE user_id = ? GROUP BY feature_type`,

  // Complex queries
  selectChartsWithPagination: `SELECT * FROM chart_records WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?`,
  selectRecentCharts: `SELECT * FROM chart_records WHERE user_id = ? AND created_at > datetime('now', '-7 days')`,
  searchChartsByName: `SELECT * FROM chart_records WHERE user_id = ? AND chart_name LIKE ?`,

  // Join queries
  selectUserWithSubscription: `
    SELECT u.*, s.tier, s.status as sub_status 
    FROM users u 
    LEFT JOIN subscriptions s ON u.id = s.user_id AND s.status = 'active'
    WHERE u.id = ?
  `,
  selectChartsWithFavorites: `
    SELECT c.*, f.id as favorite_id 
    FROM chart_records c 
    LEFT JOIN favorites f ON c.id = f.chart_id AND f.user_id = ?
    WHERE c.user_id = ?
    ORDER BY c.created_at DESC
  `,

  // Aggregation queries
  countChartsByType: `SELECT chart_type, COUNT(*) as count FROM chart_records WHERE user_id = ? GROUP BY chart_type`,
  sumUsageByApp: `SELECT app_type, SUM(usage_count) as total FROM usage_tracking WHERE user_id = ? GROUP BY app_type`,
};

// Generate UUID (D1 doesn't have gen_random_uuid())
function generateUUID(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

// Test runner
async function runTest(
  name: string,
  testFn: () => Promise<void>
): Promise<TestResult> {
  const start = Date.now();
  try {
    await testFn();
    return { name, passed: true, duration: Date.now() - start };
  } catch (e) {
    return {
      name,
      passed: false,
      duration: Date.now() - start,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

// Main compatibility test function
export async function runD1CompatibilityTests(
  db: D1Database
): Promise<CompatibilityReport> {
  const results: TestResult[] = [];
  const recommendations: string[] = [];

  // Test 1: Schema creation
  results.push(
    await runTest('Schema Creation', async () => {
      const schema = await import('./schema.sql?raw');
      await db.exec(schema.default);
    })
  );

  // Test 2: Basic CRUD - Users
  const testUserId = generateUUID();
  results.push(
    await runTest('Insert User', async () => {
      await db
        .prepare(TEST_QUERIES.insertUser)
        .bind(testUserId, 'test@example.com', 'Test User', 'free')
        .run();
    })
  );

  results.push(
    await runTest('Select User by Email', async () => {
      const user = await db
        .prepare(TEST_QUERIES.selectUserByEmail)
        .bind('test@example.com')
        .first();
      if (!user) throw new Error('User not found');
    })
  );

  results.push(
    await runTest('Update User', async () => {
      await db
        .prepare(TEST_QUERIES.updateUser)
        .bind('Updated Name', testUserId)
        .run();
    })
  );

  // Test 3: JSON storage in TEXT columns
  const testChartId = generateUUID();
  const birthData = JSON.stringify({
    year: 1990,
    month: 5,
    day: 15,
    hour: 10,
    gender: 'male',
  });
  const chartData = JSON.stringify({
    lifePalace: '命宮',
    stars: ['紫微', '天機'],
  });

  results.push(
    await runTest('Insert Chart with JSON', async () => {
      await db
        .prepare(TEST_QUERIES.insertChart)
        .bind(testChartId, testUserId, 'ziwei', 'Test Chart', birthData, chartData)
        .run();
    })
  );

  results.push(
    await runTest('Select and Parse JSON', async () => {
      const chart = await db
        .prepare(TEST_QUERIES.selectChartById)
        .bind(testChartId)
        .first<{ birth_data: string; chart_data: string }>();
      if (!chart) throw new Error('Chart not found');
      const parsed = JSON.parse(chart.birth_data);
      if (parsed.year !== 1990) throw new Error('JSON parsing failed');
    })
  );

  // Test 4: Pagination
  results.push(
    await runTest('Pagination Query', async () => {
      const charts = await db
        .prepare(TEST_QUERIES.selectChartsWithPagination)
        .bind(testUserId, 10, 0)
        .all();
      if (!charts.success) throw new Error('Pagination failed');
    })
  );

  // Test 5: Date functions
  results.push(
    await runTest('Date Functions', async () => {
      const usageId = generateUUID();
      await db
        .prepare(TEST_QUERIES.insertUsage)
        .bind(usageId, testUserId, 'ziwei', 'chart_creation')
        .run();
      const usage = await db
        .prepare(TEST_QUERIES.selectUsageByDate)
        .bind(testUserId)
        .all();
      if (!usage.success) throw new Error('Date query failed');
    })
  );

  // Test 6: JOIN queries
  results.push(
    await runTest('LEFT JOIN Query', async () => {
      const result = await db
        .prepare(TEST_QUERIES.selectUserWithSubscription)
        .bind(testUserId)
        .first();
      if (!result) throw new Error('JOIN query failed');
    })
  );

  // Test 7: Aggregation
  results.push(
    await runTest('GROUP BY Aggregation', async () => {
      const result = await db
        .prepare(TEST_QUERIES.countChartsByType)
        .bind(testUserId)
        .all();
      if (!result.success) throw new Error('Aggregation failed');
    })
  );

  // Test 8: LIKE search
  results.push(
    await runTest('LIKE Search', async () => {
      const result = await db
        .prepare(TEST_QUERIES.searchChartsByName)
        .bind(testUserId, '%Test%')
        .all();
      if (!result.success) throw new Error('LIKE search failed');
    })
  );

  // Test 9: Batch operations
  results.push(
    await runTest('Batch Insert', async () => {
      const statements = [
        db.prepare(TEST_QUERIES.insertUsage).bind(generateUUID(), testUserId, 'ziwei', 'ai_analysis'),
        db.prepare(TEST_QUERIES.insertUsage).bind(generateUUID(), testUserId, 'zodiac', 'chart_view'),
      ];
      await db.batch(statements);
    })
  );

  // Test 10: Cleanup
  results.push(
    await runTest('Delete Cascade', async () => {
      await db.prepare(TEST_QUERIES.deleteUser).bind(testUserId).run();
      const chart = await db
        .prepare(TEST_QUERIES.selectChartById)
        .bind(testChartId)
        .first();
      if (chart) throw new Error('CASCADE delete failed');
    })
  );

  // Generate recommendations
  const failedTests = results.filter((r) => !r.passed);
  if (failedTests.length > 0) {
    recommendations.push(
      `${failedTests.length} tests failed - review and fix before proceeding`
    );
    failedTests.forEach((t) => {
      recommendations.push(`- ${t.name}: ${t.error}`);
    });
  }

  const passed = results.filter((r) => r.passed).length;
  const score = Math.round((passed / results.length) * 100);

  if (score >= 90) {
    recommendations.push('✅ D1 compatibility is excellent - proceed with migration');
  } else if (score >= 70) {
    recommendations.push('⚠️ D1 compatibility is acceptable with workarounds');
  } else {
    recommendations.push('❌ D1 compatibility issues - review migration strategy');
  }

  return {
    totalTests: results.length,
    passed,
    failed: results.length - passed,
    score,
    results,
    recommendations,
  };
}

// Export for use in Workers
export { generateUUID, TEST_QUERIES };
