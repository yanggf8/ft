# 🔧 Implementation Guide (Revised with Risk Assessment)

## 📋 Overview

Step-by-step technical implementation guide for migrating FortuneT to Cloudflare-native architecture. **Enhanced with comprehensive Phase 0 risk assessment and validation procedures** to ensure successful migration with minimal business disruption.

## ⚠️ **CRITICAL: Phase 0 Risk Assessment (Week 1-2)**

**Do NOT proceed with implementation until Phase 0 is completed successfully.** This phase validates technical feasibility and identifies showstoppers before significant investment.

### Phase 0.1: Database Compatibility Validation

#### Task 1: PostgreSQL to D1 Compatibility Assessment

```bash
# 1. Extract current PostgreSQL schema and queries
psql $SUPABASE_URL -c "\d users" > schema_analysis.txt
pg_dump $SUPABASE_URL --schema-only > supabase_schema.sql

# 2. Create D1 compatibility test database
wrangler d1 create compatibility-test
wrangler d1 execute compatibility-test --file=test_schema.sql

# 3. Test critical queries for SQLite compatibility
```

#### D1 Compatibility Test Script

```typescript
// scripts/d1-compatibility-test.ts
interface CompatibilityResult {
  query_type: string;
  postgresql_query: string;
  sqlite_version: string;
  performance_ms: number;
  compatibility_score: number; // 0-100
  issues: string[];
}

export class D1CompatibilityTester {
  async testQueryCompatibility(): Promise<CompatibilityResult[]> {
    const tests: CompatibilityResult[] = [];

    // Test 1: Complex JOIN operations
    const complexJoin = `
      SELECT u.*, up.preferences, up.subscription_status
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.created_at >= '2024-01-01'
      ORDER BY u.email
      LIMIT 10
    `;

    // Test 2: Window functions (not supported in SQLite)
    const windowFunction = `
      SELECT
        email,
        COUNT(*) OVER (PARTITION BY subscription_status) as same_plan_count,
        ROW_NUMBER() OVER (ORDER BY created_at) as user_rank
      FROM users u
      JOIN user_profiles up ON u.id = up.user_id
    `;

    // Test 3: JSON operations (limited in SQLite)
    const jsonOperations = `
      SELECT
        id,
        JSON_EXTRACT(preferences, '$.language') as language,
        JSON_EXTRACT(preferences, '$.timezone') as timezone
      FROM user_profiles
    `;

    // Test 4: CTE queries (limited support in SQLite)
    const cteQuery = `
      WITH active_users AS (
        SELECT user_id, COUNT(*) as chart_count
        FROM charts
        WHERE created_at >= DATE('now', '-30 days')
        GROUP BY user_id
      )
      SELECT u.email, au.chart_count
      FROM users u
      JOIN active_users au ON u.id = au.user_id
    `;

    return tests;
  }

  async generateCompatibilityReport(): Promise<string> {
    const results = await this.testQueryCompatibility();

    let report = "# D1 Compatibility Assessment Report\n\n";

    const compatible = results.filter(r => r.compatibility_score > 80).length;
    const needsWork = results.filter(r => r.compatibility_score >= 50 && r.compatibility_score <= 80).length;
    const incompatible = results.filter(r => r.compatibility_score < 50).length;

    report += `## Summary\n`;
    report += `- ✅ Compatible: ${compatible}\n`;
    report += `- ⚠️ Needs Modification: ${needsWork}\n`;
    report += `- ❌ Incompatible: ${incompatible}\n\n`;

    if (incompatible > 0) {
      report += "## ⚠️ CRITICAL ISSUES FOUND\n";
      report += "Incompatible queries must be redesigned before migration proceeds.\n";
    }

    return report;
  }
}

// Execute compatibility test
const tester = new D1CompatibilityTester();
tester.generateCompatibilityReport().then(console.log);
```

#### Task 2: Performance Benchmarking

```typescript
// scripts/performance-benchmark.ts
export class PerformanceBenchmark {
  async benchmarkD1Operations(): Promise<BenchmarkResult> {
    const results = {
      read_operations: await this.benchmarkReadOperations(),
      write_operations: await this.benchmarkWriteOperations(),
      complex_queries: await this.benchmarkComplexQueries(),
      concurrent_access: await this.benchmarkConcurrentAccess()
    };

    return {
      target_p95_response_time_ms: 200,
      actual_p95_response_time_ms: this.calculateP95(results),
      meets_performance_target: this.calculateP95(results) <= 200,
      recommendations: this.generateRecommendations(results)
    };
  }

  private async benchmarkReadOperations() {
    // Test single row lookup
    // Test range queries
    // Test paginated results
    // Test full-text search
  }

  private async benchmarkWriteOperations() {
    // Test single insert
    // Test batch insert
    // Test update operations
    // Test delete operations
  }
}
```

### Phase 0.2: OAuth Migration Validation

#### Task 1: User Migration Prototype

```typescript
// scripts/oauth-migration-test.ts
export class OAuthMigrationValidator {
  async testUserAccountMigration(): Promise<MigrationTestResult> {
    // Create test user accounts in current system
    const testUsers = await this.createTestUsers(10);

    // Test OAuth flow with test users
    const migrationResults = [];

    for (const user of testUsers) {
      try {
        // Simulate OAuth login
        const oauthResult = await this.simulateOAuthLogin(user.email);

        // Test account linking
        const linkResult = await this.testAccountLinking(user.id, oauthResult.oauthId);

        // Validate data consistency
        const validation = await this.validateUserDataConsistency(user.id, oauthResult.userId);

        migrationResults.push({
          original_user_id: user.id,
          oauth_user_id: oauthResult.userId,
          linking_success: linkResult.success,
          data_consistent: validation.consistent,
          issues: validation.issues
        });

      } catch (error) {
        migrationResults.push({
          original_user_id: user.id,
          error: error.message,
          success: false
        });
      }
    }

    return {
      total_tested: testUsers.length,
      successful_migrations: migrationResults.filter(r => r.success).length,
      failed_migrations: migrationResults.filter(r => !r.success).length,
      success_rate: (migrationResults.filter(r => r.success).length / testUsers.length) * 100,
      issues: migrationResults.flatMap(r => r.issues || []),
      recommendation: this.generateMigrationRecommendation(migrationResults)
    };
  }

