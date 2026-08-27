//! Session Durable Object — mirrors backend/src/durable-objects/session-do.ts.
//! Stores the session under the single storage key `"session"` via the JS-compatible
//! `serde_wasm_bindgen` contract (see `ft_schema::storage::SessionDoRecord`), so the
//! in-place Stage B deploy over `fortunet-api` keeps existing sessions valid.

use ft_schema::storage::{SessionDoRecord, SESSION_KEY};
use worker::*;

use crate::services::clock;

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
        // The TS returned `Response.json(session)`; mirror the fields.
        Response::from_json(&record_serde_json(&record))
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
            self.state.storage().delete(SESSION_KEY).await?;
            return Response::from_json(&serde_json::json!({ "error": "Session expired" }))
                .map(|r| r.with_status(401));
        }
        Response::from_json(&record_serde_json(&rec))
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
        rec.expiresAt = clock::now_ms() + 7.0 * 24.0 * 60.0 * 60.0 * 1000.0;
        self.state.storage().put(SESSION_KEY, &rec).await?;
        Response::from_json(&record_serde_json(&rec))
    }

    async fn destroy(&self) -> Result<Response> {
        self.state.storage().delete(SESSION_KEY).await?;
        Response::from_json(&serde_json::json!({ "success": true }))
    }
}

#[allow(non_snake_case)]
#[derive(serde::Deserialize)]
struct CreateBody {
    userId: String,
    email: String,
}

/// Serialize a SessionDoRecord to camelCase JSON (the TS returned the same field names).
fn record_serde_json(rec: &SessionDoRecord) -> serde_json::Value {
    serde_json::json!({
        "userId": rec.userId,
        "email": rec.email,
        "createdAt": rec.createdAt,
        "expiresAt": rec.expiresAt,
    })
}
