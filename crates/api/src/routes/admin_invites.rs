//! Invite admin routes (spec 2026-08-30). The `ADMIN_EMAIL` var decides who
//! may manage invites; unset means nobody (fail-closed). The public endpoint
//! is a preflight check for the register page and leaks nothing but validity
//! plus the label of a still-valid code.

use worker::*;

use super::super::error;
use super::super::services::{clock, db, invite};
use super::common::{auth_user, ok_json};
use super::R;

/// Admin decision, pure for testing: unset var = nobody, else case-insensitive.
pub fn is_admin_email(admin_var: &str, session_email: &str) -> bool {
    if session_email.is_empty() {
        return false;
    }
    admin_var
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .any(|admin| admin.eq_ignore_ascii_case(session_email))
}

/// 401 without a session, 403 when the session is not the admin. `auth_user`
/// yields the userId, so the email must be resolved before the comparison.
async fn require_admin(ctx: &RouteContext<()>, req: &Request) -> Result<String, Response> {
    let user_id = auth_user(req, ctx).await?;
    let db = db::Turso::from_env(&ctx.env).map_err(|_| error::error("db unavailable", 500))?;
    let uid = db::text(&user_id);
    let row: Option<EmailRow> = db::first(&db, "SELECT email, role FROM users WHERE id = ?1", &[&uid])
        .await
        .map_err(|_| error::error("db error", 500))?;
    let (email, role) = match row {
        Some(r) => (r.email, r.role.unwrap_or_default()),
        None => return Err(error::error("Forbidden", 403)),
    };
    // hesocial-style role check + FT env-var allowlist (bootstrap for first admin)
    let is_role_admin = role == "admin" || role == "super_admin";
    if is_role_admin {
        return Ok(email);
    }
    let admin = ctx
        .env
        .var("ADMIN_EMAIL")
        .map(|v| v.to_string())
        .unwrap_or_default();
    if !is_admin_email(&admin, &email) {
        return Err(error::error("Forbidden", 403));
    }
    Ok(email)
}

#[derive(Debug, serde::Deserialize)]
struct EmailRow {
    email: String,
    #[serde(default)]
    role: Option<String>,
}

pub fn register(router: R<'static>) -> R<'static> {
    router
        // Public preflight for the register page: is this code usable, and
        // whose invite is it? Invalid codes return valid=false, not an error.
        .get_async("/api/invites/:code", |req, ctx| async move {
            let code = ctx.param("code").cloned().unwrap_or_default();
            if code.is_empty() || code.len() > 16 {
                return Ok(ok_json(
                    &serde_json::json!({ "valid": false, "label": null }),
                    200,
                ));
            }
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let c = db::text(&code);
            let row: Option<InviteRowWithLabel> = match db::first(
                &db,
                "SELECT used_count, max_uses, expires_at, revoked_at, label FROM invites \
                 WHERE code = ?1",
                &[&c],
            )
            .await
            {
                Ok(r) => r,
                Err(_) => return Ok(error::error("db error", 500)),
            };
            match row {
                Some(r) if invite::is_usable(&r.invite_row(), &clock::now_iso()) => Ok(ok_json(
                    &serde_json::json!({ "valid": true, "label": r.label }),
                    200,
                )),
                _ => Ok(ok_json(
                    &serde_json::json!({ "valid": false, "label": null }),
                    200,
                )),
            }
        })
        // Create a named invite link (admin).
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
            if let Some(e) = body.expires_at.as_deref() {
                if e.len() > 32 {
                    return Ok(error::error("Validation failed", 400));
                }
            }
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let admin_email = ctx
                .env
                .var("ADMIN_EMAIL")
                .map(|v| v.to_string())
                .unwrap_or_default();
            // Mint + insert; retry once on PK collision (probability ~0).
            for _ in 0..2 {
                let code = match invite::new_code() {
                    Some(c) => c,
                    None => return Ok(error::error("unable to create invite", 500)),
                };
                let c = db::text(&code);
                let l = db::text(&label);
                let mu = db::int(max_uses as i32);
                let by = db::text(&admin_email);
                if db::exec(
                    &db,
                    "INSERT INTO invites (code, label, max_uses, created_by) \
                     VALUES (?1, ?2, ?3, ?4)",
                    &[&c, &l, &mu, &by],
                )
                .await
                .is_ok()
                {
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
            }
            Ok(error::error("unable to create invite", 500))
        })
        // List all invites with usage (admin).
        .get_async("/api/admin/invites", |req, ctx| async move {
            if let Err(r) = require_admin(&ctx, &req).await {
                return Ok(r);
            }
            let db = match db::Turso::from_env(&ctx.env) {
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
        // Revoke (idempotent; revoking a revoked code is still ok=true).
        .post_async("/api/admin/invites/:code/revoke", |req, ctx| async move {
            if let Err(r) = require_admin(&ctx, &req).await {
                return Ok(r);
            }
            let code = ctx.param("code").cloned().unwrap_or_default();
            let db = match db::Turso::from_env(&ctx.env) {
                Ok(d) => d,
                Err(_) => return Ok(error::error("db unavailable", 500)),
            };
            let c = db::text(&code);
            match db::exec(
                &db,
                "UPDATE invites SET revoked_at = datetime('now') \
                 WHERE code = ?1 AND revoked_at IS NULL",
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

/// Flat row for the public preflight SELECT; splits into the pure predicate's
/// shape plus the label it may reveal.
#[derive(Debug, serde::Deserialize)]
struct InviteRowWithLabel {
    used_count: i64,
    max_uses: i64,
    expires_at: Option<String>,
    revoked_at: Option<String>,
    label: String,
}

impl InviteRowWithLabel {
    fn invite_row(&self) -> invite::InviteRow {
        invite::InviteRow {
            used_count: self.used_count,
            max_uses: self.max_uses,
            expires_at: self.expires_at.clone(),
            revoked_at: self.revoked_at.clone(),
        }
    }
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