  private async simulateOAuthLogin(email: string) {
    // Test OAuth flow with provided email
    // Return OAuth user data
  }

  private async testAccountLinking(originalId: string, oauthId: string) {
    // Test linking existing account to OAuth
    // Verify no duplicate accounts created
  }
}
```

#### Task 2: User Experience Impact Assessment

```typescript
// scripts/ux-impact-assessment.ts
export class UserExperienceAssessment {
  async assessMigrationImpact(): Promise<UXImpactResult> {
    return {
      login_experience_change: await this.assessLoginChanges(),
      data_loss_risk: await this.assessDataLossRisk(),
      user_education_requirements: await this.identifyEducationNeeds(),
      support_ticket_projections: await this.projectSupportVolume(),
      churn_risk_assessment: await this.assessChurnRisk()
    };
  }

  private async assessLoginChanges() {
    // Compare current vs new login flows
    // Measure additional steps required
    // Identify friction points
  }

  private async assessChurnRisk() {
    // Analyze user segments
    // Identify high-risk users
    // Calculate potential churn rate
  }
}
```

### Phase 0.3: Infrastructure Validation

#### Task 1: Durable Objects Performance Testing

```typescript
// scripts/do-performance-test.ts
export class DurableObjectsPerformanceTest {
  async testDurableObjectsLimits(): Promise<DOTestResult> {
    const tests = [
      await this.testConcurrentUsers(100),    // 100 concurrent users
      await this.testConcurrentUsers(500),    // 500 concurrent users
      await this.testConcurrentUsers(1000),   // 1000 concurrent users
      await this.testMemoryUsage(),
      await this.testStatePersistence(),
      await this.testFailoverScenarios()
    ];

    return {
      performance_degradation_threshold: 20, // %
      actual_degradation: this.calculateDegradation(tests),
      meets_performance_target: this.calculateDegradation(tests) <= 20,
      max_concurrent_users_supported: this.findMaxConcurrentUsers(tests),
      memory_usage_sustainable: this.validateMemoryUsage(tests),
      recommendations: this.generatePerformanceRecommendations(tests)
    };
  }

  private async testConcurrentUsers(userCount: number) {
    // Simulate concurrent users accessing DO
    // Measure response times and resource usage
  }

  private async testFailoverScenarios() {
    // Test DO failure scenarios
    // Test recovery procedures
    // Validate data consistency
  }
}
```

### Phase 0.4: Security Validation

#### Task 1: Security Architecture Review

```typescript
// scripts/security-validation.ts
export class SecurityValidator {
  async performSecurityAssessment(): Promise<SecurityAssessmentResult> {
    const assessment = await Promise.all([
      this.testAuthenticationFlow(),
      this.testAuthorizationModel(),
      this.testDataEncryption(),
      this.testApiSecurity(),
      this.testInputValidation()
    ]);

    return {
      critical_vulnerabilities: assessment.filter(a => a.severity === 'critical'),
      high_vulnerabilities: assessment.filter(a => a.severity === 'high'),
      security_score: this.calculateSecurityScore(assessment),
      ready_for_production: assessment.filter(a => a.severity === 'critical').length === 0,
      remediation_plan: this.generateRemediationPlan(assessment)
    };
  }

  private async testAuthenticationFlow() {
    // Test OAuth implementation
    // Test JWT token security
    // Test session management
  }

  private async testDataEncryption() {
    // Test data in transit encryption
    // Test data at rest encryption
    // Test key management
  }
}
```

### Phase 0.5: Go/No-Go Decision Framework

#### Success Criteria Checklist

```typescript
interface Phase0SuccessCriteria {
  database_compatibility: {
    d1_compatibility_score: number; // Must be > 80%
    performance_meets_targets: boolean; // p95 < 200ms
    critical_issues_resolved: boolean;
  };

  oauth_migration: {
    user_migration_success_rate: number; // Must be > 95%
    data_loss_risk: 'low' | 'medium' | 'high'; // Must be 'low'
    ux_impact_acceptable: boolean; // Churn risk < 5%
  };

  infrastructure: {
    durable_objects_performance: boolean; // <20% degradation at scale
    security_vulnerabilities: {
      critical: number; // Must be 0
      high: number; // Must be < 3
    };
  };

  business_readiness: {
    team_skills_assessment: boolean; // Team confident with Cloudflare
    budget_approval: boolean; // Migration budget secured
    stakeholder_approval: boolean; // All stakeholders signed off
  };
}

export function evaluateGoNoGo(criteria: Phase0SuccessCriteria): {
  decision: 'go' | 'no-go' | 'conditional-go';
  blockers: string[];
  recommendations: string[];
} {
  const blockers = [];
  const recommendations = [];

  // Database validation
  if (criteria.database_compatibility.d1_compatibility_score <= 80) {
    blockers.push('D1 compatibility score too low - requires query redesign');
  }

  if (!criteria.database_compatibility.performance_meets_targets) {
    blockers.push('D1 performance does not meet targets');
  }

  // OAuth validation
  if (criteria.oauth_migration.user_migration_success_rate <= 95) {
    blockers.push('User migration success rate too low');
  }

  if (criteria.oauth_migration.data_loss_risk !== 'low') {
    blockers.push('Data loss risk too high');
  }

  // Security validation
  if (criteria.infrastructure.security_vulnerabilities.critical > 0) {
    blockers.push('Critical security vulnerabilities must be resolved');
  }

  const decision = blockers.length === 0 ? 'go' : 'no-go';

  return {
    decision,
    blockers,
    recommendations
  };
}
```

### Phase 0.6: Deliverables

Before proceeding to implementation, deliver:

1. **D1 Compatibility Report** - Complete with performance benchmarks
2. **OAuth Migration Validation** - User testing results and risk assessment
3. **Infrastructure Performance Tests** - Durable Objects limits and scaling
4. **Security Assessment** - Vulnerability scan and remediation plan
5. **Go/No-Go Decision Document** - Signed off by all stakeholders

**⚠️ CRITICAL: Do not proceed with Phase 1 until all Phase 0 deliverables are approved and success criteria met.**

---

## 🎯 Prerequisites (Post-Phase 0)

### Required Tools & Services

```bash
# Development Tools
- Node.js 18+ (for Workers and frontend)
- Git (version control)
- VS Code + Cloudflare extension (recommended)
- Wrangler CLI (Cloudflare Workers)
- Web browser (Chrome/Edge for DevTools)

