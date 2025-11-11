# 🗄️ D1 Database Schema Design

## 📋 Overview

Complete database schema design for FortuneT's migration from PostgreSQL (Current Database) to Cloudflare D1. This schema is optimized for D1's SQLite architecture while maintaining full feature parity with the current system.

## 🎯 Design Principles

- **SQLite Optimization**: Leverage D1's SQLite-based architecture
- **Performance First**: Strategic indexing for sub-10ms query times
- **Scalability Ready**: Designed for 1000+ DAU growth
- **Cost Effective**: Optimized for D1's free tier limits
- **Data Integrity**: Foreign key constraints and validation

## 🏗️ Core Schema Structure

### Users & Authentication

```sql
-- Users table (replaces Current Database auth.users)
CREATE TABLE users (
    id TEXT PRIMARY KEY,                    -- UUID v4
    cloudflare_id TEXT UNIQUE,              -- Cloudflare Access sub
    google_id TEXT UNIQUE,                  -- Google OAuth ID (fallback)
    email TEXT UNIQUE NOT NULL,
    email_verified BOOLEAN DEFAULT FALSE,
    full_name TEXT,
    avatar_url TEXT,
    provider TEXT DEFAULT 'google' CHECK(provider IN ('google', 'github', 'email')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- User profiles (extended information)
CREATE TABLE user_profiles (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    preferences TEXT,                        -- JSON: language, timezone, theme, etc.
    subscription_status TEXT DEFAULT 'free' CHECK(subscription_status IN ('free', 'premium', 'professional')),
    stripe_customer_id TEXT UNIQUE,
    notification_settings TEXT,              -- JSON: email, push, etc.
    privacy_settings TEXT,                  -- JSON: data sharing, visibility
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- OAuth tokens (for custom OAuth implementation)
CREATE TABLE oauth_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL CHECK(provider IN ('google', 'github')),
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    token_type TEXT DEFAULT 'Bearer',
    scope TEXT,
    expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- User sessions (managed by Durable Objects, stored here for persistence)
CREATE TABLE user_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    session_token_hash TEXT NOT NULL,
    device_info TEXT,                       -- JSON: browser, OS, device type
    ip_address TEXT,
    user_agent TEXT,
    expires_at DATETIME NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_accessed DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
```

### Astrology Charts & Data

```sql
-- Unified charts storage (replaces multiple chart tables)
CREATE TABLE charts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    chart_type TEXT NOT NULL CHECK(chart_type IN ('ziwei', 'zodiac')),
    chart_name TEXT,
    birth_info TEXT NOT NULL,                 -- JSON: date, time, location, etc.
    chart_data TEXT NOT NULL,                 -- JSON: calculated chart data
    analysis_result TEXT,                     -- JSON: AI analysis results
    chart_image_url TEXT,                     -- R2 URL to generated chart image
    chart_version INTEGER DEFAULT 1,
    is_public BOOLEAN DEFAULT FALSE,
    is_template BOOLEAN DEFAULT FALSE,
    tags TEXT,                               -- JSON: searchable tags
    metadata TEXT,                            -- JSON: additional chart metadata
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Chart calculation cache (optimization for expensive calculations)
CREATE TABLE chart_cache (
    id TEXT PRIMARY KEY,
    chart_hash TEXT UNIQUE NOT NULL,          -- SHA-256 hash of birth data
    chart_type TEXT NOT NULL,
    chart_data TEXT NOT NULL,
    calculation_time_ms INTEGER,
    cache_hit_count INTEGER DEFAULT 0,
    last_accessed DATETIME DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME,                      -- TTL-based expiration
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- AI analysis cache (for expensive AI operations)
CREATE TABLE ai_analysis_cache (
    id TEXT PRIMARY KEY,
    chart_hash TEXT NOT NULL,
    model_name TEXT NOT NULL,                 -- groq-llama3, claude-3, etc.
    analysis_result TEXT NOT NULL,
    prompt_version TEXT DEFAULT 'v1.0',
    analysis_cost_usd DECIMAL(10, 4),
    cache_hit_count INTEGER DEFAULT 0,
    expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chart_hash, model_name)
);
```

### User Interactions & Favorites

