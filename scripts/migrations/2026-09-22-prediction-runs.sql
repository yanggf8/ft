-- Allow multiple prediction batches in one cycle.
-- Run once on existing Turso before deploying the multi-run API.
ALTER TABLE predictions ADD COLUMN run_id TEXT;
DROP INDEX IF EXISTS idx_predictions_user_cycle_domain;
CREATE INDEX IF NOT EXISTS idx_predictions_user_cycle_run
  ON predictions(user_id, cycle_id, run_id);

CREATE TABLE IF NOT EXISTS prediction_runs (
  id         TEXT PRIMARY KEY,
  user_id    TEXT NOT NULL,
  cycle_id   TEXT NOT NULL,
  profile_id TEXT NOT NULL,
  work       INTEGER NOT NULL,
  love       INTEGER NOT NULL,
  family     INTEGER NOT NULL,
  money     INTEGER NOT NULL,
  health    INTEGER NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_prediction_runs_user_cycle
  ON prediction_runs(user_id, cycle_id, created_at);