# Cloudflare Account
- Cloudflare account (free tier sufficient)
- Custom domains (already owned: ahexagram.com)
- DNS access for domain configuration

# Development Environment
- Local development machine
- Internet connection
- Terminal/command line access
```

### Environment Setup

```bash
# 1. Install Node.js (if not already installed)
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# 2. Install Wrangler CLI
npm install -g wrangler

# 3. Login to Cloudflare
wrangler login

# 4. Verify installation
wrangler whoami
node --version
npm --version
```

## 🏗️ Phase 1: Repository Setup (Day 1-2)

### 1.1 Create New Repository Structure

```bash
# Create new repository
mkdir fortune-teller-v2
cd fortune-teller-v2

# Initialize Git repository
git init

# Create project structure
mkdir -p frontend/src/{components,pages,services,hooks,types,utils}
mkdir -p frontend/public
mkdir -p backend/src/{routes,services,durable-objects,middleware,types,utils}
mkdir -p backend/scripts
mkdir -p backend/tests
mkdir -p shared/types
mkdir -p docs
mkdir -p scripts

# Create initial files
touch README.md
touch .gitignore
touch LICENSE
```

### 1.2 Configure Git Repository

```bash
# Create .gitignore
cat > .gitignore << 'EOF'
# Dependencies
node_modules/
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Environment variables
.env
.env.local
.env.development.local
.env.test.local
.env.production.local

# Build outputs
dist/
build/
.wrangler/

# IDE
.vscode/
.idea/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Logs
logs/
*.log

# Runtime data
pids/
*.pid
*.seed
*.pid.lock

# Coverage directory used by tools like istanbul
coverage/
*.lcov

# nyc test coverage
.nyc_output

# Temporary folders
tmp/
temp/
EOF

# Create initial README
cat > README.md << 'EOF'
# FortuneT V2 - Cloudflare Native

Modern astrology platform built on Cloudflare Workers, D1 Database, and Durable Objects.

## Architecture

- **Frontend**: React + TypeScript + Vite on Cloudflare Pages
- **Backend**: Cloudflare Workers with Hono framework
- **Database**: Cloudflare D1 (SQLite)
- **Caching**: Durable Objects for stateful storage
- **Authentication**: Cloudflare Access + Custom OAuth

## Getting Started

See [docs/MIGRATION_PLAN.md](./docs/MIGRATION_PLAN.md) for complete migration strategy.
EOF

git add .
git commit -m "Initial repository setup"
```

### 1.3 Setup Backend (Cloudflare Workers)

```bash
cd backend

# Initialize Workers project
npm create cloudflare@latest . -- --yes

# Install dependencies
npm install @cloudflare/workers-types hono
npm install -D @types/node vitest wrangler

# Install additional dependencies
npm install jsonwebtoken @types/jsonwebtoken
npm install bcryptjs @types/bcryptjs
npm install nanoid
npm install @supabase/supabase-js

# Create wrangler.toml
cat > wrangler.toml << 'EOF'
name = "fortune-teller-api"
main = "src/index.ts"
compatibility_date = "2024-01-01"
compatibility_flags = ["nodejs_compat"]

[env.production]
name = "fortune-teller-api-prod"

# D1 Database binding
[[env.production.d1_databases]]
binding = "DB"
database_name = "fortune-teller-db"
database_id = "your-database-id"

# R2 Bucket binding
[[env.production.r2_buckets]]
binding = "R2_BUCKET"
bucket_name = "fortune-teller-assets"

# Durable Objects bindings
[[env.production.durable_objects.bindings]]
name = "SESSION_MANAGER"
class_name = "SessionManagerDurableObject"

[[env.production.durable_objects.bindings]]
name = "CHART_CACHE"
class_name = "ChartCacheDurableObject"

[[env.production.durable_objects.bindings]]
name = "RATE_LIMITER"
class_name = "RateLimiterDurableObject"

[[env.production.durable_objects.bindings]]
name = "REALTIME_COLLAB"
class_name = "RealtimeCollaborationDurableObject"

# Environment variables
[env.production.vars]
ENVIRONMENT = "production"
JWT_SECRET = "your-jwt-secret"
APP_URL = "https://v2.ahexagram.com"
CLOUDFLARE_ACCESS_DOMAIN = "ahexagram.com"
CLOUDFLARE_ACCESS_APP_ID = "your-app-id"

# D1 Database binding (development)
[[d1_databases]]
binding = "DB"
database_name = "fortune-teller-db"
database_id = "your-database-id"

# R2 Bucket binding (development)
[[r2_buckets]]
binding = "R2_BUCKET"
bucket_name = "fortune-teller-assets"

# Durable Objects bindings (development)
[[durable_objects.bindings]]
name = "SESSION_MANAGER"
class_name = "SessionManagerDurableObject"

[[durable_objects.bindings]]
name = "CHART_CACHE"
class_name = "ChartCacheDurableObject"

[[durable_objects.bindings]]
name = "RATE_LIMITER"
class_name = "RateLimiterDurableObject"

[[durable_objects.bindings]]
name = "REALTIME_COLLAB"
class_name = "RealtimeCollaborationDurableObject"

# Environment variables (development)
[vars]
ENVIRONMENT = "development"
JWT_SECRET = "dev-jwt-secret-change-in-production"
APP_URL = "http://localhost:3000"

# Durable Objects classes
[[durable_objects.classes]]
name = "SessionManagerDurableObject"
script_name = "fortune-teller-api"

[[durable_objects.classes]]
name = "ChartCacheDurableObject"
script_name = "fortune-teller-api"

[[durable_objects.classes]]
name = "RateLimiterDurableObject"
script_name = "fortune-teller-api"

