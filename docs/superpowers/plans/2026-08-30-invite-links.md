# 邀請連結系統 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Archived 2026-08-31:** this feature shipped and the residual unchecked boxes are bookkeeping, not open work — see git log for the shipped commits.


**Goal:** 內測邀請制:多條命名邀請連結 + 原子消費 + /admin 管理頁。

**Architecture:** D1 `invites` 表 + login_tokens/users 各加一欄;register 兩階段
(SELECT 驗證 → 攜碼 → verify 建帳號時原子消費);admin API 以 `ADMIN_EMAIL` var
把關;Leptos 登入頁加邀請碼欄位、新增 /admin 頁。

**Tech Stack:** Rust workers-rs + D1 + Leptos CSR。既有模式:原子單次消費
(`UPDATE ... WHERE ...` + `meta().changes`)、`secure_token_hex` crypto 隨機、
`db::text/opt_text` 綁定。

**Spec:** `docs/superpowers/specs/2026-08-30-invite-links-design.md`

## Global Constraints

- 機器約束:cargo 一次一條、一律 `timeout 500 cargo ...`;agent 禁跑 wrangler
  (D1 遷移與部署由主線程執行);agent 可 commit 自己的 task,push 由主線程/用戶處理。
- D1 變更全加法;既有 wire key / storage key 不得改名。
- Rust 2-space、serde camelCase 走 rename;錯誤訊息用繁體中文(面向用戶)。
- 金鑰/碼一律 fail-closed(crypto 不可用 → 拒絕),禁 Math::random。
- 時間比較一律 app 寫入 ISO 對 ISO(見 schema.sql 註記)。

---

### Task 1: invite 純邏輯服務(碼產生 + 有效性述職)

**Files:**
- Create: `crates/api/src/services/invite.rs`
- Modify: `crates/api/src/services/mod.rs`(加 `pub mod invite;`)
- Test: `crates/api/src/services/invite.rs` 內 `#[cfg(test)]`

**Interfaces:**
- Produces: `INVITE_REQUIRED: bool`(= true)、`CODE_LEN: usize`(= 10)、
  `new_code() -> Option<String>`、`InviteRow { used_count: i64, max_uses: i64,
  expires_at: Option<String>, revoked_at: Option<String> }`、
  `is_usable(row: &InviteRow, now_iso: &str) -> bool`

- [ ] **Step 1: 寫失敗測試**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_uses_restricted_alphabet_and_length() {
        for _ in 0..32 {
            let c = new_code().expect("crypto available in workerd");
            assert_eq!(c.len(), CODE_LEN);
            assert!(c.chars().all(|ch| CHARSET.contains(&(ch as u8))));
        }
    }

    #[test]
    fn usable_fresh_invite() {
        let row = row(0, 20, None, None);
        assert!(is_usable(&row, "2026-08-30T00:00:00.000Z"));
    }

    #[test]
    fn unusable_when_revoked() {
        let row = row(0, 20, None, Some("2026-08-29T00:00:00.000Z".into()));
        assert!(!is_usable(&row, "2026-08-30T00:00:00.000Z"));
    }

    #[test]
    fn unusable_when_expired_boundary_counts_as_expired() {
        let row = row(0, 20, Some("2026-08-30T00:00:00.000Z".into()), None);
        assert!(!is_usable(&row, "2026-08-30T00:00:00.000Z"));
        assert!(is_usable(&row, "2026-08-29T23:59:59.999Z"));
    }

    #[test]
    fn unusable_at_capacity() {
        let row = row(20, 20, None, None);
        assert!(!is_usable(&row, "2026-08-30T00:00:00.000Z"));
    }

    #[test]
    fn null_expiry_never_expires() {
        let row = row(0, 1, None, None);
        assert!(is_usable(&row, "2099-01-01T00:00:00.000Z"));
    }

    fn row(used: i64, max: i64, exp: Option<String>, rev: Option<String>) -> InviteRow {
        InviteRow { used_count: used, max_uses: max, expires_at: exp, revoked_at: rev }
    }
}
```

- [ ] **Step 2: 跑測試確認失敗**

Run: `timeout 300 cargo test --locked -p ft-api --lib invite`
Expected: FAIL (module 不存在)

- [ ] **Step 3: 最小實作**

```rust
//! Invite codes: generation + usability predicate (spec 2026-08-30).
//! Registration requires a valid invite during beta; flip INVITE_REQUIRED to
//! open registration. Codes are crypto-random (fail-closed), drawn from a
//! 30-glyph alphabet without 0/O/1/I/L/U so a code read off a phone screen
//! cannot be misread. Modulo bias exists (256 % 30 != 0) and is acceptable:
//! an invite code is not a sole secret, it is rate-limited and quantity-bound.

