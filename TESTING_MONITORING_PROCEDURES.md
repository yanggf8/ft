# 🧪 Testing & Monitoring Procedures

## 📋 Overview

Comprehensive testing and monitoring procedures for the FortuneT V2 Cloudflare migration. This document covers all testing phases, monitoring setup, alerting strategies, and quality assurance processes to ensure a successful migration.

## 🎯 Testing Strategy

### Testing Pyramid

```
                    ┌─────────────────┐
                    │  E2E Tests (5%) │
                    └─────────────────┘
                ┌─────────────────────────────┐
                │   Integration Tests (15%)   │
                └─────────────────────────────┘
        ┌─────────────────────────────────────────────┐
        │           Unit Tests (80%)                   │
        └─────────────────────────────────────────────┘
```

### Coverage Targets

| Test Type | Target Coverage | Tools |
|-----------|-----------------|-------|
| **Unit Tests** | 90%+ line coverage | Vitest, @cloudflare/vitest-pool-workers |
| **Integration Tests** | 85%+ API coverage | Vitest, Test Containers |
| **E2E Tests** | 100% critical user flows | Playwright, Cypress |
| **Performance Tests** | All endpoints under load | Artillery, k6 |
| **Security Tests** | 100% OWASP Top 10 | OWASP ZAP, Burp Suite |

## 🧪 Phase 0: Validation Testing

### D1 Compatibility Testing

```typescript
// tests/d1-compatibility.test.ts
import { describe, test, expect } from 'vitest';
import { D1CompatibilityTester } from '../scripts/d1-compatibility-test';

describe('D1 Database Compatibility', () => {
  const tester = new D1CompatibilityTester();

  test('complex joins work correctly', async () => {
    const result = await tester.testQueryCompatibility(`
      SELECT u.*, up.preferences, up.subscription_status
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.created_at >= '2024-01-01'
      ORDER BY u.email
      LIMIT 10
    `);

    expect(result.compatibility_score).toBeGreaterThan(80);
    expect(result.performance_ms).toBeLessThan(100);
  });

  test('json operations work within limits', async () => {
    const result = await tester.testQueryCompatibility(`
      SELECT id, JSON_EXTRACT(preferences, '$.language') as language
      FROM user_profiles
    `);

    expect(result.compatibility_score).toBeGreaterThan(70);
    expect(result.issues).not.toContain('JSON not supported');
  });

  test('performance meets targets', async () => {
    const benchmark = await tester.benchmarkD1Operations();

    expect(benchmark.actual_p95_response_time_ms).toBeLessThan(200);
    expect(benchmark.meets_performance_target).toBe(true);
  });
});
```

### OAuth Migration Testing

```typescript
// tests/oauth-migration.test.ts
import { describe, test, expect, beforeEach } from 'vitest';
import { OAuthMigrationValidator } from '../scripts/oauth-migration-test';

describe('OAuth Migration Validation', () => {
  let validator: OAuthMigrationValidator;

  beforeEach(() => {
    validator = new OAuthMigrationValidator();
  });

  test('user account migration works correctly', async () => {
    const result = await validator.testUserAccountMigration();

    expect(result.success_rate).toBeGreaterThan(95);
    expect(result.failed_migrations).toBeLessThan(1);
    expect(result.issues).toHaveLength(0);
  });

  test('account linking prevents duplicates', async () => {
    // Test that same user can't create multiple accounts
  });

  test('data consistency is maintained', async () => {
    // Test that all user data is preserved
  });
});
```

### Durable Objects Performance Testing

```typescript
// tests/durable-objects-performance.test.ts
import { describe, test, expect } from 'vitest';
import { DurableObjectsPerformanceTest } from '../scripts/do-performance-test';

describe('Durable Objects Performance', () => {
  const perfTest = new DurableObjectsPerformanceTest();

  test('handles 1000 concurrent users', async () => {
    const result = await perfTest.testDurableObjectsLimits();

    expect(result.meets_performance_target).toBe(true);
    expect(result.actual_degradation).toBeLessThan(20);
    expect(result.max_concurrent_users_supported).toBeGreaterThanOrEqual(1000);
  });

  test('memory usage stays within limits', async () => {
    const result = await perfTest.testDurableObjectsLimits();

    expect(result.memory_usage_sustainable).toBe(true);
  });

  test('failover scenarios work correctly', async () => {
    // Test recovery procedures
  });
});
```