[[durable_objects.classes]]
name = "RealtimeCollaborationDurableObject"
script_name = "fortune-teller-api"
EOF
```

### 1.4 Setup Frontend (React + Vite)

```bash
cd ../frontend

# Initialize Vite React project
npm create vite@latest . -- --template react-ts

# Install additional dependencies
npm install @tanstack/react-query axios zustand
npm install react-router-dom @types/react-router-dom
npm install tailwindcss postcss autoprefixer
npm install -D @types/node

# Setup Tailwind CSS
npx tailwindcss init -p

# Configure Tailwind
cat > tailwind.config.js << 'EOF'
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#f0f9ff',
          500: '#3b82f6',
          600: '#2563eb',
          700: '#1d4ed8',
        }
      }
    },
  },
  plugins: [],
}
EOF

# Update PostCSS config
cat > postcss.config.js << 'EOF'
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
}
EOF

# Update CSS file
cat > src/index.css << 'EOF'
@tailwind base;
@tailwind components;
@tailwind utilities;

body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
    'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
    sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
EOF

# Create basic folder structure
mkdir -p src/components/ui
mkdir -p src/components/charts
mkdir -p src/components/auth
mkdir -p src/pages
mkdir -p src/services
mkdir -p src/hooks
mkdir -p src/types
mkdir -p src/utils
```

### 1.5 Configure Shared Types

```bash
cd ../shared/types

# Create shared type definitions
cat > index.ts << 'EOF'
// Shared types for frontend and backend

export interface User {
  id: string;
  email: string;
  fullName: string;
  avatar?: string;
  emailVerified: boolean;
  subscriptionStatus: 'free' | 'premium' | 'professional';
  preferences: UserPreferences;
  createdAt: Date;
  updatedAt: Date;
}

export interface UserPreferences {
  language: string;
  timezone: string;
  theme: 'light' | 'dark';
  notifications: {
    email: boolean;
    push: boolean;
    astrology_insights: boolean;
    subscription_reminders: boolean;
  };
  astrology: {
    default_system: 'ziwei' | 'zodiac';
    chart_preferences: {
      show_transitions: boolean;
      include_yearly_prediction: boolean;
      detail_level: 'basic' | 'comprehensive';
    };
    ai_settings: {
      analysis_depth: 'basic' | 'detailed';
      language: 'traditional_chinese' | 'english';
    };
  };
}

export interface Chart {
  id: string;
  userId: string;
  chartType: 'ziwei' | 'zodiac';
  chartName: string;
  birthInfo: BirthInfo;
  chartData: any;
  analysisResult?: any;
  chartImageUrl?: string;
  isPublic: boolean;
  createdAt: Date;
  updatedAt: Date;
}

export interface BirthInfo {
  date: string;
  time: string;
  timezone: string;
  location: {
    name: string;
    latitude: number;
    longitude: number;
    utc_offset: string;
  };
  gender: 'male' | 'female' | 'other';
  name?: string;
}

export interface APIResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
  message?: string;
}

export interface AuthTokens {
  accessToken: string;
  refreshToken?: string;
  expiresIn: number;
  tokenType: string;
}
EOF

cd ../..
```

## 🔧 Phase 2: Core Infrastructure (Day 3-4)

### 2.1 Setup D1 Database

```bash
# Create D1 database
wrangler d1 create fortune-teller-db

# Note the database ID and update wrangler.toml
# Your database ID will be shown in the output

# Initialize database schema
cd backend

# Create database migration script
cat > scripts/init-db.sql << 'EOF'
-- Users table (replaces Supabase auth.users)
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    cloudflare_id TEXT UNIQUE,
    google_id TEXT UNIQUE,
    email TEXT UNIQUE NOT NULL,
    email_verified BOOLEAN DEFAULT FALSE,
    full_name TEXT,
    avatar_url TEXT,
    provider TEXT DEFAULT 'google',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- User profiles (extended information)
