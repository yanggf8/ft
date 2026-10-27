-- 塔羅 v2：AI 命盤感知解牌欄位（spec §Tarot v2-1；schema.sql 已同步）
-- ai_status 只有 done|failed（同步端點，pending 不可見）；NULL＝從未嘗試。
-- 規則式 interpretation 永不覆寫（歷史快照原則）。
ALTER TABLE tarot_readings ADD COLUMN ai_interpretation TEXT;
ALTER TABLE tarot_readings ADD COLUMN ai_status TEXT CHECK (ai_status IN ('done', 'failed'));
ALTER TABLE tarot_readings ADD COLUMN ai_prompt_version TEXT;
ALTER TABLE tarot_readings ADD COLUMN ai_provider TEXT;
ALTER TABLE tarot_readings ADD COLUMN ai_model TEXT;
ALTER TABLE tarot_readings ADD COLUMN ai_generated_at TEXT;
