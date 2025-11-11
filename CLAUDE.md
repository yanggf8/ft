# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FortuneT V2 is a comprehensive Cloudflare-native astrology platform migration project. This repository contains **revised and realistic** documentation for migrating an existing multi-provider stack (Render + Vercel) to a unified Cloudflare platform using Workers, D1 Database, Durable Objects, and Pages.

**Current Status**: Enhanced documentation phase with comprehensive risk assessment and realistic planning. No actual code implementation yet.

## 🚨 Critical Updates (Revised Plan)

### Timeline & Performance Adjustments
- **Timeline**: Extended from 8 weeks to **16 weeks** with Phase 0 risk assessment
- **Performance Targets**: Revised to **3-5x improvement** (not 10x) with sub-200ms response times
- **Cost Savings**: Realistic **50-70% reduction** (not 70-90%)
- **Success Probability**: **80%** with revised plan (vs 30% originally)

### Phase 0 Risk Assessment (NEW - Week 1-2)
**Do NOT proceed with implementation until Phase 0 is completed successfully.**
- D1 database compatibility validation
- OAuth migration user testing
- Durable Objects performance testing
- Security vulnerability assessment
- Go/No-Go decision framework

## Architecture

### Target Cloudflare Architecture
- **Frontend**: React + TypeScript + Vite on Cloudflare Pages
- **Backend**: Cloudflare Workers with Hono framework
- **Database**: Cloudflare D1 (SQLite-based) with compatibility validation
- **Stateful Storage**: Durable Objects for caching and real-time features
- **Authentication**: Cloudflare Access + custom OAuth implementation
- **Storage**: Cloudflare R2 for files and images
- **AI Integration**: Multi-LLM support (Groq, OpenRouter, Claude)

### Key Features
- Zi Wei Dou Shu astrology calculations
- Western Zodiac calculations
- AI-powered chart interpretations
- User authentication and subscriptions
- Chart storage and sharing
- Real-time collaboration
- Rate limiting and analytics

## Common Development Commands

Since this is currently a documentation-only repository, there are no build or deployment commands yet. When implementation begins, typical commands would include:

```bash
# Phase 0 Risk Assessment (CRITICAL)
npm run test:compatibility    # D1 compatibility tests
npm run test:oauth           # OAuth migration tests
npm run test:performance     # Performance benchmarks
npm run validate:security    # Security validation

# Backend (Cloudflare Workers)
npm run dev                  # Start local development server
npm run deploy               # Deploy to Workers
npm run test                 # Run tests

# Frontend (React/Vite)
npm run dev                  # Start development server
npm run build                # Build for production
npm run preview              # Preview production build

# Database
wrangler d1 execute <db> --file=<migration-file>
wrangler d1 info <db>
```

## Project Structure (Planned)

```
fortune-teller-v2/
├── frontend/                 # React frontend
│   ├── src/
│   │   ├── components/      # UI components
│   │   ├── pages/          # Route pages
│   │   ├── services/       # API services
│   │   ├── hooks/          # Custom React hooks
│   │   └── types/          # TypeScript types
│   └── package.json
├── backend/                 # Cloudflare Workers
│   ├── src/
│   │   ├── routes/         # API routes
│   │   ├── services/       # Business logic
│   │   ├── durable-objects/ # Durable Object classes
│   │   └── middleware/     # Request middleware
│   ├── scripts/            # Database migrations
│   ├── tests/               # Unit and integration tests
│   ├── wrangler.toml       # Cloudflare config
│   └── package.json
├── phase0-tests/            # Phase 0 validation tests
│   ├── compatibility/       # D1 compatibility tests
│   ├── oauth-migration/     # OAuth validation tests
│   ├── performance/         # Performance benchmarks
│   └── security/           # Security validation
├── shared/                  # Shared types and utilities
└── docs/                    # Documentation (current)
```

## Key Technical Decisions (Revised)

### Database Design
- **D1 SQLite** with **compatibility validation required**
- Strategic indexing for sub-100ms query times (revised from sub-10ms)
- JSON columns for flexible user preferences and chart data
- **Query translation required** from PostgreSQL to SQLite

### Authentication Strategy (Enhanced)
Two options documented with migration risk assessment:
1. **Cloudflare Access** (recommended) - Enterprise SSO with Zero Trust
2. **Custom OAuth** - Full control with Google/GitHub providers
3. **User migration validation** required before implementation

### Performance Architecture (Realistic)
- **Multi-layer caching**: Edge cache → Durable Objects → D1 queries
- **Global CDN** via Cloudflare Pages
- **Sub-200ms response times** globally (revised from sub-50ms)
- **Auto-scaling** with cost monitoring and limits

### State Management
- **Session Manager DO** for user sessions and real-time features
- **Chart Cache DO** for expensive calculation results
- **Rate Limiter DO** for abuse prevention
- **Real-time Collaboration DO** for multi-user editing

## Cost Optimization (Revised)

### Projected Monthly Costs (Realistic)
- **Current**: $437-1,062/month (including third-party services)
- **New Cloudflare**: $324-587/month (revised from $35-95)
- **Savings**: 50-70% cost reduction (revised from 70-90%)
- **ROI**: 45-220% first-year return (revised from 215-750%)

### Migration Investment
- **One-time costs**: $2,150-5,040 (including security audit, monitoring setup)
- **Parallel infrastructure**: $800-2,240 (16 weeks)
- **Migration payback**: 3.5-9 months (revised from 1-3)

### Free Tier Optimization
- Workers: 100K requests/day free
- D1: 5GB storage + 25M reads/day free
- R2: 10GB storage + 1M operations/month free
- Pages: Unlimited bandwidth free

