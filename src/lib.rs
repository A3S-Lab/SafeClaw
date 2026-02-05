//! SafeClaw - Secure Personal AI Assistant with TEE Support
//!
//! SafeClaw is a privacy-focused personal AI assistant that combines the
//! multi-channel capabilities of OpenClaw with the hardware-isolated
//! execution environment provided by A3S Box.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                        SafeClaw Gateway                              │
//! │  ┌─────────────────────────────────────────────────────────────┐   │
//! │  │                    Channel Manager                           │   │
//! │  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │   │
//! │  │  │ Telegram │ │  Slack   │ │ Discord  │ │   WebChat    │   │   │
//! │  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └──────┬───────┘   │   │
//! │  └───────┼────────────┼────────────┼──────────────┼───────────┘   │
//! │          └────────────┴────────────┴──────────────┘               │
//! │                              │                                     │
//! │  ┌───────────────────────────▼───────────────────────────────┐   │
//! │  │                   Session Router                           │   │
//! │  │  - Route messages to appropriate TEE sessions              │   │
//! │  │  - Handle multi-agent routing                              │   │
//! │  │  - Manage session lifecycle                                │   │
//! │  └───────────────────────────┬───────────────────────────────┘   │
//! │                              │                                     │
//! │  ┌───────────────────────────▼───────────────────────────────┐   │
//! │  │                   Privacy Classifier                       │   │
//! │  │  - Classify data sensitivity                               │   │
//! │  │  - Route sensitive data to TEE                             │   │
//! │  │  - Handle encryption/decryption                            │   │
//! │  └───────────────────────────┬───────────────────────────────┘   │
//! └──────────────────────────────┼────────────────────────────────────┘
//!                                │ vsock / encrypted channel
//! ┌──────────────────────────────▼────────────────────────────────────┐
//! │                    TEE Environment (A3S Box)                       │
//! │  ┌─────────────────────────────────────────────────────────────┐  │
//! │  │                    Secure Agent Runtime                      │  │
//! │  │  ┌─────────────────┐  ┌─────────────────────────────────┐   │  │
//! │  │  │  A3S Code Agent │  │     Secure Data Store           │   │  │
//! │  │  │  - LLM Client   │  │  - Encrypted credentials        │   │  │
//! │  │  │  - Tool Exec    │  │  - Private conversation history │   │  │
//! │  │  │  - HITL         │  │  - Sensitive user data          │   │  │
//! │  │  └─────────────────┘  └─────────────────────────────────┘   │  │
//! │  └─────────────────────────────────────────────────────────────┘  │
//! │                         MicroVM (Hardware Isolated)                │
//! └────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Features
//!
//! ### Multi-Channel Support
//! - Telegram, Slack, Discord, WebChat
//! - Extensible channel architecture
//! - Unified message routing
//!
//! ### TEE-Based Privacy Protection
//! - Sensitive data processing in hardware-isolated environment
//! - Encrypted communication between gateway and TEE
//! - Secure credential storage
//! - Private conversation history
//!
//! ### Privacy Classification
//! - Automatic sensitivity detection
//! - Configurable classification rules
//! - Data routing based on sensitivity level
//!
//! ## Modules
//!
//! - [`gateway`]: WebSocket control plane and HTTP API
//! - [`channels`]: Multi-channel message adapters
//! - [`session`]: Session management and routing
//! - [`privacy`]: Privacy classification and data protection
//! - [`tee`]: TEE environment integration with A3S Box
//! - [`crypto`]: Cryptographic utilities for secure communication
//! - [`config`]: Configuration management

pub mod channels;
pub mod config;
pub mod crypto;
pub mod error;
pub mod gateway;
pub mod privacy;
pub mod session;
pub mod tee;

pub use config::SafeClawConfig;
pub use error::{Error, Result};