/// Beta gate: true = register demands a valid invite code.
pub const INVITE_REQUIRED: bool = true;

pub const CODE_LEN: usize = 10;

const CHARSET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ"; // 30 glyphs, no 0O1ILU

/// One `invites` row, as needed for the usability check.
#[derive(Debug, serde::Deserialize)]
pub struct InviteRow {
    pub used_count: i64,
    pub max_uses: i64,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

/// Mint a code, or `None` when crypto is unavailable (fail closed).
pub fn new_code() -> Option<String> {
    let bytes = crate::services::uuid::secure_bytes(CODE_LEN)?;
    Some(
        bytes
            .iter()
            .map(|b| CHARSET[(*b as usize) % CHARSET.len()] as char)
            .collect(),
    )
}

/// ISO-vs-ISO comparison only (see scripts/schema.sql note). The exact expiry
/// instant counts as expired, mirroring login_token semantics.
pub fn is_usable(row: &InviteRow, now_iso: &str) -> bool {
    row.revoked_at.is_none()
        && row.used_count < row.max_uses
        && row
            .expires_at
            .as_deref()
            .map_or(true, |e| e > now_iso)
}
```

`services/mod.rs` 加一行 `pub mod invite;`

- [ ] **Step 4: 跑測試通過 + fmt + check**

Run: `timeout 120 cargo fmt --all && timeout 300 cargo test --locked -p ft-api --lib invite && timeout 500 cargo check -p ft-api --target wasm32-unknown-unknown`
Expected: 全 PASS / Finished

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/services/invite.rs crates/api/src/services/mod.rs
git commit -m "feat(api): invite code generation + usability predicate"
```

---

### Task 2: schema + ADMIN_EMAIL + 一次性 prod 遷移

**Files:**
- Modify: `scripts/schema.sql`(invites 表 + 兩句 ALTER,標註一次性)
- Modify: `crates/api/wrangler.toml`([vars] 加 ADMIN_EMAIL)

**Interfaces:**
- Produces: `invites` 表欄位(code, label, max_uses, used_count, expires_at,
  revoked_at, created_by, created_at);`login_tokens.pending_invite_code`;
  `users.invited_by`;`ADMIN_EMAIL` var。

- [ ] **Step 1: schema.sql 加表與欄位**

在 `login_tokens` 表之後插入:

```sql
-- Beta invite links (spec: docs/superpowers/specs/2026-08-30-invite-links-design.md).
-- expires_at / revoked_at are app-written ISO strings; compare ISO to ISO only.
CREATE TABLE IF NOT EXISTS invites (
    code TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    max_uses INTEGER NOT NULL,
    used_count INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT,
    revoked_at TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);

-- One-time migrations for EXISTING databases (SQLite has no ADD COLUMN IF NOT
-- EXISTS; run each once via `wrangler d1 execute --command`). Fresh installs
-- get these from the CREATE TABLE statements above/below.
--   ALTER TABLE login_tokens ADD COLUMN pending_invite_code TEXT;
--   ALTER TABLE users ADD COLUMN invited_by TEXT;
```

同時直接修改兩處 CREATE TABLE 定義(全新環境一次到位):
- `login_tokens` 表加一行 `pending_invite_code TEXT,`(放在 `pending_full_name` 後)
- `users` 表加一行 `invited_by TEXT,`(放在 `birth_data_hash` 後)

- [ ] **Step 2: wrangler.toml 加 ADMIN_EMAIL**

`[vars]` 區段加:

```toml
# Invite-system admin (spec 2026-08-30). Session email matching this value
# (case-insensitive) may manage invites; unset = nobody (fail-closed).
ADMIN_EMAIL = "yanggf@msn.com"
```

- [ ] **Step 3: prod 一次性遷移(主線程執行,非 agent)**

```bash
unset CLOUDFLARE_API_TOKEN
wrangler d1 execute fortunet-db --remote --command "ALTER TABLE login_tokens ADD COLUMN pending_invite_code TEXT"
wrangler d1 execute fortunet-db --remote --command "ALTER TABLE users ADD COLUMN invited_by TEXT"
```

Expected: 各回 "success"(重跑報 duplicate column 屬預期,不再執行)

- [ ] **Step 4: Commit**

```bash
git add scripts/schema.sql crates/api/wrangler.toml
git commit -m "feat(api): invites table + invite columns + ADMIN_EMAIL var"
```

---

### Task 3: register 收邀請碼 + 攜碼(兩階段上半)

**Files:**
- Modify: `crates/api/src/routes/auth.rs`(RegisterBody、register handler、issue_login_link、verify 建帳號段)

**Interfaces:**
- Consumes: Task 1 `invite::{INVITE_REQUIRED, is_usable, InviteRow}`、Task 2 欄位
- Produces: register body `{"email", "full_name"?, "invite"?}`;無效/缺碼 →
  400 `"邀請碼無效或已失效"` / `"邀請碼必填"`;`login_tokens` INSERT 帶
  `pending_invite_code`;verify 建帳號時消費 + `users.invited_by`(Task 4 補)。
  本 task 先讓 verify 對「無 pending_invite_code 的建帳號請求」回 409 `"缺少邀請碼"`
  (fail-closed),Task 4 換成真正消費。

- [ ] **Step 1: RegisterBody 加欄位 + register handler 驗證**

```rust
#[derive(serde::Deserialize)]
struct RegisterBody {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    full_name: Option<String>,
    #[serde(default)]
    invite: Option<String>,
}
```

register handler 在 `rate_limited(...)` 之後、`issue_login_link(...)` 之前:

```rust
let invite_code = if crate::services::invite::INVITE_REQUIRED {
    match body.invite.as_deref() {
        Some(c) if !c.trim().is_empty() && c.len() <= 16 => Some(c.trim().to_ascii_uppercase()),
        _ => return Ok(error::error("邀請碼必填", 400)),
    }
} else {
    body.invite.as_deref().map(|c| c.trim().to_ascii_uppercase())
};
```

- [ ] **Step 2: issue_login_link 簽名加 invite、SELECT 驗證、INSERT 攜碼**

簽名改為 `async fn issue_login_link(ctx, email_addr, new_full_name, invite_code: Option<String>)`。
login handler 傳 `None`,register handler 傳 `invite_code`。

`exists` SELECT 之後加入(db 取用處同段):

```rust
// Two-phase invite: validate now, consume at verify (a register that never
// verifies must not burn a slot). Fail loud on invalid codes - invites are
// not accounts, the uniform-202 anti-enumeration does not apply here.
let invite_code = match invite_code {
    Some(code) => {
        let c = db::text(&code);
        let row: Option<invite::InviteRow> = match db::first(
            &db,
            "SELECT used_count, max_uses, expires_at, revoked_at FROM invites WHERE code = ?1",
            &[&c],
        )
        .await
        {
            Ok(r) => r,
            Err(_) => return Ok(error::error("db error", 500)),
        };
        let now = clock::now_iso();
        match row {
            Some(r) if invite::is_usable(&r, &now) => Some(code),
            _ => return Ok(error::error("邀請碼無效或已失效", 400)),
        }
    }
    None => None,
};
```

INSERT 語句改為:

```rust
"INSERT INTO login_tokens (token_hash, email, expires_at, pending_full_name, pending_invite_code) VALUES (?1, ?2, ?3, ?4, ?5)"
```

繫結加 `let ic = db::opt_text(invite_code.as_deref());` → `&[&h, &em, &exp, &pending, &ic]`。

- [ ] **Step 3: verify 建帳號暫時 fail-closed**

`Ok(None) => match row.pending_full_name.as_deref()` 分支前,於 `Ok(None)` 進入點加:

```rust
// Task 4 replaces this with real consumption; until then a register verify
// without a stamped invite must not create an account (fail-closed).
```

並在建立 user 之前:

```rust
if row.pending_invite_code.as_deref().is_none() {
    return Ok(error::error("缺少邀請碼", 409));
}
```

(TokenRow 加 `pending_invite_code: Option<String>`,SELECT 兩句同步加欄位。)

- [ ] **Step 4: 驗證**

Run: `timeout 120 cargo fmt --all && timeout 300 cargo test --locked -p ft-api --lib && timeout 500 cargo check -p ft-api --target wasm32-unknown-unknown`
Expected: PASS / Finished

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/routes/auth.rs
git commit -m "feat(api): register validates invite codes and carries them to verify"
```

---

### Task 4: verify 建帳號時原子消費邀請

**Files:**
- Modify: `crates/api/src/routes/auth.rs`(verify handler 建帳號分支)

**Interfaces:**
- Consumes: Task 3 的 `pending_invite_code`(TokenRow 已帶)
- Produces: 建帳號路徑語意 — 邀請被搶光 → 409 `"邀請名額已用完"`;
  `users.invited_by` 落碼;無碼建帳號 → 409 `"缺少邀請碼"`(fail-closed)

- [ ] **Step 1: 換掉 Task 3 的暫時檢查**

`Ok(None) => match row.pending_full_name.as_deref()` 建帳號分支內、INSERT users 之前:

```rust
// Consume the invite atomically - the register-time check can go stale
// while the user sat on the email link.
let invite_code = match row.pending_invite_code.as_deref() {
    Some(c) if crate::services::invite::INVITE_REQUIRED => c.to_string(),
    Some(c) => c.to_string(),
    None if crate::services::invite::INVITE_REQUIRED => {
        return Ok(error::error("缺少邀請碼", 409));
    }
    None => String::new(),
};
let consumed_invite = !invite_code.is_empty();
if consumed_invite {
    let now = clock::now_iso();
    let c = db::text(&invite_code);
    let n = db::text(&now);
    let stmt = match db
        .prepare(
            "UPDATE invites SET used_count = used_count + 1 \
             WHERE code = ?1 AND used_count < max_uses \
             AND (expires_at IS NULL OR expires_at > ?2) AND revoked_at IS NULL",
        )
        .bind_refs([&c, &n].into_iter())
    {
        Ok(s) => s,
        Err(_) => return Ok(error::error("db error", 500)),
    };
    let consumed = match stmt.run().await {
        Ok(r) => r.meta().ok().flatten().and_then(|m| m.changes).unwrap_or(0),
        Err(_) => return Ok(error::error("db error", 500)),
    };
    if consumed == 0 {
        return Ok(error::error("邀請名額已用完", 409));
    }
}
```

INSERT users 語句加 `invited_by`:

```rust
"INSERT INTO users (id, email, full_name, trial_ends_at, invited_by) VALUES (?1, ?2, ?3, ?4, ?5)"
```

繫結加 `let ib = db::opt_text(if consumed_invite { Some(invite_code.as_str()) } else { None });`
→ `&[&uid, &em, &name, &trial, &ib]`。

- [ ] **Step 2: 驗證 + Commit**

Run: `timeout 120 cargo fmt --all && timeout 300 cargo test --locked -p ft-api --lib && timeout 500 cargo check -p ft-api --target wasm32-unknown-unknown`

```bash
git add crates/api/src/routes/auth.rs
git commit -m "feat(api): consume invite atomically at account creation, stamp users.invited_by"
```

---

### Task 5: admin API(建/列/撤 + guard)與公開預檢

**Files:**
- Create: `crates/api/src/routes/admin_invites.rs`
- Modify: `crates/api/src/routes/mod.rs`(wire)、`crates/api/src/routes/users.rs`(isAdmin)
- Test: `crates/api/src/routes/admin_invites.rs` 內 guard 的單元測試(純比對邏輯抽函式)

**Interfaces:**
- Produces:
  - `GET /api/invites/:code` → `{"valid": bool, "label": string|null}`
  - `POST /api/admin/invites` `{label?, maxUses?, expiresAt?}` → `{code, url, label, maxUses, expiresAt}`
  - `GET /api/admin/invites` → `{"invites": [...]}`
  - `POST /api/admin/invites/:code/revoke` → `{"ok": true}`
  - `/api/users/me` 回應加 `"isAdmin": bool`
  - `fn is_admin_email(admin_var: &str, session_email: &str) -> bool`(純函式,測這個)

- [ ] **Step 1: 先寫 guard 純函式測試**

```rust
#[cfg(test)]
mod tests {
    use super::is_admin_email;