```sql
-- User favorites (replaces user_favorites)
CREATE TABLE favorites (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    chart_id TEXT NOT NULL,
    favorite_type TEXT DEFAULT 'chart' CHECK(favorite_type IN ('chart', 'analysis', 'insight')),
    notes TEXT,
    is_pinned BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (chart_id) REFERENCES charts(id) ON DELETE CASCADE,
    UNIQUE(user_id, chart_id, favorite_type)
);

-- Chart sharing and collaboration
CREATE TABLE chart_shares (
    id TEXT PRIMARY KEY,
    chart_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    share_token TEXT UNIQUE NOT NULL,          -- UUID for public sharing
    access_level TEXT DEFAULT 'view' CHECK(access_level IN ('view', 'comment', 'edit')),
    expires_at DATETIME,
    access_count INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (chart_id) REFERENCES charts(id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Chart comments and interactions
CREATE TABLE chart_comments (
    id TEXT PRIMARY KEY,
    chart_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    comment_text TEXT NOT NULL,
    parent_comment_id TEXT,                    -- For threaded comments
    is_edited BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (chart_id) REFERENCES charts(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_comment_id) REFERENCES chart_comments(id) ON DELETE CASCADE
);
```

### Subscription & Payment Management

```sql
-- Subscription plans and pricing
CREATE TABLE subscription_plans (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,                 -- 'free', 'premium', 'professional'
    display_name TEXT NOT NULL,
    description TEXT,
    price_usd_monthly DECIMAL(10, 2),
    price_usd_yearly DECIMAL(10, 2),
    features TEXT,                            -- JSON: feature list and limits
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- User subscriptions
CREATE TABLE subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE,
    plan_id TEXT NOT NULL,
    stripe_subscription_id TEXT UNIQUE,
    stripe_customer_id TEXT,
    status TEXT NOT NULL CHECK(status IN ('active', 'canceled', 'past_due', 'unpaid', 'trialing')),
    current_period_start DATETIME,
    current_period_end DATETIME,
    cancel_at_period_end BOOLEAN DEFAULT FALSE,
    trial_end DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (plan_id) REFERENCES subscription_plans(id)
);

-- Payment methods
CREATE TABLE payment_methods (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    stripe_payment_method_id TEXT UNIQUE,
    type TEXT NOT NULL,                        -- 'card', 'bank_account'
    brand TEXT,                               -- 'visa', 'mastercard', etc.
    last4 TEXT,
    expiry_month INTEGER,
    expiry_year INTEGER,
    is_default BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Invoices and payments
CREATE TABLE invoices (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    stripe_invoice_id TEXT UNIQUE,
    subscription_id TEXT,
    amount_usd DECIMAL(10, 2) NOT NULL,
    currency TEXT DEFAULT 'usd',
    status TEXT NOT NULL CHECK(status IN ('draft', 'open', 'paid', 'void', 'uncollectible')),
    due_date DATETIME,
    paid_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
);
```

### Usage Analytics & Tracking

```sql
-- Daily usage tracking (aggregated to reduce write operations)
CREATE TABLE usage_tracking (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    date DATE NOT NULL,
    charts_created INTEGER DEFAULT 0,
    ai_analyses INTEGER DEFAULT 0,
    api_calls INTEGER DEFAULT 0,
    storage_used_mb INTEGER DEFAULT 0,
    page_views INTEGER DEFAULT 0,
    session_duration_minutes INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, date)
);

-- Feature usage analytics
CREATE TABLE feature_usage (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    feature_name TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN ('view', 'use', 'create', 'edit', 'delete', 'share')),
    metadata TEXT,                            -- JSON: additional event data
    ip_address TEXT,
    user_agent TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

-- System performance metrics
CREATE TABLE performance_metrics (
    id TEXT PRIMARY KEY,
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL,
    metric_unit TEXT,
    tags TEXT,                                -- JSON: endpoint, method, status_code, etc.
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Error tracking
CREATE TABLE error_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    error_type TEXT NOT NULL,
    error_message TEXT,
    stack_trace TEXT,
    request_url TEXT,
    request_method TEXT,
    user_agent TEXT,
    ip_address TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);
```

### System Configuration & Feature Flags

