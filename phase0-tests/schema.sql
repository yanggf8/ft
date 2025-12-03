-- Phase 0: D1 Compatibility Test Schema
-- Converted from PostgreSQL to SQLite-compatible syntax

-- Users table (replaces Supabase auth.users + user_profiles)
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    full_name TEXT,
    phone TEXT,
    date_of_birth TEXT,
    gender TEXT CHECK (gender IN ('male', 'female')),
    avatar_url TEXT,
    birth_location TEXT, -- JSON string
    preferred_astrology_types TEXT, -- JSON array as string
    subscription_tier TEXT DEFAULT 'free' CHECK (subscription_tier IN ('free', 'premium', 'professional')),
    subscription_status TEXT DEFAULT 'inactive' CHECK (subscription_status IN ('active', 'inactive', 'cancelled', 'trial')),
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_subscription_tier ON users(subscription_tier);

-- Subscriptions table
CREATE TABLE IF NOT EXISTS subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stripe_customer_id TEXT,
    stripe_subscription_id TEXT UNIQUE,
    tier TEXT NOT NULL CHECK (tier IN ('premium', 'professional')),
    status TEXT NOT NULL CHECK (status IN ('active', 'inactive', 'cancelled', 'past_due', 'trialing')),
    current_period_start TEXT,
    current_period_end TEXT,
    trial_start TEXT,
    trial_end TEXT,
    cancel_at TEXT,
    cancelled_at TEXT,
    metadata TEXT DEFAULT '{}', -- JSON string
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_stripe_id ON subscriptions(stripe_subscription_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_status ON subscriptions(status);

-- Chart records table
CREATE TABLE IF NOT EXISTS chart_records (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chart_type TEXT NOT NULL CHECK (chart_type IN ('ziwei', 'western_natal', 'western_transit', 'composite', 'synastry')),
    chart_name TEXT NOT NULL,
    birth_data TEXT NOT NULL, -- JSON string
    chart_data TEXT NOT NULL, -- JSON string
    analysis_data TEXT DEFAULT '{}', -- JSON string
    tags TEXT DEFAULT '[]', -- JSON array as string
    is_favorite INTEGER DEFAULT 0,
    is_public INTEGER DEFAULT 0,
    share_code TEXT UNIQUE,
    view_count INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_chart_records_user_id ON chart_records(user_id);
CREATE INDEX IF NOT EXISTS idx_chart_records_chart_type ON chart_records(chart_type);
CREATE INDEX IF NOT EXISTS idx_chart_records_is_favorite ON chart_records(is_favorite);
CREATE INDEX IF NOT EXISTS idx_chart_records_share_code ON chart_records(share_code);

-- Related charts junction table (replaces UUID[] array)
CREATE TABLE IF NOT EXISTS chart_relations (
    id TEXT PRIMARY KEY,
    chart_id TEXT NOT NULL REFERENCES chart_records(id) ON DELETE CASCADE,
    related_chart_id TEXT NOT NULL REFERENCES chart_records(id) ON DELETE CASCADE,
    relation_type TEXT DEFAULT 'related',
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(chart_id, related_chart_id)
);

CREATE INDEX IF NOT EXISTS idx_chart_relations_chart_id ON chart_relations(chart_id);

-- Usage tracking table
CREATE TABLE IF NOT EXISTS usage_tracking (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    visitor_id TEXT,
    app_type TEXT NOT NULL CHECK (app_type IN ('ziwei', 'zodiac', 'unified')),
    feature_type TEXT NOT NULL,
    feature_details TEXT DEFAULT '{}', -- JSON string
    usage_count INTEGER DEFAULT 1,
    usage_date TEXT DEFAULT (date('now')),
    session_id TEXT,
    ip_address TEXT,
    user_agent TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_usage_tracking_user_id ON usage_tracking(user_id);
CREATE INDEX IF NOT EXISTS idx_usage_tracking_visitor_id ON usage_tracking(visitor_id);
CREATE INDEX IF NOT EXISTS idx_usage_tracking_usage_date ON usage_tracking(usage_date);

-- Favorites table (explicit instead of is_favorite flag for flexibility)
CREATE TABLE IF NOT EXISTS favorites (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chart_id TEXT NOT NULL REFERENCES chart_records(id) ON DELETE CASCADE,
    created_at TEXT DEFAULT (datetime('now')),
    UNIQUE(user_id, chart_id)
);

CREATE INDEX IF NOT EXISTS idx_favorites_user_id ON favorites(user_id);

-- Subscription features table
CREATE TABLE IF NOT EXISTS subscription_features (
    id TEXT PRIMARY KEY,
    tier TEXT NOT NULL CHECK (tier IN ('free', 'premium', 'professional')),
    app_type TEXT NOT NULL CHECK (app_type IN ('ziwei', 'zodiac', 'unified')),
    feature_name TEXT NOT NULL,
    feature_limit INTEGER,
    is_enabled INTEGER DEFAULT 1,
    metadata TEXT DEFAULT '{}',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(tier, app_type, feature_name)
);

-- Sessions table (for Durable Objects fallback)
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    last_accessed TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
