//! Settings module — application settings management
//!
//! Provides REST endpoints for reading, updating, and resetting SafeClaw
//! configuration. API keys are masked in responses.

pub mod handler;
pub mod types;

pub use handler::{settings_router, SettingsState};