    #[test]
    fn matches_case_insensitively() {
        assert!(is_admin_email("Yanggf@MSN.com", "yanggf@msn.com"));
    }

    #[test]
    fn empty_admin_var_is_nobody() {
        assert!(!is_admin_email("", "yanggf@msn.com"));
    }

    #[test]
    fn non_matching_email_is_rejected() {
        assert!(!is_admin_email("yanggf@msn.com", "attacker@example.com"));
    }
}
```

- [ ] **Step 2: 實作**

```rust
//! Invite admin routes (spec 2026-08-30). ADMIN_EMAIL var decides who may
//! manage; unset means nobody (fail-closed).

use worker::*;

use super::super::error;
use super::super::services::{clock, db, invite};
use super::common::{auth_user, ok_json};
use super::R;

pub fn is_admin_email(admin_var: &str, session_email: &str) -> bool {
    !admin_var.is_empty() && admin_var.eq_ignore_ascii_case(session_email)
}

async fn require_admin(ctx: &RouteContext<()>, req: &Request) -> Result<String, Response> {
    let user = auth_user(req, ctx).await?;
    let admin = ctx
        .env
        .var("ADMIN_EMAIL")
        .map(|v| v.to_string())
        .unwrap_or_default();
    if !is_admin_email(&admin, &user) {
        return Err(error::error("Forbidden", 403));
    }
    Ok(user)
}

pub fn register(router: R<'static>) -> R<'static> {
    router
        .get_async("/api/invites/:code", |req, ctx| async move {
            let code = ctx.param("code").unwrap_or_default();
            if code.is_empty() || code.len() > 16 {
                return Ok(ok_json(&serde_json::json!({"valid": false, "label": null}), 200));
            }
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let c = db::text(&code);
            let row: Option<invite::InviteRow> = match db::first(
                &db,
                "SELECT used_count, max_uses, expires_at, revoked_at FROM invites WHERE code = ?1",
                &[&c],
            )
            .await
            {
                Ok(r) => r,
                Err(_) => return Ok(error::error("db error", 500)),
            };
            match row {
                Some(r) if invite::is_usable(&r, &clock::now_iso()) => {
                    let l: Option<LabelRow> = db::first(
                        &db,
                        "SELECT label FROM invites WHERE code = ?1",
                        &[&c],
                    )
                    .await
                    .ok()
                    .flatten();
                    Ok(ok_json(
                        &serde_json::json!({
                            "valid": true,
                            "label": l.map(|x| x.label),
                        }),
                        200,
                    ))
                }
                _ => Ok(ok_json(&serde_json::json!({"valid": false, "label": null}), 200)),
            }
        })
        .post_async("/api/admin/invites", |mut req, ctx| async move {
            if let Err(r) = require_admin(&ctx, &req).await {
                return Ok(r);
            }
            let body: CreateBody = match req.json().await {
                Ok(b) => b,
                Err(_) => return Ok(error::error("Invalid JSON", 400)),
            };
            let label = body
                .label
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "邀請".into());
            let max_uses = body.max_uses.unwrap_or(20).clamp(1, 500);
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let admin_email = ctx
                .env
                .var("ADMIN_EMAIL")
                .map(|v| v.to_string())
                .unwrap_or_default();
            // Mint + insert; PK collision retry once (probability ~0).
            for _ in 0..2 {
                let code = match invite::new_code() {
                    Some(c) => c,
                    None => return Ok(error::error("unable to create invite", 500)),
                };
                let c = db::text(&code);
                let l = db::text(&label);
                let mu = db::int(max_uses as i32);
                let by = db::text(&admin_email);
                match db::exec(
                    &db,
                    "INSERT INTO invites (code, label, max_uses, created_by) VALUES (?1, ?2, ?3, ?4)",
                    &[&c, &l, &mu, &by],
                )
                .await
                {
                    Ok(_) => {
                        let origin = ctx
                            .env
                            .var("WEB_ORIGIN")
                            .map(|v| v.to_string())
                            .ok()
                            .filter(|v| !v.is_empty())
                            .unwrap_or_else(|| "https://fortunet.pages.dev".into());
                        return Ok(ok_json(
                            &serde_json::json!({
                                "code": code,
                                "url": format!("{}/register?invite={}", origin, code),
                                "label": label,
                                "maxUses": max_uses,
                                "expiresAt": body.expires_at,
                            }),
                            201,
                        ));
                    }
                    Err(_) => continue, // PK collision: mint another
                }
            }
            Ok(error::error("unable to create invite", 500))
        })
        .get_async("/api/admin/invites", |req, ctx| async move {
            if let Err(r) = require_admin(&ctx, &req).await {
                return Ok(r);
            }
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let rows: Vec<AdminInviteRow> = match db::all(
                &db,
                "SELECT code, label, max_uses, used_count, expires_at, revoked_at, created_at \
                 FROM invites ORDER BY created_at DESC",
                &[],
            )
            .await
            {
                Ok(v) => v,
                Err(_) => return Ok(error::error("db error", 500)),
            };
            let invites: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "code": r.code,
                        "label": r.label,
                        "maxUses": r.max_uses,
                        "usedCount": r.used_count,
                        "expiresAt": r.expires_at,
                        "revokedAt": r.revoked_at,
                        "createdAt": r.created_at,
                    })
                })
                .collect();
            Ok(ok_json(&serde_json::json!({ "invites": invites }), 200))
        })
        .post_async("/api/admin/invites/:code/revoke", |req, ctx| async move {
            if let Err(r) = require_admin(&ctx, &req).await {
                return Ok(r);
            }
            let code = ctx.param("code").unwrap_or_default();
            let db = match ctx.env.d1("DB") {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let c = db::text(&code);
            match db::exec(
                &db,
                "UPDATE invites SET revoked_at = datetime('now') WHERE code = ?1 AND revoked_at IS NULL",
                &[&c],
            )
            .await
            {
                Ok(_) => Ok(ok_json(&serde_json::json!({ "ok": true }), 200)),
                Err(_) => Ok(error::error("db error", 500)),
            }
        })
}

