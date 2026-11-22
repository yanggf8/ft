-- 上手小實驗（天賦線 v1；spec docs/superpowers/specs/2026-11-22-talent-tryout-design.md §9、§13）
-- 依站長全域規則：除 PRIMARY KEY 外不加 constraint（FK／UNIQUE／CHECK／NOT NULL 皆無）。
-- 欄位必有值由程式端（routes/talent.rs）保證；item_id 為 ft_schema::talent CATALOG 的 id（不可改名）。
-- 部署順序：fortunet-staging → fortunet → deploy-api（staging 先）→ deploy-web。
-- 同時只有一筆進行中：INSERT … SELECT … WHERE NOT EXISTS（原子）；回報用條件 UPDATE。
-- skipped：1＝這次沒做（reached NULL）、0＝已回報步數、未回報為 NULL。過期（7 天）由讀取時計算，不落庫。
-- 日後新增帳號刪除路由須一併刪本表。
-- 紅線：天賦線資料永不寫入 predictions/prediction_feedback/f8_assignments。
CREATE TABLE IF NOT EXISTS talent_trials (
  id                 TEXT PRIMARY KEY,
  user_id            TEXT,
  item_id            TEXT,
  catalog_version    TEXT,
  predicted          INTEGER,   -- 0–3
  created_at         TEXT,      -- RFC3339 UTC（clock::now_iso）
  create_request_id  TEXT,
  reached            INTEGER,   -- 0–3；沒做為 NULL
  skipped            INTEGER,
  stuck              TEXT,      -- ft_schema::talent::STUCK_REASONS wire
  reported_at        TEXT,
  report_request_id  TEXT
);
CREATE INDEX IF NOT EXISTS idx_talent_trials_user_created ON talent_trials(user_id, created_at);
