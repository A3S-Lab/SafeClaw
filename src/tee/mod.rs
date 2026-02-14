//! TEE (Trusted Execution Environment) integration
//!
//! Provides integration with A3S Box for running sensitive
//! computations in hardware-isolated environments.
//!
//! - `client` — Frame-based TEE client (uses `Transport` trait)
//! - `channel` — RA-TLS communication channel to the TEE guest (requires `real-tee` feature)
//! - `orchestrator` — MicroVM lifecycle and RA-TLS communication (requires `real-tee` feature)
//! - `protocol` — Shared protocol types re-exported from `a3s-transport`

mod client;
#[cfg(feature = "real-tee")]
pub mod channel;
#[cfg(feature = "real-tee")]
pub mod orchestrator;
mod protocol;
mod stub;

#[cfg(feature = "real-tee")]
pub use channel::RaTlsChannel;
pub use client::TeeClient;
#[cfg(feature = "real-tee")]
pub use orchestrator::TeeOrchestrator;
pub use protocol::{TeeMessage, TeeRequest, TeeResponse};

// When real-tee is disabled, export stub types so the rest of the crate compiles.
#[cfg(not(feature = "real-tee"))]
pub use stub::TeeOrchestrator;