#[derive(serde::Deserialize)]
struct CreateBody {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    max_uses: Option<i64>,
    #[serde(default)]
    expires_at: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct LabelRow {
    label: String,
}

#[derive(Debug, serde::Deserialize)]
struct AdminInviteRow {
    code: String,
    label: String,
    max_uses: i64,
    used_count: i64,
    expires_at: Option<String>,
    revoked_at: Option<String>,
    created_at: Option<String>,
}
```

(上面 `db::int` 若簽名不合,以 `db.rs` 實際簽名為準微調,不改語意。)

- [ ] **Step 3: users/me 加 isAdmin**

`users.rs` 的 `GET /api/users/me` 回應 json! 加:

```rust
"isAdmin": {
    let admin = ctx.env.var("ADMIN_EMAIL").map(|v| v.to_string()).unwrap_or_default();
    crate::routes::admin_invites::is_admin_email(&admin, &user)
}
```

(在 json! 巨集外先算好 `let is_admin = ...;` 再放進去,避免巨集內語句。)

- [ ] **Step 4: wire + 驗證 + Commit**

`routes/mod.rs`:`mod admin_invites;` + `let ai = admin_invites::register(p);` 鏈尾。

Run: `timeout 120 cargo fmt --all && timeout 300 cargo test --locked -p ft-api --lib && timeout 500 cargo check -p ft-api --target wasm32-unknown-unknown`

```bash
git add crates/api/src/routes/admin_invites.rs crates/api/src/routes/mod.rs crates/api/src/routes/users.rs
git commit -m "feat(api): invite admin routes, public precheck, users/me isAdmin"
```

---

### Task 6: 前端 — 登入頁邀請碼欄位 + 預填預檢

**Files:**
- Modify: `crates/web/src/api.rs`(check_invite + RegisterRequest 加 invite)
- Modify: `crates/web/src/pages/login.rs`(註冊 tab 欄位、?invite= 預填、預檢顯示)

**Interfaces:**
- Consumes: Task 5 `GET /api/invites/:code`
- Produces: register 請求帶 `"invite": code`;頁面顯示「✓ 已套用邀請:{label}」/「✗ 邀請碼無效」

- [ ] **Step 1: api.rs**

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InviteCheck {
    pub valid: bool,
    pub label: Option<String>,
}

pub async fn check_invite(code: &str) -> Result<InviteCheck, ApiErr> {
    let path = format!("/api/invites/{}", code);
    // 沿用既有 request helper;404/非 2xx 視為 invalid 而非錯誤
    match get::<InviteCheck>(&path).await {
        Ok(v) => Ok(v),
        Err(_) => Ok(InviteCheck { valid: false, label: None }),
    }
}
```

(`get::<T>` 對應既有 helper 名稱;RegisterRequest / register 函式加 `invite: Option<String>`
欄位並隨 body 送出。)

- [ ] **Step 2: login.rs 註冊 tab**

- signal `invite_code: RwSignal<String>`;掛載時讀
  `window().location().search()` 解析 `invite=` 預填(照 VerifyPage 既有解析樣式)。
- 預填或使用者輸入後(trigger:輸入框 on:blur 或送出前)呼叫 `check_invite`,
  顯示:`✓ 已套用邀請:{label}`(綠)/ `✗ 邀請碼無效或已失效`(紅)。
- register 送出:INVITE 必填(前端先擋空值,提示「請填邀請碼」),隨 body 送
  `invite`;後端 400 時把 `error` 訊息顯示在表單上方(紅字,既有錯誤顯示樣式)。

- [ ] **Step 3: 驗證 + Commit**

Run: `timeout 500 cargo check -p ft-web --target wasm32-unknown-unknown && timeout 120 cargo fmt --all`

```bash
git add crates/web/src/api.rs crates/web/src/pages/login.rs
git commit -m "feat(web): invite code field with prefill and precheck on register"
```

---

### Task 7: 前端 — /admin 管理頁

**Files:**
- Create: `crates/web/src/pages/admin.rs`
- Modify: `crates/web/src/pages/mod.rs`、`crates/web/src/lib.rs`(/admin 路由,Protected 內)、
  `crates/web/src/api.rs`(admin CRUD)

**Interfaces:**
- Consumes: Task 5 admin API 三端點、`/api/users/me` 的 `isAdmin`
- Produces: `/admin` 頁 — 建連結表單(備註/人數/過期日)、列表、複製連結、撤銷

- [ ] **Step 1: api.rs admin 函式**

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdminInvite {
    #[serde(rename = "code")]
    pub code: String,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "maxUses")]
    pub max_uses: i64,
    #[serde(rename = "usedCount")]
    pub used_count: i64,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
    #[serde(rename = "revokedAt")]
    pub revoked_at: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}