CREATE TABLE IF NOT EXISTS user_profiles (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    preferences TEXT,
    subscription_status TEXT DEFAULT 'free',
    stripe_customer_id TEXT,
    notification_settings TEXT,
    privacy_settings TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Charts table
CREATE TABLE IF NOT EXISTS charts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    chart_type TEXT NOT NULL CHECK(chart_type IN ('ziwei', 'zodiac')),
    chart_name TEXT,
    birth_info TEXT NOT NULL,
    chart_data TEXT NOT NULL,
    analysis_result TEXT,
    chart_image_url TEXT,
    is_public BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Favorites table
CREATE TABLE IF NOT EXISTS favorites (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    chart_id TEXT NOT NULL,
    favorite_type TEXT DEFAULT 'chart',
    notes TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (chart_id) REFERENCES charts(id) ON DELETE CASCADE,
    UNIQUE(user_id, chart_id)
);

-- Subscriptions table
CREATE TABLE IF NOT EXISTS subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    plan_id TEXT NOT NULL,
    stripe_subscription_id TEXT UNIQUE,
    stripe_customer_id TEXT,
    status TEXT NOT NULL,
    current_period_start DATETIME,
    current_period_end DATETIME,
    cancel_at_period_end BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_users_cloudflare_id ON users(cloudflare_id);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_charts_user_id ON charts(user_id);
CREATE INDEX IF NOT EXISTS idx_charts_type_created ON charts(chart_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_favorites_user_id ON favorites(user_id);
EOF

# Execute database migration
wrangler d1 execute fortune-teller-db --file=./scripts/init-db.sql

# Verify database creation
wrangler d1 info fortune-teller-db
```

### 2.2 Setup R2 Storage

```bash
# Create R2 bucket
wrangler r2 bucket create fortune-teller-assets

# Test bucket access
wrangler r2 object list fortune-teller-assets

# Update wrangler.toml with R2 bucket binding if not already done
```

### 2.3 Basic Workers Application

```typescript
// backend/src/index.ts
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { prettyJSON } from 'hono/pretty-json';

type App = Hono<{
  Bindings: Env;
}>;

const app = new App();

// Middleware
app.use('*', cors({
  origin: ['http://localhost:3000', 'https://v2.ahexagram.com'],
  credentials: true
}));
app.use('*', logger());
app.use('*', prettyJSON());

// Health check endpoint
app.get('/api/health', (c) => {
  return c.json({
    status: 'ok',
    timestamp: new Date().toISOString(),
    version: '2.0.0-cloudflare'
  });
});

// API routes (will be expanded in later phases)
app.get('/api/v1/test', (c) => {
  return c.json({
    message: 'FortuneT V2 API is working!',
    environment: c.env.ENVIRONMENT
  });
});

export default app;
```

### 2.4 Environment Configuration

```bash
# Create environment files
cd backend

# Development environment
cat > .env << 'EOF'
ENVIRONMENT=development
JWT_SECRET=dev-jwt-secret-change-in-production
APP_URL=http://localhost:3000
CLOUDFLARE_ACCESS_DOMAIN=ahexagram.com
CLOUDFLARE_ACCESS_APP_ID=your-app-id

# Google OAuth (for custom implementation)
GOOGLE_OAUTH_CLIENT_ID=your-google-client-id
GOOGLE_OAUTH_CLIENT_SECRET=your-google-client-secret

# Supabase (for migration)
SUPABASE_URL=your-supabase-url
SUPABASE_SERVICE_KEY=your-supabase-service-key

# AI Services
GROQ_API_KEY=your-groq-api-key
OPENROUTER_API_KEY=your-openrouter-api-key

# Stripe
STRIPE_SECRET_KEY=sk_test_your-stripe-secret
STRIPE_PUBLISHABLE_KEY=pk_test_your-stripe-publishable
EOF

# Production environment (will be set via Wrangler secrets)
# wrangler secret put JWT_SECRET
# wrangler secret put GOOGLE_OAUTH_CLIENT_ID
# wrangler secret put GOOGLE_OAUTH_CLIENT_SECRET
# wrangler secret put GROQ_API_KEY
# wrangler secret put OPENROUTER_API_KEY
# wrangler secret put STRIPE_SECRET_KEY
```

### 2.5 Local Development Setup

```bash
# Create development script
cat > package.json << 'EOF'
{
  "name": "fortune-teller-backend",
  "version": "1.0.0",
  "description": "FortuneT V2 Backend - Cloudflare Workers",
  "main": "src/index.ts",
  "scripts": {
    "dev": "wrangler dev",
    "deploy": "wrangler deploy",
    "deploy:prod": "wrangler deploy --env production",
    "test": "vitest",
    "test:watch": "vitest --watch",
    "db:migrate": "wrangler d1 execute fortune-teller-db --file=./scripts/init-db.sql",
    "db:info": "wrangler d1 info fortune-teller-db"
  },
  "devDependencies": {
    "@cloudflare/workers-types": "^4.20231218.0",
    "@types/jsonwebtoken": "^9.0.5",
    "@types/node": "^20.10.5",
    "typescript": "^5.3.3",
    "vitest": "^1.1.0",
    "wrangler": "^3.22.1"
  },
  "dependencies": {
    "hono": "^3.11.7",
    "jsonwebtoken": "^9.0.2",
    "nanoid": "^5.0.4",
    "@supabase/supabase-js": "^2.38.5"
  }
}
EOF

# Install dependencies
npm install

# Start development server
npm run dev
```

### 2.6 Frontend Development Setup

```bash
cd ../frontend

# Update package.json scripts
cat > package.json << 'EOF'
{
  "name": "fortune-teller-frontend",
  "private": true,
  "version": "2.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "lint": "eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0",
    "preview": "vite preview",
    "test": "vitest"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-router-dom": "^6.20.1",
    "@tanstack/react-query": "^5.12.2",
    "axios": "^1.6.2",
    "zustand": "^4.4.7",
    "clsx": "^2.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.43",
    "@types/react-dom": "^18.2.17",
    "@types/node": "^20.10.5",
    "@typescript-eslint/eslint-plugin": "^6.14.0",
    "@typescript-eslint/parser": "^6.14.0",
    "@vitejs/plugin-react": "^4.2.1",
    "autoprefixer": "^10.4.16",
    "eslint": "^8.55.0",
    "eslint-plugin-react-hooks": "^4.6.0",
    "eslint-plugin-react-refresh": "^0.4.5",
    "postcss": "^8.4.32",
    "tailwindcss": "^3.3.6",
    "typescript": "^5.2.2",
    "vite": "^5.0.8",
    "vitest": "^1.1.0"
  }
}
EOF

# Install dependencies
npm install

# Create environment file
cat > .env << 'EOF'
VITE_API_URL=http://localhost:8787
VITE_APP_NAME=FortuneT V2
VITE_CLOUDFLARE_ACCESS_DOMAIN=ahexagram.com
EOF

# Create basic App component
cat > src/App.tsx << 'EOF'
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import './index.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Router>
        <div className="min-h-screen bg-gray-50">
          <header className="bg-white shadow-sm border-b">
            <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
              <div className="flex justify-between h-16 items-center">
                <h1 className="text-xl font-semibold text-gray-900">
                  FortuneT V2
                </h1>
                <nav className="flex space-x-4">
                  <a href="/api/health" className="text-gray-600 hover:text-gray-900">
                    API Health
                  </a>
                </nav>
              </div>
            </div>
          </header>

          <main className="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
            <Routes>
              <Route path="/" element={
                <div className="text-center py-12">
                  <h2 className="text-2xl font-bold text-gray-900 mb-4">
                    Welcome to FortuneT V2
                  </h2>
                  <p className="text-gray-600 mb-8">
                    Modern astrology platform built on Cloudflare
                  </p>
                  <div className="space-y-4">
                    <div className="bg-white p-6 rounded-lg shadow">
                      <h3 className="text-lg font-semibold mb-2">🚀 Architecture</h3>
                      <p className="text-gray-600">
                        Cloudflare Workers + D1 + Durable Objects
                      </p>
                    </div>
                    <div className="bg-white p-6 rounded-lg shadow">
                      <h3 className="text-lg font-semibold mb-2">⚡ Performance</h3>
                      <p className="text-gray-600">
                        Global edge network with sub-50ms response times
                      </p>
                    </div>
                    <div className="bg-white p-6 rounded-lg shadow">
                      <h3 className="text-lg font-semibold mb-2">💰 Cost Effective</h3>
                      <p className="text-gray-600">
                        70-90% cost reduction vs traditional hosting
                      </p>
                    </div>
                  </div>
                </div>
              } />
            </Routes>
          </main>
        </div>
      </Router>
    </QueryClientProvider>
  );
}