```sql
-- Application settings
CREATE TABLE app_settings (
    id TEXT PRIMARY KEY,
    key TEXT UNIQUE NOT NULL,
    value TEXT NOT NULL,
    description TEXT,
    is_public BOOLEAN DEFAULT FALSE,          -- Whether setting is exposed to frontend
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Feature flags for controlled rollouts
CREATE TABLE feature_flags (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    enabled BOOLEAN DEFAULT FALSE,
    rollout_percentage INTEGER DEFAULT 0,     -- 0-100% of users
    user_segment TEXT,                        -- JSON: user segments for targeting
    conditions TEXT,                          -- JSON: activation conditions
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Feature flag assignments to users
CREATE TABLE user_feature_flags (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    feature_flag_id TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    override_reason TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (feature_flag_id) REFERENCES feature_flags(id) ON DELETE CASCADE,
    UNIQUE(user_id, feature_flag_id)
);
```

## 🎯 Strategic Indexing

### Performance-Critical Indexes

```sql
-- Users and authentication indexes
CREATE INDEX idx_users_cloudflare_id ON users(cloudflare_id);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_provider ON users(provider);
CREATE INDEX idx_users_created_at ON users(created_at);

-- User profiles indexes
CREATE INDEX idx_user_profiles_user_id ON user_profiles(user_id);
CREATE INDEX idx_user_profiles_subscription_status ON user_profiles(subscription_status);
CREATE INDEX idx_user_profiles_stripe_customer ON user_profiles(stripe_customer_id);

-- Charts indexes
CREATE INDEX idx_charts_user_id ON charts(user_id);
CREATE INDEX idx_charts_type_created ON charts(chart_type, created_at DESC);
CREATE INDEX idx_charts_public ON charts(is_public, created_at DESC);
CREATE INDEX idx_charts_template ON charts(is_template, chart_type);
CREATE INDEX idx_charts_updated_at ON charts(updated_at DESC);

-- Cache indexes
CREATE INDEX idx_chart_cache_hash ON chart_cache(chart_hash);
CREATE INDEX idx_chart_cache_expires ON chart_cache(expires_at);
CREATE INDEX idx_ai_cache_chart_model ON ai_analysis_cache(chart_hash, model_name);
CREATE INDEX idx_ai_cache_expires ON ai_analysis_cache(expires_at);

-- Favorites indexes
CREATE INDEX idx_favorites_user_id ON favorites(user_id);
CREATE INDEX idx_favorites_chart_id ON favorites(chart_id);
CREATE INDEX idx_favorites_type_created ON favorites(favorite_type, created_at DESC);

-- Subscription indexes
CREATE INDEX idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX idx_subscriptions_status ON subscriptions(status);
CREATE INDEX idx_subscriptions_plan ON subscriptions(plan_id, status);
CREATE INDEX idx_subscriptions_current_period ON subscriptions(current_period_end);

-- Usage tracking indexes
CREATE INDEX idx_usage_tracking_user_date ON usage_tracking(user_id, date DESC);
CREATE INDEX idx_usage_tracking_date ON usage_tracking(date);
CREATE INDEX idx_feature_usage_user_feature ON feature_usage(user_id, feature_name);
CREATE INDEX idx_feature_usage_created_at ON feature_usage(created_at DESC);

-- Analytics indexes
CREATE INDEX idx_performance_metrics_name_timestamp ON performance_metrics(metric_name, timestamp DESC);
CREATE INDEX idx_performance_metrics_timestamp ON performance_metrics(timestamp DESC);
CREATE INDEX idx_error_logs_user_created ON error_logs(user_id, created_at DESC);
CREATE INDEX idx_error_logs_type_created ON error_logs(error_type, created_at DESC);
```

### Partial Indexes for Optimization

```sql
-- Indexes for active subscriptions only
CREATE INDEX idx_subscriptions_active ON subscriptions(user_id, current_period_end)
WHERE status = 'active';

-- Indexes for public charts only
CREATE INDEX idx_charts_public_active ON charts(chart_type, created_at DESC)
WHERE is_public = TRUE;

-- Indexes for premium users
CREATE INDEX idx_user_profiles_premium ON user_profiles(user_id, updated_at)
WHERE subscription_status IN ('premium', 'professional');

-- Indexes for recent errors only
CREATE INDEX idx_error_logs_recent ON error_logs(error_type, created_at DESC)
WHERE created_at > datetime('now', '-7 days');
```

