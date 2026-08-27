//! Durable Objects — mirrors backend/src/durable-objects/*.ts.
//! Same class names (`SessionDO`, `AIMutexDO`) and same storage keys as the JS
//! versions so the Stage B in-place deploy over `fortunet-api` keeps existing
//! session / AI-metric storage bit-compatible (see `ft_schema::storage`).

mod session_do;
mod ai_mutex_do;

pub use session_do::SessionDO;
pub use ai_mutex_do::AIMutexDO;