export default App;
EOF

# Start development servers
# Terminal 1: Backend
cd ../backend && npm run dev

# Terminal 2: Frontend
cd ../frontend && npm run dev
```

## 🔧 Phase 3: Authentication System (Day 5-7)

### 3.1 Implement Cloudflare Access Integration

```typescript
// backend/src/services/cloudflare-access-auth.ts
import { CloudflareAccessUser, UserProfile, UserPreferences } from '../../shared/types';

export class CloudflareAccessAuth {
  private jwtSecret: string;
  private authDomain: string;
  private appId: string;

  constructor(private env: Env) {
    this.jwtSecret = env.JWT_SECRET;
    this.authDomain = env.CLOUDFLARE_ACCESS_DOMAIN;
    this.appId = env.CLOUDFLARE_ACCESS_APP_ID;
  }

  async verifyAccessToken(token: string): Promise<CloudflareAccessUser | null> {
    try {
      // Fetch Cloudflare Access public keys
      const jwksResponse = await fetch(
        `https://${this.authDomain}/cdn-cgi/access/certs`
      );

      if (!jwksResponse.ok) {
        throw new Error('Failed to fetch JWKS');
      }

      const jwks = await jwksResponse.json();

      // For simplicity, we'll use a basic JWT verification
      // In production, implement proper JWKS verification
      const payload = this.decodeJWT(token);

      if (!payload || payload.iss !== `https://${this.authDomain}`) {
        return null;
      }

      return payload as CloudflareAccessUser;

    } catch (error) {
      console.error('Access token verification failed:', error);
      return null;
    }
  }

  async handleUserLogin(accessUser: CloudflareAccessUser): Promise<UserProfile> {
    // Check if user exists by cloudflare ID
    let user = await this.getUserByCloudflareId(accessUser.sub);

    if (!user) {
      // Check if user exists by email (migration from other providers)
      user = await this.getUserByEmail(accessUser.email);

      if (user) {
        // Update existing user with Cloudflare ID
        user = await this.updateUserCloudflareId(user.id, accessUser.sub);
      } else {
        // Create new user
        user = await this.createUserFromAccess(accessUser);
      }
    } else {
      // Update existing user data
      user = await this.updateUserProfile(user.id, accessUser);
    }

    return user;
  }

  async createSessionToken(user: UserProfile): Promise<string> {
    const now = Math.floor(Date.now() / 1000);

    const payload = {
      sub: user.id,
      email: user.email,
      name: user.fullName,
      role: user.subscriptionStatus,
      iat: now,
      exp: now + (7 * 24 * 60 * 60), // 7 days
      aud: this.appId,
      iss: `https://${this.authDomain}`
    };

    return this.signJWT(payload);
  }

  private decodeJWT(token: string): any {
    try {
      const [, payload] = token.split('.');
      return JSON.parse(atob(payload));
    } catch {
      return null;
    }
  }

  private signJWT(payload: any): string {
    // Simple JWT signing (use proper crypto in production)
    const header = { alg: 'HS256', typ: 'JWT' };
    const encodedHeader = btoa(JSON.stringify(header));
    const encodedPayload = btoa(JSON.stringify(payload));
    const signature = btoa('simple-signature'); // Replace with proper signing

    return `${encodedHeader}.${encodedPayload}.${signature}`;
  }

  private async getUserByCloudflareId(cloudflareId: string): Promise<UserProfile | null> {
    const result = await this.env.DB.prepare(`
      SELECT u.*, up.preferences, up.subscription_status
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.cloudflare_id = ?
    `).bind(cloudflareId).first();

    return result ? this.mapDbRowToProfile(result) : null;
  }

  private async getUserByEmail(email: string): Promise<UserProfile | null> {
    const result = await this.env.DB.prepare(`
      SELECT u.*, up.preferences, up.subscription_status
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.email = ?
    `).bind(email).first();

    return result ? this.mapDbRowToProfile(result) : null;
  }