## 🔧 Unit Testing Framework

### Backend Testing Setup

```typescript
// vitest.config.ts
import { defineConfig } from 'vitest/config';
import { cloudflareWorkers } from '@cloudflare/vitest-pool-workers';

export default defineConfig({
  test: {
    globals: true,
    pool: cloudflareWorkers(),
    poolOptions: {
      workers: {
        wrangler: { configPath: './wrangler.toml' },
        miniflare: {
          bindings: {
            // Mock D1, R2, DO bindings
          }
        }
      }
    }
  }
});
```

### Service Layer Testing

```typescript
// tests/services/chart-calculation.test.ts
import { describe, test, expect, beforeEach } from 'vitest';
import { ZiWeiCalculationService } from '../src/services/ziwei-calculation';

describe('Zi Wei Calculation Service', () => {
  let service: ZiWeiCalculationService;

  beforeEach(() => {
    service = new ZiWeiCalculationService();
  });

  test('calculates zi wei chart correctly', async () => {
    const input = {
      birthYear: 1990,
      birthMonth: 6,
      birthDay: 15,
      birthHour: 14,
      gender: 'male',
      timezone: 'Asia/Taipei'
    };

    const result = await service.calculateChart(input);

    expect(result).toBeDefined();
    expect(result.palaces).toHaveLength(12);
    expect(result.mainStars).toBeDefined();
    expect(result.calculationTimeMs).toBeLessThan(100);
  });

  test('handles invalid input gracefully', async () => {
    const invalidInput = {
      birthYear: 1800, // Invalid year
      birthMonth: 13,  // Invalid month
      birthDay: 32,    // Invalid day
      birthHour: 25,   // Invalid hour
      gender: 'unknown',
      timezone: 'Invalid/Timezone'
    };

    await expect(service.calculateChart(invalidInput))
      .rejects.toThrow('Invalid birth date');
  });

  test('caching works correctly', async () => {
    const input = {
      birthYear: 1990,
      birthMonth: 6,
      birthDay: 15,
      birthHour: 14,
      gender: 'male',
      timezone: 'Asia/Taipei'
    };

    // First call
    const start1 = Date.now();
    const result1 = await service.calculateChart(input);
    const time1 = Date.now() - start1;

    // Second call (should be cached)
    const start2 = Date.now();
    const result2 = await service.calculateChart(input);
    const time2 = Date.now() - start2;

    expect(result1).toEqual(result2);
    expect(time2).toBeLessThan(time1);
  });
});
```

### API Endpoint Testing

```typescript
// tests/routes/charts.test.ts
import { describe, test, expect, beforeEach } from 'vitest';
import app from '../src';
import { createTestContext } from './helpers/test-context';

describe('/charts API endpoints', () => {
  let testEnv: any;

  beforeEach(() => {
    testEnv = createTestContext();
  });

  test('POST /charts creates new chart', async () => {
    const requestBody = {
      type: 'ziwei',
      birthData: {
        birthYear: 1990,
        birthMonth: 6,
        birthDay: 15,
        birthHour: 14,
        gender: 'male',
        timezone: 'Asia/Taipei'
      }
    };

    const response = await app.request('/charts', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Bearer test-token'
      },
      body: JSON.stringify(requestBody)
    }, testEnv);

    expect(response.status).toBe(201);
    const data = await response.json();
    expect(data.success).toBe(true);
    expect(data.data.chart).toBeDefined();
    expect(data.data.chart.id).toBeDefined();
  });

  test('GET /charts/:id returns chart data', async () => {
    const chartId = 'test-chart-id';

    const response = await app.request(`/charts/${chartId}`, {
      method: 'GET',
      headers: {
        'Authorization': 'Bearer test-token'
      }
    }, testEnv);

    expect(response.status).toBe(200);
    const data = await response.json();
    expect(data.success).toBe(true);
    expect(data.data.chart.id).toBe(chartId);
  });

  test('handles invalid chart ID', async () => {
    const response = await app.request('/charts/invalid-id', {
      method: 'GET',
      headers: {
        'Authorization': 'Bearer test-token'
      }
    }, testEnv);

    expect(response.status).toBe(404);
    const data = await response.json();
    expect(data.success).toBe(false);
    expect(data.error).toContain('Chart not found');
  });
});
```

