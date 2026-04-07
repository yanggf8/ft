-- FortuneT V2 D1 Schema
-- Phase 2: Birth-data centric model

-- Users table (with birth data as foundation for all divination)
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    full_name TEXT,
    avatar_url TEXT,
    
    -- Birth data (foundation for divination)
    birth_year INTEGER,
    birth_month INTEGER,
    birth_day INTEGER,
    birth_hour INTEGER,              -- NULL = unknown, use 12 as default
    birth_minute INTEGER,
    gender TEXT CHECK (gender IN ('male', 'female')),
    timezone TEXT DEFAULT 'Asia/Taipei',
    latitude REAL,
    longitude REAL,
    birth_data_hash TEXT,            -- for cache invalidation
    
    -- Subscription
    subscription_tier TEXT DEFAULT 'free' CHECK (subscription_tier IN ('free', 'premium', 'professional')),
    trial_ends_at TEXT,              -- 試用期結束時間，NULL = 無試用
    
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Cached interpretations (one per user per divination type)
CREATE TABLE IF NOT EXISTS interpretations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    divination_type TEXT NOT NULL CHECK (divination_type IN ('ziwei', 'western', 'bazi')),
    chart_data TEXT NOT NULL,        -- calculated chart JSON
    ai_interpretation TEXT,          -- AI generated text
    birth_data_hash TEXT NOT NULL,   -- invalidate if user birth data changes
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(user_id, divination_type)
);

CREATE INDEX IF NOT EXISTS idx_interpretations_user ON interpretations(user_id);

-- Subscriptions table
CREATE TABLE IF NOT EXISTS subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stripe_customer_id TEXT,
    stripe_subscription_id TEXT UNIQUE,
    tier TEXT NOT NULL CHECK (tier IN ('premium', 'professional')),
    status TEXT NOT NULL CHECK (status IN ('active', 'inactive', 'cancelled', 'past_due', 'trialing')),
    current_period_end TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user ON subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_status ON subscriptions(status);

-- Usage tracking table
CREATE TABLE IF NOT EXISTS usage_tracking (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    feature_type TEXT NOT NULL,
    usage_date TEXT DEFAULT (date('now')),
    usage_count INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_usage_user ON usage_tracking(user_id);
CREATE INDEX IF NOT EXISTS idx_usage_date ON usage_tracking(usage_date);

-- AI provider quota tracking
CREATE TABLE IF NOT EXISTS ai_quota (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    date TEXT NOT NULL DEFAULT (date('now')),
    tokens_used INTEGER DEFAULT 0,
    requests_count INTEGER DEFAULT 0,
    last_request_at TEXT DEFAULT (datetime('now')),
    UNIQUE(provider, date)
);

CREATE INDEX IF NOT EXISTS idx_ai_quota_provider_date ON ai_quota(provider, date);
