//! Stub TEE orchestrator for builds without `real-tee` feature.
//!
//! Provides the same public API as the real `TeeOrchestrator` but all
//! operations return errors. This allows the rest of the crate (especially
//! `SessionManager`) to compile and run tests without `libkrun`.

use crate::config::TeeConfig;
use crate::error::{Error, Result};
use std::path::PathBuf;

/// Stub TEE orchestrator — all operations return errors.
pub struct TeeOrchestrator {
    config: TeeConfig,
    attest_socket_path: PathBuf,
}

impl std::fmt::Debug for TeeOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeeOrchestrator")
            .field("config", &self.config)
            .field("stub", &true)
            .finish_non_exhaustive()
    }
}

impl TeeOrchestrator {
    /// Create a new stub TEE orchestrator.
    pub fn new(config: TeeConfig) -> Self {
        let socket_dir = config
            .socket_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("safeclaw"));
        let attest_socket_path = socket_dir.join("attest.sock");

        Self {
            config,
            attest_socket_path,
        }
    }

    pub async fn boot(&self) -> Result<()> {
        Err(Error::Tee(
            "TEE not available: build with `real-tee` feature to enable MicroVM support"
                .to_string(),
        ))
    }

    pub async fn verify(&self) -> Result<()> {
        Err(Error::Tee(
            "TEE not available: build with `real-tee` feature".to_string(),
        ))
    }

    pub async fn inject_secrets(
        &self,
        _secret_refs: &[crate::config::SecretRef],
    ) -> Result<usize> {
        Err(Error::Tee(
            "TEE not available: build with `real-tee` feature".to_string(),
        ))
    }

    pub async fn process_message(
        &self,
        _session_id: &str,
        _content: &str,
    ) -> Result<StubProcessResponse> {
        Err(Error::Tee(
            "TEE not available: build with `real-tee` feature".to_string(),
        ))
    }

    pub async fn is_ready(&self) -> bool {
        false
    }

    pub async fn is_booted(&self) -> bool {
        false
    }

    pub fn attest_socket_path(&self) -> &PathBuf {
        &self.attest_socket_path
    }

    pub fn allow_simulated(&self) -> bool {
        self.config.allow_simulated
    }

    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Stub process response matching the real `channel::ProcessResponse` shape.
#[derive(Debug, Clone)]
pub struct StubProcessResponse {
    pub session_id: String,
    pub content: String,
    pub success: bool,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_creation() {
        let config = TeeConfig::default();
        let orch = TeeOrchestrator::new(config);
        assert!(orch.attest_socket_path().to_string_lossy().contains("safeclaw"));
    }

    #[tokio::test]
    async fn test_stub_boot_fails() {
        let orch = TeeOrchestrator::new(TeeConfig::default());
        let result = orch.boot().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("real-tee"));
    }

    #[tokio::test]
    async fn test_stub_not_ready() {
        let orch = TeeOrchestrator::new(TeeConfig::default());
        assert!(!orch.is_ready().await);
        assert!(!orch.is_booted().await);
    }

    #[tokio::test]
    async fn test_stub_shutdown_ok() {
        let orch = TeeOrchestrator::new(TeeConfig::default());
        assert!(orch.shutdown().await.is_ok());
    }
}