## 🔗 Integration Testing

### Database Integration Tests

```typescript
// tests/integration/database.test.ts
import { describe, test, expect, beforeEach, afterEach } from 'vitest';
import { createClient } from '@libsql/client';

describe('Database Integration', () => {
  let db: any;

  beforeEach(async () => {
    db = createClient({
      url: ':memory:'
    });

    // Setup test schema
    await db.execute(`
      CREATE TABLE users (
        id TEXT PRIMARY KEY,
        email TEXT UNIQUE NOT NULL,
        created_at TEXT NOT NULL
      );
    `);
  });

  afterEach(async () => {
    await db.close();
  });

  test('user creation and retrieval', async () => {
    const userId = 'test-user-id';
    const email = 'test@example.com';

    // Insert user
    await db.execute({
      sql: 'INSERT INTO users (id, email, created_at) VALUES (?, ?, ?)',
      args: [userId, email, new Date().toISOString()]
    });

    // Retrieve user
    const result = await db.execute({
      sql: 'SELECT * FROM users WHERE id = ?',
      args: [userId]
    });

    expect(result.rows).toHaveLength(1);
    expect(result.rows[0].email).toBe(email);
  });

  test('handles database constraints', async () => {
    const email = 'test@example.com';

    // Insert first user
    await db.execute({
      sql: 'INSERT INTO users (id, email, created_at) VALUES (?, ?, ?)',
      args: ['user-1', email, new Date().toISOString()]
    });

    // Try to insert duplicate email
    await expect(
      db.execute({
        sql: 'INSERT INTO users (id, email, created_at) VALUES (?, ?, ?)',
        args: ['user-2', email, new Date().toISOString()]
      })
    ).rejects.toThrow();
  });
});
```

### Durable Objects Integration Tests

