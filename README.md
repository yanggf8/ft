# 🚀 FortuneT V2 - AI-Powered Astrology Platform

**Status**: ✅ Production Live
**Timeline**: Phase 6 Go-Live — Beta Testing

---

## 🌐 Live URLs

- **Frontend**: https://fortunet.pages.dev
- **Backend API**: https://fortunet-api.yanggf.workers.dev
- **Engine Worker** (service binding): https://fortunet-engine.yanggf.workers.dev

---

## 📋 Quick Start

**For Developers**: [CLAUDE.md](./CLAUDE.md) + [AGENTS.md](./AGENTS.md)
**For Planning**: [MASTER_PLAN.md](./MASTER_PLAN.md)

---

## 🎯 What's Live

### Core Features ✅
- **ZiWei (紫微斗數)** - Traditional 4×4 palace grid, Wuxing + BaZi, 12 palaces with stars/attributes
- **Western Zodiac** - Sun/Moon signs, planets, houses (hybrid ephemeris: solar-ephemeris + vsop87)
- **我的命格** - Unified profile: personality, generation story, Wuxing/BaZi, Western summary
- **Generation Tags** - Birth-year derived tags (1940s–2010s), selectable as birth attribute for story grounding
- **AI Interpretation** - 3-provider failover (iFlow → Groq → Cerebras), serialized via AIMutexDO, 45s provider timeout
- **Free Trial** - 30 days for all new users
- **Auth** - Passwordless email + Google OAuth (state cookie, session via SessionDO, 7-day TTL)

### Billing Direction (Taiwan-First)
- **Native App IAP** - Planned (LINE Pay / 台灣支付優先, then Apple/Google store)
- **Web Payments** - Deferred; if implemented, Taiwan local methods (LINE Pay, 街口支付) before Stripe
- **Current**: 30-day free trial only (no payments live)

### Infrastructure ✅
- **Frontend**: Leptos CSR (Rust → WASM) on Cloudflare Pages
- **Backend**: Cloudflare Workers (`workers-rs` 0.8, no Hono) — `fortunet-api`
- **Engine**: Isolated Worker `fortunet-engine` via service binding (`FT_ENGINE`), ZiWei `x-iztro` + Western `vsop87`
- **Database**: Turso (libSQL, Hrana over Fetch — `worker ^0.8` compat, no D1)
- **Cache**: Durable Objects — `SessionDO` + `AIMutexDO` (queue depth 8, 60s wait, 1 concurrent)
- **AI**: iFlow GLM-4.6, Groq Kimi-K2, Cerebras Llama-3.3-70b

---

## 💻 Development

### Workspace (Cargo)

```bash
cargo build -p ft-api -p ft-worker -p ft-web   # add --target wasm32-unknown-unknown for worker/web
cargo check -p ft-api --target wasm32-unknown-unknown
cargo fmt --all                                # CI gates on --check
cargo clippy --target wasm32-unknown-unknown   # report-only
cargo test -p ft-schema -p ft-ziwei -p ft-western -p ft-big5 -p ft-api  # native only
```

### Backend Worker (crates/api) & Engine Worker (crates/worker)

```bash
# Backend API
cd crates/api && worker-build --release && wrangler deploy   # OAuth only

# Engine Worker
./scripts/deploy-engine.sh

# Frontend (Leptos CSR)
./scripts/deploy-web.sh              # build-web.sh + wrangler pages deploy dist --project-name=fortunet
cd crates/web && ./scripts/build-web.sh   # build only (cargo + wasm-bindgen, no trunk)

# Verify
./scripts/verify-deployment.sh

# Database (single source of truth: scripts/schema.sql)
turso db shell fortunet < scripts/schema.sql
gwebcdb-mint turso --tier write --db fortunet --export  # group token (covers all DBs)
```

Always `unset CLOUDFLARE_API_TOKEN` before `wrangler` — use OAuth (`wrangler whoami` first). One-person project, all work lands directly on `main`.

---

## 📁 Documentation

| Document | Purpose |
|----------|---------|
| **[CLAUDE.md](./CLAUDE.md)** | Workspace, routes, DOs, services, engine versions |
| **[AGENTS.md](./AGENTS.md)** | Dev guide & critical rules |
| **[MASTER_PLAN.md](./MASTER_PLAN.md)** | Migration timeline & phases |
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
Phase 6:  Go-Live             Week 21     ← LIVE (Beta Testing)
```

Recent highlights (2026-09): `我的命盤` → `我的命格` rename, Wuxing+BaZi display, personality merged into profile, generation story, ZiWei 4×4 palace layout, generation tags as selectable birth attribute.

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

- **Frontend**: Rust, Leptos CSR, wasm-bindgen, gloo-net (Cloudflare Pages)
- **Backend**: Rust, Cloudflare Workers (`workers-rs` 0.8), Durable Objects
- **Engine**: Rust, `x-iztro` (ZiWei), `solar-ephemeris` + `vsop87` (Western)
- **Database**: Turso (libSQL, Hrana over `worker::Fetch`)
- **AI**: iFlow / Groq / Cerebras (free tiers, failover)
- **Shared DTOs**: `ft-schema` crate (wire + storage contract)

---

## 📊 Cost Summary

| Phase | Monthly Cost |
|-------|--------------|
| Testing (5-10 DAU) | **$0** |
| Early Growth (20-50 DAU) | $30-80 |
| Scale (50+ DAU) | $100-303 |

**Current**: $0 (within free tiers)

---

**Last Updated**: 2026-09-02
**Version**: 2.1.0
**License**: Private
