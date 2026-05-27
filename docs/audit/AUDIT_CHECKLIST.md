# 📋 Phase -1: System Audit Checklist

**Status**: ✅ COMPLETED
**Audit Date**: 2025-12-03
**Auditor**: Automated + Manual Review
**Source**: `/home/yanggf/a/fortuneT` (current production codebase)

---

## 1. Database Audit ✅

### Database Provider
- **Provider**: Supabase (PostgreSQL)
- **Type**: Managed PostgreSQL with Row Level Security (RLS)

### Schema Summary

| Table | Purpose | PostgreSQL Features Used |
|-------|---------|-------------------------|
| `user_profiles` | Extended user info | RLS, Triggers, UUID FK |
| `subscriptions` | Stripe subscriptions | RLS, JSONB, Triggers |
| `usage_tracking` | Analytics/billing | RLS, INET, GIN index |
| `chart_records` | All chart data | RLS, JSONB, UUID[], TEXT[], GIN |
| `subscription_features` | Feature limits | UNIQUE constraint |

### PostgreSQL-Specific Features (D1 Compatibility Issues)

| Feature | Used | D1 Support | Migration Strategy |
|---------|------|------------|-------------------|
| **Row Level Security (RLS)** | ✅ Heavy | ❌ No | Application-level auth |
| **Triggers** | ✅ Yes | ❌ No | Application-level logic |
| **JSONB columns** | ✅ Heavy | ⚠️ TEXT+JSON | Store as TEXT, parse in app |
| **UUID type** | ✅ Yes | ⚠️ TEXT | Use TEXT with UUID format |
| **UUID[] arrays** | ✅ Yes | ❌ No | Separate junction table |
| **TEXT[] arrays** | ✅ Yes | ❌ No | Separate table or JSON |
| **INET type** | ✅ Yes | ⚠️ TEXT | Store as TEXT |
| **GIN indexes** | ✅ Yes | ❌ No | Full-text search alternative |
| **gen_random_uuid()** | ✅ Yes | ❌ No | Generate in application |
| **auth.uid()** | ✅ Yes | ❌ No | Application-level auth |
| **timezone()** | ✅ Yes | ⚠️ Limited | Use ISO strings |

### Data Volume Estimates (Need Production Verification)

| Metric | Estimated | D1 Limit | Status |
|--------|-----------|----------|--------|
| Total DB Size | <100MB | 10GB | ✅ Safe |
| Largest Table | chart_records | - | ⚠️ Verify |
| Daily Writes | <1000 | 50K safe | ✅ Safe |
| Total Users | <100 | - | ✅ Safe |

### ⚠️ ACTION REQUIRED
```bash
# Run these queries against production Supabase to get exact numbers:
# 1. Total database size
# 2. Row counts per table
# 3. Daily write volume (check Supabase dashboard)
```

### Checklist
- [x] Schema documented
- [x] PostgreSQL features identified
- [x] D1 compatibility issues listed
- [ ] **TODO**: Get exact production data sizes from Supabase dashboard

---

## 2. API Endpoint Inventory ✅

### Summary
- **Total Endpoints**: ~108 routes
- **Backend Framework**: Flask (Python)
- **Authentication**: Supabase Auth + JWT

### Endpoint Categories

| Category | Count | Auth Required | Notes |
|----------|-------|---------------|-------|
| Health/Status | 6 | No | `/health`, `/api/health`, `/api/status` |
| Chart Operations | 12 | Yes | CRUD for charts |
| AI Analysis | 3 | Yes | Groq integration |
| Favorites | 3 | Yes | User favorites |
| User Profile | 4 | Yes | Profile management |
| Visitor Tracking | 2 | No | Anonymous tracking |
| Admin | 15 | Admin | Dashboard, stats |
| Zodiac | 8 | Mixed | Western astrology |
| Subscriptions | 5 | Yes | Stripe integration |
| Auth | 6 | Mixed | Login/logout |

### Key Endpoints to Migrate

| Endpoint | Method | Priority | Complexity |
|----------|--------|----------|------------|
| `/api/chart` | POST | High | Medium (calculation) |
| `/api/calculate-chart` | POST | High | Medium |
| `/api/fortuneteller` | POST | High | High (AI) |
| `/api/ai-analysis` | POST | High | High (AI) |
| `/api/charts` | GET/POST | High | Low |
| `/api/charts/<id>` | GET/PUT/DELETE | High | Low |
| `/api/favorites` | GET/POST/DELETE | Medium | Low |
| `/api/profile` | GET/POST/PUT | Medium | Low |
| `/health` | GET | High | Low |

