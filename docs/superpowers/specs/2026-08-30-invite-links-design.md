# 邀請連結系統設計(內測邀請制)

**日期**:2026-08-30
**狀態**:設計已核准(對話中四項決策 + 分節設計一次通過)
**上游依賴**:magic link 認證(`docs/launch-record-2026-08-29.md`)

## 目的

站長透過 Messenger 發送邀請連結給內測人員;內測期註冊必須持有效邀請碼;
站長有管理頁可建立、追蹤、撤銷連結。未來開放註冊 = 改一個程式內 const。

## 已核准的決策

1. 註冊**邀請制**(無效碼不寄信);開放註冊之後靠程式內 const 切換,不用環境變數。
2. **多條命名連結**:每條有備註、人數上限、過期日、可個別撤銷;一條連結貼群組先到先得。
3. 管理介面 = 登入後的 **/admin 頁**,非管理員不可見不可用。
4. 管理員 = `ADMIN_EMAIL` var 比對 session email(大小寫不敏感),不寫死程式碼。

## 資料模型(D1,全加法)

```sql
CREATE TABLE IF NOT EXISTS invites (
    code TEXT PRIMARY KEY,        -- 10 碼 crypto 隨機 Crockford-base32(去 0/O/1/I)
    label TEXT NOT NULL,
    max_uses INTEGER NOT NULL,
    used_count INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT,              -- NULL = 永不過期;app 寫入 ISO(見 schema.sql 註記)
    revoked_at TEXT,              -- NULL = 有效
    created_by TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);
ALTER TABLE login_tokens ADD COLUMN pending_invite_code TEXT;
ALTER TABLE users ADD COLUMN invited_by TEXT;
```

### 消費語意(兩階段,不燒名額)

- **register**:只 SELECT 驗證(存在、未撤銷、未過期、未滿)→ 無效回 400
  「邀請碼無效或已失效」;有效則把 code 記入 `login_tokens.pending_invite_code`。
  此處刻意不套帳號防枚舉的統一 202 — 邀請碼不是帳號,持有者輸入錯誤必須立刻知道。
- **verify**(建帳號當下):原子消費
  `UPDATE invites SET used_count = used_count + 1 WHERE code = ?1 AND used_count < max_uses
   AND (expires_at IS NULL OR expires_at > ?now) AND revoked_at IS NULL`,
  affected=0 → 回 409「邀請名額已用完」;成功則 `users.invited_by = code`。
  login(既有帳號)不需要消費 — 只有註冊路徑消費。
- 碼產生:`secure_token_hex` 同源 crypto 隨機,映射到 32 字元集(去 0/O/1/I 的 base32),
  10 碼;PK 衝突機率可忽略,重試一次即足。

## API

| 路由 | 授權 | 請求 → 回應 |
| --- | --- | --- |
| `GET /api/invites/:code` | 公開 | — → `{valid: bool, label: Option}`(無效也 200,valid=false) |
| `POST /api/admin/invites` | admin | `{label, max_uses, expires_at?}` → `{code, url, label, maxUses, expiresAt}` |
| `GET /api/admin/invites` | admin | — → `{invites: [{code, label, maxUses, usedCount, expiresAt, revokedAt, createdAt}]}` |
| `POST /api/admin/invites/:code/revoke` | admin | — → `{ok: true}`(冪等) |

- admin 判定:handler 內比對 `ADMIN_EMAIL` var 與 session email(小寫化後),不符 → 403。
  var 未設 → 一律 403(fail-closed)。
- `/api/users/me` 加 `isAdmin: bool`。
- register body 加 `invite: Option<String>`;內測期間 invite 為必填(缺/無效 → 400),
  開關 `INVITE_REQUIRED` const 落在 auth.rs。
- 完整連結 = `{WEB_ORIGIN}/register?invite={code}`,由後端組好回傳。

## 前端(Leptos)

- 登入頁註冊 tab:邀請碼欄位;URL 帶 `?invite=` 自動預填;預檢
  `GET /api/invites/:code` 顯示「✓ 已套用邀請:{label}」或錯誤。
- `/admin` 路由(Protected + is_admin):建連結表單(備註/人數/過期日 optional)、
  列表(碼、備註、used/max、過期、狀態、「複製連結」、「撤銷」按鈕)。
- 非管理員直接打 `/admin` → 頁面顯示無權限(資料層仍由 API 403 擋,前端只是體驗)。

## 測試

- 單元(ft-api native):碼產生(字元集、長度、排除字元)、邀請有效性述職
  (未過期/未撤銷/未滿的組合)。
- 生產 E2E(人工,同 magic link 流程):建連結 → 複製 → 無痕視窗註冊 →
  收信 → 驗證 → `users.invited_by` 與 `used_count` 落值。

## 遷移與部署

1. `scripts/schema.sql` 加 CREATE TABLE + 兩個 ALTER(schema.sql 是唯一事實來源,
   供全新環境一次建全)。對**既有** production DB:SQLite 的 ADD COLUMN 無
   `IF NOT EXISTS`,兩句 ALTER 以 `wrangler d1 execute --command` 逐句執行一次
   (重跑會報 duplicate column,屬預期;不再放進 `--file schema.sql` 的常規執行)。
2. `wrangler.toml [vars]` 加 `ADMIN_EMAIL=yanggf@msn.com`。
3. `wrangler d1 execute --remote` + `wrangler deploy`(API),前端 `deploy-web.sh`。

## 明確不做的(YAGNI)

- 不做 invite_uses 明細表(`users.invited_by` 夠用)。
- 不做角色系統、不做多管理員。
- 不做邀請碼自訂字串(crypto 隨機即可)。
- 不做環境變數的邀請/開放切換(const 一行,改版即可)。
