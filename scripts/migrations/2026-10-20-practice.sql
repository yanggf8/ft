-- Apply once to existing Turso before deploying weekly practice API.
-- The CREATE statements are repeatable; SQLite ADD COLUMN is not.
ALTER TABLE users ADD COLUMN practice_epoch INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS practice_plans (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    goal_code TEXT NOT NULL,
    action_code TEXT NOT NULL,
    catalog_version TEXT NOT NULL,
    replace_active INTEGER NOT NULL CHECK (replace_active IN (0, 1)),
    replaces_plan_id TEXT,
    request_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    ended_at TEXT,
    ended_by_request_id TEXT,
    CHECK ((replace_active = 0 AND replaces_plan_id IS NULL)
        OR (replace_active = 1 AND replaces_plan_id IS NOT NULL)),
    UNIQUE(user_id, request_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_practice_one_active
    ON practice_plans(user_id) WHERE ended_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_practice_plans_user_created
    ON practice_plans(user_id, created_at);

CREATE TABLE IF NOT EXISTS practice_attempts (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL REFERENCES practice_plans(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    cycle_id TEXT NOT NULL,
    tried INTEGER NOT NULL CHECK (tried IN (0, 1)),
    helpfulness TEXT,
    obstacle TEXT,
    request_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    CHECK (
        (tried = 1 AND helpfulness IS NOT NULL
            AND helpfulness IN ('helped', 'unsure', 'not_helped') AND obstacle IS NULL)
        OR
        (tried = 0 AND obstacle IS NOT NULL
            AND obstacle IN ('no_chance', 'forgot', 'too_big', 'other') AND helpfulness IS NULL)
    ),
    UNIQUE(user_id, request_id)
);
CREATE INDEX IF NOT EXISTS idx_practice_attempts_user_cycle
    ON practice_attempts(user_id, cycle_id, created_at);
CREATE INDEX IF NOT EXISTS idx_practice_attempts_plan_created
    ON practice_attempts(plan_id, created_at);