```typescript
// tests/integration/durable-objects.test.ts
import { describe, test, expect, beforeEach } from 'vitest';
import { SessionManagerDO } from '../src/durable-objects/session-manager';

describe('Session Manager Durable Object', () => {
  let do: DurableObjectStub;
  let env: any;

  beforeEach(() => {
    env = getMiniflareBindings();
    do = new DurableObjectStub(env, env.SESSION_MANAGER);
  });

  test('creates and retrieves sessions', async () => {
    const userId = 'test-user-id';
    const sessionData = {
      userId,
      email: 'test@example.com',
      subscriptionStatus: 'premium',
      activeCharts: [],
      lastActivity: Date.now()
    };

    // Create session
    await do.fetch(new Request('https://example.com/session', {
      method: 'POST',
      body: JSON.stringify(sessionData)
    }));

    // Retrieve session
    const response = await do.fetch(new Request(`https://example.com/session/${userId}`));
    const data = await response.json();

    expect(data.success).toBe(true);
    expect(data.session.email).toBe('test@example.com');
  });

  test('handles session expiration', async () => {
    // Test session cleanup
  });
});
```

## 🌐 End-to-End Testing

### Critical User Journey Tests

```typescript
// tests/e2e/user-journeys.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Critical User Journeys', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test
    await page.goto('/login');
    await page.fill('[data-testid="email"]', 'test@example.com');
    await page.click('[data-testid="login-button"]');
    await page.waitForURL('/dashboard');
  });

  test('chart creation and sharing', async ({ page }) => {
    // Create new chart
    await page.click('[data-testid="create-chart-button"]');
    await page.selectOption('[data-testid="chart-type"]', 'ziwei');
    await page.fill('[data-testid="birth-year"]', '1990');
    await page.fill('[data-testid="birth-month"]', '6');
    await page.fill('[data-testid="birth-day"]', '15');
    await page.click('[data-testid="calculate-button"]');

    // Wait for calculation
    await page.waitForSelector('[data-testid="chart-results"]');
    await expect(page.locator('[data-testid="chart-title"]')).toContainText('紫微斗數');

    // Save chart
    await page.fill('[data-testid="chart-name"]', 'Test Chart');
    await page.click('[data-testid="save-chart-button"]');

    // Share chart
    await page.click('[data-testid="share-button"]');
    const shareUrl = await page.inputValue('[data-testid="share-url"]');
    expect(shareUrl).toContain('https://');

    // Test sharing link
    await page.goto(shareUrl);
    await expect(page.locator('[data-testid="shared-chart"]')).toBeVisible();
  });

  test('subscription upgrade flow', async ({ page }) => {
    // Navigate to subscription page
    await page.click('[data-testid="upgrade-button"]');
    await page.waitForURL('/subscription');

    // Select plan
    await page.click('[data-testid="premium-plan"]');
    await page.click('[data-testid="subscribe-button"]');

    // Mock Stripe checkout (in test environment)
    await page.fill('[data-testid="card-number"]', '4242424242424242');
    await page.fill('[data-testid="card-expiry"]', '12/25');
    await page.fill('[data-testid="card-cvc"]', '123');
    await page.click('[data-testid="complete-payment"]');

    // Verify subscription activated
    await page.waitForURL('/dashboard');
    await expect(page.locator('[data-testid="subscription-status"]')).toContainText('Premium');
  });

  test('real-time collaboration', async ({ page, context }) => {
    // Create chart
    await page.goto('/charts/new');
    await page.fill('[data-testid="chart-name"]', 'Collaboration Test');
    await page.click('[data-testid="save-chart"]');

    // Get share link
    await page.click('[data-testid="collaborate-button"]');
    const shareUrl = await page.inputValue('[data-testid="collaborate-url"]');

    // Open collaboration in second browser
    const page2 = await context.newPage();
    await page2.goto(shareUrl);
    await page2.click('[data-testid="join-collaboration"]');

    // Test real-time features
    await page.click('[data-testid="add-element"]');
    await page2.waitForSelector('[data-testid="new-element"]');
    await expect(page2.locator('[data-testid="new-element"]')).toBeVisible();
  });
});
```

## ⚡ Performance Testing

### Load Testing Script

```yaml
# artillery-load-test.yml
config:
  target: 'https://api.fortune-teller-v2.workers.dev'
  phases:
    - duration: 60
      arrivalRate: 10
      name: "Warm up"
    - duration: 120
      arrivalRate: 50
      name: "Load test"
    - duration: 120
      arrivalRate: 100
      name: "Stress test"
    - duration: 120
      arrivalRate: 200
      name: "Peak load"

scenarios:
  - name: "Chart Calculation"
    weight: 40
    flow:
      - post:
          url: "/auth/login"
          headers:
            Authorization: "Bearer test-token"
      - post:
          url: "/charts"
          json:
            type: "ziwei"
            birthData:
              birthYear: 1990
              birthMonth: 6
              birthDay: 15
              birthHour: 14
              gender: "male"
              timezone: "Asia/Taipei"

  - name: "User Profile"
    weight: 30
    flow:
      - get:
          url: "/users/me"
          headers:
            Authorization: "Bearer test-token"

  - name: "Chart Retrieval"
    weight: 30
    flow:
      - get:
          url: "/charts/{{ $randomString() }}"
          headers:
            Authorization: "Bearer test-token"
```

### Performance Monitoring

```typescript
// tests/performance/performance-monitor.test.ts
import { describe, test, expect } from 'vitest';
import { performance } from 'perf_hooks';

