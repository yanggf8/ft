//! AI provider layer — mirrors backend/src/services/ai/.

pub mod prompts;
pub mod providers;

pub use providers::{call_provider, ProviderResult};
