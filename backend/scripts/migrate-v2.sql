-- Migration: v1 → v2 (birth-data centric model)
-- Run with: npx wrangler d1 execute fortunet-db --remote --file=scripts/migrate-v2.sql

-- Add birth data columns to users
ALTER TABLE users ADD COLUMN birth_year INTEGER;
ALTER TABLE users ADD COLUMN birth_month INTEGER;
ALTER TABLE users ADD COLUMN birth_day INTEGER;
ALTER TABLE users ADD COLUMN birth_hour INTEGER;
ALTER TABLE users ADD COLUMN birth_minute INTEGER;
ALTER TABLE users ADD COLUMN gender TEXT CHECK (gender IN ('male', 'female'));
ALTER TABLE users ADD COLUMN timezone TEXT DEFAULT 'Asia/Taipei';
ALTER TABLE users ADD COLUMN latitude REAL;
ALTER TABLE users ADD COLUMN longitude REAL;
ALTER TABLE users ADD COLUMN birth_data_hash TEXT;

-- Create interpretations table
CREATE TABLE IF NOT EXISTS interpretations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    divination_type TEXT NOT NULL CHECK (divination_type IN ('ziwei', 'western', 'bazi')),
    chart_data TEXT NOT NULL,
    ai_interpretation TEXT,
    birth_data_hash TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(user_id, divination_type)
);

CREATE INDEX IF NOT EXISTS idx_interpretations_user ON interpretations(user_id);

-- Drop old chart_records table
DROP TABLE IF EXISTS chart_records;
