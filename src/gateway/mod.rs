//! Gateway server for SafeClaw
//!
//! Provides WebSocket control plane, HTTP API, and a3s-gateway integration
//! for managing the SafeClaw assistant.

mod handler;
pub mod integration;
mod server;
mod websocket;

pub use handler::ApiHandler;
pub use integration::{build_service_descriptor, ServiceDescriptor};
pub use server::{Gateway, GatewayBuilder, GatewayState, GatewayStatus, ProcessedResponse};
pub use websocket::{WebSocketHandler, WsMessage};