## Migration Strategy (Revised)

### 16-Week Migration Plan (Enhanced)
0. **Phase 0**: Risk assessment and validation (Week 1-2) ⚠️ **NEW**
1. **Phase 1**: Foundation (Week 3-5) - Setup & Infrastructure
2. **Phase 2**: Core features (Week 6-9) - Business logic
3. **Phase 3**: Frontend development (Week 10-12) - User interface
4. **Phase 4**: Integration & testing (Week 13-14) - Quality assurance
5. **Phase 5**: Pre-migration (Week 15) - Final preparation
6. **Phase 6**: Migration (Week 16) - Gradual go-live

### Data Migration Approach (Enhanced)
- **Phase 0 validation** before any code implementation
- Parallel deployment (new repository)
- Gradual traffic migration (5% → 100% with monitoring)
- **Comprehensive rollback procedures** and testing
- Zero business disruption with proper execution

## Security Considerations (Enhanced)

### Authentication Security (Risk Mitigated)
- JWT token management with proper expiration
- Cloudflare Access Zero Trust model
- **User migration validation** to prevent churn
- Session invalidation on logout
- **Account linking procedures** for existing users

### Data Protection
- TLS 1.3 for all communications
- Encrypted data at rest
- GDPR compliance considerations
- Input validation and sanitization
- **Security audit required** before production

### Rate Limiting (Enhanced)
- Per-user and per-IP limits
- Plan-based usage limits
- Durable Object-based rate limiting
- Abuse prevention and blocking
- **Load testing validation** required

## Development Guidelines (Enhanced)

### Code Style
- TypeScript throughout the stack
- Hono framework for Workers API
- React with functional components and hooks
- Tailwind CSS for styling
- Zustand for state management

### Testing Strategy (Comprehensive)
- **Phase 0 validation testing** (compatibility, performance, security)
- Unit tests with Vitest (90%+ coverage)
- Integration tests for API endpoints
- **Durable Objects testing patterns** (critical for success)
- **Load testing for performance validation**
- **End-to-end testing** for critical user journeys

### Performance Requirements (Realistic)
- API response times: p95 < 200ms (revised from <200ms)
- Chart calculations: <150ms (cached), <500ms (fresh)
- AI analysis: <3 seconds (revised from <2 seconds)
- 99.9% uptime target
- **Performance monitoring** required

## Risk Mitigation (Critical Additions)

### High-Risk Areas Identified
1. **D1 Database Compatibility** - PostgreSQL to SQLite translation challenges
2. **Durable Objects Complexity** - State management and debugging challenges
3. **OAuth User Migration** - User experience and churn risk
4. **Performance Targets** - Realistic achievement under load
5. **Security Vulnerabilities** - New attack vectors in cloud environment

### Mitigation Strategies
- **Phase 0 validation** before significant investment
- **Comprehensive testing** at each phase
- **Gradual migration** with rollback capability
- **User communication strategy** to minimize churn
- **24/7 monitoring** during migration period

## Important Notes

### Current Repository State
- This contains **enhanced documentation** with comprehensive risk assessment
- No actual code implementation yet
- **Phase 0 validation procedures** are critical prerequisites
- **Realistic timelines and cost projections** with proper contingencies

### Implementation Priority (Revised)
1. **Complete Phase 0 validation** (Week 1-2) - CRITICAL
2. Set up Cloudflare infrastructure (Week 3-5)
3. Implement authentication system with migration validation (Week 5)
4. Port chart calculation engines with performance testing (Week 6-7)
5. Build frontend with feature parity and testing (Week 10-12)
6. Add AI integration and caching with optimization (Week 8-9)
7. Implement real-time collaboration features (Week 12)
8. **Gradual migration with monitoring** (Week 16)

### Risk Mitigation (Enhanced)
- **Phase 0 prevents costly mistakes**
- Parallel deployment prevents business disruption
- **Comprehensive testing before each phase**
- **Automated rollback procedures** and testing
- **Performance monitoring and alerting**
- **User communication and support preparation**

## Documentation Files (Enhanced)

Key documentation files in this repository:
- `README.md` - Project overview and quick start
- `MIGRATION_PLAN.md` - **Complete 16-week migration strategy** (revised)
- `CLOUDFLARE_ARCHITECTURE.md` - **Technical architecture with risk mitigation** (enhanced)
- `D1_DATABASE_SCHEMA.md` - Database design and optimization
- `DURABLE_OBJECTS_DESIGN.md` - Stateful caching and real-time features
- `AUTHENTICATION_MIGRATION.md` - OAuth implementation with migration strategy
- `IMPLEMENTATION_GUIDE.md` - **Step-by-step guide with Phase 0 assessment** (enhanced)
- `DETAILED_MONTHLY_COSTS.md` - **Comprehensive cost breakdown** (NEW)
- `TESTING_MONITORING_PROCEDURES.md` - **Complete testing and monitoring strategy** (NEW)

## Next Steps

When ready to begin implementation:
1. **Complete Phase 0 validation** - D1 compatibility, OAuth testing, performance benchmarks
2. Review **detailed cost breakdown** in `DETAILED_MONTHLY_COSTS.md`
3. Study **testing procedures** in `TESTING_MONITORING_PROCEDURES.md`
4. Create new code repository based on validated requirements
5. Set up Cloudflare account and infrastructure
6. Follow the **16-week migration plan** with risk mitigation
7. Monitor performance metrics against realistic targets
8. Execute gradual migration with comprehensive monitoring

This documentation provides a **realistic and comprehensive blueprint** for transforming FortuneT into a modern, Cloudflare-native platform with proper risk management and achievable goals.