## 📊 Data Type Optimization

### JSON Column Structures

```sql
-- User preferences JSON structure
INSERT INTO user_profiles (id, user_id, preferences) VALUES (
  'profile-id',
  'user-id',
  '{
    "language": "zh-TW",
    "timezone": "Asia/Taipei",
    "theme": "light",
    "notifications": {
      "email": true,
      "push": false,
      "astrology_insights": true,
      "subscription_reminders": true
    },
    "astrology": {
      "default_system": "ziwei",
      "chart_preferences": {
        "show_transitions": true,
        "include_yearly_prediction": false,
        "detail_level": "comprehensive"
      },
      "ai_settings": {
        "analysis_depth": "detailed",
        "language": "traditional_chinese"
      }
    },
    "ui": {
      "default_chart_view": "detailed",
      "show_tutorial": true,
      "compact_mode": false
    }
  }'
);

-- Birth info JSON structure
INSERT INTO charts (id, user_id, chart_type, birth_info) VALUES (
  'chart-id',
  'user-id',
  'ziwei',
  '{
    "date": "1990-01-01",
    "time": "14:30:00",
    "timezone": "Asia/Taipei",
    "location": {
      "name": "Taipei, Taiwan",
      "latitude": 25.0330,
      "longitude": 121.5654,
      "utc_offset": "+08:00"
    },
    "gender": "female",
    "name": "Jane Doe"
  }'
);
```

## 🔄 Migration Scripts

### Data Migration from Current Database

