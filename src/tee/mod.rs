//! TEE (Trusted Execution Environment) integration
//!
//! Provides integration with A3S Box for running sensitive
//! computations in hardware-isolated environments.

mod client;
mod protocol;

pub use client::TeeClient;
pub use protocol::{TeeMessage, TeeRequest, TeeResponse};