### Checklist
- [x] All endpoints counted (~108)
- [x] Categories documented
- [x] Priority assigned
- [x] Auth requirements noted

---

## 3. User & Usage Metrics ⚠️

### Estimated Metrics (Need Production Verification)

| Metric | Estimated | Notes |
|--------|-----------|-------|
| Total Registered Users | <100 | Early stage |
| Daily Active Users (DAU) | 5-10 | Target for free tier |
| Monthly Active Users (MAU) | 20-50 | Estimated |
| Peak Concurrent Users | <10 | Low traffic |
| Avg Session Duration | 5-10 min | Estimated |
| Charts Created (total) | <500 | Estimated |
| Charts Created (daily) | <20 | Estimated |
| AI Analyses (daily) | <50 | Groq free tier safe |

### ⚠️ ACTION REQUIRED
```bash
# Get actual metrics from:
# 1. Supabase dashboard - user counts
# 2. Render dashboard - request counts
# 3. Groq dashboard - API usage
```

### Checklist
- [x] Metrics categories identified
- [ ] **TODO**: Get actual production metrics

---

## 4. Dependency Audit ✅

### Backend Dependencies (Python)

| Package | Version | Purpose | Cloudflare Alternative |
|---------|---------|---------|------------------------|
| flask | >=3.0.0 | Web framework | Hono |
| flask-cors | 3.0.10 | CORS | Hono middleware |
| python-dotenv | 1.0.0 | Env vars | Wrangler secrets |
| requests | >=2.31.0 | HTTP client | fetch() |
| lunardate | 0.2.2 | Lunar calendar | Port to TS |
| lunar-python | 1.4.4 | Chinese calendar | Port to TS |
| sxtwl | >=2.0.6 | Chinese calendar | Port to TS |
| psutil | >=5.9.0 | System monitoring | Workers metrics |
| python-jose | >=3.3.0 | JWT | jose npm package |
| PyJWT | >=2.10.0 | JWT | jose npm package |
| openai | >=1.84.0 | AI client | fetch() to Groq |
| supabase | >=2.3.0 | Database | D1 + custom auth |
| stripe | >=12.0.0 | Payments | stripe npm package |

### Frontend Dependencies (npm)

| Package | Version | Keep/Replace |
|---------|---------|--------------|
| react | ^18.2.0 | Keep |
| react-dom | ^18.2.0 | Keep |
| react-router-dom | ^7.6.2 | Keep |
| typescript | ^5.3.3 | Keep |
| axios | ^1.9.0 | Keep or use fetch |
| @supabase/supabase-js | ^2.50.0 | Replace with custom |
| @stripe/stripe-js | ^7.8.0 | Keep |
| echarts | ^5.6.0 | Keep |
| echarts-for-react | ^3.0.2 | Keep |
| tailwindcss | (in node_modules) | Keep |

### External Services

| Service | Purpose | Current Cost | Free Tier |
|---------|---------|--------------|-----------|
| Supabase | Auth + DB | $0-25/mo | Yes |
| Groq | AI Analysis | $0 | 14K req/day |
| Stripe | Payments | % of revenue | Yes |
| Render | Backend hosting | $7-25/mo | Limited |
| Vercel | Frontend hosting | $0-20/mo | Yes |

### Environment Variables Required

| Variable | Purpose | Migrate To |
|----------|---------|------------|
| SUPABASE_URL | Database | D1 binding |
| SUPABASE_ANON_KEY | Auth | Custom JWT |
| SUPABASE_SERVICE_ROLE_KEY | Admin | Worker secret |
| GROQ_API_KEY | AI | Worker secret |
| STRIPE_SECRET_KEY | Payments | Worker secret |
| JWT_SECRET_KEY | Auth | Worker secret |

### Checklist
- [x] Python dependencies listed
- [x] npm dependencies listed
- [x] External services documented
- [x] Environment variables listed
- [x] Migration alternatives identified

---

## 5. Current Performance Baseline ⚠️

### Estimated Performance (Need Measurement)

