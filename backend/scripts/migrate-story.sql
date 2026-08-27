-- Migration: relax interpretations.divination_type CHECK to allow 'story' (and any future type)
-- SQLite/D1 cannot ALTER ... DROP CONSTRAINT, so we recreate the table without the CHECK.
-- Run LOCAL first, verify, then remote:
--   cd backend && unset CLOUDFLARE_API_TOKEN && npx wrangler d1 execute fortunet-db --local  --file=scripts/migrate-story.sql
--   cd backend && unset CLOUDFLARE_API_TOKEN && npx wrangler d1 execute fortunet-db --remote --file=scripts/migrate-story.sql
-- Idempotent guard: if interpretations_old already exists from a half-run, drop it first.

PRAGMA foreign_keys=OFF;

DROP TABLE IF EXISTS interpretations_old;

ALTER TABLE interpretations RENAME TO interpretations_old;

CREATE TABLE interpretations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    divination_type TEXT NOT NULL,   -- 'ziwei' | 'western' | 'bazi' | 'story' (no CHECK: flexible per project policy)
    chart_data TEXT NOT NULL,
    ai_interpretation TEXT,
    birth_data_hash TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(user_id, divination_type)
);

INSERT INTO interpretations (id, user_id, divination_type, chart_data, ai_interpretation, birth_data_hash, created_at, updated_at)
SELECT id, user_id, divination_type, chart_data, ai_interpretation, birth_data_hash, created_at, updated_at
FROM interpretations_old;

DROP TABLE interpretations_old;

CREATE INDEX IF NOT EXISTS idx_interpretations_user ON interpretations(user_id);

PRAGMA foreign_keys=ON;

-- Verify after running:
--   SELECT count(*) FROM interpretations;
--   INSERT INTO interpretations (id,user_id,divination_type,chart_data,birth_data_hash) VALUES ('test','test','story','{}','h') ; -- should succeed (then delete)
