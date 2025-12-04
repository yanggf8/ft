# Frontend API Contract Fixes

**Date**: 2025-12-04
**Status**: ✅ All issues resolved

---

## Issues Identified & Fixed

### 1. Auth Flow Mismatch ✅ FIXED

**Problem**: Frontend expected password-based auth with `token`, but backend uses passwordless auth with `sessionId`.

**Fix Applied**:
- `frontend/src/lib/api.ts`:
  - Changed `setToken/getToken` → `setSession/getSession`
  - Changed localStorage key: `token` → `sessionId`
  - `register()`: sends `{ email, full_name }`, stores `data.sessionId`
  - `login()`: sends `{ email }`, stores `data.sessionId`
  - Authorization header: `Bearer ${sessionId}`

- `frontend/src/contexts/AuthContext.tsx`:
  - Removed password parameters from `login()` and `register()`
  - Added optional `fullName` parameter to `register()`

- `frontend/src/pages/LoginPage.tsx`:
  - Removed password input field
  - Added optional full name field for registration
  - Added note: "本系統使用無密碼登入，僅需 Email"

**Result**: Auth flow now matches backend contract exactly.

---

### 2. Chart Creation Payload Incomplete ✅ FIXED

**Problem**: Frontend only sent `{ chart_type, birth_data }`, but backend requires `{ chart_type, chart_name, birth_data, chart_data }`.

**Fix Applied**:
- `frontend/src/lib/api.ts` → `createChart()`:
  ```typescript
  async createChart(chartType: 'ziwei' | 'western', birthData: any) {
    // Step 1: Calculate chart via public endpoint
    const calcEndpoint = chartType === 'ziwei' 
      ? '/api/charts/calculate/ziwei' 
      : '/api/charts/calculate/western';
    const chartData = await this.request(calcEndpoint, {
      method: 'POST',
      body: JSON.stringify(birthData)
    });

    // Step 2: Save with all required fields
    return this.request('/api/charts', {
      method: 'POST',
      body: JSON.stringify({
        chart_type: chartType,
        chart_name: `${chartType} - ${new Date().toLocaleDateString()}`,
        birth_data: birthData,
        chart_data: chartData
      })
    });
  }
  ```

**Result**: Chart creation now sends complete payload matching backend schema.

---

### 3. AI Interpretation Call Mismatch ✅ FIXED

**Problem**: Frontend sent `{ chart_id }`, but backend expects `{ chartType, chartData, language?, focus? }` and returns `{ interpretation, provider, model, tokensUsed }`.

**Fix Applied**:
- `frontend/src/lib/api.ts` → `interpretChart()`:
  ```typescript
  async interpretChart(chartId: string) {
    // Step 1: Fetch the chart to get chart_data
    const chart = await this.getChart(chartId);
    
    // Step 2: Call interpret with correct payload
    const result = await this.request('/api/charts/interpret', {
      method: 'POST',
      body: JSON.stringify({
        chartType: chart.chart_type,
        chartData: chart.chart_data,
        language: 'zh'
      })
    });

    // Step 3: Return merged chart with interpretation
    return { ...chart, ai_interpretation: result.interpretation };
  }
  ```

**Result**: AI interpretation now matches backend contract and returns usable data.

---

### 4. Type Definitions Updated ✅ FIXED

**Changes**:
- `frontend/src/types/index.ts`:
  - Changed `calculation_result` → `chart_data`
  - Added `chart_name: string` to Chart interface
  - Added `full_name?: string` to User interface
  - Made `minute?: number` optional in BirthData

- `frontend/src/pages/ChartPage.tsx`:
  - Changed `chart.calculation_result` → `chart.chart_data`

**Result**: Types now match backend response structure.

---

## Verification

```bash
cd frontend
npm run build
# ✅ Build successful: 179KB (57KB gzipped)
```

---

## API Contract Summary

### Auth Endpoints
```typescript
POST /api/auth/register
  Request:  { email: string, full_name?: string }
  Response: { sessionId: string, userId: string, email: string }

POST /api/auth/login
  Request:  { email: string }
  Response: { sessionId: string, userId: string, email: string }

GET /api/users/me
  Headers:  Authorization: Bearer {sessionId}
  Response: { id, email, full_name?, billing: {...} }
```

### Chart Endpoints
```typescript
POST /api/charts/calculate/ziwei
  Request:  { year, month, day, hour, gender }
  Response: { ...chartData, engineVersion }

POST /api/charts/calculate/western
  Request:  { year, month, day, hour?, minute?, latitude?, longitude? }
  Response: { ...chartData, engineVersion }

POST /api/charts
  Headers:  Authorization: Bearer {sessionId}
  Request:  { chart_type, chart_name, birth_data, chart_data }
  Response: { id, user_id, chart_type, chart_name, birth_data, chart_data, created_at }

GET /api/charts
  Headers:  Authorization: Bearer {sessionId}
  Response: { charts: [...], limit, offset }

GET /api/charts/:id
  Headers:  Authorization: Bearer {sessionId}
  Response: { id, user_id, chart_type, chart_name, birth_data, chart_data, created_at }

POST /api/charts/interpret
  Headers:  Authorization: Bearer {sessionId}
  Request:  { chartType, chartData, language?, focus? }
  Response: { interpretation, provider, model, tokensUsed, engineVersion }
```

---

## Status: ✅ Ready for Testing

All frontend-backend contract mismatches have been resolved. The application is now ready for integration testing.
