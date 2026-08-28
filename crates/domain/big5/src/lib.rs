//! ft-big5 — Big5 personality domain logic (F1 slice).
//! 純計算、無 IO；native 與 wasm32 皆可編。
//! Spec: docs/superpowers/specs/2026-08-28-big5-f1-design.md

pub mod careless;
pub mod norm;
pub mod scoring;

pub use careless::{any_triggered, detect_careless, CarelessFlags};
pub use scoring::{score, validate, ValidationErr};
