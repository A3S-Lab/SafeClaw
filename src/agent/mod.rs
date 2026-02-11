//! Agent module — CLI Agent process management and WebSocket bridge
//!
//! This module integrates Claude Code CLI agent management into SafeClaw,
//! providing process lifecycle management, WebSocket message bridging,
//! and session persistence.
//!
//! ## Architecture
//!
//! ```text
//! Tauri UI (React) ←→ WS (JSON) ←→ SafeClaw Gateway (axum) ←→ WS (NDJSON) ←→ Claude Code CLI
//!                    /ws/agent/       agent module              /ws/agent/       (--sdk-url)
//!                    browser/:id      ├ launcher                cli/:id
//!                                     ├ bridge
//!                                     ├ session_store
//!                                     └ handler
//! ```

pub mod bridge;
pub mod handler;
pub mod launcher;
pub mod session_store;
pub mod types;

pub use handler::{agent_router, AgentState};
pub use launcher::AgentLauncher;
pub use bridge::AgentBridge;
pub use session_store::AgentSessionStore;
