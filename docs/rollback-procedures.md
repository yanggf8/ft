# Rollback Procedures

**Purpose**: Quick recovery from deployment issues
**Audience**: DevOps, On-call engineers

---

## Quick Reference

| Issue | Command | Time |
|-------|---------|------|
| Bad Workers deployment | `wrangler rollback` | < 1 min |
| Database corruption | Contact Cloudflare support | 15-30 min |
| AI provider down | Automatic failover | Instant |
| Frontend issue | Revert Pages deployment | < 2 min |

---

## Scenario 1: Workers API Rollback

### When to Use
- New deployment causing errors
- Performance degradation
- Breaking API changes

### Steps
```bash
# 1. Check current deployment
cd backend
wrangler deployments list

# 2. Identify last good deployment
# Look for deployment before the issue started

# 3. Rollback
wrangler rollback

# 4. Verify
./scripts/verify-deployment.sh

# 5. Notify team
# Post in Slack/Discord: "Rolled back API to [version] due to [issue]"
```

### Verification
- [ ] Health check passes
- [ ] Error rate drops
- [ ] User reports stop
- [ ] All tests pass

### Time to Recovery: < 5 minutes

---

## Scenario 2: Frontend Rollback (Cloudflare Pages)

### When to Use
- UI breaking changes
- Build errors
- Asset loading failures

### Steps
```bash
# Via Cloudflare Dashboard
1. Go to Workers & Pages → fortunet-frontend
2. Click "View builds"
3. Find last good deployment
4. Click "..." → "Rollback to this deployment"
5. Confirm

# Or via CLI
cd frontend
# Pages doesn't have direct rollback, redeploy previous commit
git log --oneline
git checkout <previous-commit>
npm run build
wrangler pages deploy dist
git checkout main
```

### Verification
- [ ] Site loads correctly
- [ ] No console errors
- [ ] All pages accessible
- [ ] Auth flow works

### Time to Recovery: < 5 minutes

---

## Scenario 3: Database Issues

### Issue: Schema Migration Failed

**Prevention**: Always test migrations locally first
```bash
# Test locally
npm run db:init:local

# Then apply to production
npm run db:init
```

**Recovery**:
- D1 doesn't support automatic rollback
- Must manually write reverse migration
- Contact Cloudflare support for point-in-time recovery

### Issue: Data Corruption

**Steps**:
1. Identify affected records
2. Contact Cloudflare support immediately
3. Request point-in-time recovery
4. Verify data integrity after recovery

### Time to Recovery: 30-60 minutes (depends on Cloudflare support)

---

## Scenario 4: Durable Objects Issues

### Issue: Session DO Causing Errors

**Quick Fix**: Disable feature temporarily
```typescript
// In backend/src/middleware/auth.ts
// Comment out DO calls, use stateless auth temporarily
```

**Proper Fix**:
1. Identify bug in session-do.ts
2. Fix and test locally
3. Deploy fix
4. Re-enable feature

### Issue: AI Mutex DO Stuck

**Symptoms**: AI requests timing out

**Fix**:
```bash
# Reset the DO (loses in-memory state)
# Via Cloudflare Dashboard:
# Workers & Pages → Durable Objects → AI_MUTEX → Delete instances
```

**Note**: DO state is in SQLite, so quota tracking persists

### Time to Recovery: < 10 minutes

---

## Scenario 5: AI Provider Failures

### Issue: Primary Provider (iFlow) Down

**Automatic**: Failover to Groq (secondary)
**Manual**: Check AI Mutex DO logs

### Issue: All Providers Down

**Temporary Disable**:
```typescript
// In backend/src/routes/charts.ts
// Add at top of /interpret endpoint:
return c.json({ error: 'AI service temporarily unavailable' }, 503);
```

**User Communication**:
- Update status page
- Show banner: "AI interpretations temporarily unavailable"

### Time to Recovery: Depends on provider

---

## Scenario 6: Rate Limiting Too Aggressive

### Issue: Legitimate users getting 429s

**Quick Fix**: Increase limits temporarily
```typescript
// In backend/src/routes/charts.ts
const CALC_LIMIT = 30; // Change to 60
const AI_LIMIT = 10;   // Change to 20
```

**Deploy**:
```bash
cd backend
npm run deploy
```

**Proper Fix**: Implement per-user rate limiting (not per-IP)

### Time to Recovery: < 5 minutes

---

## Scenario 7: Security Incident

### Issue: API Key Leaked

**Immediate**:
```bash
# Rotate all secrets
wrangler secret put IFLOW_API_KEY
wrangler secret put GROQ_API_KEY
wrangler secret put CEREBRAS_API_KEY
```

**Follow-up**:
1. Review access logs
2. Check for unauthorized usage
3. Update key in password manager
4. Notify team

### Issue: SQL Injection Discovered

**Immediate**: Take API offline
```bash
# Deploy maintenance mode
# Or disable affected endpoint
```

**Fix**: Patch vulnerability, test, redeploy

### Time to Recovery: Varies (security first)

---

## Scenario 8: Complete Outage

### Issue: Cloudflare Infrastructure Down

**Check**: https://www.cloudflarestatus.com/

**Action**: Wait for Cloudflare to resolve

**Communication**:
- Update status page
- Post on social media
- Email users (if critical)

### Issue: Account Suspended

**Action**: Contact Cloudflare support immediately

---

## Rollback Checklist

Before any rollback:
- [ ] Identify the issue clearly
- [ ] Determine root cause (if possible)
- [ ] Choose appropriate rollback method
- [ ] Notify team
- [ ] Execute rollback
- [ ] Verify recovery
- [ ] Document incident
- [ ] Plan proper fix

After rollback:
- [ ] Monitor for 30 minutes
- [ ] Check error rates
- [ ] Review user feedback
- [ ] Create post-mortem
- [ ] Implement prevention measures

---

## Testing Rollback Procedures

### Monthly Drill
1. Deploy a "bad" version to staging
2. Practice rollback
3. Time the process
4. Update procedures if needed

### Staging Environment
- Use separate Cloudflare account
- Mirror production setup
- Test all rollback scenarios

---

## Emergency Contacts

| Role | Contact | Availability |
|------|---------|--------------|
| Primary On-call | [TBD] | 24/7 |
| Backup On-call | [TBD] | 24/7 |
| Cloudflare Support | support.cloudflare.com | 24/7 |
| AI Provider Support | [Provider docs] | Business hours |

---

## Post-Incident Review

### Template
```markdown
# Incident Report: [Date]

## Summary
[Brief description]

## Timeline
- [Time]: Issue detected
- [Time]: Rollback initiated
- [Time]: Service restored
- [Time]: Root cause identified

## Impact
- Duration: [X minutes]
- Users affected: [Estimate]
- Requests failed: [Count]

## Root Cause
[Technical explanation]

## Resolution
[What was done]

## Prevention
- [ ] Action item 1
- [ ] Action item 2

## Lessons Learned
[What we learned]
```

---

**Status**: Procedures documented and ready
**Last Updated**: 2025-12-05
**Next Review**: Before Phase 6 (Go-Live)
