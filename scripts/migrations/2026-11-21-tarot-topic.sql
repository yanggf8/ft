-- 塔羅子單元（sub-units spec 2026-11-21 §B.4）：三張牌陣主題 general|love|career。
-- 新寫入一律存 wire 字串（含 'general'）；舊列 NULL 由 API 讀成 general
-- （冪等比對亦 NULL ≡ general）。無 CHECK（全站規則：不加 DB 約束）。
-- 部署順序：本 migration 先於 API（新 API 的 INSERT 會寫 topic 欄）。
ALTER TABLE tarot_readings ADD COLUMN topic TEXT;
