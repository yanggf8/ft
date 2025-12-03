# 🚀 FortuneT V2 - Master Migration Plan (Consolidated)

**Version**: 3.0 (Revised December 2025)
**Timeline**: 24 weeks core + 8 weeks storytelling (32 weeks total)
**Status**: Ready for Phase -1

---

## 📋 Executive Summary

Migrate FortuneT from Render + Vercel to unified Cloudflare-native platform.

| Metric | Current | Month 1-2 | Month 6+ |
|--------|---------|-----------|----------|
| **Cost** | $437-1,062/mo | **$0** | $100-303/mo |
| **Performance (p95)** | 500-2000ms | <200ms | <150ms |
| **Uptime** | 99.5% | 99.9% | 99.95% |

**Key Changes from Previous Plan:**
- Extended timeline: 16 → 24 weeks (+50% buffer)
- Added Phase -1: System Audit (NEW)
- Added 4-week stabilization before storytelling
- Added rollback procedures and cost monitoring

---

## 📅 Revised Timeline Overview

```
Phase -1: System Audit        Week 0      (1 week)   ← NEW
Phase 0:  Risk Assessment     Week 1-3    (3 weeks)  ← Extended
Phase 1:  Foundation          Week 4-6    (3 weeks)
Phase 2:  Core Features       Week 7-11   (5 weeks)
Phase 3:  Frontend            Week 12-15  (4 weeks)
Phase 4:  Integration/Test    Week 16-18  (3 weeks)
Phase 5:  Pre-Migration       Week 19-20  (2 weeks)
Phase 6:  Go-Live             Week 21     (1 week)
------- STABILIZATION -------  Week 22-25  (4 weeks)  ← NEW
Phase 7:  Storytelling        Week 26-33  (8 weeks)
```

---

## 🔍 Phase -1: System Audit (Week 0) - NEW

**Goal**: Document current system completely before any migration work.

### Deliverables

#### 1. Database Audit
```bash
# Run these against current PostgreSQL
psql $DATABASE_URL -c "SELECT table_name, pg_size_pretty(pg_total_relation_size(quote_ident(table_name))) FROM information_schema.tables WHERE table_schema='public' ORDER BY pg_total_relation_size(quote_ident(table_name)) DESC;"

# Export schema
pg_dump $DATABASE_URL --schema-only > current_schema.sql

# Count rows per table
psql $DATABASE_URL -c "SELECT schemaname, relname, n_live_tup FROM pg_stat_user_tables ORDER BY n_live_tup DESC;"
```

**Required Output**: `docs/audit/database_audit.md`
- Total database size (GB)
- Row counts per table
- Largest tables
- Daily write volume
- Growth rate (last 3 months)

#### 2. API Endpoint Inventory
```bash
# If using Flask, extract routes
grep -r "@app.route\|@blueprint.route" --include="*.py" > api_routes.txt
```

**Required Output**: `docs/audit/api_inventory.md`
- List all endpoints (method, path, description)
- Request/response formats
- Authentication requirements
- Rate limits

#### 3. User & Usage Metrics
**Required Output**: `docs/audit/usage_metrics.md`
- Total registered users
- Daily active users (DAU)
- Monthly active users (MAU)
- Peak concurrent users
- Top 10 most-used endpoints
- Average session duration

#### 4. Dependency Audit
```bash
# Python dependencies
pip freeze > requirements_current.txt

# Frontend dependencies
cat package.json | jq '.dependencies, .devDependencies'
```

**Required Output**: `docs/audit/dependencies.md`
- All Python packages with versions
- All npm packages with versions
- External API dependencies (Groq, Stripe, etc.)
- Environment variables list

### Phase -1 Exit Criteria
- [ ] Database audit complete with exact sizes
- [ ] All API endpoints documented
- [ ] User metrics captured
- [ ] Dependencies listed
- [ ] Current performance baseline measured

---

## ⚠️ Phase 0: Risk Assessment (Week 1-3) - EXTENDED

**Goal**: Validate technical feasibility before committing resources.

### Week 1: Database Compatibility

