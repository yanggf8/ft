# Monitoring & Observability Setup

**Phase**: 5 (Pre-Migration)
**Goal**: Set up monitoring before beta testing

---

## Cloudflare Analytics (Built-in)

### Workers Analytics
**Location**: Cloudflare Dashboard → Workers & Pages → fortunet-api

**Metrics Available**:
- Requests per second
- Success rate (2xx/3xx/4xx/5xx)
- CPU time
- Duration (p50, p75, p95, p99)
- Errors

**Alerts** (Configure in Dashboard):
- [ ] Error rate > 5% for 5 minutes
- [ ] p95 latency > 500ms for 5 minutes
- [ ] Requests drop to 0 (service down)

### D1 Analytics
**Location**: Cloudflare Dashboard → D1 → fortunet-db

**Metrics**:
- Queries per second
- Read/write ratio
- Storage usage
- Query duration

### Durable Objects Analytics
**Metrics**:
- Active objects
- Requests per object
- CPU time per object

---

## Custom Logging (Structured)

### Log Format
```typescript
interface LogEntry {
  timestamp: string;
  level: 'info' | 'warn' | 'error';
  requestId: string;
  userId?: string;
  action: string;
  duration?: number;
  error?: string;
  metadata?: Record<string, any>;
}
```

### Key Events to Log
1. **User Actions**
   - Registration
   - Login
   - Chart creation
   - AI interpretation request

2. **System Events**
   - AI provider failover
   - Rate limit hit
   - Database errors
   - Session creation/destruction

3. **Performance**
   - Slow queries (> 100ms)
   - Slow AI responses (> 15s)
   - Large payloads (> 100KB)

### Implementation
```typescript
// backend/src/lib/logger.ts
export function log(entry: LogEntry) {
  console.log(JSON.stringify(entry));
  // Cloudflare automatically captures console.log
}
```

---

## AI Provider Monitoring

### Metrics to Track (via AI Mutex DO)
- Requests per provider per day
- Tokens used per provider per day
- Error rate per provider
- Failover events
- Average latency per provider

### Query AI Stats
```bash
# Via Durable Object storage
curl https://fortunet-api.yanggf.workers.dev/api/admin/ai-stats
```

---

## User Metrics (Application Level)

### Track in D1
```sql
-- Daily active users
SELECT COUNT(DISTINCT user_id) 
FROM chart_records 
WHERE DATE(created_at) = DATE('now');

-- Charts created per day
SELECT DATE(created_at) as date, COUNT(*) as count
FROM chart_records
GROUP BY DATE(created_at)
ORDER BY date DESC
LIMIT 30;

-- AI interpretation usage
SELECT COUNT(*) 
FROM chart_records 
WHERE ai_interpretation IS NOT NULL;

-- Trial users vs paid
SELECT subscription_tier, COUNT(*) 
FROM users 
GROUP BY subscription_tier;
```

---

## Alerting Strategy

### Critical Alerts (Immediate Action)
- 🚨 API error rate > 10% for 5 minutes
- 🚨 Database unavailable
- 🚨 All AI providers failing
- 🚨 Workers deployment failed

### Warning Alerts (Monitor)
- ⚠️ API error rate > 5% for 10 minutes
- ⚠️ p95 latency > 500ms for 10 minutes
- ⚠️ AI provider failover occurred
- ⚠️ Rate limit hit frequently (> 100 times/hour)

### Info Alerts (Daily Summary)
- ℹ️ Daily active users
- ℹ️ Charts created today
- ℹ️ AI interpretations requested
- ℹ️ New user registrations

---

## Dashboard Setup (Optional)

### Option 1: Cloudflare Dashboard
- Built-in, no setup required
- Good for basic metrics
- Limited customization

### Option 2: Grafana Cloud (Free Tier)
- More detailed dashboards
- Custom queries
- Requires log shipping

### Option 3: Simple Admin Page
```typescript
// backend/src/routes/admin.ts (protected)
app.get('/admin/stats', authMiddleware, async (c) => {
  const stats = {
    users: await c.env.DB.prepare('SELECT COUNT(*) as count FROM users').first(),
    charts: await c.env.DB.prepare('SELECT COUNT(*) as count FROM chart_records').first(),
    chartsToday: await c.env.DB.prepare(
      "SELECT COUNT(*) as count FROM chart_records WHERE DATE(created_at) = DATE('now')"
    ).first(),
  };
  return c.json(stats);
});
```

---

## Health Check Endpoints

### Current Endpoints
- `GET /health` - Basic health
- `GET /health/db` - Database connectivity

### Additional Endpoints (TODO)
- `GET /health/ai` - AI providers status
- `GET /health/do` - Durable Objects status

---

## Incident Response

### When Alert Fires
1. **Check Cloudflare Dashboard** for error details
2. **Check recent deployments** (rollback if needed)
3. **Check AI provider status** (external services)
4. **Check rate limits** (legitimate traffic spike?)
5. **Review logs** for error patterns

### Escalation
1. **Level 1**: Automated rollback (if deployment issue)
2. **Level 2**: Disable failing feature (e.g., AI interpretation)
3. **Level 3**: Contact Cloudflare support (infrastructure issue)

---

## Beta Testing Monitoring

### Daily Checklist
- [ ] Check error rate (should be < 1%)
- [ ] Check p95 latency (should be < 200ms)
- [ ] Review user feedback
- [ ] Check for new bugs reported
- [ ] Verify AI providers working

### Weekly Report
- Total users registered
- Total charts created
- AI interpretations requested
- Error rate trend
- Performance trend
- Top 5 errors

---

## Post-Launch Monitoring

### Week 1 (Intensive)
- Monitor every 4 hours
- Daily summary report
- Quick response to issues

### Week 2-4 (Active)
- Monitor daily
- Weekly summary report
- Standard response time

### Month 2+ (Steady State)
- Monitor weekly
- Monthly summary report
- Automated alerts only

---

**Status**: Ready for beta testing
**Next**: Configure Cloudflare alerts before beta launch