```typescript
// Migration script structure
export class Current DatabaseToD1Migration {
  async migrateUsers() {
    // Export from Current Database
    const supabaseUsers = await this.exportCurrent DatabaseUsers();

    // Transform for D1 schema
    const transformedUsers = supabaseUsers.map(user => ({
      id: user.id,
      cloudflare_id: null,                    // Will be set on next login
      email: user.email,
      email_verified: !!user.email_confirmed_at,
      full_name: user.user_metadata?.full_name || '',
      avatar_url: user.user_metadata?.avatar_url,
      provider: 'email',
      created_at: user.created_at,
      updated_at: user.updated_at
    }));

    // Import to D1
    await this.importUsersToD1(transformedUsers);
  }

  async migrateCharts() {
    // Migrate fortune_charts -> charts
    const supabaseCharts = await this.exportCurrent DatabaseCharts();

    for (const chart of supabaseCharts) {
      await this.env.DB.prepare(`
        INSERT INTO charts (
          id, user_id, chart_type, chart_name, birth_info,
          chart_data, analysis_result, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
      `).bind(
        chart.id,
        chart.user_id,
        'ziwei', // Determine from chart data
        chart.chart_name,
        JSON.stringify(chart.birth_info),
        JSON.stringify(chart.chart_data),
        chart.analysis_result,
        chart.created_at,
        chart.updated_at
      ).run();
    }
  }

  async migrateSubscriptions() {
    // Migrate subscription data
    const subscriptions = await this.exportCurrent DatabaseSubscriptions();

    for (const sub of subscriptions) {
      await this.env.DB.prepare(`
        INSERT INTO subscriptions (
          id, user_id, plan_id, stripe_subscription_id,
          stripe_customer_id, status, current_period_start,
          current_period_end, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      `).bind(
        sub.id,
        sub.user_id,
        this.mapPlanId(sub.plan_type),
        sub.stripe_subscription_id,
        sub.stripe_customer_id,
        sub.status,
        sub.current_period_start,
        sub.current_period_end,
        sub.created_at,
        sub.updated_at
      ).run();
    }
  }
}
```

## 🚀 Performance Optimization

### Query Optimization Examples

```sql
-- Optimized user profile lookup with preferences
SELECT
  u.id, u.email, u.full_name, u.avatar_url, u.email_verified,
  up.subscription_status, up.preferences, up.stripe_customer_id,
  s.status as subscription_status, s.current_period_end
FROM users u
LEFT JOIN user_profiles up ON u.id = up.user_id
LEFT JOIN subscriptions s ON u.id = s.user_id AND s.status = 'active'
WHERE u.id = ?;

-- Optimized chart listing with pagination
SELECT
  c.id, c.chart_name, c.chart_type, c.created_at, c.updated_at,
  CASE WHEN c.analysis_result IS NOT NULL THEN true ELSE false END as has_analysis
FROM charts c
WHERE c.user_id = ?
ORDER BY c.updated_at DESC
LIMIT ? OFFSET ?;

-- Optimized usage statistics
SELECT
  SUM(charts_created) as total_charts,
  SUM(ai_analyses) as total_ai_analyses,
  AVG(session_duration_minutes) as avg_session_time,
  COUNT(*) as active_days
FROM usage_tracking
WHERE user_id = ?
AND date >= date('now', '-30 days');

-- Optimized analytics query with caching
SELECT
  DATE(created_at) as date,
  COUNT(*) as chart_count,
  chart_type
FROM charts
WHERE created_at >= date('now', '-7 days')
GROUP BY DATE(created_at), chart_type
ORDER BY date DESC, chart_count DESC;
```

## 📈 Scaling Considerations

### D1 Limits Monitoring

```typescript
// Usage monitoring and alerting
export class D1UsageMonitor {
  async checkUsageLimits() {
    const stats = await this.getUsageStats();

    return {
      storage: {
        used: stats.storage_used_mb,
        limit: 5120, // 5GB in MB
        percentage: (stats.storage_used_mb / 5120) * 100
      },
      reads: {
        used: stats.reads_today,
        limit: 25000000, // 25M per day
        percentage: (stats.reads_today / 25000000) * 100
      },
      writes: {
        used: stats.writes_today,
        limit: 100000, // 100k per day
        percentage: (stats.writes_today / 100000) * 100
      }
    };
  }

  async optimizeQueries() {
    // Identify slow queries
    const slowQueries = await this.identifySlowQueries();

    // Suggest indexes
    const suggestedIndexes = await this.analyzeMissingIndexes();

    // Recommend query optimizations
    const optimizations = await this.generateQueryOptimizations();

    return { slowQueries, suggestedIndexes, optimizations };
  }
}
```

## 🛡️ Data Integrity & Validation

### Constraint Examples

```sql
-- Check constraints for data validation
CREATE TRIGGER validate_birth_info
BEFORE INSERT ON charts
BEGIN
  -- Validate JSON structure for birth_info
  IF json_valid(NEW.birth_info) = 0 THEN
    RAISE(ABORT, 'Invalid JSON in birth_info');
  END IF;

  -- Validate required birth_info fields
  IF json_extract(NEW.birth_info, '$.date') IS NULL THEN
    RAISE(ABORT, 'Birth date is required');
  END IF;
END;

-- Trigger to update timestamps
CREATE TRIGGER update_user_timestamp
BEFORE UPDATE ON users
BEGIN
  NEW.updated_at = CURRENT_TIMESTAMP;
END;

-- Trigger for analytics aggregation
CREATE TRIGGER update_daily_usage
AFTER INSERT ON feature_usage
BEGIN
  INSERT OR REPLACE INTO usage_tracking (
    user_id, date, charts_created, ai_analyses, api_calls, updated_at
  ) VALUES (
    NEW.user_id,
    DATE(NEW.created_at),
    CASE WHEN NEW.feature_name = 'chart_create' THEN 1 ELSE 0 END,
    CASE WHEN NEW.feature_name = 'ai_analysis' THEN 1 ELSE 0 END,
    CASE WHEN NEW.feature_name LIKE 'api_%' THEN 1 ELSE 0 END,
    CURRENT_TIMESTAMP
  );
END;
```

---

## 📚 Related Documentation

- [Migration Plan](./MIGRATION_PLAN.md) - Complete migration strategy
- [Cloudflare Architecture](./CLOUDFLARE_ARCHITECTURE.md) - Technical architecture overview
- [Durable Objects Design](./DURABLE_OBJECTS_DESIGN.md) - Stateful caching patterns
- [Authentication Migration](./AUTHENTICATION_MIGRATION.md) - OAuth implementation
- [Testing Strategy](./TESTING_STRATEGY.md) - Database testing approach

This schema provides a robust foundation for FortuneT's data needs while optimizing for D1's characteristics and ensuring excellent performance and scalability.