//! Service layer — pure logic / client code mirroring backend/src/services/.

pub mod clock;
pub mod uuid;

pub mod ai;
pub mod billing;
pub mod birth_hash;
pub mod db;
pub mod email;
pub mod engine;
pub mod engine_version;
pub mod generation;
pub mod invite;
pub mod login_token;
pub mod oauth;