#### Task 1: D1 Compatibility Test
```typescript
// phase0-tests/d1-compatibility.ts
interface CompatibilityResult {
  query: string;
  postgresql_works: boolean;
  d1_works: boolean;
  migration_needed: string | null;
}

async function testD1Compatibility(db: D1Database): Promise<CompatibilityResult[]> {
  const criticalQueries = [
    // Test your actual queries here
    'SELECT * FROM users WHERE email = ?',
    'SELECT * FROM charts WHERE user_id = ? ORDER BY created_at DESC LIMIT 10',
    // Add all queries from your API inventory
  ];
  
  const results: CompatibilityResult[] = [];
  for (const query of criticalQueries) {
    try {
      await db.prepare(query).bind('test').all();
      results.push({ query, postgresql_works: true, d1_works: true, migration_needed: null });
    } catch (e) {
      results.push({ query, postgresql_works: true, d1_works: false, migration_needed: e.message });
    }
  }
  return results;
}
```

**Success Criteria**: >90% queries work without modification

#### Task 2: Data Volume Validation
```typescript
// Validate data fits in D1 limits
const D1_LIMITS = {
  max_storage_gb: 10,        // Free tier
  max_rows_per_table: 'unlimited but performance degrades >1M',
  max_write_per_sec: 1000
};

// From Phase -1 audit, check:
// - Total data size < 5GB (safe margin)
// - Largest table < 500K rows
// - Daily writes < 50K (safe margin)
```

### Week 2: OAuth & Infrastructure

#### Task 3: OAuth Migration Prototype
```typescript
// phase0-tests/oauth-migration.ts
async function testOAuthMigration() {
  // 1. Create test user in current system
  // 2. Simulate OAuth login in new system
  // 3. Verify account linking works
  // 4. Test session persistence
  // 5. Test logout/re-login flow
}
```

**Success Criteria**: 
- 100% of test users can login
- Session persists across page refresh
- Logout works correctly

#### Task 4: Durable Objects Performance
```typescript
// phase0-tests/do-performance.ts
async function testDOPerformance() {
  const results = {
    concurrent_10: await testConcurrent(10),   // Must pass
    concurrent_50: await testConcurrent(50),   // Must pass
    concurrent_100: await testConcurrent(100), // Should pass
  };
  
  // Success: p95 < 100ms for all tests
  return results;
}
```

### Week 3: Security & Go/No-Go

#### Task 5: Security Validation
- [ ] Authentication flow penetration test
- [ ] Input validation testing
- [ ] SQL injection testing (D1)
- [ ] XSS testing (frontend)
- [ ] CORS configuration validation

#### Task 6: Go/No-Go Decision

```typescript
interface GoNoGoDecision {
  database: {
    compatibility_score: number;      // Must be > 90%
    data_fits_limits: boolean;        // Must be true
    performance_acceptable: boolean;  // p95 < 200ms
  };
  oauth: {
    migration_success_rate: number;   // Must be > 95%
    user_experience_acceptable: boolean;
  };
  infrastructure: {
    do_performance_ok: boolean;       // p95 < 100ms
    security_issues_critical: number; // Must be 0
  };
}

function makeDecision(criteria: GoNoGoDecision): 'GO' | 'NO-GO' | 'GO-WITH-MITIGATIONS' {
  const blockers = [];
  
  if (criteria.database.compatibility_score < 90) blockers.push('D1 compatibility too low');
  if (!criteria.database.data_fits_limits) blockers.push('Data exceeds D1 limits');
  if (criteria.oauth.migration_success_rate < 95) blockers.push('OAuth migration unreliable');
  if (criteria.infrastructure.security_issues_critical > 0) blockers.push('Critical security issues');
  
  if (blockers.length === 0) return 'GO';
  if (blockers.length <= 2 && !blockers.includes('Data exceeds D1 limits')) return 'GO-WITH-MITIGATIONS';
  return 'NO-GO';
}
```

### Phase 0 Deliverables
1. `docs/phase0/d1_compatibility_report.md`
2. `docs/phase0/oauth_migration_report.md`
3. `docs/phase0/performance_benchmarks.md`
4. `docs/phase0/security_assessment.md`
5. `docs/phase0/go_no_go_decision.md` ← **REQUIRED BEFORE PHASE 1**

---

## 🏗️ Phase 1: Foundation (Week 4-6)

**Goal**: Setup infrastructure and development environment.

### Week 4: Repository & Cloudflare Setup

