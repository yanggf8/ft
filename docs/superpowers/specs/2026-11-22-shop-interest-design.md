# 開運小物 v1 — 展示＋「我有興趣」登記（設計 v1）

日期：2026-11-22　狀態：設計 v1.2 定稿（自審修 2 處；獨立審 0 blocker、6 should-fix＋5 nit 全數採納，見 §10）
來源：站長 2026-11-20「主頁面幫我加上 開運小物／要賣東西」；2026-11-22「開運小物也照推薦先做起來」。

## 1. 目標與非目標

**目標**：首頁新增「開運小物」入口，一頁展示 6 件候選小物，每件可按「我有興趣」。
用登記數量決定要不要做、先做哪幾件，之後再談金流。

**非目標（v1 不做）**：
- 金流、購物車、價格、庫存、出貨、收件資料。商業模式未定（cort `backlog-units-shop`）。
- 不顯示任何價格或「預購」字樣——沒有定價就不寫數字（no default data）。
- 不收姓名、信箱、電話；匿名登記不記 IP。
- 不顯示登記人數（量少時是假社會證明，也容易被灌）。
- 不接 AI、不接命盤（不做「依命盤推薦小物」：那會把命理論斷變成銷售話術）。

## 2. 定位與文案紅線

小物＝**給自己的提醒物**，連回站上某個單元的反思內容；**不宣稱任何改運效果**。
與 `GUARDRAIL_ZH`（傾向非注定、不做確定性斷言）及風水語料禁字一致。

- 頁首固定說明：「還沒開賣。這些是我們考慮做的小物，按『我有興趣』讓我們知道你想要哪一個。
  小物是給自己的提醒，不會改變運勢。」
- 資料說明：「只記錄你點了哪一件。登入時會連到你的帳號，方便之後取消；不收姓名、信箱或付款資料。」
- 單元名稱「開運小物」是站長指定的入口名，**只出現在標題與首頁卡**；商品文字禁字（schema 測試守門）：
  開運、招財、化煞、擋煞、避邪、驅邪、轉運、改運、旺、靈驗、加持、保證、一定、必定、
  能量、磁場、功效、療效、治療、神奇、好運、幸運、預購、價格、NT$，以及「數字＋元」金額（單字「元」不禁，避免誤傷「單元」）。

## 3. 商品目錄（schema 單一來源）

`crates/schema/src/shop.rs`：`SHOP_CATALOG_VERSION = "1"`（改文字或增刪品項必 bump）。
`ShopItem { id, name, tagline, body, unit_href, unit_label, image }`，`CATALOG: &[ShopItem]`，6 件：

| id | 名稱 | 連回 |
|---|---|---|
| `tarot-postcards` | 塔羅牌卡明信片（三張一組） | `/tarot` |
| `zodiac-bookmark` | 十二星座色票書籤 | `/zodiac` |
| `ziwei-stickers` | 紫微主星手帳貼紙 | `/ziwei` |
| `year-journal` | 今年主題反思手帳 | `/ziwei/year` |
| `desk-plant` | 桌面小盆栽 | `/fengshui` |
| `warm-lamp` | 暖光小檯燈 | `/fengshui` |

`id` 為 wire/儲存鍵，**不可改名**（只能新增或下架；下架＝自 CATALOG 移除＋bump，舊列保留）。
`pub fn find(id) -> Option<&ShopItem>`。測試：id 唯一且符合 `[a-z0-9-]+`、禁字、`unit_href` 以 `/` 開頭、
`image` 以 `/assets/shop/` 開頭。

## 4. API（fortunet-api）

Wire DTO 在 `ft_schema::api`：`ShopInterestRequest{itemId, anonId?}`、`ShopInterestResponse{itemId, registered, already}`、
`ShopInterestMeResponse{itemIds}`。

**身分**：`anonId`＝瀏覽器 localStorage `shop_interest_v1` 內的隨機 UUID（前端首次進 /shop 產生）；
不是 PII、不記 IP／UA。登入可選：新增 `common.rs` `auth_user_opt(req, ctx) -> Option<String>`
（`auth_user(..).await.ok()`）。注意：SessionDO 故障時登入者會被當匿名記錄（可接受，`anonId` 仍去重）。
`anonId` 驗證：長度 8–64、`[A-Za-z0-9-]`，不合 → 400 `INVALID_ANON_ID`；缺省允許（舊客戶端）。

- `POST /api/shop/interest` body `ShopInterestRequest`：
  1. `body_too_large`（64 KiB）。
  2. `rate_limit`（fail-open，無成本端點）key `shop:ip:{client_ip}` 30 次／小時 → 429 `RATE_LIMITED`。
  3. `itemId` 不在 CATALOG → 400 `INVALID_ITEM`。
  4. 去重：已有列滿足 `item_id=? AND (user_id=?uid OR anon_id=?anon)`（只帶有值的條件）→
     200 `already:true`；若既有列 user_id 為 NULL 而本次有登入 → `UPDATE` 補上 user_id（匿名轉登入不重算）。
  5. 否則插一列 `(id, item_id, catalog_version, user_id, anon_id, created_at)` → 200 `already:false`。
  - 無效／過期 token 視同匿名，不回 401。DB 錯誤 → 500 `"db error"`。
- `GET /api/shop/interest/me?anonId=…`：登入者或 anonId 至少一個，否則 400 `NO_IDENTITY`；
  回兩者聯集、去重、只留目前 CATALOG 內的 id。
