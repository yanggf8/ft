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
cd crates/api
unset CLOUDFLARE_API_TOKEN && wrangler deployments list

# 2. Identify last good deployment
# Look for deployment before the issue started

# 3. Rollback
unset CLOUDFLARE_API_TOKEN && wrangler rollback

# 4. Verify
./scripts/verify-deployment.sh

# 5. Notify team
# Post in Slack/Discord: "Rolled back API to [version] due to [issue]"
```
> Historical: `cd backend` / bare `wrangler` (without `unset CLOUDFLARE_API_TOKEN &&`) superseded by Rust workspace 98d3521 — `backend/` removed, OAuth via `unset CLOUDFLARE_API_TOKEN && wrangler`.

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

# Or via CLI (Rust/Leptos)
# Pages doesn't have direct rollback, redeploy previous commit
git log --oneline
git checkout <previous-commit>
./scripts/build-web.sh   # or: cargo build -p ft-web --target wasm32-unknown-unknown
unset CLOUDFLARE_API_TOKEN && wrangler pages deploy crates/web/dist --project-name=fortunet
git checkout main
```
> Historical: `cd frontend` / `npm run build` / `wrangler pages deploy dist` superseded by Rust workspace 98d3521 — `frontend/` removed, use `crates/web` + `scripts/build-web.sh` + `scripts/deploy-web.sh`.

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
# Test locally (Turso)
turso db shell fortunet < scripts/schema.sql   # single source of truth

# Then apply to production (same file; Turso, not D1)
turso db shell fortunet < scripts/schema.sql
```
> Historical: `npm run db:init:local` / `npm run db:init` (D1) removed in 98d3521 — now Turso (`scripts/schema.sql`).

**Recovery**:
- Turso point-in-time recovery via Turso dashboard/support (not D1)
- Must manually write reverse migration if needed
- Contact Turso/Cloudflare support for point-in-time recovery

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
```rust
// In crates/api/src/durable_objects/session.rs (SessionDO) or crates/api/src/routes/common.rs
// Comment out DO calls, use stateless auth temporarily
```
> Historical: `backend/src/middleware/auth.ts` removed in 98d3521 — now `crates/api/src/durable_objects/session.rs` + `crates/api/src/routes/` (Rust/workers-rs).

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
```rust
// In crates/api/src/routes/charts.rs
// Add at top of /interpret endpoint:
return Err(ApiError::ServiceUnavailable("AI service temporarily unavailable"));
```
> Historical: `backend/src/routes/charts.ts` (`c.json(..., 503)`) removed in 98d3521 — now `crates/api/src/routes/charts.rs` (Rust/workers-rs).

**User Communication**:
- Update status page
- Show banner: "AI interpretations temporarily unavailable"

### Time to Recovery: Depends on provider

---

## Scenario 6: Rate Limiting Too Aggressive

### Issue: Legitimate users getting 429s

**Quick Fix**: Increase limits temporarily
```rust
// In crates/api/src/routes/charts.rs (or crates/api/src/routes/common.rs limiter)
const CALC_LIMIT: u32 = 30; // Change to 60
const AI_LIMIT: u32 = 10;   // Change to 20
```
> Historical: `backend/src/routes/charts.ts` removed in 98d3521 — now `crates/api/src/routes/charts.rs` (Rust).

**Deploy**:
```bash
unset CLOUDFLARE_API_TOKEN && wrangler deploy   # from crates/api, or: ./scripts/deploy-engine.sh / ./scripts/deploy-web.sh
# Historical: cd backend && npm run deploy removed in 98d3521
```

**Proper Fix**: Implement per-user rate limiting (not per-IP)

### Time to Recovery: < 5 minutes

---

## Scenario 7: Security Incident

### Issue: API Key Leaked

**Immediate**:
```bash
# Rotate all secrets
unset CLOUDFLARE_API_TOKEN && wrangler secret put IFLOW_API_KEY
unset CLOUDFLARE_API_TOKEN && wrangler secret put GROQ_API_KEY
unset CLOUDFLARE_API_TOKEN && wrangler secret put CEREBRAS_API_KEY
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
