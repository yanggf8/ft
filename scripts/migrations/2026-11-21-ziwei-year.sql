-- 紫微「今年運勢」AI 解讀快取（spec docs/superpowers/specs/2026-11-21-sub-units-design.md §A.5）
-- 依站長全域規則：不加任何 constraint（FK／UNIQUE／CHECK／NOT NULL 皆無；只保留 PRIMARY KEY）。
-- 欄位必有值由程式端（routes/ziwei.rs）保證。
-- 部署順序：本 migration 必須先於 API 部署（新 API 會讀寫此表；生辰 PUT 會刪此表）。
-- 快取鍵＝(user_id, luck_year, engine_version_ziwei, horoscope_version, prompt_version)；
-- 先查後插，併發雙寫最多多一筆，讀取取最新。失敗不落庫——只有 AI 成功且輸出檢查通過才寫入。
-- 生辰變更（PUT /api/users/me/birth）一併刪除該使用者的列（快取鍵不含生辰）。
-- 紅線：今年運勢資料永不寫入 predictions/prediction_feedback/f8_assignments。
CREATE TABLE IF NOT EXISTS ziwei_year_readings (
  id                   TEXT PRIMARY KEY,
  user_id              TEXT,
  luck_year            INTEGER,
  engine_version_ziwei TEXT,
  horoscope_version    TEXT,
  prompt_version       TEXT,
  ai_interpretation    TEXT,
  ai_provider          TEXT,
  ai_model             TEXT,
  created_at           TEXT
);
CREATE INDEX IF NOT EXISTS idx_zyr_lookup
  ON ziwei_year_readings(user_id, luck_year);