pub async fn list_invites() -> Result<Vec<AdminInvite>, ApiErr> { /* GET /api/admin/invites */ }
pub async fn create_invite(label: String, max_uses: i64, expires_at: Option<String>) -> Result<InviteCreated, ApiErr> { /* POST */ }
pub async fn revoke_invite(code: &str) -> Result<(), ApiErr> { /* POST .../revoke */ }
```

`InviteCreated { code, url, label, maxUses, expiresAt }`(serde rename)。

- [ ] **Step 2: admin.rs 頁面**

- 掛載時 `list_invites()`;`isAdmin=false` 的回應(403)→ 顯示「無權限」。
- 表單:備註(text)、人數(number, 預設 20)、過期日(date,可空)→ 建立後把
  回傳的 `url` 放到醒目區塊 + 「複製」按鈕
  (`navigator.clipboard().write_text(&url)`),列表自動重整。
- 列表:碼、備註、`used/max`、過期日、狀態(有效/已撤銷/已過期 — 前端以
  revokedAt/expiresAt 判斷顯示字串)、「撤銷」(confirm 後呼叫 revoke、重整)。
- 版面沿用既有頁面的樣式 class(照 personality/login 頁的 CSS 慣例)。

- [ ] **Step 3: 路由**

`lib.rs` Protected 內加 `/admin` → `pages::admin::AdminPage`;`pages/mod.rs` 註冊模組。
`/api/users/me` 的 `isAdmin` 加進既有 MeResponse 型別與 Layout/Home 的入口連結
(僅 is_admin 顯示「邀請管理」連結)。

- [ ] **Step 4: 驗證 + Commit**

Run: `timeout 500 cargo check -p ft-web --target wasm32-unknown-unknown && timeout 120 cargo fmt --all`

```bash
git add crates/web/src
git commit -m "feat(web): /admin invite management page (create, list, copy, revoke)"
```

---

### Task 8: 部署 + 生產 E2E(主線程執行,非 agent)

- [ ] **Step 1: 全綠門檻**

`timeout 120 cargo fmt --all --check && timeout 400 cargo test --locked -p ft-schema -p ft-ziwei -p ft-western -p ft-big5 -p ft-api && 三個 wasm check`

- [ ] **Step 2: D1 一次性遷移(Task 2 Step 3 的兩句 ALTER)**

- [ ] **Step 3: API + 前端同時部署**(`worker-build --release && wrangler deploy`;`deploy-web.sh`)

- [ ] **Step 4: E2E(照 spec)**

1. `wrangler d1 execute --remote --json --command "SELECT ..."` 確認表/欄位
2. curl `POST /api/admin/invites`(帶 session)建連結;403 驗非 admin
3. curl `GET /api/invites/:code` 預檢 valid
4. 無效碼 register → 400「邀請碼無效或已失效」;正確碼 → 202
5. 用户點信 → verify → `users.invited_by` 落碼、`used_count` +1
6. 撤銷後同一碼 register → 400

- [ ] **Step 5: Commit(如有殘餘)+ push 由用戶把關**
