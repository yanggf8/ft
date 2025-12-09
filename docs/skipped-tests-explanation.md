# Why Tests Are Skipped

## Backend: 3 Integration Tests Skipped

**File**: `backend/src/__tests__/integration/charts.test.ts`

**Reason**: Line 4 shows `describe.skip()`
```typescript
// Integration tests require deployed API - skip by default
// Run with: npm test -- --run integration
describe.skip('Charts API Integration', () => {
```

**Why skipped**:
- These tests call the **live production API**
- Would hit real Cloudflare Workers
- Would consume AI API credits
- Meant for manual verification, not CI/CD

**How to run them**:
```bash
npm test -- --run integration
```

**Tests skipped**:
1. ZiWei chart calculation
2. Invalid year rejection  
3. Western chart calculation (likely)

---

## Summary

- **Unit tests**: ✅ 15 passing (run automatically)
- **Integration tests**: ⏭️ 3 skipped (require live API)
- **Total coverage**: Unit tests only

**Impact**: None - unit tests cover the logic, integration tests are for manual verification.

**Recommendation**: Run integration tests manually before beta launch.