```bash
# 1. Create new repository
mkdir fortune-teller-v2 && cd fortune-teller-v2
git init

# 2. Setup structure
mkdir -p backend/src/{routes,services,durable-objects,middleware}
mkdir -p backend/scripts
mkdir -p frontend/src/{components,pages,services,hooks,types}
mkdir -p shared/types
mkdir -p docs/{audit,phase0,architecture}

# 3. Initialize packages
cd backend && npm init -y
npm install hono @cloudflare/workers-types
npm install -D wrangler vitest typescript

cd ../frontend && npm init -y
npm install react react-dom react-router-dom zustand
npm install -D vite @vitejs/plugin-react typescript
```

#### Backend Setup: `backend/wrangler.toml`
```toml
name = "fortunet-api"
main = "src/index.ts"
compatibility_date = "2024-01-01"

[[d1_databases]]
binding = "DB"
database_name = "fortunet-db"
database_id = "your-database-id"

[[durable_objects.bindings]]
name = "SESSION_DO"
class_name = "SessionDO"

[[durable_objects.bindings]]
name = "CACHE_DO"
class_name = "CacheDO"

[[r2_buckets]]
binding = "STORAGE"
bucket_name = "fortunet-storage"

[vars]
ENVIRONMENT = "development"
```

#### Backend Entry: `backend/src/index.ts`
```typescript
import { Hono } from 'hono';
import { cors } from 'hono/cors';

type Bindings = {
  DB: D1Database;
  SESSION_DO: DurableObjectNamespace;
  CACHE_DO: DurableObjectNamespace;
  STORAGE: R2Bucket;
  GROQ_API_KEY: string;
  STRIPE_SECRET_KEY: string;
  JWT_SECRET: string;
};

const app = new Hono<{ Bindings: Bindings }>();

// Middleware
app.use('*', cors());

// Health check
app.get('/health', (c) => c.json({ status: 'ok', timestamp: Date.now() }));

// Routes will be added in Phase 2
// app.route('/api/auth', authRoutes);
// app.route('/api/charts', chartRoutes);
// app.route('/api/ai', aiRoutes);

export default app;

// Durable Objects exports
export { SessionDO } from './durable-objects/session-do';
export { CacheDO } from './durable-objects/cache-do';
```

### Week 5: Database Schema & Migrations

#### D1 Schema: `backend/scripts/schema.sql`
```sql
-- Users table
CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  email TEXT UNIQUE NOT NULL,
  name TEXT,
  oauth_provider TEXT,
  oauth_id TEXT,
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_oauth ON users(oauth_provider, oauth_id);

-- Charts table
CREATE TABLE IF NOT EXISTS charts (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  chart_type TEXT NOT NULL, -- 'ziwei' | 'zodiac'
  birth_data TEXT NOT NULL, -- JSON
  chart_data TEXT NOT NULL, -- JSON (calculated result)
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_charts_user ON charts(user_id);
CREATE INDEX idx_charts_type ON charts(chart_type);

-- Favorites table
CREATE TABLE IF NOT EXISTS favorites (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  chart_id TEXT NOT NULL REFERENCES charts(id),
  created_at TEXT DEFAULT (datetime('now')),
  UNIQUE(user_id, chart_id)
);

-- Subscriptions table
CREATE TABLE IF NOT EXISTS subscriptions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  stripe_customer_id TEXT,
  stripe_subscription_id TEXT,
  plan TEXT NOT NULL, -- 'free' | 'premium'
  status TEXT NOT NULL, -- 'active' | 'canceled' | 'past_due'
  current_period_end TEXT,
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX idx_subscriptions_user ON subscriptions(user_id);
CREATE INDEX idx_subscriptions_stripe ON subscriptions(stripe_subscription_id);
```

### Week 6: Authentication System

#### Session Durable Object: `backend/src/durable-objects/session-do.ts`
```typescript
export class SessionDO implements DurableObject {
  private state: DurableObjectState;
  
  constructor(state: DurableObjectState) {
    this.state = state;
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    
    switch (url.pathname) {
      case '/create':
        return this.createSession(request);
      case '/get':
        return this.getSession();
      case '/destroy':
        return this.destroySession();
      default:
        return new Response('Not found', { status: 404 });
    }
  }

  private async createSession(request: Request): Promise<Response> {
    const { userId, email } = await request.json();
    const session = {
      userId,
      email,
      createdAt: Date.now(),
      expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000, // 7 days
    };
    await this.state.storage.put('session', session);
    return Response.json(session);
  }

  private async getSession(): Promise<Response> {
    const session = await this.state.storage.get('session');
    if (!session) return new Response('No session', { status: 401 });
    if (session.expiresAt < Date.now()) {
      await this.state.storage.delete('session');
      return new Response('Session expired', { status: 401 });
    }
    return Response.json(session);
  }

  private async destroySession(): Promise<Response> {
    await this.state.storage.delete('session');
    return Response.json({ success: true });
  }
}
```