describe('Performance Monitoring', () => {
  test('API response times meet targets', async () => {
    const endpoints = [
      '/auth/login',
      '/charts',
      '/users/me',
      '/charts/{{chartId}}'
    ];

    for (const endpoint of endpoints) {
      const start = performance.now();
      const response = await fetch(`https://api.example.com${endpoint}`);
      const end = performance.now();

      const responseTime = end - start;
      expect(responseTime).toBeLessThan(200); // p95 target
      expect(response.status).toBeLessThan(500);
    }
  });

  test('chart calculation performance', async () => {
    const calculationStart = performance.now();

    // Perform chart calculation
    const response = await fetch('https://api.example.com/charts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        type: 'ziwei',
        birthData: {
          birthYear: 1990,
          birthMonth: 6,
          birthDay: 15,
          birthHour: 14,
          gender: 'male',
          timezone: 'Asia/Taipei'
        }
      })
    });

    const calculationEnd = performance.now();
    const calculationTime = calculationEnd - calculationStart;

    expect(calculationTime).toBeLessThan(500); // Chart calculation target
    expect(response.ok).toBe(true);
  });
});
```

## 🔒 Security Testing

### OWASP Top 10 Tests

```typescript
// tests/security/security-tests.test.ts
import { describe, test, expect } from 'vitest';

describe('Security Tests', () => {
  const baseUrl = 'https://api.fortune-teller-v2.workers.dev';

  test('SQL Injection protection', async () => {
    const maliciousInputs = [
      "'; DROP TABLE users; --",
      "' OR '1'='1",
      "1' UNION SELECT * FROM users --"
    ];

    for (const input of maliciousInputs) {
      const response = await fetch(`${baseUrl}/charts/${input}`, {
        headers: { 'Authorization': 'Bearer test-token' }
      });

      expect(response.status).toBe(404);
      expect(response.ok).toBe(false);
    }
  });

  test('XSS protection', async () => {
    const xssPayloads = [
      '<script>alert("xss")</script>',
      'javascript:alert("xss")',
      '<img src=x onerror=alert("xss")>'
    ];

    for (const payload of xssPayloads) {
      const response = await fetch(`${baseUrl}/charts`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': 'Bearer test-token'
        },
        body: JSON.stringify({
          name: payload,
          type: 'ziwei'
        })
      });

      if (response.ok) {
        const data = await response.json();
        expect(data.data.chart.name).not.toContain('<script>');
      }
    }
  });

  test('Rate limiting', async () => {
    const requests = [];
    const limit = 100; // Adjust based on your rate limit

    // Send rapid requests
    for (let i = 0; i < limit + 10; i++) {
      requests.push(
        fetch(`${baseUrl}/charts`, {
          headers: { 'Authorization': 'Bearer test-token' }
        })
      );
    }

    const responses = await Promise.all(requests);
    const rateLimitedResponses = responses.filter(r => r.status === 429);

    expect(rateLimitedResponses.length).toBeGreaterThan(0);
  });

  test('Authentication bypass protection', async () => {
    const unauthorizedRequests = [
      fetch(`${baseUrl}/users/me`),
      fetch(`${baseUrl}/charts`, { method: 'POST' }),
      fetch(`${baseUrl}/admin/users`)
    ];

    const responses = await Promise.all(unauthorizedRequests);
    responses.forEach(response => {
      expect(response.status).toBe(401);
    });
  });
});
```

## 📊 Monitoring Setup

### Application Performance Monitoring

```typescript
// src/monitoring/apm.ts
export class APMService {
  private tracer: any;
  private metrics: any;

  constructor(private env: Env) {
    this.setupMonitoring();
  }

  private setupMonitoring() {
    // Initialize OpenTelemetry
    this.tracer = this.initializeTracer();
    this.metrics = this.initializeMetrics();
  }

  async traceRequest(request: Request, handler: () => Promise<Response>) {
    const span = this.tracer.startSpan('http_request');

    try {
      // Record request attributes
      span.setAttributes({
        'http.method': request.method,
        'http.url': request.url,
        'user.agent': request.headers.get('user-agent'),
        'cf.ip': request.headers.get('cf-connecting-ip')
      });

      const start = Date.now();
      const response = await handler();
      const duration = Date.now() - start;

      // Record response metrics
      span.setAttributes({
        'http.status_code': response.status,
        'response.duration_ms': duration
      });

      // Record custom metrics
      this.metrics.record('http_requests', 1, {
        method: request.method,
        status: response.status.toString()
      });

      this.metrics.record('response_time', duration, {
        method: request.method,
        endpoint: new URL(request.url).pathname
      });

      return response;

    } catch (error) {
      span.recordException(error);
      this.metrics.record('errors', 1, {
        error_type: error.constructor.name,
        endpoint: new URL(request.url).pathname
      });
      throw error;
    } finally {
      span.end();
    }
  }
}
```

### Health Check Endpoints

```typescript
// src/routes/health.ts
import { Hono } from 'hono';

