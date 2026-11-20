-- 星座配對 AI 解讀（plan docs/superpowers/plans/2026-11-19-zodiac-match.md §2.5、§6.8）
-- 依站長全域規則：不加任何 constraint（FK／UNIQUE／CHECK／NOT NULL 皆無；只保留 PRIMARY KEY）。
-- 欄位必有值由程式端（routes/zodiac.rs）保證。
-- 部署順序：本 migration 必須先於 API 部署（新 API 會讀寫此表）。
-- 快取鍵＝(user_id, target_sign, sign_key, corpus_version, prompt_version)；先查後插，
-- 併發雙寫最多多一筆，讀取取最新。失敗不落庫——只有 AI 成功才寫入。
-- sign_key 固定格式 "Taurus|Aquarius|Virgo"（無上升／不明為 "-"）。
-- 生辰變更（PUT /api/users/me/birth）一併刪除該使用者的列。
-- 紅線：配對資料永不寫入 predictions/prediction_feedback/f8_assignments。
CREATE TABLE IF NOT EXISTS zodiac_match_readings (
  id                TEXT PRIMARY KEY,
  user_id           TEXT,
  target_sign       TEXT,
  sign_key          TEXT,
  corpus_version    TEXT,
  prompt_version    TEXT,
  ai_interpretation TEXT,
  ai_provider       TEXT,
  ai_model          TEXT,
  created_at        TEXT
);
CREATE INDEX IF NOT EXISTS idx_zmr_lookup
  ON zodiac_match_readings(user_id, target_sign, sign_key, corpus_version, prompt_version);
