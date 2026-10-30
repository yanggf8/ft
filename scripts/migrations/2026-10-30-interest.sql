-- Apply once to existing Turso before deploying interest explore v1 API.
-- The CREATE statements are repeatable; SQLite ADD COLUMN is not.
ALTER TABLE users ADD COLUMN interest_epoch INTEGER NOT NULL DEFAULT 0;
-- 興趣探索 v1（RIASEC，spec 2026-10-27-interest-explore-design.md §4.1）：
-- answers＝48 個 1..5 的 JSON 陣列（保留原始作答以便改版重算，永不上 wire）；
-- scores＝伺服器計分 JSON {"R":3.25,...}；instrument_version 對應 ft_schema
-- INTEREST_INSTRUMENT_VERSION。冪等：inline UNIQUE(user_id, request_id)。
-- 紅線：興趣資料永不寫入 predictions/prediction_feedback/f8_assignments。
CREATE TABLE IF NOT EXISTS interest_results (
  id                 TEXT PRIMARY KEY,
  user_id            TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  instrument_version TEXT NOT NULL,
  form               TEXT NOT NULL CHECK (form IN ('A')),   -- v1 僅 Set A
  answers            TEXT NOT NULL,
  scores             TEXT NOT NULL,
  request_id         TEXT NOT NULL,
  created_at         TEXT NOT NULL,
  UNIQUE(user_id, request_id)
);
CREATE INDEX IF NOT EXISTS idx_interest_user_created ON interest_results(user_id, created_at);
