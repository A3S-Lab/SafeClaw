//! TEE orchestrator — manages MicroVM lifecycle and RA-TLS communication.
//!
//! Coordinates the full TEE lifecycle:
//! 1. Boot A3S Box MicroVM via `VmController`
//! 2. Wait for the guest's RA-TLS attestation server
//! 3. Verify TEE attestation via RA-TLS handshake
//! 4. Inject secrets into the verified TEE
//! 5. Provide seal/unseal operations bound to TEE identity
//! 6. Graceful shutdown

use crate::config::{SecretRef, TeeConfig};
use crate::error::{Error, Result};

use a3s_box_runtime::vmm::{InstanceSpec, VmController, VmHandler};
use a3s_box_runtime::tee::AttestationPolicy;
use a3s_box_runtime::VmmProvider;
use a3s_box_runtime::{
    RaTlsAttestationClient, SealClient, SealResult, SecretEntry, SecretInjector,
    VerificationResult, PlatformInfo, PolicyResult,
};

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;

/// TEE orchestrator — central coordinator for MicroVM lifecycle and RA-TLS communication.
///
/// Manages a single A3S Box MicroVM that serves all TEE sessions. The VM is booted
/// lazily on first `boot()` call (triggered by `upgrade_to_tee()`).
pub struct TeeOrchestrator {
    config: TeeConfig,
    /// Running VM handler (None if not booted)
    vm: RwLock<Option<Box<dyn VmHandler>>>,
    /// Attestation socket path (Unix socket bridged to vsock 4091)
    attest_socket_path: PathBuf,
    /// Whether TEE has been verified via RA-TLS
    verified: AtomicBool,
    /// Attestation policy derived from config
    policy: AttestationPolicy,
}

