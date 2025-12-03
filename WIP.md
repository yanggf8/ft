# FortuneT V2 - Work In Progress

**Last Updated**: 2025-12-03
**Current Phase**: Phase 2 (Core Features) - Week 9 ✅

---

## Completed

### Phase 1: Foundation (Week 4-6) ✅
- [x] Cloudflare Workers API deployed
- [x] D1 database with schema
- [x] R2 storage bucket
- [x] Session Durable Object (SQLite-backed)
- [x] Auth endpoints (register/login/logout)
- [x] User & Chart CRUD endpoints
- [x] CI/CD pipeline (.github/workflows/deploy.yml)
- [x] Rate limiting on auth (10 req/min/IP)
- [x] Request-ID headers

### Phase 2: Core Features (Week 7-9) ✅
- [x] ZiWei calculation engine (紫微斗數)
  - Solar-to-lunar conversion (1900-2100)
  - Four pillars (四柱)
  - Life/body palace (命宮/身宮)
  - Five elements (五行局)
  - 14 main stars + auxiliary stars
- [x] Western zodiac engine
  - Sun sign calculation
  - Approximate moon sign
  - Basic planetary positions
- [x] Input validation with structured errors
- [x] Engine versioning
- [x] Groq AI integration
  - Chart interpretation (ZiWei/Western)
  - Chinese/English language support
  - Focus parameter (career/love/health)
  - Rate limiting (10 req/min/IP)

---

## In Progress

### Phase 2: Week 10-11 - Payments
- [ ] Stripe integration
- [ ] Subscription management
- [ ] Webhook handling

---

## Deployed Infrastructure

| Component | URL/ID |
|-----------|--------|
| Workers API | https://fortunet-api.yanggf.workers.dev |
| D1 Database | `88d074eb-7331-402b-bc76-1ac3cb0588da` |
| R2 Storage | `fortunet-storage` |

---

## API Endpoints

### Public
```
GET  /health
GET  /health/db
POST /api/auth/register
POST /api/auth/login
POST /api/charts/calculate/ziwei
POST /api/charts/calculate/western
```

### Authenticated
```
POST /api/auth/logout
GET  /api/users/me
PUT  /api/users/me
GET  /api/charts
POST /api/charts
GET  /api/charts/:id
PUT  /api/charts/:id
DELETE /api/charts/:id
POST /api/charts/interpret
```

---

## Secrets Configured

- [x] `GROQ_API_KEY` - AI interpretation

## Secrets Pending (Week 10-11)

- [ ] `STRIPE_SECRET_KEY`
- [ ] `STRIPE_WEBHOOK_SECRET`

---

## Next Steps

1. Stripe integration for payments
2. Subscription tiers (free/premium)
3. Webhook handling for payment events
4. Usage tracking per subscription tier
