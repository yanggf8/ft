# Week 20: Beta Testing Quick Guide

**Duration**: 4 days
**Goal**: Validate with 10-20 real users

---

## Pre-Beta Checklist

- [ ] Internal testing complete (Day 1-3)
- [ ] All P0/P1 bugs fixed
- [ ] Frontend deployed
- [ ] Backend deployed with security headers
- [ ] Monitoring alerts configured
- [ ] Google Form created
- [ ] Beta invitation email ready

---

## Day-by-Day Plan

### Day 1: Launch
**Morning**:
1. Send invitations to 10-15 beta users
2. Monitor registrations in real-time
3. Check Cloudflare Dashboard for errors

**Afternoon**:
4. Respond to any questions
5. Check first charts created
6. Monitor AI interpretation requests

**Evening**:
7. Review metrics (registrations, charts, errors)
8. Fix any urgent issues

### Day 2-3: Active Testing
**Daily**:
1. Check metrics every 4 hours
2. Respond to user questions
3. Monitor error rates
4. Fix P0/P1 bugs immediately
5. Document P2/P3 bugs for later

### Day 4: Wrap-up
**Morning**:
1. Send reminder for feedback form
2. Check final metrics

**Afternoon**:
3. Review all feedback
4. Analyze metrics
5. Make go/no-go decision

---

## Monitoring Checklist (Every 4 Hours)

- [ ] Check Cloudflare Dashboard
  - Error rate < 1%?
  - p95 latency < 200ms?
  - Any 5xx errors?

- [ ] Check user activity
  - New registrations?
  - Charts created?
  - AI interpretations requested?

- [ ] Check for issues
  - Any error reports?
  - Any stuck requests?
  - Any user complaints?

---

## Quick Commands

### Check API health
```bash
./scripts/verify-deployment.sh
```

### Check recent errors (Cloudflare Dashboard)
1. Go to Workers & Pages → fortunet-api
2. Click "Logs" tab
3. Filter by "error"

### Check user stats (D1)
```bash
# Via wrangler
wrangler d1 execute fortunet-db --remote --command "SELECT COUNT(*) FROM users"
wrangler d1 execute fortunet-db --remote --command "SELECT COUNT(*) FROM chart_records"
```

### Rollback if needed
```bash
cd backend
wrangler rollback
```

---

## Communication Templates

### Daily Update (Internal)
```
Beta Testing Day [X] Update:
- Registrations: X/10
- Charts created: X
- AI interpretations: X
- Errors: X (P0: X, P1: X, P2: X)
- Uptime: X%
- Issues: [List any]
```

### User Response (Bug Report)
```
Hi [Name],

Thanks for reporting this! We've logged it as [Bug #X] and will fix it [today/this week].

In the meantime, [workaround if available].

Thanks for helping us improve!
```

### Feedback Reminder (Day 4)
```
Hi [Name],

Thanks for testing FortuneT! If you haven't already, please fill out our 2-minute feedback form:

[Google Form URL]

Your trial has been extended to 60 days as a thank you!
```

---

## Go/No-Go Criteria

### ✅ GO if:
- Uptime > 99.5%
- Error rate < 1%
- User satisfaction > 3.5/5
- All P0/P1 bugs fixed
- No data loss

### ❌ NO-GO if:
- Any P0 bugs unfixed
- Data corruption
- Security vulnerability
- User satisfaction < 3.0/5
- Uptime < 95%

---

## Post-Beta Actions

### If GO:
1. Fix all P2 bugs
2. Schedule Phase 6 (Go-Live)
3. Prepare launch announcement
4. Thank beta users

### If NO-GO:
1. Fix critical issues
2. Extend beta by 1 week
3. Re-test with same users
4. Re-evaluate

---

## Files to Use

1. **Beta invitation**: `docs/beta-invitation.md`
2. **Feedback form**: `docs/beta-feedback-form.md`
3. **Internal testing**: `docs/internal-testing-checklist.md`
4. **Tracking**: `docs/beta-testing-tracker.md`

---

**Ready to start? Begin with internal testing!**
