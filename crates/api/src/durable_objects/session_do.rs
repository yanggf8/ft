//! Session Durable Object — mirrors backend/src/durable-objects/session-do.ts.
//! Stores the session under the single storage key `"session"` via the JS-compatible
//! `serde_wasm_bindgen` contract (see `ft_schema::storage::SessionDoRecord`), so the
//! in-place Stage B deploy over `fortunet-api` keeps existing sessions valid.

use ft_schema::storage::{SessionDoRecord, SESSION_KEY};
use worker::*;

use crate::services::clock;

/// A-02 session revocation epoch: `create` additionally stores the creation
/// instant as an ISO string under this key. `get`/`refresh` reject any session
/// without it — every session minted before the magic-link deploy is revoked
/// at once. A separate key keeps the JS-compat `SessionDoRecord` shape under
/// `SESSION_KEY` bit-identical (additive-only storage change).
const CREATED_AT_ISO_KEY: &str = "createdAtIso";

#[durable_object(fetch)]
pub struct SessionDO {
    state: State,
}

impl DurableObject for SessionDO {
    fn new(state: State, _env: Env) -> Self {
        Self { state }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let path = req.path();
        match path.as_str() {
            "/create" => self.create(&mut req).await,
            "/get" => self.get().await,
            "/refresh" => self.refresh().await,
            "/destroy" => self.destroy().await,
            _ => Response::error("Not found", 404),
        }
    }
}

impl SessionDO {
    async fn create(&self, req: &mut Request) -> Result<Response> {
        let body: CreateBody = req.json().await.map_err(|_| Error::from("invalid body"))?;
        let now = clock::now_ms();
        let record = SessionDoRecord {
            userId: body.userId,
            email: body.email,
            createdAt: now,
            expiresAt: now + 7.0 * 24.0 * 60.0 * 60.0 * 1000.0,
        };
        self.state.storage().put(SESSION_KEY, &record).await?;
        // A-02: ISO creation stamp alongside userId/email — its absence later
        // marks the session as pre-fix and revoked.
        let created_iso = clock::now_iso();
        self.state
            .storage()
            .put(CREATED_AT_ISO_KEY, &created_iso)
            .await?;
        // The TS returned `Response.json(session)`; mirror the fields.
        Response::from_json(&record_serde_json(&record, Some(&created_iso)))
    }

    async fn get(&self) -> Result<Response> {
        let rec = self
            .state
            .storage()
            .get::<SessionDoRecord>(SESSION_KEY)
            .await?;
        let Some(rec) = rec else {
            return Response::from_json(&serde_json::json!({ "error": "No session" }))
                .map(|r| r.with_status(401));
        };
        if rec.expiresAt < clock::now_ms() {
            self.delete_all().await;
            return Response::from_json(&serde_json::json!({ "error": "Session expired" }))
                .map(|r| r.with_status(401));
        }
        // A-02 revocation epoch: no ISO createdAt = pre-fix session -> invalid.
        let created: Option<String> = self.state.storage().get(CREATED_AT_ISO_KEY).await?;
        let Some(created_iso) = created.filter(|s| !s.is_empty()) else {
            self.delete_all().await;
            return Response::from_json(&serde_json::json!({ "error": "Session expired" }))
                .map(|r| r.with_status(401));
        };
        Response::from_json(&record_serde_json(&rec, Some(&created_iso)))
    }

    async fn refresh(&self) -> Result<Response> {
        let rec = self
            .state
            .storage()
            .get::<SessionDoRecord>(SESSION_KEY)
            .await?;
        let Some(mut rec) = rec else {
            return Response::from_json(&serde_json::json!({ "error": "Invalid session" }))
                .map(|r| r.with_status(401));
        };
        if rec.expiresAt < clock::now_ms() {
            return Response::from_json(&serde_json::json!({ "error": "Invalid session" }))
                .map(|r| r.with_status(401));
        }
        // Same epoch check as `get` — a pre-fix session cannot extend itself.
        let created: Option<String> = self.state.storage().get(CREATED_AT_ISO_KEY).await?;
        let Some(created_iso) = created.filter(|s| !s.is_empty()) else {
            self.delete_all().await;
            return Response::from_json(&serde_json::json!({ "error": "Invalid session" }))
                .map(|r| r.with_status(401));
        };
        rec.expiresAt = clock::now_ms() + 7.0 * 24.0 * 60.0 * 60.0 * 1000.0;
        self.state.storage().put(SESSION_KEY, &rec).await?;
        Response::from_json(&record_serde_json(&rec, Some(&created_iso)))
    }

    async fn destroy(&self) -> Result<Response> {
        self.delete_all().await;
        Response::from_json(&serde_json::json!({ "success": true }))
    }
}

impl SessionDO {
    /// Remove both the record and its epoch marker (keep the keys in sync).
    async fn delete_all(&self) {
        let _ = self.state.storage().delete(SESSION_KEY).await;
        let _ = self.state.storage().delete(CREATED_AT_ISO_KEY).await;
    }
}

#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct CreateBody {
    userId: String,
    email: String,
}

/// Serialize a SessionDoRecord to camelCase JSON (the TS returned the same field names).
/// `createdAt` carries the ISO epoch stamp when one exists (post-fix sessions);
/// otherwise it falls back to the stored numeric value (storage format unchanged).
fn record_serde_json(rec: &SessionDoRecord, created_at_iso: Option<&str>) -> serde_json::Value {
    let created = match created_at_iso {
        Some(iso) => serde_json::json!(iso),
        None => serde_json::json!(rec.createdAt),
    };
    serde_json::json!({
        "userId": rec.userId,
        "email": rec.email,
        "createdAt": created,
        "expiresAt": rec.expiresAt,
    })
}