const health = new Hono();

// Basic health check
health.get('/health', async (c) => {
  return c.json({
    status: 'healthy',
    timestamp: new Date().toISOString(),
    version: c.env.APP_VERSION || '1.0.0'
  });
});

// Detailed health check
health.get('/health/detailed', async (c) => {
  const checks = await Promise.allSettled([
    checkDatabaseHealth(c.env.DB),
    checkDurableObjectsHealth(c.env.SESSION_MANAGER),
    checkExternalServicesHealth(c.env),
    checkDiskSpace(c.env)
  ]);

  const results = checks.map(check => ({
    status: check.status === 'fulfilled' ? 'healthy' : 'unhealthy',
    details: check.status === 'fulfilled' ? check.value : check.reason
  }));

  const overallHealthy = results.every(r => r.status === 'healthy');

  return c.json({
    status: overallHealthy ? 'healthy' : 'unhealthy',
    timestamp: new Date().toISOString(),
    checks: results
  }, { status: overallHealthy ? 200 : 503 });
});

async function checkDatabaseHealth(db: D1Database) {
  try {
    const result = await db.prepare('SELECT 1').first();
    return { database: 'connected', result: result };
  } catch (error) {
    throw { database: 'error', message: error.message };
  }
}

async function checkDurableObjectsHealth(sessionDO: DurableObjectNamespace) {
  try {
    const id = sessionDO.idFromName('health-check');
    const stub = sessionDO.get(id);
    const response = await stub.fetch(new Request('https://example.com/health'));
    return { durable_objects: 'healthy', status: response.status };
  } catch (error) {
    throw { durable_objects: 'error', message: error.message };
  }
}

export { health };
```

### Error Tracking Integration

```typescript
// src/monitoring/error-tracking.ts
export class ErrorTrackingService {
  private sentry: any;

  constructor(private env: Env) {
    if (env.SENTRY_DSN) {
      this.initializeSentry();
    }
  }

  private initializeSentry() {
    // Initialize Sentry with appropriate configuration
    this.sentry = {
      captureException: (error: Error) => {
        console.error('Error captured:', error);
        // Send to Sentry
      },
      captureMessage: (message: string, level: string) => {
        console.error(`Message captured [${level}]:`, message);
        // Send to Sentry
      }
    };
  }

  trackError(error: Error, context?: any) {
    if (this.sentry) {
      this.sentry.captureException(error, {
        extra: context,
        tags: {
          service: 'fortune-teller-v2',
          version: this.env.APP_VERSION
        }
      });
    }
  }

  trackMessage(message: string, level: 'info' | 'warning' | 'error' = 'info') {
    if (this.sentry) {
      this.sentry.captureMessage(message, level);
    }
  }
}
```

## 📈 Monitoring Dashboards

### Key Performance Indicators

```typescript
// src/monitoring/metrics.ts
export class MetricsCollector {
  private metrics: Map<string, number> = new Map();

  recordApiCall(endpoint: string, method: string, statusCode: number, duration: number) {
    const key = `api_${method}_${endpoint}`;
    this.metrics.set(key, duration);

    // Also record error rates
    if (statusCode >= 400) {
      const errorKey = `api_errors_${endpoint}`;
      this.metrics.set(errorKey, (this.metrics.get(errorKey) || 0) + 1);
    }
  }

  recordChartCalculation(type: string, duration: number, cached: boolean) {
    const key = `chart_calc_${type}_${cached ? 'cached' : 'fresh'}`;
    this.metrics.set(key, duration);
  }