### Phase 1 Exit Criteria
- [ ] Repository structure created
- [ ] Wrangler configured and working locally
- [ ] D1 database created with schema
- [ ] R2 bucket created
- [ ] Session DO working
- [ ] Health endpoint responding
- [ ] CI/CD pipeline configured

---

## ⚙️ Phase 2: Core Features (Week 7-11)

**Goal**: Port calculation engines and integrate AI/payments.

### Week 7-8: Chart Calculation Engines

#### Zi Wei Engine: `backend/src/services/ziwei-engine.ts`
```typescript
interface BirthData {
  year: number;
  month: number;
  day: number;
  hour: number;
  gender: 'male' | 'female';
}

interface ZiWeiChart {
  lifePalace: Palace;
  palaces: Palace[];
  stars: Star[];
  tenYearLuck: TenYearPeriod[];
}

export class ZiWeiEngine {
  calculate(birthData: BirthData): ZiWeiChart {
    // Port existing calculation logic here
    // This is the core business logic from your Flask app
    throw new Error('TODO: Port from existing codebase');
  }
}
```

#### Zodiac Engine: `backend/src/services/zodiac-engine.ts`
```typescript
interface ZodiacChart {
  sunSign: string;
  moonSign: string;
  ascendant: string;
  planets: PlanetPosition[];
  houses: House[];
  aspects: Aspect[];
}

export class ZodiacEngine {
  calculate(birthData: BirthData): ZodiacChart {
    // Port existing calculation logic here
    throw new Error('TODO: Port from existing codebase');
  }
}
```

### Week 9-10: AI Integration

#### AI Gateway: `backend/src/services/ai-gateway.ts`
```typescript
interface AIAnalysisRequest {
  chartType: 'ziwei' | 'zodiac';
  chartData: ZiWeiChart | ZodiacChart;
  analysisType: 'basic' | 'detailed' | 'story';
}

export class AIGateway {
  constructor(
    private groqKey: string,
    private cacheNamespace: DurableObjectNamespace
  ) {}

  async analyze(request: AIAnalysisRequest): Promise<string> {
    // Check cache first
    const cacheKey = this.getCacheKey(request);
    const cached = await this.getFromCache(cacheKey);
    if (cached) return cached;

    // Call Groq API
    const response = await fetch('https://api.groq.com/openai/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.groqKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: 'llama-3.1-70b-versatile',
        messages: [
          { role: 'system', content: this.getSystemPrompt(request.chartType) },
          { role: 'user', content: this.formatChartForAI(request.chartData) }
        ],
        max_tokens: 2000,
      }),
    });

    const result = await response.json();
    const analysis = result.choices[0].message.content;

    // Cache result
    await this.saveToCache(cacheKey, analysis);
    return analysis;
  }

  private getCacheKey(request: AIAnalysisRequest): string {
    return `ai:${request.chartType}:${request.analysisType}:${JSON.stringify(request.chartData)}`;
  }

  private getSystemPrompt(chartType: string): string {
    // Return appropriate system prompt
    return chartType === 'ziwei' 
      ? 'You are an expert in Zi Wei Dou Shu astrology...'
      : 'You are an expert in Western astrology...';
  }

  private formatChartForAI(chartData: any): string {
    return JSON.stringify(chartData, null, 2);
  }

  private async getFromCache(key: string): Promise<string | null> {
    // Use Durable Object for caching
    return null; // TODO: Implement
  }

  private async saveToCache(key: string, value: string): Promise<void> {
    // Save to Durable Object cache
  }
}
```

### Week 11: Payment Integration