- `DELETE /api/shop/interest/:itemId` body `{"itemId","anonId"}`（itemId 以路徑為準）：
  刪 `item_id=? AND (user_id=?uid OR anon_id=?anon)` 的列 → 200 `registered:false`；兩者皆無 → 400 `NO_IDENTITY`。

### 資料表（migration `scripts/migrations/2026-11-22-shop-interest.sql`，同步 `schema.sql`）

```sql
-- 依站長全域規則：除 PRIMARY KEY 外不加 constraint。不記 IP、UA。
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
```

紅線：小物資料永不寫入 predictions／prediction_feedback／f8_assignments；不 import `symbolic`。
目前沒有帳號刪除路由；日後新增帳號刪除時須一併刪 `shop_interest`（寫進 ARCHITECTURE）。

**判讀**：決策看 `COUNT(DISTINCT COALESCE(user_id, anon_id))`。清 localStorage／換瀏覽器／
免費註冊（INVITE_REQUIRED=false）都能灌，數字只是方向參考，不是精確需求。並發雙擊可能多一列，去重計數不受影響。

營運查詢（寫進 ARCHITECTURE）：
`SELECT item_id, COUNT(*), COUNT(DISTINCT COALESCE(user_id, anon_id)) FROM shop_interest GROUP BY item_id;`

## 5. 前端（Leptos）

- 首頁 feature-grid 新卡 `<A href="/shop" class="feature">`「開運小物」＋NEW，新 `FeatureIcon kind="shop"`
  （小禮物盒描邊 path；`.fi-shop` 圓底色用既有 token）。卡文用 `HOME_CARD_BLURB`。
- 新頁 `crates/web/src/pages/shop.rs`，路由 `/shop`：
  - 頁首說明（§2 兩段，取自 schema 常數）。
  - 6 張卡（沿用 `.card` 金邊）：圖、名稱、tagline、body、「相關單元：{unit_label} →」連結、按鈕。
  - 按鈕兩態：「我有興趣」／「已登記 ✓（再按取消）」。
  - localStorage `shop_interest_v1` = `{"anonId": uuid, "items": [...]}`；狀態＝`GET /me?anonId=` 結果
    （失敗時退回 localStorage 的 items）。所有請求都帶 anonId；有 session 也帶 Bearer。
  - 請求進行中按鈕 disabled（防雙擊）；送出失敗（含 429）→ 卡內錯誤訊息，按鈕回原態。
  - 頁首／資料說明／首頁卡文一律用 `ft_schema::shop::{PAGE_INTRO, PRIVACY_NOTE, HOME_CARD_BLURB}`（受禁字測試）。
  - 「今年運勢」連結需登入（Protected），可接受。
- 圖片：`crates/web/assets/shop/{id}.webp`（build-web.sh 既有 `cp -r assets/*` 已涵蓋），約 600×600、
  kawaii 亮底手繪風、無文字、無品牌、無人臉；用 m4m `imgen` 產，挑圖後壓到每張 ≤ 120 KB。
  `<img alt=名稱 loading="lazy" width height>`。
- 色彩只用 token；跑 css-guard、contrast-check。手機 420px 單欄、桌機兩欄或三欄。
- 名詞解釋不需新增。

## 6. 驗收

- `cargo test`（schema 新測試）、fmt、clippy wasm32、wasm build 全過。
- `verify-deployment.sh` 新探測：`POST /api/shop/interest {"itemId":"nope"}` → 400 `INVALID_ITEM`；
  `GET /api/shop/interest/me` 無 token → 401。（不在正式站插真列。）
- 正式站自測腳本 `scripts/shop-selftest.py`：匿名登記、登入登記／重複 already／me／取消、INVALID_ITEM，
  結束 SQL 硬清測試列。
- 瀏覽器自玩：首頁卡→/shop→匿名按→重整仍顯示已登記；登入後按、取消。

## 7. 部署順序

migration → api → web。

## 8. 分工（多 AI，各自 worktree；推送前各自 `gh auth switch -h github.com -u yanggf8`）

- 我（主線）：先寫 `ft_schema::shop`（目錄＋測試）並 commit，兩軌都從此分支。
- 軌 S（api）：migration、schema.sql、`routes/shop.rs`、mod 註冊、ARCHITECTURE/CLAUDE.md、verify 探測、自測腳本。
- 軌 W（web）：首頁卡＋圖示、`pages/shop.rs`、路由、CSS、m4m 產 6 張圖。
- 合併、全套檢查、部署、自測自玩由主線做。

## 9. 開放問題（交 cort，不等）

- 金流與定價：等登記數據（建議 30 天後看）再決定；屆時另開 spec。

## 10. 審查紀錄

- 自審：禁字「元」誤傷「單元」→改「數字＋元」；匿名可灌票→判讀改以去重數為主。
- 獨立審（fresh Claude，2026-11-22）：0 blocker，全數採納——
  1 無選擇性登入 helper→`auth_user_opt`；2 匿名轉登入重複計→`anonId` 欄位＋聯集去重＋匿名可取消；
  3 檯燈文案借研究背書→改；4 明信片「三張一組」與「抽到的那張」矛盾→「自選三張」；
  5 禁字不足、全形／中文數字價格、頁面文案未測→擴禁字、`has_price`、文案收進 schema 常數；
  6 DTO 不在 schema→加進 `ft_schema::api`；nit：盆栽 tagline 去風水效果暗示、localStorage key
  改 snake_case、按鈕 in-flight disabled、判讀註明免費註冊可灌、今年運勢連結需登入。
- 查核（主線）：抽驗 `auth_user`（common.rs:51）確實對所有失敗回 Err(401)；其餘依據已逐條對照。
