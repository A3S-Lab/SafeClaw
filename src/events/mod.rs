//! Events module — event routing and subscription management
//!
//! Provides REST endpoints for listing, creating, and filtering events,
//! plus per-persona subscription configuration. Events are persisted as
//! JSON files under `~/.safeclaw/events/`.

pub mod handler;
pub mod store;
pub mod types;

pub use handler::{events_router, EventsState};
pub use store::EventStore;
