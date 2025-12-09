# Integration Test Coverage Analysis

**Date**: 2025-12-09  
**Question**: Are current tests sufficient to catch regressions?

---

## ✅ What's Covered (3 tests)

1. **ZiWei calculation** - Happy path
2. **ZiWei validation** - Invalid year rejection
3. **Western calculation** - Happy path

---

## ❌ Critical Gaps - Will NOT Catch These Bugs

### Authentication & Authorization
- [ ] Login flow (POST /api/auth/login)
- [ ] Session validation
- [ ] Protected route access
- [ ] Trial period expiration
- [ ] Unauthorized access rejection

### AI Interpretation
- [ ] POST /api/charts/interpret (ZiWei)
- [ ] POST /api/charts/interpret (Western)
- [ ] AI provider failover (iFlow → Groq → Cerebras)
- [ ] AI error handling
- [ ] Rate limiting on AI calls

### User Management
- [ ] GET /api/users/me
- [ ] User profile retrieval
- [ ] Billing status check
- [ ] Trial period tracking

### Chart Storage & Retrieval
- [ ] Saving charts to D1
- [ ] Retrieving user's charts
- [ ] Chart history
- [ ] Chart deletion

### Error Handling
- [ ] Malformed JSON
- [ ] Missing required fields
- [ ] Database errors
- [ ] Network timeouts
- [ ] AI API failures

### Edge Cases
- [ ] Leap year dates
- [ ] Timezone handling
- [ ] Concurrent requests
- [ ] Large payloads
- [ ] Special characters in input

---

## 🎯 Coverage Assessment

| Category | Coverage | Risk |
|----------|----------|------|
| **Chart Calculation** | 67% | Medium (2/3 engines tested) |
| **Authentication** | 0% | **HIGH** |
| **AI Interpretation** | 0% | **HIGH** |
| **User Management** | 0% | **HIGH** |
| **Error Handling** | 10% | **HIGH** |
| **Edge Cases** | 0% | Medium |

**Overall Coverage**: ~15%  
**Regression Risk**: **HIGH**

---

## 🚨 Critical Missing Tests

### Priority 1: Core User Flows
```typescript
// 1. Full user journey
it('should complete full user flow', async () => {
  // Login
  const login = await fetch(`${API_URL}/api/auth/login`, {
    method: 'POST',
    body: JSON.stringify({ email: 'test@example.com' })
  });
  const { sessionId } = await login.json();
  
  // Calculate chart
  const chart = await fetch(`${API_URL}/api/charts/calculate/ziwei`, {
    method: 'POST',
    headers: { 'X-Session-ID': sessionId },
    body: JSON.stringify({ year: 1990, month: 5, day: 15, hour: 14, gender: 'male' })
  });
  
  // Get AI interpretation
  const interpret = await fetch(`${API_URL}/api/charts/interpret`, {
    method: 'POST',
    headers: { 'X-Session-ID': sessionId },
    body: JSON.stringify({ chartType: 'ziwei', chartData: await chart.json() })
  });
  
  expect(interpret.status).toBe(200);
});
```

### Priority 2: AI Failover
```typescript
// 2. AI provider failover (requires API key manipulation)
it('should failover to Groq when iFlow fails', async () => {
  // This requires testing with invalid iFlow key
  // Would catch: failover logic, Groq integration, error handling
});
```

### Priority 3: Authorization
```typescript
// 3. Protected routes
it('should reject requests without session', async () => {
  const response = await fetch(`${API_URL}/api/charts/interpret`, {
    method: 'POST',
    body: JSON.stringify({ chartType: 'ziwei', chartData: {} })
  });
  expect(response.status).toBe(401);
});
```

---

## 📊 Recommended Test Suite

### Minimum Viable (10 tests)
1. ✅ ZiWei calculation (exists)
2. ✅ Western calculation (exists)
3. ✅ Invalid input rejection (exists)
4. ❌ **Login flow**
5. ❌ **Session validation**
6. ❌ **AI interpretation (ZiWei)**
7. ❌ **AI interpretation (Western)**
8. ❌ **Protected route rejection**
9. ❌ **User profile retrieval**
10. ❌ **Trial period check**

### Comprehensive (20+ tests)
- Add all Priority 1-3 tests above
- Add edge cases (leap years, timezones)
- Add error scenarios (network, DB, AI failures)
- Add concurrent request handling
- Add rate limiting tests

---

## 🎯 Answer to Your Question

### Can current tests catch regressions?

**NO** - Only 15% coverage

**What they WILL catch**:
- ✅ ZiWei calculation breaks
- ✅ Western calculation breaks
- ✅ Input validation breaks

**What they WON'T catch**:
- ❌ Authentication breaks
- ❌ AI interpretation breaks
- ❌ Session management breaks
- ❌ Authorization breaks
- ❌ Database operations break
- ❌ Failover logic breaks
- ❌ 85% of the system

---

## 💡 Recommendation

### Before Beta Launch
Add **minimum 7 more tests**:
1. Login flow
2. Session validation
3. AI interpretation (ZiWei)
4. AI interpretation (Western)
5. Protected route rejection
6. User profile retrieval
7. Trial period check

**Effort**: 2-3 hours  
**Coverage**: 15% → 60%  
**Risk reduction**: HIGH → MEDIUM

### Current State
**Risk**: You'll only catch 15% of regressions  
**Recommendation**: **Add more tests before beta**

---

## 🚦 Decision

- **Ship with current tests?** ⚠️ Risky (85% blind spots)
- **Add minimum tests first?** ✅ Recommended (2-3 hours)
- **Add comprehensive tests?** 📅 Post-beta (Week 22-25)

**Bottom line**: Current tests are insufficient. Add at least 7 more before beta.
