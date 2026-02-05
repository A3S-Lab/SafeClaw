//! Gateway server for SafeClaw
//!
//! Provides WebSocket control plane and HTTP API for managing
//! the SafeClaw assistant.

mod handler;
mod server;
mod websocket;

pub use handler::ApiHandler;
pub use server::{Gateway, GatewayBuilder};
pub use websocket::WebSocketHandler;
