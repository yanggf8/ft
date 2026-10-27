-- 塔羅 v2 補充：AI 解牌是否使用命盤 context（Muse 前審 S2）
-- NULL＝從未嘗試；1＝含命盤摘要；0＝純塔羅（?chart=0 或無命盤資料）
ALTER TABLE tarot_readings ADD COLUMN ai_chart_context INTEGER;