#### Stripe Handler: `backend/src/services/stripe-handler.ts`
```typescript
import Stripe from 'stripe';

export class StripeHandler {
  private stripe: Stripe;

  constructor(secretKey: string) {
    this.stripe = new Stripe(secretKey, { apiVersion: '2023-10-16' });
  }

  async createCheckoutSession(userId: string, priceId: string): Promise<string> {
    const session = await this.stripe.checkout.sessions.create({
      mode: 'subscription',
      payment_method_types: ['card'],
      line_items: [{ price: priceId, quantity: 1 }],
      success_url: 'https://fortunet.com/success?session_id={CHECKOUT_SESSION_ID}',
      cancel_url: 'https://fortunet.com/cancel',
      metadata: { userId },
    });
    return session.url!;
  }

  async handleWebhook(payload: string, signature: string, webhookSecret: string): Promise<void> {
    const event = this.stripe.webhooks.constructEvent(payload, signature, webhookSecret);
    
    switch (event.type) {
      case 'checkout.session.completed':
        // Create subscription in DB
        break;
      case 'customer.subscription.updated':
        // Update subscription status
        break;
      case 'customer.subscription.deleted':
        // Cancel subscription
        break;
    }
  }
}
```

### Phase 2 Exit Criteria
- [ ] Zi Wei engine ported and tested
- [ ] Zodiac engine ported and tested
- [ ] AI integration working with Groq
- [ ] AI caching implemented
- [ ] Stripe integration complete
- [ ] All endpoints tested
- [ ] Performance targets met (<200ms calculations, <3s AI)

---

## 🎨 Phase 3: Frontend (Week 12-15)

**Goal**: Build React frontend with all user-facing features.

### Week 12-13: Core Application

#### App Entry: `frontend/src/App.tsx`
```tsx
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { AuthProvider } from './contexts/AuthContext';
import { ProtectedRoute } from './components/ProtectedRoute';
import { HomePage } from './pages/HomePage';
import { ChartPage } from './pages/ChartPage';
import { ProfilePage } from './pages/ProfilePage';
import { LoginPage } from './pages/LoginPage';

export function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/login" element={<LoginPage />} />
          <Route path="/chart/:id" element={<ProtectedRoute><ChartPage /></ProtectedRoute>} />
          <Route path="/profile" element={<ProtectedRoute><ProfilePage /></ProtectedRoute>} />
        </Routes>
      </BrowserRouter>
    </AuthProvider>
  );
}
```

### Week 14-15: Features & Polish

- Chart creation/editing UI
- Chart visualization components
- User profile management
- Subscription management
- Mobile responsive design
- Performance optimization

### Phase 3 Exit Criteria
- [ ] All pages implemented
- [ ] Authentication flow working
- [ ] Chart creation/viewing working
- [ ] Mobile responsive
- [ ] Lighthouse score > 90
- [ ] Accessibility compliant

---

## 🧪 Phase 4: Integration & Testing (Week 16-18)

**Goal**: Comprehensive testing and production readiness.

### Testing Requirements

| Type | Coverage Target | Tools |
|------|-----------------|-------|
| Unit Tests | >90% | Vitest |
| Integration Tests | >85% | Vitest + Miniflare |
| E2E Tests | Critical paths | Playwright |
| Performance | p95 < 200ms | k6 |

### Week 16-17: Automated Testing
```bash
# Run all tests
npm run test --prefix backend
npm run test --prefix frontend
npm run test:e2e
```

### Week 18: Security & Performance Audit
- Penetration testing
- Load testing (target: 100 concurrent users)
- Security headers validation
- OWASP top 10 check

### Phase 4 Exit Criteria
- [ ] All tests passing
- [ ] Coverage targets met
- [ ] Security audit passed
- [ ] Performance targets validated
- [ ] No critical bugs

---

## 🚀 Phase 5: Pre-Migration (Week 19-20)

**Goal**: Data migration and beta testing.

### Week 19: Data Migration

```typescript
// scripts/migrate-data.ts
async function migrateData() {
  // 1. Export from PostgreSQL
  const users = await exportUsers();
  const charts = await exportCharts();
  const subscriptions = await exportSubscriptions();

  // 2. Transform data
  const transformedUsers = users.map(transformUser);
  const transformedCharts = charts.map(transformChart);

  // 3. Import to D1
  await importToD1(transformedUsers, 'users');
  await importToD1(transformedCharts, 'charts');

  // 4. Verify integrity
  await verifyMigration();
}
```

### Week 20: Beta Testing
- Internal team testing (3 days)
- Beta users (10-20 users, 4 days)
- Bug fixes and optimization
- Final go/no-go decision

### Phase 5 Exit Criteria
- [ ] Data migrated successfully
- [ ] Data integrity verified
- [ ] Beta testing complete
- [ ] All critical bugs fixed
- [ ] Rollback procedures tested

---

## 🎯 Phase 6: Go-Live (Week 21)

**Goal**: Gradual traffic migration with rollback capability.

### Migration Schedule