  private async createUserFromAccess(accessUser: CloudflareAccessUser): Promise<UserProfile> {
    const userId = crypto.randomUUID();

    const profile: UserProfile = {
      id: userId,
      email: accessUser.email,
      fullName: accessUser.name,
      avatar: accessUser.picture,
      emailVerified: accessUser.email_verified,
      subscriptionStatus: 'free',
      preferences: this.getDefaultPreferences(),
      createdAt: new Date(),
      updatedAt: new Date()
    };

    // Insert into D1 database
    await this.env.DB.prepare(`
      INSERT INTO users (
        id, cloudflare_id, email, full_name, avatar_url,
        email_verified, provider, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).bind(
      profile.id,
      accessUser.sub,
      profile.email,
      profile.fullName,
      profile.avatar,
      profile.emailVerified,
      'cloudflare_access',
      profile.createdAt.toISOString(),
      profile.updatedAt.toISOString()
    ).run();

    // Create user profile entry
    await this.env.DB.prepare(`
      INSERT INTO user_profiles (
        id, user_id, preferences, subscription_status, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?)
    `).bind(
      crypto.randomUUID(),
      profile.id,
      JSON.stringify(profile.preferences),
      profile.subscriptionStatus,
      profile.createdAt.toISOString(),
      profile.updatedAt.toISOString()
    ).run();

    return profile;
  }

  private getDefaultPreferences(): UserPreferences {
    return {
      language: 'zh-TW',
      timezone: 'Asia/Taipei',
      theme: 'light',
      notifications: {
        email: true,
        push: false,
        astrology_insights: true,
        subscription_reminders: true
      },
      astrology: {
        default_system: 'ziwei',
        chart_preferences: {
          show_transitions: true,
          include_yearly_prediction: false,
          detail_level: 'comprehensive'
        },
        ai_settings: {
          analysis_depth: 'detailed',
          language: 'traditional_chinese'
        }
      }
    };
  }

  private mapDbRowToProfile(row: any): UserProfile {
    return {
      id: row.id,
      email: row.email,
      fullName: row.full_name,
      avatar: row.avatar_url,
      emailVerified: row.email_verified,
      subscriptionStatus: row.subscription_status || 'free',
      preferences: JSON.parse(row.preferences || '{}'),
      createdAt: new Date(row.created_at),
      updatedAt: new Date(row.updated_at)
    };
  }
}
```

### 3.2 Create Authentication Routes

```typescript
// backend/src/routes/auth.ts
import { Hono } from 'hono';
import { setCookie } from 'hono/cookie';
import { CloudflareAccessAuth } from '../services/cloudflare-access-auth';

const auth = new Hono<{ Bindings: Env }>();

// Initiate Cloudflare Access login
auth.get('/login', async (c) => {
  const redirectTo = c.req.query('redirect') || '/';
  const appId = c.env.CLOUDFLARE_ACCESS_APP_ID;

  // Redirect to Cloudflare Access login
  const loginUrl = `https://${c.env.CLOUDFLARE_ACCESS_DOMAIN}/cdn-cgi/access/login/${appId}?redirect=${encodeURIComponent(redirectTo)}`;

  return c.redirect(loginUrl);
});

// Handle Cloudflare Access callback
auth.get('/callback', async (c) => {
  try {
    // Extract Cloudflare Access token from headers
    const cfAccessJwt = c.req.header('Cf-Access-Jwt-Assertion');

    if (!cfAccessJwt) {
      return c.redirect('/login?error=no_token');
    }

    const authService = new CloudflareAccessAuth(c.env);

    // Verify Cloudflare Access token
    const accessUser = await authService.verifyAccessToken(cfAccessJwt);
    if (!accessUser) {
      return c.redirect('/login?error=invalid_token');
    }

    // Create or update user profile
    const userProfile = await authService.handleUserLogin(accessUser);

    // Create application session token
    const sessionToken = await authService.createSessionToken(userProfile);

    // Set session cookie
    setCookie(c, 'session_token', sessionToken, {
      httpOnly: true,
      secure: true,
      sameSite: 'Lax',
      maxAge: 7 * 24 * 60 * 60, // 7 days
      path: '/'
    });

    // Redirect to intended destination
    const redirectTo = c.req.query('redirect') || '/dashboard';
    return c.redirect(redirectTo);

  } catch (error) {
    console.error('Auth callback error:', error);
    return c.redirect('/login?error=callback_failed');
  }
});

// Get current user profile
auth.get('/me', async (c) => {
  const sessionToken = c.req.header('Authorization')?.replace('Bearer ', '');

  if (!sessionToken) {
    return c.json({ error: 'No session token' }, 401);
  }

  const authService = new CloudflareAccessAuth(c.env);

  try {
    const payload = authService.decodeJWT(sessionToken);

    if (!payload || !payload.sub) {
      return c.json({ error: 'Invalid token' }, 401);
    }

    const user = await c.env.DB.prepare(`
      SELECT u.*, up.preferences, up.subscription_status
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.id = ?
    `).bind(payload.sub).first();

    if (!user) {
      return c.json({ error: 'User not found' }, 404);
    }

    return c.json({
      success: true,
      data: {
        id: user.id,
        email: user.email,
        fullName: user.full_name,
        avatar: user.avatar_url,
        subscriptionStatus: user.subscription_status || 'free',
        preferences: JSON.parse(user.preferences || '{}')
      }
    });

  } catch (error) {
    console.error('Get user error:', error);
    return c.json({ error: 'Failed to get user' }, 500);
  }
});

export { auth };
```

### 3.3 Add Authentication to Main App

```typescript
// backend/src/index.ts (updated)
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { prettyJSON } from 'hono/pretty-json';
import { auth } from './routes/auth';

type App = Hono<{
  Bindings: Env;
}>;

const app = new App();

// Middleware
app.use('*', cors({
  origin: ['http://localhost:3000', 'https://v2.ahexagram.com'],
  credentials: true
}));
app.use('*', logger());
app.use('*', prettyJSON());

// Health check endpoint
app.get('/api/health', (c) => {
  return c.json({
    status: 'ok',
    timestamp: new Date().toISOString(),
    version: '2.0.0-cloudflare'
  });
});

// API routes
app.route('/api/auth', auth);

app.get('/api/v1/test', (c) => {
  return c.json({
    message: 'FortuneT V2 API is working!',
    environment: c.env.ENVIRONMENT
  });
});

export default app;
```

### 3.4 Frontend Authentication Integration

```typescript
// frontend/src/services/auth-service.ts
import { User, APIResponse } from '../../../shared/types';

export class CloudflareAuthService {
  private baseUrl: string;

  constructor() {
    this.baseUrl = import.meta.env.VITE_API_URL;
  }

  // Initiate login
  async login(redirectTo?: string): Promise<void> {
    const params = new URLSearchParams({
      redirect: redirectTo || window.location.pathname
    });

    window.location.href = `${this.baseUrl}/api/auth/login?${params.toString()}`;
  }

  // Logout
  async logout(): Promise<void> {
    try {
      await fetch(`${this.baseUrl}/api/auth/logout`, {
        method: 'POST',
        credentials: 'include'
      });
    } catch (error) {
      console.error('Logout error:', error);
    }

    // Force redirect to login page
    window.location.href = '/login';
  }

  // Get current user
  async getCurrentUser(): Promise<User | null> {
    try {
      const response = await fetch(`${this.baseUrl}/api/auth/me`, {
        credentials: 'include'
      });

      if (!response.ok) {
        return null;
      }

      const result: APIResponse<User> = await response.json();
      return result.data || null;
    } catch (error) {
      console.error('Get current user error:', error);
      return null;
    }
  }

