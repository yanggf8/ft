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
    generation_tags TEXT,            -- JSON array e.g. '["1980s","1990s"]'
    birth_data_hash TEXT,            -- for cache invalidation
    invited_by TEXT,                 -- invite code that created this account
    
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
    divination_type TEXT NOT NULL,   -- 'ziwei' | 'western' | 'bazi' | 'story' (no CHECK: flexible per project policy)
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

-- ── Big5 personality (F1) ──
-- measurement_status: 'complete' | 'careless_suspected' | 'skipped_prior_only'
CREATE TABLE IF NOT EXISTS personality_profiles (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  ipip_answers TEXT,                 -- JSON [15] int 1–5；skipped 記錄為 null
  ocean_measured TEXT,               -- JSON 五維 0–100；僅 complete 有值
  measurement_status TEXT NOT NULL,  -- complete | careless_suspected | skipped_prior_only
  item_duration_ms INTEGER,          -- 總作答時長 ms（client 量測上送）
  careless_flags TEXT,               -- JSON {too_fast,straight_lining,inconsistent,dims}；
                                     -- per-signal/per-dim 校準日誌（Grok 對抗審 #5）
  created_at TEXT NOT NULL           -- 一律由 app 寫 clock::now_iso()（ISO）；不用 DEFAULT
                                     -- datetime('now')：空格格式與 ISO 字典序不一致，混用會亂序
);
-- 註解（F9/後續切片防誤讀）：complete 與 careless_suspected 記錄都帶 ipip_answers；
-- skipped_prior_only **兩種都可能**——主動 skip（{skip:true}）為 null、亂答升級
-- 為帶 answers。狀態一律以 measurement_status 為準，不以 answers 是否為 null 推斷。

-- ── Magic-link login tokens (P0-01) ──
-- 只存 token 的 SHA-256（hex），明碼 token 只存在 email 連結裡。
-- expires_at / created_at 一律由 app 寫 ISO（services/login_token.rs）；
-- 過期判斷用 expires_at > :now_iso，勿與 datetime('now') 的空格格式混比。
CREATE TABLE IF NOT EXISTS login_tokens (
    token_hash TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    -- Register flow: the requested full_name rides on the token row; the
    -- users row is created by /api/auth/verify only after ownership is proven.
    pending_full_name TEXT,
    -- Two-phase invite (spec 2026-08-30): the register request's validated
    -- invite code rides here; verify consumes it when creating the account.
    pending_invite_code TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_login_tokens_email ON login_tokens(email);

-- ── Beta invite links (spec: docs/superpowers/specs/2026-08-30-invite-links-design.md) ──
-- expires_at / revoked_at are app-written ISO strings; compare ISO to ISO only.
CREATE TABLE IF NOT EXISTS invites (
    code TEXT PRIMARY KEY,           -- 10 glyphs, crypto random, no 0/O/1/I/L/U
    label TEXT NOT NULL,             -- owner's note, e.g. "Messenger 群 A"
    max_uses INTEGER NOT NULL,
    used_count INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT,                 -- NULL = never expires
    revoked_at TEXT,                 -- NULL = active
    created_by TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

-- One-time migrations for EXISTING databases (SQLite has no ADD COLUMN IF NOT
-- EXISTS; run each once via `wrangler d1 execute --command`). Fresh installs
-- get these from the CREATE TABLE definitions above:
--   ALTER TABLE login_tokens ADD COLUMN pending_invite_code TEXT;
--   ALTER TABLE users ADD COLUMN invited_by TEXT;
--   ALTER TABLE users ADD COLUMN generation_tags TEXT;  -- P0: users.generation_tags JSON array; idempotent — re-running on a DB that already has the column will error "duplicate column name", which is safe to ignore

-- Idempotent helper for existing Turso DBs (run once; ignore duplicate-column error):
--   turso db shell fortunet "ALTER TABLE users ADD COLUMN generation_tags TEXT"

-- ── F5 predictions / situation_checks / prediction_feedback ──
-- Spec: docs/superpowers/specs/2026-09-03-f5-rule-anchors-design.md §3
-- 遷移註記：rev.3 舊 predictions(situation_id, divination_type, prediction_text, cache_key)
-- 與舊 situation_checks(id, domains JSON) 已作廢；本 DDL 為重建權威，舊 prod 表需 DROP 後重建
-- cycle_id = Asia/Taipei 週一 00:00 起算的週起始日 YYYY-MM-DD，對齊 7 天視野與 F6 回訪
CREATE TABLE IF NOT EXISTS predictions (
  id                TEXT PRIMARY KEY,
  user_id           TEXT NOT NULL,
  profile_id        TEXT NOT NULL,
  cycle_id          TEXT NOT NULL,
  domain            TEXT NOT NULL,
  trigger           TEXT NOT NULL,
  tendency          TEXT NOT NULL,
  forecast          TEXT NOT NULL,
  experiment        TEXT,
  anchor_ids        TEXT NOT NULL,
  anchor_coverage   TEXT NOT NULL,
  source            TEXT NOT NULL DEFAULT 'rule_anchor',
  rules_version     TEXT NOT NULL,
  is_control        INTEGER NOT NULL DEFAULT 0,
  created_at        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_predictions_user_cycle ON predictions(user_id, cycle_id);
CREATE INDEX IF NOT EXISTS idx_predictions_profile ON predictions(profile_id);

CREATE TABLE IF NOT EXISTS situation_checks (
  user_id     TEXT NOT NULL,
  cycle_id    TEXT NOT NULL,
  trigger     TEXT NOT NULL,
  situation   TEXT NOT NULL,
  created_at  TEXT NOT NULL,
  PRIMARY KEY (user_id, cycle_id, trigger)
);

-- F6 第 2 段（僅在 occurred 時）
CREATE TABLE IF NOT EXISTS prediction_feedback (
  prediction_id TEXT PRIMARY KEY,
  response      TEXT NOT NULL,
  created_at    TEXT NOT NULL,
  FOREIGN KEY (prediction_id) REFERENCES predictions(id) ON DELETE CASCADE
);