| Day | Traffic | Users | Rollback Window |
|-----|---------|-------|-----------------|
| 1 | 5% | Internal + power users | Immediate |
| 2 | 10% | Early adopters | 1 hour |
| 3 | 25% | General users | 2 hours |
| 4 | 50% | All users | 4 hours |
| 5 | 100% | Everyone | 24 hours |

### Rollback Procedure
```bash
# If issues detected:
# 1. Revert DNS to old system
# 2. Notify users
# 3. Investigate and fix
# 4. Re-attempt migration
```

### Phase 6 Exit Criteria
- [ ] 100% traffic on new system
- [ ] Error rate < 0.5%
- [ ] No critical issues for 48 hours
- [ ] User feedback positive

---

## 🛡️ Stabilization Period (Week 22-25) - NEW

**Goal**: Monitor, optimize, and stabilize before adding features.

### Week 22-23: Monitoring & Optimization
- Monitor error rates and performance
- Optimize slow queries
- Fix edge case bugs
- Gather user feedback

### Week 24-25: Documentation & Cleanup
- Update all documentation
- Archive old system
- Knowledge transfer
- Plan storytelling phase

### Stabilization Exit Criteria
- [ ] 4 weeks stable operation
- [ ] Error rate < 0.1%
- [ ] User satisfaction > 4.5/5
- [ ] Cost within projections
- [ ] Ready for storytelling phase

---

## 🌌 Phase 7: Storytelling (Week 26-33)

**Goal**: Add AI-powered narrative features.

*Detailed in STORYTELLING_ROADMAP.md - only proceed after stabilization complete.*

### Quick Overview
- Week 26-27: MVP (text-only narratives)
- Week 28-31: Enhanced (Cosmic Loom, Celestial Sage)
- Week 32-33: Premium (Soul Symbols, Audio)

---

## 💰 Cost Summary

### Migration Investment (One-Time)
| Item | Cost |
|------|------|
| Phase 0 testing | $100-200 |
| Security audit | $500-1,000 |
| Parallel infrastructure (24 weeks) | $1,200-3,000 |
| **Total** | **$1,800-4,200** |

### Monthly Costs

| Phase | Timeline | Cost |
|-------|----------|------|
| Testing (5-10 DAU) | Month 1-2 | **$0** |
| Early Growth (20-50 DAU) | Month 3-4 | $30-80 |
| Scale (50+ DAU) | Month 5+ | $100-303 |

### ROI
- Current cost: $437-1,062/month
- New cost: $0-303/month
- **Annual savings: $1,600-11,550**

---

## 📋 Cost Monitoring Alerts - NEW

```typescript
// Set up alerts at 70% of free tier limits
const COST_ALERTS = {
  workers_requests: { limit: 100000, alert_at: 70000 },
  d1_storage_mb: { limit: 5000, alert_at: 3500 },
  d1_reads: { limit: 25000000, alert_at: 17500000 },
  do_requests: { limit: 400000, alert_at: 280000 },
  r2_storage_gb: { limit: 10, alert_at: 7 },
  groq_requests: { limit: 14000, alert_at: 9800 },
};
```

---

## 🔄 Rollback Procedures - NEW

### DNS Rollback (< 5 minutes)
```bash
# Revert DNS to old system
# Old: fortunet.com -> new-workers.fortunet.workers.dev
# New: fortunet.com -> old-render-app.onrender.com
```

### Data Rollback (< 1 hour)
```bash
# 1. Stop writes to new system
# 2. Export any new data from D1
# 3. Import to PostgreSQL
# 4. Switch DNS
# 5. Verify data integrity
```

### Full Rollback Checklist
- [ ] DNS reverted
- [ ] Old system verified working
- [ ] Users notified
- [ ] Data synced (if needed)
- [ ] Post-mortem scheduled

---

## ✅ Quick Reference Checklist

### Before Starting
- [ ] Read this entire document
- [ ] Complete Phase -1 audit
- [ ] Get Go/No-Go approval for Phase 0

### Weekly Check-ins
- [ ] Are we on schedule?
- [ ] Any blockers?
- [ ] Cost within budget?
- [ ] Tests passing?

### Go-Live Readiness
- [ ] All phases complete
- [ ] All exit criteria met
- [ ] Rollback tested
- [ ] Team ready for support

---

**Document Version**: 3.0
**Last Updated**: 2025-12-03
**Next Action**: Start Phase -1 System Audit

