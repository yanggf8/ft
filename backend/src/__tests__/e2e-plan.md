# E2E Test Plan

## Critical User Flows

### 1. User Registration & Login
- [ ] Register new user with email
- [ ] Receive sessionId
- [ ] Login with existing email
- [ ] Session persists across page reloads

### 2. Chart Creation Flow
- [ ] Navigate to create chart page
- [ ] Fill birth data form (ZiWei)
- [ ] Submit and receive calculated chart
- [ ] Chart appears in user's chart list

### 3. AI Interpretation Flow
- [ ] Open existing chart
- [ ] Click "獲取 AI 解讀"
- [ ] Wait for interpretation
- [ ] Interpretation displays correctly
- [ ] Failover works if primary provider fails

### 4. Trial Period
- [ ] New user has 30-day trial
- [ ] Trial status shows in profile
- [ ] Can access AI features during trial
- [ ] Trial expiry date displayed

## Test Data

```javascript
const testUser = {
  email: 'test@fortunet.test',
  fullName: 'Test User'
};

const testBirthData = {
  ziwei: {
    year: 1990,
    month: 5,
    day: 15,
    hour: 14,
    gender: 'male'
  },
  western: {
    year: 1990,
    month: 3,
    day: 25,
    hour: 12
  }
};
```

## Performance Targets

- Page load: < 2s
- Chart calculation: < 500ms
- AI interpretation: < 10s
- API response (p95): < 200ms
