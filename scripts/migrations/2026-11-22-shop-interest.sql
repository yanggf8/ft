-- 開運小物 v1「我有興趣」登記（spec docs/superpowers/specs/2026-11-22-shop-interest-design.md §4）
-- 依站長全域規則：除 PRIMARY KEY 外不加 constraint。不記 IP、UA。
-- 欄位必有值由程式端（routes/shop.rs）保證；item_id 為 ft_schema::shop CATALOG 的 id（不可改名）。
-- 部署順序：本 migration 必須先於 API 部署（新 API 會讀寫此表）。
-- 去重＝item_id 且 (user_id 或 anon_id) 相符；匿名轉登入以 UPDATE 補 user_id，不新增列。
-- 判讀看 COUNT(DISTINCT COALESCE(user_id, anon_id))。日後新增帳號刪除路由須一併刪本表。
-- 紅線：小物資料永不寫入 predictions/prediction_feedback/f8_assignments。
CREATE TABLE IF NOT EXISTS shop_interest (
  id               TEXT PRIMARY KEY,
  item_id          TEXT,
  catalog_version  TEXT,
  user_id          TEXT,      -- 匿名為 NULL
  anon_id          TEXT,      -- 瀏覽器隨機 UUID
  created_at       TEXT
);
CREATE INDEX IF NOT EXISTS idx_shop_interest_item ON shop_interest(item_id);
CREATE INDEX IF NOT EXISTS idx_shop_interest_user ON shop_interest(user_id, item_id);
CREATE INDEX IF NOT EXISTS idx_shop_interest_anon ON shop_interest(anon_id, item_id);
