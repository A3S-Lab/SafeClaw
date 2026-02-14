//! TEE (Trusted Execution Environment) integration
//!
//! Provides integration with A3S Box for running sensitive
//! computations in hardware-isolated environments.
//!
//! - `client` — Frame-based TEE client (uses `Transport` trait)
//! - `channel` — RA-TLS communication channel to the TEE guest
//! - `orchestrator` — MicroVM lifecycle and RA-TLS communication
//! - `protocol` — Shared protocol types re-exported from `a3s-transport`

mod client;
pub mod channel;
pub mod orchestrator;
mod protocol;

pub use channel::RaTlsChannel;
pub use client::TeeClient;
pub use orchestrator::TeeOrchestrator;
pub use protocol::{TeeMessage, TeeRequest, TeeResponse};
