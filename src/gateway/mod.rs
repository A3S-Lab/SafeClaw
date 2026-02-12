//! Gateway server for SafeClaw
//!
//! Provides WebSocket control plane, HTTP API, and a3s-gateway integration
//! for managing the SafeClaw assistant.

mod handler;
pub mod integration;
mod server;
mod websocket;

pub use handler::ApiHandler;
pub use integration::{generate_gateway_config, generate_routing_config, GatewayRoutingConfig};
pub use server::{Gateway, GatewayBuilder, GatewayState, GatewayStatus, ProcessedResponse};
pub use websocket::{WebSocketHandler, WsMessage};