impl std::fmt::Debug for TeeOrchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeeOrchestrator")
            .field("config", &self.config)
            .field("attest_socket_path", &self.attest_socket_path)
            .field("verified", &self.verified.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl TeeOrchestrator {
    /// Create a new TEE orchestrator from configuration.
    pub fn new(config: TeeConfig) -> Self {
        let socket_dir = config
            .socket_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("safeclaw"));

        let attest_socket_path = socket_dir.join("attest.sock");

        let policy = build_attestation_policy(&config);

        Self {
            config,
            vm: RwLock::new(None),
            attest_socket_path,
            verified: AtomicBool::new(false),
            policy,
        }
    }

    /// Boot the A3S Box MicroVM.
    ///
    /// Builds an `InstanceSpec` from config, spawns the shim subprocess via
    /// `VmController`, and waits for the attestation socket to appear.
    pub async fn boot(&self) -> Result<()> {
        // Already booted?
        if self.vm.read().await.is_some() {
            return Ok(());
        }

        if !self.config.enabled {
            return Err(Error::Tee("TEE is not enabled".to_string()));
        }

        tracing::info!("Booting TEE MicroVM");

        // Resolve shim binary path
        let shim_path = match &self.config.shim_path {
            Some(p) => p.clone(),
            None => VmController::find_shim().map_err(|e| {
                Error::Tee(format!("Failed to find a3s-box-shim: {}", e))
            })?,
        };

        let controller = VmController::new(shim_path).map_err(|e| {
            Error::Tee(format!("Failed to create VmController: {}", e))
        })?;

        let spec = self.build_instance_spec()?;

        // Ensure socket directory exists
        if let Some(dir) = self.attest_socket_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| {
                Error::Tee(format!(
                    "Failed to create socket directory {}: {}",
                    dir.display(),
                    e
                ))
            })?;
        }

        // Start the VM
        let handler = controller.start(&spec).await.map_err(|e| {
            Error::Tee(format!("Failed to start MicroVM: {}", e))
        })?;

        *self.vm.write().await = Some(handler);

        // Wait for attestation socket to appear
        self.wait_for_socket(&self.attest_socket_path, Duration::from_secs(30))
            .await?;

        tracing::info!(
            socket = %self.attest_socket_path.display(),
            "TEE MicroVM booted, attestation socket ready"
        );

        Ok(())
    }

    /// Verify TEE attestation via RA-TLS handshake.
    ///
    /// Connects to the guest's RA-TLS attestation server and verifies the
    /// SNP report embedded in the TLS certificate against the attestation policy.
    pub async fn verify(&self) -> Result<VerificationResult> {
        if self.verified.load(Ordering::Relaxed) {
            // Already verified — return a cached-equivalent result
            return Ok(VerificationResult {
                verified: true,
                platform: PlatformInfo::default(),
                policy_result: PolicyResult::pass(),
                signature_valid: true,
                cert_chain_valid: true,
                nonce_valid: true,
                failures: vec![],
            });
        }

        self.ensure_booted().await?;

        tracing::info!("Verifying TEE attestation via RA-TLS");

        let client = RaTlsAttestationClient::new(&self.attest_socket_path);
        let result = client
            .verify(self.policy.clone(), self.config.allow_simulated)
            .await
            .map_err(|e| Error::Tee(format!("RA-TLS attestation failed: {}", e)))?;

        if !result.verified {
            return Err(Error::Tee(format!(
                "TEE attestation verification failed: {:?}",
                result.failures
            )));
        }

        if !result.policy_result.passed {
            tracing::warn!(
                violations = ?result.policy_result.violations,
                "TEE attestation policy violations"
            );
        }

        tracing::info!("TEE attestation verified successfully");

        self.verified.store(true, Ordering::Relaxed);
        Ok(result)
    }

    /// Inject secrets into the verified TEE.
    ///
    /// Reads secret values from environment variables (as specified in `SecretRef`)
    /// and injects them into the TEE via RA-TLS. The guest stores them in
    /// `/run/secrets/` and optionally sets them as environment variables.
    pub async fn inject_secrets(&self, secret_refs: &[SecretRef]) -> Result<usize> {
        self.ensure_verified().await?;

        if secret_refs.is_empty() {
            return Ok(0);
        }

        let entries: Vec<SecretEntry> = secret_refs
            .iter()
            .filter_map(|r| {
                match std::env::var(&r.env_var) {
                    Ok(value) => Some(SecretEntry {
                        name: r.name.clone(),
                        value,
                        set_env: r.set_env,
                    }),
                    Err(_) => {
                        tracing::warn!(
                            env_var = %r.env_var,
                            name = %r.name,
                            "Secret env var not set, skipping"
                        );
                        None
                    }
                }
            })
            .collect();

        if entries.is_empty() {
            tracing::warn!("No secrets resolved from environment, nothing to inject");
            return Ok(0);
        }

        tracing::info!(count = entries.len(), "Injecting secrets into TEE");

        let injector = SecretInjector::new(&self.attest_socket_path);
        let result = injector
            .inject(&entries, self.policy.clone(), self.config.allow_simulated)
            .await
            .map_err(|e| Error::Tee(format!("Secret injection failed: {}", e)))?;

        if !result.errors.is_empty() {
            tracing::warn!(
                errors = ?result.errors,
                "Some secrets failed to inject"
            );
        }

        tracing::info!(injected = result.injected, "Secrets injected into TEE");
        Ok(result.injected)
    }

    /// Seal data bound to the TEE identity.
    ///
    /// Encrypts data using a key derived from the TEE's identity (measurement,
    /// host data). The sealed blob can only be unsealed by the same TEE.
    pub async fn seal(&self, data: &[u8], context: &str) -> Result<SealResult> {
        self.ensure_verified().await?;

        let client = SealClient::new(&self.attest_socket_path);
        client
            .seal(
                data,
                context,
                "measurement",
                self.policy.clone(),
                self.config.allow_simulated,
            )
            .await
            .map_err(|e| Error::Tee(format!("Seal failed: {}", e)))
    }

    /// Unseal data previously sealed by this TEE.
    pub async fn unseal(&self, blob: &str, context: &str) -> Result<Vec<u8>> {
        self.ensure_verified().await?;

        let client = SealClient::new(&self.attest_socket_path);
        client
            .unseal(
                blob,
                context,
                "measurement",
                self.policy.clone(),
                self.config.allow_simulated,
            )
            .await
            .map_err(|e| Error::Tee(format!("Unseal failed: {}", e)))
    }

    /// Get an RA-TLS channel for direct communication with the TEE guest.
    ///
    /// The channel can be used for `POST /process` requests and status checks.
    /// Requires the TEE to be verified first.
    pub fn channel(&self) -> crate::tee::channel::RaTlsChannel {
        crate::tee::channel::RaTlsChannel::new(
            &self.attest_socket_path,
            self.policy.clone(),
            self.config.allow_simulated,
        )
    }

    /// Process a message through the TEE-resident agent.
    ///
    /// Convenience method that creates a channel and sends a process request.
    /// Requires the TEE to be booted and verified.
    pub async fn process_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> Result<crate::tee::channel::ProcessResponse> {
        self.ensure_verified().await?;
        self.channel().process(session_id, content, "process_message").await
    }

    /// Check if the TEE is ready (VM booted + attestation verified).
    pub async fn is_ready(&self) -> bool {
        self.vm.read().await.is_some() && self.verified.load(Ordering::Relaxed)
    }

    /// Check if the VM is booted (but not necessarily verified).
    pub async fn is_booted(&self) -> bool {
        self.vm.read().await.is_some()
    }

    /// Get the attestation socket path.
    pub fn attest_socket_path(&self) -> &PathBuf {
        &self.attest_socket_path
    }

    /// Get the attestation policy.
    pub fn policy(&self) -> &AttestationPolicy {
        &self.policy
    }

    /// Whether simulated mode is allowed.
    pub fn allow_simulated(&self) -> bool {
        self.config.allow_simulated
    }

    /// Shutdown the TEE MicroVM.
    ///
    /// Stops the VM process gracefully (SIGTERM → wait → SIGKILL).
    pub async fn shutdown(&self) -> Result<()> {
        let mut vm_guard = self.vm.write().await;
        if let Some(ref mut handler) = *vm_guard {
            tracing::info!("Shutting down TEE MicroVM");
            handler
                .stop(a3s_box_runtime::vmm::DEFAULT_SHUTDOWN_TIMEOUT_MS)
                .map_err(|e| Error::Tee(format!("Failed to stop MicroVM: {}", e)))?;
        }
        *vm_guard = None;
        self.verified.store(false, Ordering::Relaxed);

        // Clean up socket file
        if self.attest_socket_path.exists() {
            let _ = std::fs::remove_file(&self.attest_socket_path);
        }

        tracing::info!("TEE MicroVM shutdown complete");
        Ok(())
    }

    // ---- Private helpers ----

    /// Build an `InstanceSpec` from the SafeClaw `TeeConfig`.
    fn build_instance_spec(&self) -> Result<InstanceSpec> {
        let socket_dir = self
            .config
            .socket_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("safeclaw"));

        let box_id = format!("safeclaw-tee-{}", uuid::Uuid::new_v4());

        let mut spec = InstanceSpec {
            box_id,
            vcpus: self.config.cpu_cores as u8,
            memory_mib: self.config.memory_mb,
            grpc_socket_path: socket_dir.join("grpc.sock"),
            exec_socket_path: socket_dir.join("exec.sock"),
            pty_socket_path: socket_dir.join("pty.sock"),
            attest_socket_path: self.attest_socket_path.clone(),
            ..Default::default()
        };

        // Add workspace mount if configured
        if let Some(ref workspace) = self.config.workspace_dir {
            spec.fs_mounts.push(a3s_box_runtime::vmm::FsMount {
                tag: "workspace".to_string(),
                host_path: workspace.clone(),
                read_only: false,
            });
        }

        Ok(spec)
    }

    /// Wait for a Unix socket file to appear on disk.
    async fn wait_for_socket(&self, path: &PathBuf, timeout: Duration) -> Result<()> {
        let start = tokio::time::Instant::now();
        let poll_interval = Duration::from_millis(100);

        loop {
            if path.exists() {
                return Ok(());
            }

            // Check if VM is still running
            let vm_guard = self.vm.read().await;
            if let Some(ref handler) = *vm_guard {
                if !handler.is_running() {
                    return Err(Error::Tee(
                        "MicroVM process exited before attestation socket appeared".to_string(),
                    ));
                }
            }
            drop(vm_guard);

            if start.elapsed() > timeout {
                return Err(Error::Tee(format!(
                    "Timed out waiting for attestation socket at {} ({}s)",
                    path.display(),
                    timeout.as_secs()
                )));
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Ensure the VM is booted, returning an error if not.
    async fn ensure_booted(&self) -> Result<()> {
        if self.vm.read().await.is_none() {
            return Err(Error::Tee(
                "TEE MicroVM is not booted — call boot() first".to_string(),
            ));
        }
        Ok(())
    }

    /// Ensure the TEE is verified, returning an error if not.
    async fn ensure_verified(&self) -> Result<()> {
        self.ensure_booted().await?;
        if !self.verified.load(Ordering::Relaxed) {
            return Err(Error::Tee(
                "TEE is not verified — call verify() first".to_string(),
            ));
        }
        Ok(())
    }
}

/// Build an `AttestationPolicy` from SafeClaw's `TeeConfig`.
fn build_attestation_policy(config: &TeeConfig) -> AttestationPolicy {
    let mut policy = AttestationPolicy::default();

    // Map expected measurements from config
    if let Some(measurement) = config.attestation.expected_measurements.get("launch") {
        policy.expected_measurement = Some(measurement.clone());
    }

    // In development mode, relax debug requirement
    if config.allow_simulated {
        policy.require_no_debug = false;
    }

    policy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let config = TeeConfig::default();
        let orch = TeeOrchestrator::new(config);

        assert!(!orch.verified.load(Ordering::Relaxed));
        assert!(orch.attest_socket_path.to_string_lossy().contains("safeclaw"));
        assert!(orch.attest_socket_path.to_string_lossy().ends_with("attest.sock"));
    }

    #[test]
    fn test_orchestrator_custom_socket_dir() {
        let config = TeeConfig {
            socket_dir: Some(PathBuf::from("/tmp/my-tee")),
            ..Default::default()
        };
        let orch = TeeOrchestrator::new(config);

        assert_eq!(
            orch.attest_socket_path,
            PathBuf::from("/tmp/my-tee/attest.sock")
        );
    }

    #[test]
    fn test_build_attestation_policy_default() {
        let config = TeeConfig::default();
        let policy = build_attestation_policy(&config);

        assert!(policy.require_no_debug);
        assert!(policy.expected_measurement.is_none());
    }

    #[test]
    fn test_build_attestation_policy_simulated() {
        let config = TeeConfig {
            allow_simulated: true,
            ..Default::default()
        };
        let policy = build_attestation_policy(&config);

        // Simulated mode relaxes debug requirement
        assert!(!policy.require_no_debug);
    }

    #[test]
    fn test_build_attestation_policy_with_measurement() {
        let mut config = TeeConfig::default();
        config
            .attestation
            .expected_measurements
            .insert("launch".to_string(), "ab".repeat(48));

        let policy = build_attestation_policy(&config);
        assert_eq!(policy.expected_measurement, Some("ab".repeat(48)));
    }

    #[test]
    fn test_build_instance_spec() {
        let config = TeeConfig {
            cpu_cores: 4,
            memory_mb: 4096,
            workspace_dir: Some(PathBuf::from("/home/user/project")),
            socket_dir: Some(PathBuf::from("/tmp/tee-test")),
            ..Default::default()
        };
        let orch = TeeOrchestrator::new(config);
        let spec = orch.build_instance_spec().unwrap();

        assert_eq!(spec.vcpus, 4);
        assert_eq!(spec.memory_mib, 4096);
        assert!(spec.box_id.starts_with("safeclaw-tee-"));
        assert_eq!(spec.attest_socket_path, PathBuf::from("/tmp/tee-test/attest.sock"));
        assert_eq!(spec.grpc_socket_path, PathBuf::from("/tmp/tee-test/grpc.sock"));
        assert_eq!(spec.exec_socket_path, PathBuf::from("/tmp/tee-test/exec.sock"));
        assert_eq!(spec.pty_socket_path, PathBuf::from("/tmp/tee-test/pty.sock"));
        assert_eq!(spec.fs_mounts.len(), 1);
        assert_eq!(spec.fs_mounts[0].tag, "workspace");
        assert_eq!(
            spec.fs_mounts[0].host_path,
            PathBuf::from("/home/user/project")
        );
    }

    #[test]
    fn test_build_instance_spec_no_workspace() {
        let config = TeeConfig {
            workspace_dir: None,
            ..Default::default()
        };
        let orch = TeeOrchestrator::new(config);
        let spec = orch.build_instance_spec().unwrap();

        assert!(spec.fs_mounts.is_empty());
    }

    #[tokio::test]
    async fn test_orchestrator_not_booted() {
        let config = TeeConfig::default();
        let orch = TeeOrchestrator::new(config);

        assert!(!orch.is_booted().await);
        assert!(!orch.is_ready().await);
    }

    #[tokio::test]
    async fn test_ensure_booted_fails_when_not_booted() {
        let config = TeeConfig::default();
        let orch = TeeOrchestrator::new(config);

        let result = orch.ensure_booted().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not booted"));
    }

    #[tokio::test]
    async fn test_ensure_verified_fails_when_not_booted() {
        let config = TeeConfig::default();
        let orch = TeeOrchestrator::new(config);

        let result = orch.ensure_verified().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_boot_disabled_tee() {
        let config = TeeConfig {
            enabled: false,
            ..Default::default()
        };
        let orch = TeeOrchestrator::new(config);

        let result = orch.boot().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not enabled"));
    }

    #[tokio::test]
    async fn test_shutdown_when_not_booted() {
        let config = TeeConfig::default();
        let orch = TeeOrchestrator::new(config);

        // Shutdown on a non-booted orchestrator should be a no-op
        let result = orch.shutdown().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_orchestrator_debug() {
        let config = TeeConfig::default();
        let orch = TeeOrchestrator::new(config);
        let debug = format!("{:?}", orch);
        assert!(debug.contains("TeeOrchestrator"));
        assert!(debug.contains("verified"));
    }

    #[tokio::test]
    async fn test_inject_secrets_empty() {
        let config = TeeConfig::default();
        let orch = TeeOrchestrator::new(config);

        // Should fail because not verified
        let result = orch.inject_secrets(&[]).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_secret_ref_resolution() {
        // Test that SecretRef correctly maps env vars
        let secret = SecretRef {
            name: "api_key".to_string(),
            env_var: "TEST_SAFECLAW_API_KEY_NONEXISTENT".to_string(),
            set_env: true,
        };

        // Env var doesn't exist, so it should be filtered out
        let entries: Vec<SecretEntry> = [secret]
            .iter()
            .filter_map(|r| {
                std::env::var(&r.env_var).ok().map(|value| SecretEntry {
                    name: r.name.clone(),
                    value,
                    set_env: r.set_env,
                })
            })
            .collect();

        assert!(entries.is_empty());
    }
}
