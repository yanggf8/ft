-- 塔羅 v1b：tarot_readings 表（一次性遷移；schema.sql 已同步）
-- 套用：turso db shell fortunet < scripts/migrations/2026-10-27-tarot.sql
CREATE TABLE IF NOT EXISTS tarot_readings (
  id             TEXT PRIMARY KEY,
  user_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  request_id     TEXT NOT NULL,
  question       TEXT,
  spread         TEXT NOT NULL CHECK (spread IN ('three')),
  cards          TEXT NOT NULL,
  interpretation TEXT NOT NULL,
  corpus_version TEXT NOT NULL,
  created_at     TEXT NOT NULL,
  UNIQUE(user_id, request_id)
);
CREATE INDEX IF NOT EXISTS idx_tarot_user_created ON tarot_readings(user_id, created_at);
