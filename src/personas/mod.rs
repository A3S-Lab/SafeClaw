//! Personas module — agent identity management
//!
//! Provides REST endpoints for listing, creating, and updating agent personas.
//! Ships with builtin personas; users can create custom ones.
//! Also includes the user profile endpoint.

pub mod handler;
pub mod store;
pub mod types;

pub use handler::{personas_router, PersonasState};
pub use store::PersonaStore;