| Metric | Estimated | Target (New) |
|--------|-----------|--------------|
| API Response (p50) | 200-400ms | <100ms |
| API Response (p95) | 500-1000ms | <200ms |
| Chart Calculation | 500-2000ms | <150ms |
| AI Analysis | 2000-5000ms | <3000ms |
| Page Load Time | 2-4s | <2s |
| Error Rate | 2-3% | <0.5% |
| Uptime | 99.5% | >99.9% |

### ⚠️ ACTION REQUIRED
```bash
# Measure actual performance:
# 1. Use browser DevTools Network tab
# 2. Check Render dashboard for response times
# 3. Run load test with k6 or similar
```

### Checklist
- [x] Metrics categories defined
- [x] Targets set
- [ ] **TODO**: Measure actual baseline

---

## 6. Code Complexity Assessment ✅

### Code Size

| Component | Lines of Code | Files |
|-----------|---------------|-------|
| Backend (Python) | 38,429 | ~100+ |
| Frontend (TS/JS) | 19,397 | ~50+ |
| **Total** | **57,826** | ~150+ |

### Complexity Factors

| Factor | Assessment | Migration Impact |
|--------|------------|------------------|
| Zi Wei Calculation Engine | Complex | High - needs porting |
| Zodiac Calculation | Medium | Medium - needs porting |
| AI Integration | Medium | Low - API calls |
| Auth System | Medium | Medium - new approach |
| Database Queries | Medium | High - RLS removal |
| Admin Dashboard | Low | Low - can defer |

### Estimated Porting Effort

| Component | Effort (hours) | Priority |
|-----------|----------------|----------|
| Zi Wei Engine | 40-60 | P0 |
| Zodiac Engine | 20-30 | P0 |
| Auth System | 20-30 | P0 |
| API Routes | 30-40 | P0 |
| Database Schema | 10-15 | P0 |
| Frontend Updates | 20-30 | P1 |
| Admin Dashboard | 20-30 | P2 |
| **Total** | **160-235 hours** | - |

### Checklist
- [x] Code size measured
- [x] Complexity assessed
- [x] Effort estimated
- [x] Priorities assigned

---

## 📊 Audit Summary

### Completed: 6/6 sections (with TODOs)

### Go/No-Go Indicators

| Indicator | Status | Notes |
|-----------|--------|-------|
| Data fits D1 limits (<5GB) | ✅ Likely | Need production verification |
| No blocking PostgreSQL features | ⚠️ Caution | RLS, triggers need app-level replacement |
| User base manageable | ✅ Yes | <100 users, low traffic |
| Dependencies have alternatives | ✅ Yes | All can be migrated |
| Performance baseline established | ⚠️ Partial | Need actual measurements |

### Critical Migration Challenges

1. **Row Level Security (RLS)** - Must implement in application layer
2. **Triggers** - Must implement `updated_at` in application
3. **JSONB/Arrays** - Must use TEXT columns with JSON parsing
4. **Zi Wei Calculation Engine** - Complex Python → TypeScript port
5. **Chinese Calendar Libraries** - Need TypeScript alternatives

### Recommendation

**✅ PROCEED TO PHASE 0** with the following conditions:

1. Get actual production metrics from Supabase/Render dashboards
2. Verify database size is under 1GB
3. Plan for RLS replacement in application layer
4. Research TypeScript lunar calendar libraries
5. Allocate 160-235 hours for core migration work

---

## 📋 Outstanding TODOs

> **⚠️ HISTORICAL DOCUMENT** — Phase -1 audit completed. All items below were either resolved during subsequent phases or superseded by Cloudflare-native architecture decisions.

**Audit Period**: 2025-12-03 (Pre-Phase 0)
**Outcome**: ✅ GO decision — Migration proceeded successfully

### Items Resolved During Migration:
- [x] Database compatibility verified (D1/SQLite vs PostgreSQL)
- [x] Actual metrics gathered during Phase 0
- [x] API response times validated in production
- [x] Free tier limits documented (Workers/D1/DO/R2/AI)

### Superseded / Not Applicable:
- [x] Supabase dashboard access — N/A (migrated away from Supabase)
- [x] PostgreSQL custom functions — N/A (simplified to D1 schema)
- [x] RLS (Row Level Security) — Replaced by application-layer auth (SessionDO)

---

**Audit Completed By**: Kiro (Automated Analysis)
**Date**: 2025-12-03
**Status**: ✅ Historical reference — All critical items addressed in Phases 0-5