  recordUserActivity(userId: string, action: string) {
    const key = `user_activity_${action}`;
    this.metrics.set(key, (this.metrics.get(key) || 0) + 1);
  }

  getMetrics(): Record<string, number> {
    return Object.fromEntries(this.metrics);
  }
}
```

### Alert Configuration

```yaml
# alerts.yml
groups:
  - name: api_performance
    rules:
      - alert: HighAPIResponseTime
        expr: avg(response_time_ms) > 500
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "API response time is high"
          description: "Average API response time is {{ $value }}ms"

      - alert: HighErrorRate
        expr: rate(api_errors_total[5m]) > 0.05
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }}"

  - name: business_metrics
    rules:
      - alert: LowChartCalculationSuccess
        expr: rate(chart_calculations_successful_total[5m]) / rate(chart_calculations_total[5m]) < 0.95
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Chart calculation success rate is low"
          description: "Success rate is {{ $value | humanizePercentage }}"

      - alert: DatabaseConnectionIssues
        expr: up{job="database"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Database connection lost"
          description: "Cannot connect to D1 database"
```

## 🔄 Continuous Testing

### CI/CD Pipeline Integration

```yaml
# .github/workflows/test.yml
name: Test Suite

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install dependencies
        run: npm ci

      - name: Run unit tests
        run: npm run test:unit

      - name: Upload coverage
        uses: codecov/codecov-action@v3

  integration-tests:
    runs-on: ubuntu-latest
    needs: unit-tests
    steps:
      - uses: actions/checkout@v3

      - name: Run integration tests
        run: npm run test:integration

  e2e-tests:
    runs-on: ubuntu-latest
    needs: integration-tests
    steps:
      - uses: actions/checkout@v3

      - name: Install Playwright
        run: npx playwright install

      - name: Run E2E tests
        run: npm run test:e2e

      - name: Upperformance test results
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: playwright-report
          path: playwright-report/

  performance-tests:
    runs-on: ubuntu-latest
    needs: e2e-tests
    steps:
      - uses: actions/checkout@v3

      - name: Install Artillery
        run: npm install -g artillery

      - name: Run performance tests
        run: artillery run tests/performance/artillery-load-test.yml

  security-tests:
    runs-on: ubuntu-latest
    needs: unit-tests
    steps:
      - uses: actions/checkout@v3

      - name: Run security audit
        run: npm audit --audit-level high

      - name: Run OWASP ZAP Baseline Scan
        uses: zaproxy/action-baseline@v0.7.0
        with:
          target: 'https://api.fortune-teller-v2.workers.dev'
```

### Test Data Management

```typescript
// tests/helpers/test-data.ts
export class TestDataManager {
  private static testData: Map<string, any> = new Map();

  static createTestUser(overrides: any = {}) {
    const defaultUser = {
      id: 'test-user-' + Math.random().toString(36).substr(2, 9),
      email: `test-${Math.random().toString(36).substr(2, 9)}@example.com`,
      subscriptionStatus: 'free',
      preferences: {
        language: 'zh-TW',
        timezone: 'Asia/Taipei'
      },
      createdAt: new Date().toISOString()
    };

    const user = { ...defaultUser, ...overrides };
    this.testData.set(user.id, user);
    return user;
  }

  static createTestChart(overrides: any = {}) {
    const defaultChart = {
      id: 'test-chart-' + Math.random().toString(36).substr(2, 9),
      type: 'ziwei',
      name: 'Test Chart',
      birthData: {
        birthYear: 1990,
        birthMonth: 6,
        birthDay: 15,
        birthHour: 14,
        gender: 'male',
        timezone: 'Asia/Taipei'
      },
      createdAt: new Date().toISOString()
    };

    const chart = { ...defaultChart, ...overrides };
    this.testData.set(chart.id, chart);
    return chart;
  }

  static cleanup() {
    this.testData.clear();
  }

  static getTestData(id: string) {
    return this.testData.get(id);
  }
}
```

This comprehensive testing and monitoring procedure ensures the migration is successful, maintainable, and meets all performance and security requirements.