# Internal Testing Checklist

**Duration**: 3 days
**Team**: Dev team + QA
**Goal**: Validate all features before beta users

---

## Day 1: Core Functionality

### Registration & Authentication
- [ ] Register new account (test1@fortunet.test)
- [ ] Verify email format validation
- [ ] Login with registered account
- [ ] Session persists after browser restart
- [ ] Logout works correctly
- [ ] Try registering duplicate email (should fail)

### Trial Period
- [ ] New user has 30-day trial
- [ ] Trial end date displays correctly in profile
- [ ] Trial status shows "試用中" badge

**Test Accounts Created**:
1. test1@fortunet.test - ✅ / ❌
2. test2@fortunet.test - ✅ / ❌
3. test3@fortunet.test - ✅ / ❌
4. test4@fortunet.test - ✅ / ❌
5. test5@fortunet.test - ✅ / ❌

---

## Day 2: Chart Features

### ZiWei Chart Creation
- [ ] Create chart with valid data (1990-05-15, 14:00, male)
- [ ] Chart displays in profile list
- [ ] Click chart to view details
- [ ] Chart data shows correctly (palaces, stars)
- [ ] Create chart with edge case (2000-02-29, leap year)
- [ ] Try invalid year (1800) - should fail with error

**Test Cases**:
| Birth Date | Time | Gender | Result |
|------------|------|--------|--------|
| 1990-05-15 | 14:00 | Male | ✅ / ❌ |
| 1995-08-20 | 10:00 | Female | ✅ / ❌ |
| 2000-02-29 | 23:00 | Male | ✅ / ❌ |
| 1985-12-31 | 00:00 | Female | ✅ / ❌ |
| 2020-01-01 | 12:00 | Male | ✅ / ❌ |

### Western Chart Creation
- [ ] Create chart with valid data (1990-03-25, 12:00)
- [ ] Sun sign displays correctly (Aries)
- [ ] Moon sign shows (approximate)
- [ ] Chart displays in profile list
- [ ] Create multiple Western charts

**Test Cases**:
| Birth Date | Expected Sun Sign | Result |
|------------|-------------------|--------|
| 1990-03-25 | Aries | ✅ / ❌ |
| 1990-07-15 | Cancer | ✅ / ❌ |
| 1990-11-10 | Scorpio | ✅ / ❌ |

---

## Day 3: AI Features & Edge Cases

### AI Interpretation
- [ ] Request AI interpretation for ZiWei chart
- [ ] Interpretation completes (< 15 seconds)
- [ ] Interpretation text displays correctly
- [ ] Interpretation is in Chinese (zh)
- [ ] Request AI interpretation for Western chart
- [ ] Check AI provider failover (disable primary in code)
- [ ] Verify failover to secondary provider works

**AI Test Results**:
| Chart Type | Response Time | Quality (1-5) | Provider Used |
|------------|---------------|---------------|---------------|
| ZiWei | ___ sec | ___ | iFlow / Groq / Cerebras |
| Western | ___ sec | ___ | iFlow / Groq / Cerebras |

### Cross-Browser Testing
- [ ] Chrome (desktop) - all features work
- [ ] Firefox (desktop) - all features work
- [ ] Safari (desktop) - all features work
- [ ] Chrome (mobile) - all features work
- [ ] Safari (iOS) - all features work

### Performance Testing
- [ ] Page load < 2 seconds
- [ ] Chart calculation < 500ms
- [ ] AI interpretation < 15 seconds
- [ ] No console errors
- [ ] No network errors (check DevTools)

### Error Handling
- [ ] Invalid birth date (e.g., 2000-02-30)
- [ ] Missing required fields
- [ ] Network timeout simulation
- [ ] Rate limit hit (make 35+ requests quickly)
- [ ] All errors show user-friendly messages

---

## Issues Found

| # | Priority | Description | Status |
|---|----------|-------------|--------|
| 1 | P0/P1/P2/P3 | [Description] | Open/Fixed |
| 2 | | | |
| 3 | | | |

**Priority Levels**:
- **P0**: Blocker - prevents core functionality
- **P1**: Critical - major feature broken
- **P2**: High - minor feature broken
- **P3**: Medium - UX issue

---

## Go/No-Go Decision

### GO Criteria (all must be ✅)
- [ ] All P0 bugs fixed
- [ ] All P1 bugs fixed or have workarounds
- [ ] Core features working (registration, chart creation, AI)
- [ ] No data loss or corruption
- [ ] Performance acceptable (< 2s page load)

### Decision: ✅ GO / ❌ NO-GO

**Reason**: _______________________

**Sign-off**: ___________ Date: _______

---

**Next**: Proceed to beta user testing
