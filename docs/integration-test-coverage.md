# Integration Test Coverage Analysis

**Date**: 2025-12-09  
**Question**: Are current tests sufficient to catch regressions?

---

## 2026-08-29 現況更新（人工 gap 盤點後）

對生產 fortnet-api（v`5beb9de1`）以 curl 盤點（`/tmp/gap-probe.sh`，同 verify-big5.sh 手法）。核心流程皆健康；兩個原本標 critic 的 gap 已在 **commit 6be4fbf** 修復並復測通過：

- **重複註冊 email** → 原回 500（duplicate 檢查 `SELECT id` 缺 `email` 欄位 → 反序列化失敗）；已改 `SELECT id, email` → **現回 409**。
- **日曆無效生日**（2000-02-30 / 04-31 / 2021-02-29）→ 原本只驗 range、被接受；已加 days-in-month（含閏年）驗證 → **現回 400**。

本次盤點**驗證通過**的項目（原文皆標 critic gap，已非盲區）：auth 401（無/壞 token）＋ GET /users/me、建圖（ziwei `palaces` / western `sunSign`）、GET /api/charts 列表、AI interpret（ZiWei 200、約 4s、帶 provider/model、bogus type→400）、無 birth data→400、year/month 範圍。

**Rate limiting**：確認為 **per-isolate 設計限制**（`limiter()` 是 isolate 本機 `OnceLock` static，Cloudflare 多 isolate 各自計數 → 跨請求不會累計到 10），**非 code bug**；要全球嚴格 10/min 需上 Durable Object（獨立未來項）。

**⚠️ 過時註記**：本文件（2025-12-09）多處端點（`/api/charts/calculate/ziwei`、`/api/charts/interpret`、`X-Session-ID`）與現行 routes（crates/api/src/routes）不符；`scripts/verify-deployment.sh` 亦同。請以現行 crates/api 為準。

<details><summary>原文（2025-12-09 快照，端點未更新）</summary>

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

</details>