  // Check authentication status
  isAuthenticated(): boolean {
    return document.cookie.includes('session_token=');
  }
}

// React Hook for authentication
export function useAuth() {
  const [user, setUser] = React.useState<User | null>(null);
  const [loading, setLoading] = React.useState(true);
  const authService = new CloudflareAuthService();

  React.useEffect(() => {
    const checkAuth = async () => {
      try {
        const currentUser = await authService.getCurrentUser();
        setUser(currentUser);
      } catch (error) {
        console.error('Auth check failed:', error);
      } finally {
        setLoading(false);
      }
    };

    checkAuth();
  }, []);

  const login = (redirectTo?: string) => {
    authService.login(redirectTo);
  };

  const logout = async () => {
    await authService.logout();
    setUser(null);
  };

  return {
    user,
    loading,
    isAuthenticated: !!user,
    login,
    logout
  };
}
```

### 3.5 Create Authentication Components

```typescript
// frontend/src/components/auth/LoginPage.tsx
import React from 'react';
import { useAuth } from '../../services/auth-service';

export function LoginPage() {
  const { login } = useAuth();

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-md w-full space-y-8">
        <div>
          <h2 className="mt-6 text-center text-3xl font-extrabold text-gray-900">
            Sign in to FortuneT
          </h2>
          <p className="mt-2 text-center text-sm text-gray-600">
            Modern astrology platform built on Cloudflare
          </p>
        </div>
        <div className="mt-8 space-y-6">
          <div className="rounded-md shadow-sm -space-y-px">
            <button
              onClick={() => login()}
              className="group relative w-full flex justify-center py-3 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            >
              <span className="absolute left-0 inset-y-0 flex items-center pl-3">
                <svg className="h-5 w-5 text-blue-500 group-hover:text-blue-400" viewBox="0 0 20 20" fill="currentColor">
                  <path fillRule="evenodd" d="M10 9a3 3 0 100-6 3 3 0 000 6zm-7 9a7 7 0 1114 0H3z" clipRule="evenodd" />
                </svg>
              </span>
              Sign in with Cloudflare Access
            </button>
          </div>

          <div className="text-center">
            <p className="text-sm text-gray-600">
              This will redirect you to Cloudflare's secure login page
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
```

### 3.6 Update Frontend Router

```typescript
// frontend/src/App.tsx (updated)
import React, { useEffect } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { useAuth } from './services/auth-service';
import { LoginPage } from './components/auth/LoginPage';
import './index.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth();

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-lg">Loading...</div>
      </div>
    );
  }

  if (!user) {
    return <Navigate to="/login" replace />;
  }

  return <>{children}</>;
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Router>
        <div className="min-h-screen bg-gray-50">
          <Routes>
            <Route path="/login" element={<LoginPage />} />

            <Route path="/" element={
              <ProtectedRoute>
                <div className="min-h-screen">
                  <header className="bg-white shadow-sm border-b">
                    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                      <div className="flex justify-between h-16 items-center">
                        <h1 className="text-xl font-semibold text-gray-900">
                          FortuneT V2
                        </h1>
                        <UserMenu />
                      </div>
                    </div>
                  </header>

                  <main className="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
                    <Dashboard />
                  </main>
                </div>
              </ProtectedRoute>
            } />
          </Routes>
        </div>
      </Router>
    </QueryClientProvider>
  );
}

function UserMenu() {
  const { user, logout } = useAuth();

  return (
    <div className="flex items-center space-x-4">
      <span className="text-sm text-gray-700">
        Welcome, {user?.fullName}
      </span>
      <button
        onClick={logout}
        className="text-sm text-gray-500 hover:text-gray-700"
      >
        Logout
      </button>
    </div>
  );
}

function Dashboard() {
  return (
    <div className="text-center py-12">
      <h2 className="text-2xl font-bold text-gray-900 mb-4">
        Welcome to FortuneT V2 Dashboard
      </h2>
      <p className="text-gray-600 mb-8">
        Authentication successful! Ready to build amazing features.
      </p>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mt-12">
        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">🎯 Next Steps</h3>
          <p className="text-gray-600">
            Implement chart calculation engines
          </p>
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">🔧 Features</h3>
          <p className="text-gray-600">
            Add AI analysis and real-time collaboration
          </p>
        </div>

        <div className="bg-white p-6 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-2">📊 Analytics</h3>
          <p className="text-gray-600">
            Track usage and optimize performance
          </p>
        </div>
      </div>
    </div>
  );
}

export default App;
```

## 🧪 Testing the Implementation

### Test Authentication Flow

```bash
# 1. Start development servers
cd backend && npm run dev
cd frontend && npm run dev

# 2. Test API endpoints
curl http://localhost:8787/api/health

# 3. Test authentication flow
# - Open http://localhost:3000
# - Click "Sign in with Cloudflare Access"
# - Should redirect to Cloudflare Access login
# - After login, should redirect back to dashboard

# 4. Verify database operations
wrangler d1 execute fortune-teller-db --command="SELECT * FROM users"
```

### Verify Database Integration

```bash
# Create test script
cat > test-db.js << 'EOF'
export default {
  async fetch(request, env, ctx) {
    try {
      const result = await env.DB.prepare('SELECT COUNT(*) as count FROM users').first();
      return new Response(JSON.stringify(result, null, 2));
    } catch (error) {
      return new Response(JSON.stringify({ error: error.message }), { status: 500 });
    }
  }
};
EOF

# Test database connection
wrangler dev test-db.js
```

## 🎯 Next Steps

After completing Phase 3, you should have:

✅ **Working authentication system** with Cloudflare Access
✅ **Basic API infrastructure** with health checks
✅ **Frontend authentication flow** with protected routes
✅ **Database integration** with user management
✅ **Local development environment** fully configured

**Ready for Phase 4:** Chart calculation engines and core features

---

## 📚 Related Documentation

- [Migration Plan](./MIGRATION_PLAN.md) - Complete migration strategy
- [Cloudflare Architecture](./CLOUDFLARE_ARCHITECTURE.md) - Technical architecture overview
- [D1 Database Schema](./D1_DATABASE_SCHEMA.md) - Database design and migrations
- [Authentication Migration](./AUTHENTICATION_MIGRATION.md) - Detailed authentication implementation
- [Durable Objects Design](./DURABLE_OBJECTS_DESIGN.md) - Stateful caching patterns

This implementation guide provides the foundation for building a modern, scalable astrology platform on Cloudflare's edge computing infrastructure.