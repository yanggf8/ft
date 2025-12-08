# 🚀 FortuneT V2 - AI-Powered Astrology Platform

**Status**: ✅ Production Live
**Timeline**: Phase 5 Complete - Ready for Beta Testing

---

## 🌐 Live URLs

- **Frontend**: https://fortunet.pages.dev
- **Backend API**: https://fortunet-api.yanggf.workers.dev

---

## 📋 Quick Start

**For Developers**: [AGENTS.md](./AGENTS.md)
**For Planning**: [MASTER_PLAN.md](./MASTER_PLAN.md)

---

## 🎯 What's Live

### Core Features ✅
- **ZiWei (紫微斗數)** - Traditional Chinese astrology
- **Western Zodiac** - Sun/Moon signs and planets
- **AI Interpretation** - 3-provider failover (iFlow → Groq → Cerebras)
- **Free Trial** - 30 days for all new users
- **Passwordless Auth** - Email-only login

### Infrastructure ✅
- **Frontend**: React + TypeScript + Vite (179KB / 57KB gzipped)
- **Backend**: Cloudflare Workers + Hono
- **Database**: D1 (SQLite)
- **Cache**: Durable Objects (Session + AI Mutex)
- **AI**: iFlow GLM-4.6, Groq Kimi-K2, Cerebras Llama-3.3-70b

---

## 💻 Development

### Backend
```bash
cd backend
npm run dev          # Local dev (localhost:8787)
npm test             # Run tests (15 passing)
npm run typecheck    # TypeScript check

# Deploy
unset CLOUDFLARE_API_TOKEN
npm run deploy
```

### Frontend
```bash
cd frontend
npm run dev          # Local dev (localhost:5173)
npm test             # Run tests (3 passing)
npm run build        # Build for production

# Deploy
unset CLOUDFLARE_API_TOKEN
npm run deploy
```

### Verification
```bash
./scripts/verify-deployment.sh  # Verify production
```

---

## 📁 Documentation

| Document | Purpose |
|----------|---------|
| **[AGENTS.md](./AGENTS.md)** | Dev guide & critical rules |
| **[MASTER_PLAN.md](./MASTER_PLAN.md)** | Migration timeline & phases |
| [DEPLOY_FRONTEND.md](./DEPLOY_FRONTEND.md) | Deployment instructions |
| [FRONTEND_FIXES.md](./FRONTEND_FIXES.md) | API contract fixes |
| [docs/phase5-summary.md](./docs/phase5-summary.md) | Phase 5 deliverables |
| [docs/monitoring-setup.md](./docs/monitoring-setup.md) | Monitoring & alerts |
| [docs/rollback-procedures.md](./docs/rollback-procedures.md) | Emergency procedures |

---

## 📅 Project Status

```
Phase -1: System Audit        Week 0      ✅ COMPLETED
Phase 0:  Risk Assessment     Week 1-3    ✅ COMPLETED (GO)
Phase 1:  Foundation          Week 4-6    ✅ COMPLETED
Phase 2:  Core Features       Week 7-11   ✅ COMPLETED
Phase 3:  Frontend            Week 12-15  ✅ COMPLETED
Phase 4:  Integration/Test    Week 16-18  ✅ COMPLETED
Phase 5:  Pre-Migration       Week 19-20  ✅ COMPLETED
Phase 6:  Go-Live             Week 21     ← READY (Beta Testing)
```

---

## 🧪 Beta Testing

**Status**: Materials ready, awaiting execution

**Guides**:
- `docs/internal-testing-checklist.md` - 3-day internal testing
- `docs/beta-invitation.md` - User invitation template
- `docs/beta-feedback-form.md` - 21 questions for Google Forms
- `docs/beta-testing-tracker.md` - Metrics & bug tracking
- `docs/beta-week20-guide.md` - Day-by-day execution plan

**Target**: 10-20 beta users over 7 days

---

## 🛠️ Tech Stack

- **Frontend**: React 18, TypeScript 5, Vite 5
- **Backend**: Cloudflare Workers, Hono 4
- **Database**: D1 (SQLite)
- **Cache**: Durable Objects
- **Storage**: R2
- **AI**: iFlow / Groq / Cerebras (free tiers)

---

## 📊 Cost Summary

| Phase | Monthly Cost |
|-------|--------------|
| Testing (5-10 DAU) | **$0** |
| Early Growth (20-50 DAU) | $30-80 |
| Scale (50+ DAU) | $100-303 |

**Current**: $0 (within free tiers)

---

**Last Updated**: 2025-12-08
**Version**: 2.0.0
**License**: Private
